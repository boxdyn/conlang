//! Performs pattern analysis on an AST
#![allow(unused)]

use std::collections::{BTreeMap, BTreeSet};

use cl_ast::{
    fold::{Foldable, impl_fold},
    types::Symbol,
    visit::Walk,
    *,
};
use cl_parser::pat;
use cl_structures::span::Span;

/*

# What are patterns for, exactly?

Conlang's patterns
1. Validate the contents of a value (range patterns, ...)
2. Generate named accessors for components of a value (binding wildcards)
3. Provide type information for those named accessors, and for the
   values being matched against.


Why is this important?
1. It provides a unified language for talking about values and types



# Performing Name Resolution

Goal:
- Use a data structure like the Table to store name lookup info
- Remove all the bound variables from the AST in one pass, and record
  their value structure in the table
- Don't worry about place/value distinction at this point

Passes:
1. Scope identifier assignment
    i.  Go over the AST, and assign each node an ID corresponding to
        the scope it falls in.
    ii. Simultaneously, assign each scoped node a scope identifier, and
        maintain a stack of such identifiers in a way that models the
        language's semantics (i.e. `let` lasting until local-scope end.)
        - This could be done by keeping track of what BindOp the current
        scope was created with, and popping scopes when it's appropriate
        to do so.
        - It could also be done by inverting the relationship between
        `let` and the remaining subexpressions of `do` in the current
        block, but that would break `if let` binding.
2. Scoped name binding
    i.  Using the indices assigned in pass (1), bind each name in the
        local-scope or module-scope it's found in.
    ii. Keep a separate disjoint set forest
*/

type NameIndex = usize;
type ScopeIndex = usize;
type Map<T, U> = BTreeMap<T, U>;
type Set<U> = BTreeSet<U>;
type Path = Vec<Symbol>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum ScopeKind {
    /// An unordered outer scope, or the direct child of such scope.
    #[default]
    Outer,
    /// An ordered inner scope which is explicitly opened and closed.
    ///
    /// These are created in the bodies of [Bind] expressions,
    /// except for [modules][BindOp::Mod].
    Inner,
    /// An ephemeral local scope, which is enclosed by an Inner scope
    Inherited,
}

/// Tracks the information necessary to resolve a name
#[derive(Debug, Default)]
pub struct Scopes {
    scopes: Vec<Scope>,
    names: Vec<Symbol>,
}

impl Scopes {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn root_scope(&mut self) -> ScopeIndex {
        if self.scopes.is_empty() {
            self.add_scope(0, ScopeKind::Outer, "root")
        } else {
            0
        }
    }

    pub fn add_scope(
        &mut self,
        parent: ScopeIndex,
        kind: ScopeKind,
        from: &'static str,
    ) -> ScopeIndex {
        let new = self.scopes.len();
        self.scopes.push(Scope::new(parent, kind, from));
        if parent != new {
            self.scopes[parent].children.push(new);
        }
        new
    }
}

impl std::fmt::Display for Scopes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use std::fmt::Write;

        fn pretty(f: &mut dyn Write, scopes: &[Scope], at: usize) -> std::fmt::Result {
            use cl_ast::fmt::FmtAdapter;

            let Scope { kind, from, parent, children, bindings, .. } = &scopes[at];
            write!(f, "{at}: {kind:?} ({from})")?;
            for (name, index) in bindings {
                let indent = if children.is_empty() { "    " } else { "│   " };
                write!(&mut f.indent_with(indent), "\n{name}: {index}")?;
            }
            let [children @ .., last] = children.as_slice() else {
                return Ok(());
            };
            for child in children {
                write!(f, "\n├───")?;
                pretty(&mut f.indent_with("│   "), scopes, *child)?;
            }
            write!(f, "\n╰───")?;
            pretty(&mut f.indent_with("    "), scopes, *last)
        }
        pretty(f, &self.scopes, 0)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Scope {
    /// The kind of scope this is
    kind: ScopeKind,
    from: &'static str,
    /// The index of this scope's parent
    parent: ScopeIndex,
    /// The indices of this scope's children
    children: Vec<ScopeIndex>,
    // /// The bound symbols in this scope, if applicable
    bindings: Map<Symbol, NameIndex>,
    // /// The lazy imports in this scope, if applicable
    imports: Map<Symbol, Path>,
    // /// the glob-imports in this scope, if applicable
    globs: Set<Path>,
}

impl Scope {
    pub fn new(parent: ScopeIndex, kind: ScopeKind, from: &'static str) -> Self {
        Self { parent, kind, from, ..Default::default() }
    }

    /// Whether this scope is the [ScopeKind]
    pub const fn is(&self, kind: ScopeKind) -> bool {
        self.kind as i32 == kind as i32
    }
}

/// Associates each AST node with its surrounding scope
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScopedSpan {
    span: Span,
    scope: ScopeIndex,
}
impl std::fmt::Display for ScopedSpan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.span.fmt(f)?;
        write!(f, " @ {}", self.scope)
    }
}

/// AST with added scope information
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ScopedAst;
impl AstTypes for ScopedAst {
    type Annotation = ScopedSpan;
    type Literal = types::Literal;
    type MacroId = types::Symbol;
    type Symbol = types::Symbol;
    type Path = types::Path;
}

mod scoper {
    //! Calculates Conlang's scoping rules
    use super::*;

    /// Calculates the language's scoping rules
    /// based on a recursive traversal
    #[derive(Debug)]
    pub struct Scoper<'t> {
        table: &'t mut Scopes,
        stack: Vec<ScopeIndex>,
    }

    /**
     * [Scopes] and Names
     *
     * Properties of [Scope]:
     * - contains names
     * - encloses [Expr]essions and [Pat]terns
     * - encloses other [Scopes]
     *
     * Properties of Name:
     * - can be const, static, or local
     * - can be pub or priv
     * - can be mut or not
     */
    impl<'t> Scoper<'t> {
        /// Constructs a new Sniper
        pub fn new(table: &'t mut Scopes) -> Self {
            Self { stack: vec![table.root_scope()], table }
        }

        pub fn scope_index(&self) -> ScopeIndex {
            match self.stack[..] {
                [] => panic!("Should not exit last scope!"),
                [.., last] => last,
            }
        }

        pub fn get(&self) -> &Scope {
            let scope = self.scope_index();
            &self.table.scopes[scope]
        }

        pub fn get_mut(&mut self) -> &mut Scope {
            let scope = self.scope_index();
            &mut self.table.scopes[scope]
        }

        /// Enters a new scope
        pub fn enter(&mut self, scope_kind: ScopeKind, from: &'static str) -> &mut Self {
            let parent = self.scope_index();
            let idx = self.table.add_scope(parent, scope_kind, from);
            self.stack.push(idx);
            self
        }

        /// Exits the closest non-[`let`] [Scope].
        ///
        /// [Inherited][`let`] scopes, which bind [`let`]-bound
        /// variables, cannot be individually exited, and will
        /// accumulate until the end of their surrounding scope.
        ///
        /// [`let`]: ScopeKind::Inherited
        pub fn exit(&mut self) -> &mut Self {
            use ScopeKind::*;
            while self.table.scopes[self.stack.pop().expect("enclosing scope")].is(Inherited) {}
            self
        }

        /// Binds a symbol in the current scope
        ///
        /// Returns whether the symbol is unique in the scope
        pub fn bind(&mut self, name: Symbol) -> bool {
            let scope = self.scope_index();
            let Scopes { scopes, names } = self.table;
            if scopes[scope].bindings.contains_key(&name) {
                return false;
            }
            let bound = names.len();
            names.push(name);
            scopes[scope].bindings.insert(name, bound).is_some()
        }

        /// Imports a path in the current scope
        ///
        /// Returns the existing import, if one already existed
        pub fn import(&mut self, name: Symbol, path: Path) -> Option<Path> {
            self.get_mut().imports.insert(name, path)
        }

        /// Glob-imports a path in the current scope
        ///
        /// Returns whether the import is unique in the scope
        pub fn glob(&mut self, path: Path) -> bool {
            self.get_mut().globs.insert(path)
        }

        /// Opens a new "block scope" with the given [ScopeKind]
        pub fn block<R>(
            &mut self,
            kind: ScopeKind,
            from: &'static str,
            f: impl FnOnce(&mut Self) -> R,
        ) -> R {
            let out = f(self.enter(kind, from));
            if kind != ScopeKind::Inherited {
                self.exit();
            }
            out
        }
    }

    impl<'t> fold::Fold<DefaultTypes, ScopedAst> for Scoper<'t> {
        type Error = ();

        impl_fold! {
            in Fold<DefaultTypes, ScopedAst>
            fn fold_annotation(self, span: Annotation) =
                ScopedSpan { span, scope: self.scope_index() };
            fn fold_macro_id(self, from: MacroId) = from;
            fn fold_symbol(self, from: Symbol) = from;
            fn fold_path(self, from: Path) = from;
            fn fold_literal(self, from: Literal) = from;
        }

        fn fold_at_expr(
            &mut self,
            expr: At<Expr<DefaultTypes>>,
        ) -> Result<At<Expr<ScopedAst>, ScopedAst>, ()> {
            let At(expr, span) = expr;
            let span = self.fold_annotation(span)?;
            let expr = self.fold(expr)?;
            Ok(At(expr, span))
        }

        fn fold_expr(&mut self, expr: Expr<DefaultTypes>) -> Result<Expr<ScopedAst>, Self::Error> {
            match expr {
                Expr::Op(Op::Block, exprs) if !self.get().is(ScopeKind::Outer) => {
                    self.block(ScopeKind::Inner, "block", |block| {
                        Ok(Expr::Op(Op::Block, block.fold(exprs)?))
                    })
                }
                Expr::Op(op @ (Op::Loop | Op::Defer | Op::Break | Op::Return), exprs) => self
                    .block(ScopeKind::Inner, "control-flow", |block| {
                        Ok(Expr::Op(op, block.fold(exprs)?))
                    }),
                Expr::Op(op @ (Op::If | Op::While), mut exprs) => {
                    assert_eq!(exprs.len(), 3);
                    let [cond, pass, fail] = exprs.into_chunks().pop().ok_or(())?;
                    let (cond, pass) = self.block(ScopeKind::Inner, "if-while", |block| {
                        let cond = block.fold(cond)?;
                        let pass = block.fold(pass)?;
                        Ok((cond, pass))
                    })?;
                    let fail = self.block(ScopeKind::Inner, "else", |block| {
                        let fail = block.fold(fail)?;
                        Ok(fail)
                    })?;
                    Ok(Expr::Op(op, vec![cond, pass, fail]))
                }
                other => other.children(self),
            }
        }

        fn fold_bind(&mut self, bind: Bind<DefaultTypes>) -> Result<Bind<ScopedAst>, Self::Error> {
            use ScopeKind::*;
            let Bind(op, gens, pat, mut exprs) = bind;
            match op {
                BindOp::Let => {
                    let exprs = exprs
                        .into_iter()
                        .map(|e| self.block(Inner, "let body", |block| block.fold(e)))
                        .collect::<Result<_, _>>()?;
                    let bind = |scope: &mut Scoper| {
                        let gens = scope.fold(gens)?;
                        let pat = scope.fold(pat)?;
                        Ok(Bind(BindOp::Let, gens, pat, exprs))
                    };
                    // if outside body, bind in scope
                    match self.get().kind {
                        Outer => bind(self),
                        _ => self.block(Inherited, "let", bind),
                    }
                }
                // TODO: bind function names outside
                BindOp::Fn => self.block(Outer, "fn", |item| {
                    let gens = item.fold(gens)?;
                    let pat = item.fold(pat)?;
                    let exprs = item.block(Inner, "fn body", |body| body.fold(exprs))?;
                    Ok(Bind(op, gens, pat, exprs))
                }),
                BindOp::Mod => Ok(Bind(
                    BindOp::Mod,
                    self.fold(gens)?,
                    self.fold(pat)?,
                    self.block(Outer, "mod", |block| block.fold(exprs))?,
                )),
                BindOp::Type | BindOp::Struct | BindOp::Enum => {
                    self.block(Outer, "type", |block| {
                        let gens = block.fold(gens)?;
                        let pat = block.fold(pat)?;
                        let exprs = block.block(Inner, "type body", |block| block.fold(exprs))?;
                        Ok(Bind(BindOp::Mod, gens, pat, exprs))
                    })
                }
                BindOp::Impl => todo!("Scope `impl`"),
                BindOp::For => {
                    assert_eq!(exprs.len(), 3);
                    let [cond, pass, fail] = exprs.into_chunks().pop().ok_or(())?;
                    let cond = self.block(Inner, "iter", |block| block.fold(cond))?;
                    let fail = self.block(Inner, "fail", |block| block.fold(fail))?;
                    let (gens, pat, pass) = self.block(Inner, "pass", |block| {
                        Ok((block.fold(gens)?, block.fold(pat)?, block.fold(pass)?))
                    })?;
                    let exprs = vec![cond, pass, fail];
                    Ok(Bind(BindOp::For, gens, pat, exprs))
                }
            }
        }

        fn fold_pat(&mut self, pat: Pat<DefaultTypes>) -> Result<Pat<ScopedAst>, Self::Error> {
            use ScopeKind::*;
            // FIXME: This doesn't fully encapsulate scoping semantics
            match pat {
                Pat::Name(name) => {
                    self.bind(name);
                    Ok(Pat::Name(name))
                }
                Pat::Op(PatOp::TypePrefixed, mut ats) if self.get().is(Outer) => {
                    assert_eq!(ats.len(), 2);
                    let [pfx, ty] = ats.into_chunks().pop().expect("Gee, bill!");
                    // let ty = ats.pop().expect("TypePrefixed should contain 2 patterns");
                    // let pfx = ats.pop().expect("TypePrefixed should contain 2 patterns");
                    let pfx = pfx.fold_in(self)?;
                    let ty = self.block(Inherited, "args", |block| ty.fold_in(block))?;
                    Ok(Pat::Op(PatOp::TypePrefixed, vec![pfx, ty]))
                }
                _ => pat.children(self),
            }
        }

        fn fold_make(&mut self, make: Make<DefaultTypes>) -> Result<Make<ScopedAst>, Self::Error> {
            self.block(ScopeKind::Inner, "make", |block| make.children(block))
        }

        fn fold_match(
            &mut self,
            mtch: Match<DefaultTypes>,
        ) -> Result<Match<ScopedAst>, Self::Error> {
            // The scrutinee and arms of a match expression exist in a shared scope
            self.block(ScopeKind::Inner, "match", |block| mtch.children(block))
        }

        fn fold_matcharm(
            &mut self,
            arm: MatchArm<DefaultTypes>,
        ) -> Result<MatchArm<ScopedAst>, Self::Error> {
            // Each match arm is wrapped in an implicit {block},
            self.block(ScopeKind::Inner, "match arm", |block| arm.children(block))
        }
    }
}
pub use scoper::Scoper;

/// AST with name-binding information stripped out
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct BoundAst;
impl AstTypes for BoundAst {
    type Annotation = ScopedSpan;
    type Literal = types::Literal;
    type MacroId = types::Symbol;
    type Symbol = usize; // index in symbol table
    type Path = types::Path;
}

// TODO: fold ScopedAst => BoundAst

/// AST with name-binding and name-usage information stripped out
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ResolvedAst;
impl AstTypes for ResolvedAst {
    type Annotation = ScopedSpan;
    type Literal = types::Literal;
    type MacroId = types::Symbol;
    type Symbol = usize; // index in symbol table
    type Path = usize; // index in symbol table
}

// TODO: fold BoundAst => ResolvedAst

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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum ScopeKind {
    /// An unordered global scope, or the direct child of such scope
    Module,
    /// An ordered local scope which denotes a
    Body,
    /// An ephemeral local scope, which is enclosed by a Body
    Let,
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
            self.add_scope(0, ScopeKind::Module)
        } else {
            0
        }
    }

    pub fn add_scope(&mut self, parent: ScopeIndex, kind: ScopeKind) -> ScopeIndex {
        let new = self.scopes.len();
        self.scopes.push(Scope::new(parent, kind));
        println!("Entered scope {new}: {:?}", self.scopes.last().unwrap());
        if parent != new {
            self.scopes[parent].children.push(new);
        }
        new
    }
}

#[derive(Clone, Debug)]
pub struct Scope {
    /// The kind of scope this is
    kind: ScopeKind,
    /// The index of this scope's parent
    parent: ScopeIndex,
    /// The indices of this scope's children
    children: Vec<ScopeIndex>,
    // /// The bound symbols in this scope, if applicable
    // bindings: Map<Symbol, NameIndex>,
    // /// The lazy imports in this scope, if applicable
    // imports: Map<Symbol, Path>,
    // /// the glob-imports in this scope, if applicable
    // globs: Set<Path>,
}

impl Scope {
    pub fn new(parent: ScopeIndex, kind: ScopeKind) -> Self {
        Self {
            parent,
            kind,
            children: Default::default(),
            // bindings: Default::default(),
            // imports: Default::default(),
            // globs: Default::default(),
        }
    }

    /// Whether this scope is [ScopeKind::Module].
    pub fn is_mod(&self) -> bool {
        self.kind == ScopeKind::Module
    }

    /// Whether this scope is [ScopeKind::Body].
    pub fn is_body(&self) -> bool {
        self.kind == ScopeKind::Body
    }

    /// Whether this scope is [ScopeKind::Let], and can be discarded.
    pub fn is_let(&self) -> bool {
        self.kind == ScopeKind::Let
    }
}

/// Transforms an AST from one which binds variables
/// to one which binds numbers, tracking which scope
/// each expression exists within
#[derive(Debug)]
pub struct Scoper<'t> {
    table: &'t mut Scopes,
    stack: Vec<ScopeIndex>,
}

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
    pub fn enter(&mut self, scope_kind: ScopeKind) -> &mut Self {
        let parent = self.scope_index();
        let idx = self.table.add_scope(parent, scope_kind);
        self.stack.push(idx);
        self
    }

    // /// Binds a symbol in the current scope
    // ///
    // /// Returns whether the import is unique in the scope
    // pub fn bind(&mut self, name: Symbol) -> bool {
    //     let scope = self.scope_index();
    //     let Scopes { scopes, names } = self.table;
    //     if scopes[scope].bindings.contains_key(&name) {
    //         return false;
    //     }
    //     let bound = names.len();
    //     names.push(name);
    //     scopes[scope].bindings.insert(name, bound).is_some()
    // }

    // /// Imports a path in the current scope
    // ///
    // /// Returns the existing import, if one already existed
    // pub fn import(&mut self, name: Symbol, path: Path) -> Option<Path> {
    //     self.get_mut().imports.insert(name, path)
    // }

    // /// Glob-imports a path in the current scope
    // ///
    // /// Returns whether the import is unique in the scope
    // pub fn glob(&mut self, path: Path) -> bool {
    //     self.get_mut().globs.insert(path)
    // }

    /// Exits the closest non-[`let`] [Scope].
    ///
    /// [Let][`let`] scopes, which bind [`let`]-bound
    /// variables, cannot be individually exited, and will
    /// accumulate until the end of their surrounding scope.
    ///
    /// [`let`]: ScopeKind::Let
    pub fn exit(&mut self) -> &mut Self {
        while self.get().is_let() {
            self.stack.pop();
        }
        self.stack.pop().expect("exited last scope!");
        self
    }

    pub fn block<R>(&mut self, scope_kind: ScopeKind, f: impl FnOnce(&mut Self) -> R) -> R {
        let out = f(self.enter(scope_kind));
        self.exit();
        out
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
        let expr = match expr {
            // Expr::Bind(_) => todo!(),
            Expr::Op(Op::Block, exprs) if self.get().kind != ScopeKind::Module => self
                .block(ScopeKind::Body, |block| {
                    Ok(Expr::Op(Op::Block, block.fold(exprs)?))
                }),
            // Expr::Op(Op::If,)
            // Expr::Op(op, ats) => todo!(),
            other => other.children(self),
        }?;
        Ok(At::<_, _>(expr, span))
    }

    fn fold_bind(&mut self, bind: Bind<DefaultTypes>) -> Result<Bind<ScopedAst>, Self::Error> {
        let Bind(op, gens, pat, exprs) = bind;
        if let BindOp::Let = op {
            let exprs = self.fold(exprs)?;
            self.enter(ScopeKind::Let);
            let pat = self.fold(pat)?;
            return Ok(Bind(BindOp::Let, gens, pat, exprs));
        }

        // let exprs = match exprs.into_array().map(|a| *a) {
        //     Ok([iter, pass, fail]) => todo!(),
        //     Err(exprs) => exprs.visit_in(self)?,
        // };

        // TODO: clean this up; maybe make Pat less Op-focused.
        let pat = if let At(Pat::Op(PatOp::TypePrefixed, mut ats), span) = pat {
            assert_eq!(ats.len(), 2);
            let ty = self.block(ScopeKind::Module, |block| {
                ats.pop()
                    .expect("TypePrefixed should contain 2 patterns")
                    .fold_in(block)
            })?;
            let pfx = (ats.pop())
                .expect("TypePrefixed should contain 2 patterns")
                .fold_in(self)?;
            At(
                Pat::Op(PatOp::TypePrefixed, vec![pfx, ty]),
                self.fold_annotation(span)?,
            )
        } else {
            pat.fold_in(self)?
        };
        let exprs = if !exprs.is_empty() {
            self.block(
                match op {
                    BindOp::For | BindOp::Fn => ScopeKind::Body,
                    _ => ScopeKind::Module,
                },
                |block| exprs.fold_in(block),
            )?
        } else {
            vec![]
        };
        Ok(Bind(op, gens, pat, exprs))
    }

    fn fold_pat(&mut self, pat: Pat<DefaultTypes>) -> Result<Pat<ScopedAst>, Self::Error> {
        // FIXME: only works at module scope. Need another way to track this.
        match pat {
            Pat::Op(PatOp::TypePrefixed, mut ats) if !self.get().is_let() => {
                println!("{ats:?}");
                assert_eq!(ats.len(), 2);
                let ty = self.block(ScopeKind::Module, |block| {
                    ats.pop()
                        .expect("TypePrefixed should contain 2 patterns")
                        .fold_in(block)
                })?;
                let pfx = (ats.pop())
                    .expect("TypePrefixed should contain 2 patterns")
                    .fold_in(self)?;
                Ok(Pat::Op(PatOp::TypePrefixed, vec![pfx, ty]))
            }
            _ => pat.children(self),
        }
    }

    fn fold_make(&mut self, make: Make<DefaultTypes>) -> Result<Make<ScopedAst>, Self::Error> {
        self.block(ScopeKind::Body, |block| make.children(block))
    }

    fn fold_match(&mut self, mtch: Match<DefaultTypes>) -> Result<Match<ScopedAst>, Self::Error> {
        self.block(ScopeKind::Body, |block| mtch.children(block))
    }

    fn fold_matcharm(
        &mut self,
        arm: MatchArm<DefaultTypes>,
    ) -> Result<MatchArm<ScopedAst>, Self::Error> {
        // Each match arm is wrapped in an implicit {block},
        self.block(ScopeKind::Body, |outer| {
            // but binds with an implicit `let`, since that makes `fold_pat` work nicer
            arm.children(outer.enter(ScopeKind::Let))
        })
    }
}

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

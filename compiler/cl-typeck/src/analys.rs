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

mod scoper;
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

//! A [Node] contains the [NodeSource] and [Item] metadata for any
//! [Def](crate::definition::Def), as well as the [Path] of the
//! containing [Module].
//!
//! [Node]s are collected by the [Node Sorcerer](sorcerer),
//! an AST visitor that pairs [NodeSource]s with their surrounding
//! context ([Path], [struct@Span], [Meta], [Visibility])

use cl_ast::ast::*;
use cl_structures::span::Span;
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Node<'a> {
    pub in_path: Path,
    pub span: &'a Span,
    pub meta: &'a [Meta],
    pub vis: Visibility,
    pub kind: Option<NodeSource<'a>>,
}

impl<'a> Node<'a> {
    pub fn new(path: Path, kind: Option<NodeSource<'a>>) -> Self {
        const DUMMY_SPAN: Span = Span::dummy();
        Self { in_path: path, span: &DUMMY_SPAN, meta: &[], vis: Visibility::Public, kind }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NodeSource<'a> {
    Root,
    Module(&'a Module),
    Alias(&'a Alias),
    Enum(&'a Enum),
    Variant(&'a Variant),
    Struct(&'a Struct),
    Const(&'a Const),
    Static(&'a Static),
    Function(&'a Function),
    Local(&'a Let),
    Impl(&'a Impl),
    Use(&'a Use),
    Ty(&'a TyKind),
}

impl<'a> NodeSource<'a> {
    pub fn name(&self) -> Option<Sym> {
        match self {
            NodeSource::Root => None,
            NodeSource::Module(v) => Some(v.name.0),
            NodeSource::Alias(v) => Some(v.to.0),
            NodeSource::Enum(v) => Some(v.name.0),
            NodeSource::Variant(v) => Some(v.name.0),
            NodeSource::Struct(v) => Some(v.name.0),
            NodeSource::Const(v) => Some(v.name.0),
            NodeSource::Static(v) => Some(v.name.0),
            NodeSource::Function(v) => Some(v.name.0),
            NodeSource::Local(l) => Some(l.name.0),
            NodeSource::Impl(_) | NodeSource::Use(_) | NodeSource::Ty(_) => None,
        }
    }

    /// Returns `true` if this [NodeSource] defines a named value
    pub fn is_named_value(&self) -> bool {
        matches!(self, Self::Const(_) | Self::Static(_) | Self::Function(_))
    }

    /// Returns `true` if this [NodeSource] defines a named type
    pub fn is_named_type(&self) -> bool {
        matches!(
            self,
            Self::Module(_) | Self::Alias(_) | Self::Enum(_) | Self::Struct(_)
        )
    }

    /// Returns `true` if this [NodeSource] refers to a [Ty] with no name
    pub fn is_anon_type(&self) -> bool {
        matches!(self, Self::Ty(_))
    }

    /// Returns `true` if this [NodeSource] refers to an [Impl] block
    pub fn is_impl(&self) -> bool {
        matches!(self, Self::Impl(_))
    }

    /// Returns `true` if this [NodeSource] refers to a [Use] import
    pub fn is_use_import(&self) -> bool {
        matches!(self, Self::Use(_))
    }
}

impl fmt::Display for NodeSource<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Root => "🌳 root 🌳".fmt(f),
            Self::Module(arg0) => arg0.fmt(f),
            Self::Alias(arg0) => arg0.fmt(f),
            Self::Enum(arg0) => arg0.fmt(f),
            Self::Variant(arg0) => arg0.fmt(f),
            Self::Struct(arg0) => arg0.fmt(f),
            Self::Const(arg0) => arg0.fmt(f),
            Self::Static(arg0) => arg0.fmt(f),
            Self::Function(arg0) => arg0.fmt(f),
            Self::Impl(arg0) => arg0.fmt(f),
            Self::Use(arg0) => arg0.fmt(f),
            Self::Ty(arg0) => arg0.fmt(f),
            Self::Local(arg0) => arg0.fmt(f),
        }
    }
}

pub mod sorcerer {
    //! An [AST](cl_ast) analysis pass that collects [Node] entries.

    use super::{Node, NodeSource};
    use cl_ast::{ast::*, ast_visitor::visit::*};
    use cl_structures::span::Span;
    use std::mem;

    /// An AST analysis pass that collects [Node]s
    #[derive(Clone, Debug)]
    pub struct NodeSorcerer<'a> {
        path: Path,
        parts: Parts<'a>,
        defs: Vec<Node<'a>>,
    }

    type Parts<'a> = (&'a Span, &'a [Meta], Visibility);

    impl<'a> NodeSorcerer<'a> {
        pub fn into_defs(self) -> Vec<Node<'a>> {
            self.defs
        }

        fn with_parts<F>(&mut self, s: &'a Span, a: &'a [Meta], v: Visibility, f: F)
        where F: FnOnce(&mut Self) {
            let parts = mem::replace(&mut self.parts, (s, a, v));
            f(self);
            self.parts = parts;
        }

        fn with_only_span<F>(&mut self, span: &'a Span, f: F)
        where F: FnOnce(&mut Self) {
            self.with_parts(span, &[], Visibility::Public, f)
        }

        fn push(&mut self, kind: NodeSource<'a>) {
            let Self { path, parts, defs } = self;
            let (span, meta, vis) = *parts;

            defs.push(Node { in_path: path.clone(), span, meta, vis, kind: Some(kind) })
        }
    }

    impl Default for NodeSorcerer<'_> {
        fn default() -> Self {
            const DPARTS: Parts = (&Span::dummy(), &[], Visibility::Private);
            Self {
                path: Path { absolute: true, ..Default::default() },
                parts: DPARTS,
                defs: Default::default(),
            }
        }
    }

    impl<'a> Visit<'a> for NodeSorcerer<'a> {
        fn visit_module(&mut self, m: &'a Module) {
            let Module { name, kind } = m;
            self.path.push(PathPart::Ident(name.clone()));
            self.visit_module_kind(kind);
            self.path.pop();
        }
        fn visit_item(&mut self, i: &'a Item) {
            let Item { extents, attrs, vis, kind } = i;
            self.with_parts(extents, &attrs.meta, *vis, |v| {
                v.visit_item_kind(kind);
            });
        }
        fn visit_ty(&mut self, t: &'a Ty) {
            let Ty { extents, kind } = t;
            self.with_only_span(extents, |v| {
                v.push(NodeSource::Ty(kind));
                v.visit_ty_kind(kind);
            });
        }
        fn visit_stmt(&mut self, s: &'a Stmt) {
            let Stmt { extents, kind, semi } = s;
            self.with_only_span(extents, |d| {
                d.visit_stmt_kind(kind);
                d.visit_semi(semi);
            })
        }
        fn visit_item_kind(&mut self, kind: &'a ItemKind) {
            match kind {
                ItemKind::Module(i) => self.push(NodeSource::Module(i)),
                ItemKind::Alias(i) => self.push(NodeSource::Alias(i)),
                ItemKind::Enum(i) => self.push(NodeSource::Enum(i)),
                ItemKind::Struct(i) => self.push(NodeSource::Struct(i)),
                ItemKind::Const(i) => self.push(NodeSource::Const(i)),
                ItemKind::Static(i) => self.push(NodeSource::Static(i)),
                ItemKind::Function(i) => self.push(NodeSource::Function(i)),
                ItemKind::Impl(i) => self.push(NodeSource::Impl(i)),
                ItemKind::Use(i) => self.push(NodeSource::Use(i)),
            }
            or_visit_item_kind(self, kind);
        }
        fn visit_stmt_kind(&mut self, kind: &'a StmtKind) {
            if let StmtKind::Local(l) = kind {
                self.push(NodeSource::Local(l))
            }
            or_visit_stmt_kind(self, kind);
        }
    }
}

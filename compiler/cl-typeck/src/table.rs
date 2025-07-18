//! The [Table] is a monolithic data structure representing everything the type checker
//! knows about a program.
//!
//! Individual nodes in the table can be queried using the [Entry] API ([Table::entry])
//! or modified using the [EntryMut] API ([Table::entry_mut]).
//!
//! # Contents of a "node"
//! Always present:
//! - [NodeKind]: Determines how this node will be treated during the [stages](crate::stage) of
//!   compilation
//! - [Parent node](Handle): Arranges this node in the hierarchical graph structure
//!
//! Populated as needed:
//! - Children: An associative array of [names](Sym) to child nodes in the graph. Child nodes are
//!   arranged in a *strict* tree structure, with no back edges
//! - Imports: An associative array of [names](Sym) to other nodes in the graph. Not all import
//!   nodes are back edges, but all back edges *must be* import nodes.
//! - [Types](TypeKind): Contains type information populated through type checking and inference.
//!   Nodes with unpopulated types may be considered type variables in the future.
//! - [Spans][span]: Positional information from the source text. See [cl_structures::span].
//! - [Metas](Meta): Metadata decorators. These may have an effect throughout the compiler.
//! - [Sources](Source): Pointers back into the AST, for future analysis.
//! - Impl Targets: Sparse mapping of `impl` nodes to their corresponding targets.
//! - etc.
//!
//! [span]: struct@Span

use crate::{
    entry::{Entry, EntryMut},
    handle::Handle,
    source::Source,
    type_kind::TypeKind,
};
use cl_ast::{Expr, Meta, PathPart, Sym};
use cl_structures::{index_map::IndexMap, span::Span};
use std::collections::HashMap;

// TODO: Cycle detection external to this module

/// The table is a monolithic data structure representing everything the type checker
/// knows about a program.
///
/// See [module documentation](self).
#[derive(Debug)]
pub struct Table<'a> {
    root: Handle,
    /// This is the source of truth for handles
    kinds: IndexMap<Handle, NodeKind>,
    parents: IndexMap<Handle, Handle>,
    pub(crate) children: HashMap<Handle, HashMap<Sym, Handle>>,
    pub(crate) imports: HashMap<Handle, HashMap<Sym, Handle>>,
    pub(crate) use_items: HashMap<Handle, Vec<Handle>>,
    bodies: HashMap<Handle, &'a Expr>,
    types: HashMap<Handle, TypeKind>,
    spans: HashMap<Handle, Span>,
    metas: HashMap<Handle, &'a [Meta]>,
    sources: HashMap<Handle, Source<'a>>,
    impl_targets: HashMap<Handle, Handle>,
    anon_types: HashMap<TypeKind, Handle>,
    lang_items: HashMap<Sym, Handle>,

    // --- Queues for algorithms ---
    pub(crate) unchecked: Vec<Handle>,
    pub(crate) impls: Vec<Handle>,
    pub(crate) uses: Vec<Handle>,
}

impl<'a> Table<'a> {
    pub fn new() -> Self {
        let mut kinds = IndexMap::new();
        let mut parents = IndexMap::new();
        let root = kinds.insert(NodeKind::Root);
        assert_eq!(root, parents.insert(root));

        Self {
            root,
            kinds,
            parents,
            children: HashMap::new(),
            imports: HashMap::new(),
            use_items: HashMap::new(),
            bodies: HashMap::new(),
            types: HashMap::new(),
            spans: HashMap::new(),
            metas: HashMap::new(),
            sources: HashMap::new(),
            impl_targets: HashMap::new(),
            anon_types: HashMap::new(),
            lang_items: HashMap::new(),
            unchecked: Vec::new(),
            impls: Vec::new(),
            uses: Vec::new(),
        }
    }

    pub fn entry(&self, handle: Handle) -> Entry<'_, 'a> {
        handle.to_entry(self)
    }

    pub fn entry_mut(&mut self, handle: Handle) -> EntryMut<'_, 'a> {
        handle.to_entry_mut(self)
    }

    pub fn new_entry(&mut self, parent: Handle, kind: NodeKind) -> Handle {
        let entry = self.kinds.insert(kind);
        assert_eq!(entry, self.parents.insert(parent));
        entry
    }

    pub fn add_child(&mut self, parent: Handle, name: Sym, child: Handle) -> Option<Handle> {
        self.children.entry(parent).or_default().insert(name, child)
    }

    pub fn add_import(&mut self, parent: Handle, name: Sym, import: Handle) -> Option<Handle> {
        self.imports.entry(parent).or_default().insert(name, import)
    }

    pub fn mark_unchecked(&mut self, item: Handle) {
        self.unchecked.push(item);
    }

    pub fn mark_use_item(&mut self, item: Handle) {
        let parent = self.parents[item];
        self.use_items.entry(parent).or_default().push(item);
        self.uses.push(item);
    }

    pub fn mark_impl_item(&mut self, item: Handle) {
        self.impls.push(item);
    }

    pub fn mark_lang_item(&mut self, name: Sym, item: Handle) {
        self.lang_items.insert(name, item);
    }

    pub fn handle_iter(&self) -> impl Iterator<Item = Handle> + use<> {
        self.kinds.keys()
    }

    /// Returns handles to all nodes sequentially by [Entry]
    pub fn debug_entry_iter(&self) -> impl Iterator<Item = Entry<'_, 'a>> {
        self.kinds.keys().map(|key| key.to_entry(self))
    }

    /// Gets the [Handle] of an anonymous type with the provided [TypeKind].
    /// If not already present, a new one is created.
    pub(crate) fn anon_type(&mut self, kind: TypeKind) -> Handle {
        if let Some(id) = self.anon_types.get(&kind) {
            return *id;
        }
        let entry = self.new_entry(self.root, NodeKind::Type);
        // Anonymous types require a bijective map (anon_types => Def => types)
        self.types.insert(entry, kind.clone());
        self.anon_types.insert(kind, entry);
        entry
    }

    pub(crate) fn inferred_type(&mut self) -> Handle {
        let handle = self.new_entry(self.root, NodeKind::Type);
        self.types.insert(handle, TypeKind::Inferred);
        handle
    }

    pub(crate) fn type_variable(&mut self) -> Handle {
        let handle = self.new_entry(self.root, NodeKind::Type);
        self.types.insert(handle, TypeKind::Variable);
        handle
    }

    pub const fn root_entry(&self) -> Entry<'_, 'a> {
        self.root.to_entry(self)
    }

    pub fn root_entry_mut(&mut self) -> crate::entry::EntryMut<'_, 'a> {
        self.root.to_entry_mut(self)
    }

    // --- inherent properties ---

    pub const fn root(&self) -> Handle {
        self.root
    }

    pub fn kind(&self, node: Handle) -> Option<&NodeKind> {
        self.kinds.get(node)
    }

    pub fn parent(&self, node: Handle) -> Option<&Handle> {
        self.parents.get(node)
    }

    pub fn children(&self, node: Handle) -> Option<&HashMap<Sym, Handle>> {
        self.children.get(&node)
    }

    pub fn imports(&self, node: Handle) -> Option<&HashMap<Sym, Handle>> {
        self.imports.get(&node)
    }

    pub fn body(&self, node: Handle) -> Option<&'a Expr> {
        self.bodies.get(&node).copied()
    }

    pub fn ty(&self, node: Handle) -> Option<&TypeKind> {
        self.types.get(&node)
    }

    pub fn span(&self, node: Handle) -> Option<&Span> {
        self.spans.get(&node)
    }

    pub fn meta(&self, node: Handle) -> Option<&'a [Meta]> {
        self.metas.get(&node).copied()
    }

    pub fn source(&self, node: Handle) -> Option<&Source<'a>> {
        self.sources.get(&node)
    }

    pub fn impl_target(&self, node: Handle) -> Option<Handle> {
        self.impl_targets.get(&node).copied()
    }

    pub fn reparent(&mut self, node: Handle, parent: Handle) -> Handle {
        self.parents.replace(node, parent)
    }

    pub fn set_body(&mut self, node: Handle, body: &'a Expr) -> Option<&'a Expr> {
        self.mark_unchecked(node);
        self.bodies.insert(node, body)
    }

    pub fn set_ty(&mut self, node: Handle, kind: TypeKind) -> Option<TypeKind> {
        self.types.insert(node, kind)
    }

    pub fn set_span(&mut self, node: Handle, span: Span) -> Option<Span> {
        self.spans.insert(node, span)
    }

    pub fn set_meta(&mut self, node: Handle, meta: &'a [Meta]) -> Option<&'a [Meta]> {
        self.metas.insert(node, meta)
    }

    pub fn set_source(&mut self, node: Handle, source: Source<'a>) -> Option<Source<'a>> {
        self.sources.insert(node, source)
    }

    pub fn set_impl_target(&mut self, node: Handle, target: Handle) -> Option<Handle> {
        self.impl_targets.insert(node, target)
    }

    // --- derived properties ---

    /// Gets a handle to the local `Self` type, if one exists
    pub fn selfty(&self, node: Handle) -> Option<Handle> {
        match self.kinds.get(node)? {
            NodeKind::Root | NodeKind::Use => None,
            NodeKind::Type => Some(node),
            NodeKind::Impl => self.impl_target(node),
            _ => self.selfty(*self.parent(node)?),
        }
    }

    pub fn super_of(&self, node: Handle) -> Option<Handle> {
        match self.kinds.get(node)? {
            NodeKind::Root => None,
            NodeKind::Module => self.parent(node).copied(),
            _ => self.super_of(*self.parent(node)?),
        }
    }

    pub fn name(&self, node: Handle) -> Option<Sym> {
        self.source(node).and_then(|s| s.name())
    }

    pub fn is_transparent(&self, node: Handle) -> bool {
        !matches!(
            self.kind(node),
            None | Some(NodeKind::Root | NodeKind::Module)
        )
    }

    pub fn get_child(&self, node: Handle, name: &Sym) -> Option<Handle> {
        self.children.get(&node).and_then(|c| c.get(name)).copied()
    }

    pub fn get_import(&self, node: Handle, name: &Sym) -> Option<Handle> {
        self.imports.get(&node).and_then(|i| i.get(name)).copied()
    }

    pub fn get_by_sym(&self, node: Handle, name: &Sym) -> Option<Handle> {
        self.get_child(node, name)
            .or_else(|| self.get_import(node, name))
            .or_else(|| {
                self.is_transparent(node)
                    .then(|| {
                        self.parent(node)
                            .and_then(|node| self.get_by_sym(*node, name))
                    })
                    .flatten()
            })
    }

    /// Does path traversal relative to the provided `node`.
    pub fn nav(&self, node: Handle, path: &[PathPart]) -> Option<Handle> {
        match path {
            [PathPart::SuperKw, rest @ ..] => self.nav(self.super_of(node)?, rest),
            [PathPart::SelfTy, rest @ ..] => self.nav(self.selfty(node)?, rest),
            [PathPart::Ident(name), rest @ ..] => self.nav(self.get_by_sym(node, name)?, rest),
            [] => Some(node),
        }
    }
}

impl Default for Table<'_> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug)]
pub enum NodeKind {
    Root,
    Module,
    Type,
    Const,
    Static,
    Function,
    Temporary,
    Let,
    Scope,
    Impl,
    Use,
}

mod display {
    use super::*;
    use std::fmt;
    impl fmt::Display for NodeKind {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                NodeKind::Root => write!(f, "root"),
                NodeKind::Module => write!(f, "mod"),
                NodeKind::Type => write!(f, "type"),
                NodeKind::Const => write!(f, "const"),
                NodeKind::Static => write!(f, "static"),
                NodeKind::Function => write!(f, "fn"),
                NodeKind::Temporary => write!(f, "temp"),
                NodeKind::Let => write!(f, "let"),
                NodeKind::Scope => write!(f, "scope"),
                NodeKind::Use => write!(f, "use"),
                NodeKind::Impl => write!(f, "impl"),
            }
        }
    }
}

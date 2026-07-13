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
//! - [Meta](Expr): Metadata decorators. These may have an effect throughout the compiler.
//! - Impl Targets: Sparse mapping of `impl` nodes to their corresponding targets.
//! - etc.

use crate::{
    entry::{Entry, EntryMut},
    type_kind::TypeKind,
};
use cl_ast::{
    Expr,
    types::{Path, Symbol as Sym},
};
use cl_structures::{index_map::*, intern::interned::Interned};
use std::collections::{BTreeMap, HashMap};

pub type Map<K, V> = BTreeMap<K, V>;
pub type SymMap<V> = BTreeMap<Sym, V>;

make_index! {
    /// A handle to an entry in a [Table]
    Scope,
}

impl std::fmt::Display for Scope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.0.fmt(f)
    }
}

/// The table is a monolithic data structure representing everything the type checker
/// knows about a program.
///
/// See [module documentation](self).
#[derive(Debug)]
pub struct Table {
    root: Scope,
    /// This is the source of truth for handles
    kinds: IndexMap<Scope, NodeKind>,
    parents: IndexMap<Scope, Scope>,
    pub(crate) children: Map<Scope, SymMap<Scope>>,
    pub(crate) locals: Map<Scope, Vec<Scope>>,
    pub(crate) lazy_imports: Map<Scope, SymMap<Path>>,
    pub(crate) glob_imports: Map<Scope, Vec<Path>>,
    pub(crate) names: Map<Scope, Sym>,
    pub(crate) types: Map<Scope, TypeKind>,
    pub(crate) metas: Map<Scope, Vec<Expr>>,
    pub(crate) impls: Map<Scope, Vec<Scope>>,
    pub(crate) impl_targets: Map<Scope, Scope>,
    pub(crate) anon_types: HashMap<TypeKind, Scope>,
    pub(crate) lang_items: Map<&'static str, Scope>,

    // --- Queues for algorithms ---
    pub(crate) unchecked_handles: Vec<Scope>,
    pub(crate) pending_impls: Vec<Scope>,
}

impl Table {
    pub fn new() -> Self {
        let mut kinds = IndexMap::new();
        let mut parents = IndexMap::new();
        let root = kinds.insert(NodeKind::Root);
        assert_eq!(root, parents.insert(root));

        Self {
            root,
            kinds,
            parents,
            children: Map::new(),
            locals: Map::new(),
            lazy_imports: Map::new(),
            glob_imports: Map::new(),
            names: Map::new(),
            types: Map::new(),
            metas: Map::new(),
            impls: Map::new(),
            impl_targets: Map::new(),
            anon_types: HashMap::new(),
            lang_items: Map::new(),
            unchecked_handles: Vec::new(),
            pending_impls: Vec::new(),
        }
    }

    /// Gets the [Entry] for a [Handle] in the [Table]
    pub fn entry(&self, handle: Scope) -> Entry<'_> {
        handle.to_entry(self)
    }

    /// Gets the [EntryMut] for a [Handle] in the [Table]
    pub fn entry_mut(&mut self, handle: Scope) -> EntryMut<'_> {
        handle.to_entry_mut(self)
    }

    /// Creates a new entry in the table, and returns its [Handle]
    pub fn new_entry(&mut self, parent: Scope, kind: NodeKind) -> Scope {
        let entry = self.kinds.insert(kind);
        assert_eq!(entry, self.parents.insert(parent));
        entry
    }

    /// Adds an existing [Handle] as the child of another (parent) [Handle]
    pub fn add_child(&mut self, parent: Scope, name: Sym, child: Scope) -> Option<Scope> {
        self.children.entry(parent).or_default().insert(name, child)
    }

    pub fn add_local(&mut self, parent: Scope, child: Scope) {
        self.locals.entry(parent).or_default().push(child);
    }

    /// Marks this item as not having been typechecked
    pub fn mark_unchecked(&mut self, item: Scope) {
        self.unchecked_handles.push(item);
    }

    /// Marks this item as an `impl` which hasn't been linked.
    pub fn mark_impl_item(&mut self, item: Scope) {
        let parent = self.parent(item).copied().unwrap_or(item);
        self.impls.entry(parent).or_default().push(item);
        self.pending_impls.push(item);
    }

    /// Marks this item as a "lang item", to be [retrieved later](Table::get_lang_item)
    pub fn mark_lang_item(&mut self, name: &'static str, item: Scope) {
        self.lang_items.insert(name, item);
    }

    /// Gets a previously [marked](Table::get_lang_item) lang-item from the table.
    pub fn get_lang_item(&self, name: &str) -> Scope {
        match self.lang_items.get(name).copied() {
            Some(handle) => handle,
            None => todo!(),
        }
    }

    /// Gets an [Iterator] over [Handles](Handle) in the Table
    pub fn handle_iter(&self) -> impl Iterator<Item = Scope> + use<> {
        self.kinds.keys()
    }

    /// Returns handles to all nodes sequentially by [Entry]
    pub fn debug_entry_iter(&self) -> impl Iterator<Item = Entry<'_>> {
        self.kinds.keys().map(|key| key.to_entry(self))
    }

    /// Gets the [Handle] of an anonymous type with the provided [TypeKind].
    /// If not already present, a new one is created.
    pub(crate) fn anon_type(&mut self, kind: TypeKind) -> Scope {
        if let Some(id) = self.anon_types.get(&kind) {
            return *id;
        }
        let entry = self.new_entry(self.root, NodeKind::Type);
        // Anonymous types require a bijective map (anon_types => Def => types)
        self.types.insert(entry, kind.clone());
        self.anon_types.insert(kind, entry);
        entry
    }

    /// Gets a [Handle] to a new [NodeKind::Type] with [TypeKind::Inferred]
    pub(crate) fn inferred_type(&mut self) -> Scope {
        let handle = self.new_entry(self.root, NodeKind::Type);
        self.types.insert(handle, TypeKind::Inferred);
        handle
    }

    /// Gets a [Handle] to a new [NodeKind::Type] with [TypeKind::Variable]
    pub(crate) fn type_variable(&mut self) -> Scope {
        let handle = self.new_entry(self.root, NodeKind::Type);
        self.types.insert(handle, TypeKind::Variable);
        handle
    }

    /// Gets the root [Entry] in the table
    pub const fn root_entry(&self) -> Entry<'_> {
        self.root.to_entry(self)
    }

    /// Gets the root [EntryMut] in the table
    pub fn root_entry_mut(&mut self) -> crate::entry::EntryMut<'_> {
        self.root.to_entry_mut(self)
    }

    // --- inherent properties ---

    /// Gets the root [Handle] in the table.
    pub const fn root(&self) -> Scope {
        self.root
    }

    /// Gets the [NodeKind] of the given [Handle]
    pub fn kind(&self, node: Scope) -> Option<&NodeKind> {
        self.kinds.get(node)
    }

    /// Gets the parent [Handle] of the given [Handle]
    pub fn parent(&self, node: Scope) -> Option<&Scope> {
        self.parents.get(node)
    }

    /// Gets the child map of the given [Handle]
    pub fn children(&self, node: Scope) -> Option<&SymMap<Scope>> {
        self.children.get(&node)
    }

    /// Gets the lazy import map of the given [Handle]
    pub fn lazy_imports(&self, node: Scope) -> Option<&SymMap<Path>> {
        self.lazy_imports.get(&node)
    }

    /// Gets the glob-import set of the given [Handle]
    pub fn glob_imports(&self, node: Scope) -> Option<&[Path]> {
        self.glob_imports.get(&node).map(Vec::as_slice)
    }

    /// Gets the [TypeKind] of the given [Handle]
    pub fn ty(&self, node: Scope) -> Option<&TypeKind> {
        self.types.get(&node)
    }

    /// Gets the [meta-expressions](Expr) of the given [Handle]
    pub fn meta(&self, node: Scope) -> Option<&[Expr]> {
        self.metas.get(&node).map(Vec::as_slice)
    }

    /// Gets the `impl` target of the given [Handle], if there is one
    pub fn impl_target(&self, node: Scope) -> Option<Scope> {
        self.impl_targets.get(&node).copied()
    }

    /// Replaces the parent [Handle] of the given node, returning the old one
    pub fn reparent(&mut self, node: Scope, parent: Scope) -> Scope {
        self.parents.replace(node, parent)
    }

    /// Adds a lazy-import to the entry at the `node` [Handle]
    pub fn add_import(&mut self, node: Scope, name: Sym, path: Path) {
        self.lazy_imports
            .entry(node)
            .or_default()
            .insert(name, path);
    }

    /// Adds a glob-import at the given node
    pub fn add_glob(&mut self, node: Scope, path: Path) {
        self.glob_imports.entry(node).or_default().push(path);
    }

    /// Sets the preferred name of the given node
    pub fn set_name(&mut self, node: Scope, name: Sym) -> Option<Sym> {
        self.names.insert(node, name)
    }

    /// Sets the [TypeKind] of the given node
    pub fn set_ty(&mut self, node: Scope, kind: TypeKind) -> Option<TypeKind> {
        self.types.insert(node, kind)
    }

    /// Sets the [meta-expressions](Expr) of the given node, returning the old expressions
    pub fn set_meta(&mut self, node: Scope, meta: Vec<Expr>) {
        self.metas.entry(node).or_default().extend(meta);
    }

    pub fn add_meta(&mut self, node: Scope, meta: Expr) {
        self.metas.entry(node).or_default().push(meta);
    }

    /// Sets the `impl` target of the given node
    pub fn set_impl_target(&mut self, node: Scope, target: Scope) -> Option<Scope> {
        self.impl_targets.insert(node, target)
    }

    // --- derived properties ---

    /// Gets a handle to the local `Self` type, if one exists
    pub fn selfty(&self, node: Scope) -> Option<Scope> {
        match self.kinds.get(node)? {
            NodeKind::Root | NodeKind::Use => None,
            NodeKind::Type => Some(node),
            NodeKind::Impl => self.impl_target(node),
            _ => self.selfty(*self.parent(node)?),
        }
    }

    /// Gets the local parent *module*
    pub fn super_of(&self, node: Scope) -> Option<Scope> {
        match self.kinds.get(node)? {
            NodeKind::Root => None,
            NodeKind::Module => self.parent(node).copied(),
            _ => self.super_of(*self.parent(node)?),
        }
    }

    /// Gets the name of this node
    pub fn name(&self, node: Scope) -> Option<Sym> {
        self.names.get(&node).copied()
    }

    /// Returns `true` when name resolution is allowed to defer to the parent of this node
    pub fn is_transparent(&self, node: Scope) -> bool {
        !matches!(
            self.kind(node),
            None | Some(NodeKind::Root | NodeKind::Module)
        )
    }

    /// Gets the child of this `node` with the given `name`
    pub fn get_child(&self, node: Scope, name: &Sym) -> Option<Scope> {
        self.children.get(&node).and_then(|c| c.get(name)).copied()
    }

    /// Gets the local of this `node` with the given `name`
    pub fn get_local(&self, parent: Scope, name: &Sym) -> Option<Scope> {
        self.locals
            .get(&parent)?
            .iter()
            .rev()
            .find(|v| self.names[v] == *name)
            .copied()
    }

    /// Searches the import hierarchy for a particular name
    pub fn get_import(&self, node: Scope, name: &Sym) -> Option<Scope> {
        if let Some(path) = self.lazy_imports(node).and_then(|t| t.get(name)) {
            return self.nav(node, &path.parts);
        }
        self.glob_imports(node)
            .into_iter()
            .flatten()
            .rev()
            .flat_map(|path| self.nav(node, &path.parts))
            .find_map(|node| self.get_by_sym(node, name))
    }

    pub fn get_by_sym(&self, node: Scope, name: &Sym) -> Option<Scope> {
        self.get_local(node, name)
            .or_else(|| self.get_child(node, name))
            .or_else(|| {
                self.is_transparent(node)
                    .then(|| {
                        self.parent(node)
                            .and_then(|node| self.get_by_sym(*node, name))
                    })
                    .flatten()
            })
            .or_else(|| self.get_import(node, name))
    }

    /// Does path traversal relative to the provided `node`.
    pub fn nav(&self, node: Scope, path: &[Sym]) -> Option<Scope> {
        // println!(
        //     "Navigating to {}::{:?}",
        //     self.name(node).unwrap_or_default(),
        //     path
        // );
        match path {
            [Interned("", ..), rest @ ..] => self.nav(self.root, rest),
            [Interned("super", ..), rest @ ..] => self.nav(self.super_of(node)?, rest),
            [Interned("Self", ..), rest @ ..] => self.nav(self.selfty(node)?, rest),
            [name, rest @ ..] => self.nav(self.get_by_sym(node, name)?, rest),
            [] => Some(node),
        }
    }
}

impl Default for Table {
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

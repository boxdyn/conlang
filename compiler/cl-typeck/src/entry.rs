//! An [Entry] is an accessor for [nodes](Handle) in a [Table].
//!
//! There are two kinds of entry:
//! - [Entry]: Provides getters for an entry's fields, and an implementation of
//!   [Display](std::fmt::Display)
//! - [EntryMut]: Provides setters for an entry's fields, and an [`as_ref`](EntryMut::as_ref) method
//!   to demote to an [Entry].

use crate::table::SymMap;

use cl_ast::{
    Expr,
    types::{Path, Symbol as Sym},
};

use crate::{
    handle::Handle,
    stage::categorize as cat,
    table::{NodeKind, Table},
    type_expression::{self as tex, TypeExpression},
    type_kind::TypeKind,
};

mod debug;
mod display;

impl Handle {
    /// Constructs an [Entry] from this handle and a [Table] ref
    pub const fn to_entry<'t>(self, table: &'t Table) -> Entry<'t> {
        Entry { id: self, table }
    }

    /// Constructs an [EntryMut] from this handle and a [Table] ref mut
    pub const fn to_entry_mut<'t>(self, table: &'t mut Table) -> EntryMut<'t> {
        EntryMut { id: self, table }
    }
}

/// An immutable, object-like entry in a [`Table`].
///
/// [`Entry`] wraps a [`Table`] and a [`Handle`], and provides an ergonomic interface
/// for querying information about the state of the node at that [`Handle`].
///
/// Its mutable counterpart, [`EntryMut`], provides a similar interface for *modifying*
/// the state of the [`Table`].
pub struct Entry<'t> {
    table: &'t Table,
    id: Handle,
}

macro_rules! impl_entry_ {
    () => {
        /// Gets the [Handle] associated with this Entry
        pub const fn id(&self) -> Handle {
            self.id
        }

        /// Gets the [Table] associated with this Entry
        pub const fn inner(&'t self) -> &'t Table {
            self.table
        }

        /// Gets the [NodeKind] of this [Entry] in the [Table]
        pub fn kind(&self) -> Option<&NodeKind> {
            self.table.kind(self.id)
        }

        /// Gets the [Entry] of the root node in the [Table]
        pub const fn root(&self) -> Entry<'_> {
            Entry { id: self.table.root(), table: self.table }
        }

        /// Gets the children of this node
        pub fn children(&self) -> Option<&SymMap<Handle>> {
            self.table.children(self.id)
        }

        /// Gets the lazy-imports list for this node
        pub fn lazy_imports(&self) -> Option<&SymMap<Path>> {
            self.table.lazy_imports(self.id)
        }

        /// Gets the glob-imports list for this node
        pub fn glob_imports(&self) -> Option<&[Path]> {
            self.table.glob_imports(self.id)
        }

        /// Gets the meta-[Expr] list for this node
        pub fn meta(&self) -> Option<&[Expr]> {
            self.table.meta(self.id)
        }

        /// Gets an identifying [Sym]bol for this node, if possible
        pub fn name(&self) -> Option<Sym> {
            self.table.name(self.id)
        }
    };
}

impl<'t> Entry<'t> {
    /// Constructs a new [`Entry`] from shared [`&Table`](Table) and a [`Handle`].
    pub const fn new(table: &'t Table, id: Handle) -> Self {
        Self { table, id }
    }

    impl_entry_!();

    /// Constructs another [Entry] with the given [Handle]
    pub const fn with_id(&self, id: Handle) -> Entry<'t> {
        Self { table: self.table, id }
    }

    /// [Navigates](Table::nav) to another [Entry]
    pub fn nav(&self, path: &[Sym]) -> Option<Entry<'t>> {
        Some(Entry { id: self.table.nav(self.id, path)?, table: self.table })
    }

    /// Gets the [parent](Table::parent) of this [Entry]
    pub fn parent(&self) -> Option<Entry<'t>> {
        Some(Entry { id: *self.table.parent(self.id)?, ..*self })
    }

    /// Gets the [TypeKind] of this [Entry]
    pub fn ty(&self) -> Option<&'t TypeKind> {
        self.table.ty(self.id)
    }

    /// Gets the [`impl` target](Table::impl_target) of this [Entry]
    pub fn impl_target(&self) -> Option<Entry<'_>> {
        Some(Entry { id: self.table.impl_target(self.id)?, ..*self })
    }

    /// Gets the [`Self` type](Table::selfty) of this [Entry]
    pub fn selfty(&self) -> Option<Entry<'_>> {
        Some(Entry { id: self.table.selfty(self.id)?, ..*self })
    }
}

/// A mutable, object-like entry in a [`Table`].
///
/// [`Entry`] wraps a [`Table`] and a [`Handle`], and provides an ergonomic interface
/// for querying information about the state of the node at that [`Handle`].
///
/// Its immutable counterpart, [`Entry`], provides a similar interface for *querying*
/// the state of the [`Table`], which may be shared among multiple [`Entries`](Entry).
#[derive(Debug)]
pub struct EntryMut<'t> {
    table: &'t mut Table,
    id: Handle,
}

impl<'t> EntryMut<'t> {
    /// Constructs a new [`EntryMut`] from a [`&mut Table`](Table) and a [`Handle`].
    pub fn new(table: &'t mut Table, id: Handle) -> Self {
        Self { table, id }
    }

    impl_entry_!();

    /// Gets the [`TypeKind`] of this [`EntryMut`]
    pub fn ty(&self) -> Option<&TypeKind> {
        self.table.ty(self.id)
    }

    /// Reborrows the inner [`Table`] reference
    pub fn inner_mut(&mut self) -> &mut Table {
        self.table
    }

    /// Cheaply constructs an [`Entry`] from this [`EntryMut`]
    pub fn as_ref(&self) -> Entry<'_> {
        Entry { table: self.table, id: self.id }
    }

    /// Evaluates a [TypeExpression] in this entry's context
    pub fn evaluate<Out>(&mut self, ty: &impl TypeExpression<Out>) -> Result<Out, tex::Error> {
        let Self { table, id } = self;
        ty.evaluate(table, *id)
    }

    /// Calls [categorize](cat::categorize) on this node in the table
    pub fn categorize(&mut self) -> Result<(), cat::Error> {
        cat::categorize(self.table, self.id)
    }

    /// Constructs a new Handle with the provided parent [Handle]
    pub fn with_id(&mut self, parent: Handle) -> EntryMut<'_> {
        EntryMut { table: self.table, id: parent }
    }

    /// [Navigates](Table::nav) to another [`EntryMut`], reborrowing the table.
    pub fn nav(&mut self, path: &[Sym]) -> Option<EntryMut<'_>> {
        Some(EntryMut { id: self.table.nav(self.id, path)?, table: self.table })
    }

    /// Constructs a new node with the given [NodeKind], and returns its [EntryMut].
    pub fn new_entry(&mut self, kind: NodeKind) -> EntryMut<'_> {
        let id = self.table.new_entry(self.id, kind);
        self.with_id(id)
    }

    /// Adds an existing node as a `child` with the given `name`.
    ///
    /// If that name is already taken, its previous [Handle] is returned.
    pub fn add_child(&mut self, name: Sym, child: Handle) -> Option<Handle> {
        self.table.add_child(self.id, name, child)
    }

    /// Adds a [lazy-import](Table::lazy_imports) edge from [`name`](Sym) to [`path`](Path)
    pub fn add_import(&mut self, name: Sym, path: Path) {
        self.table.add_import(self.id, name, path);
    }

    /// Adds a [glob-import](Table::glob_imports) edge to [`path`](Path).
    ///
    /// Glob imports act as secondary (tertiary, etc.) parent edges,
    /// which are always transparent. They are checked in reverse order,
    /// giving later glob imports precedence over earlier ones.
    pub fn add_glob(&mut self, path: Path) {
        self.table.add_glob(self.id, path);
    }

    /// Sets the [`name`](Table::name) of this [`EntryMut`].
    pub fn set_name(&mut self, name: Sym) -> Option<Sym> {
        self.table.set_name(self.id, name)
    }

    /// Sets the [`TypeKind`] of this [`EntryMut`]
    pub fn set_ty(&mut self, kind: TypeKind) -> Option<TypeKind> {
        self.table.set_ty(self.id, kind)
    }

    /// Appends a list of [Expr]s to this [`EntryMut`]'s [meta](Table::meta).
    pub fn set_meta(&mut self, meta: Vec<Expr>) {
        self.table.set_meta(self.id, meta)
    }

    /// Adds a single [Expr] to this [`EntryMut`]'s [meta](Table::meta)
    pub fn add_meta(&mut self, meta: Expr) {
        self.table.add_meta(self.id, meta)
    }

    /// Sets the [`impl` target](Table::impl_target) to [`target`](Handle)
    pub fn set_impl_target(&mut self, target: Handle) -> Option<Handle> {
        self.table.set_impl_target(self.id, target)
    }

    /// Marks this [EntryMut] as unchecked.
    pub fn mark_unchecked(&mut self) {
        self.table.mark_unchecked(self.id)
    }

    /// Marks this [EntryMut] as an `impl` item
    pub fn mark_impl_item(&mut self) {
        self.table.mark_impl_item(self.id)
    }

    /// Marks this [EntryMut] as a [`lang item`](Table::get_lang_item)
    pub fn mark_lang_item(&mut self, lang_item: &'static str) {
        self.table.mark_lang_item(lang_item, self.id)
    }
}

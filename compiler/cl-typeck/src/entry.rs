//! An [Entry] is an accessor for [nodes](Handle) in a [Table].
//!
//! There are two kinds of entry:
//! - [Entry]: Provides getters for an entry's fields, and an implementation of
//!   [Display](std::fmt::Display)
//! - [EntryMut]: Provides setters for an entry's fields, and an [`as_ref`](EntryMut::as_ref) method
//!   to demote to an [Entry].

use std::collections::HashMap;

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
    pub const fn to_entry<'t>(self, table: &'t Table) -> Entry<'t> {
        Entry { id: self, table }
    }
    pub fn to_entry_mut<'t>(self, table: &'t mut Table) -> EntryMut<'t> {
        EntryMut { id: self, table }
    }
}

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
        pub fn children(&self) -> Option<&HashMap<Sym, Handle>> {
            self.table.children(self.id)
        }

        /// Gets the lazy-imports list for this node
        pub fn lazy_imports(&self) -> Option<&HashMap<Sym, Path>> {
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
    pub const fn new(table: &'t Table, id: Handle) -> Self {
        Self { table, id }
    }

    impl_entry_!();

    pub const fn with_id(&self, id: Handle) -> Entry<'t> {
        Self { table: self.table, id }
    }

    pub fn nav(&self, path: &[Sym]) -> Option<Entry<'t>> {
        Some(Entry { id: self.table.nav(self.id, path)?, table: self.table })
    }

    pub fn parent(&self) -> Option<Entry<'t>> {
        Some(Entry { id: *self.table.parent(self.id)?, ..*self })
    }

    pub fn ty(&self) -> Option<&'t TypeKind> {
        self.table.ty(self.id)
    }

    pub fn impl_target(&self) -> Option<Entry<'_>> {
        Some(Entry { id: self.table.impl_target(self.id)?, ..*self })
    }

    pub fn selfty(&self) -> Option<Entry<'_>> {
        Some(Entry { id: self.table.selfty(self.id)?, ..*self })
    }
}

#[derive(Debug)]
pub struct EntryMut<'t> {
    table: &'t mut Table,
    id: Handle,
}

impl<'t> EntryMut<'t> {
    pub fn new(table: &'t mut Table, id: Handle) -> Self {
        Self { table, id }
    }

    impl_entry_!();

    pub fn ty(&self) -> Option<&TypeKind> {
        self.table.ty(self.id)
    }

    pub fn inner_mut(&mut self) -> &mut Table {
        self.table
    }

    pub fn as_ref(&self) -> Entry<'_> {
        Entry { table: self.table, id: self.id }
    }

    /// Evaluates a [TypeExpression] in this entry's context
    pub fn evaluate<Out>(&mut self, ty: &impl TypeExpression<Out>) -> Result<Out, tex::Error> {
        let Self { table, id } = self;
        ty.evaluate(table, *id)
    }

    pub fn categorize(&mut self) -> Result<(), cat::Error> {
        cat::categorize(self.table, self.id)
    }

    /// Constructs a new Handle with the provided parent [Handle]
    pub fn with_id(&mut self, parent: Handle) -> EntryMut<'_> {
        EntryMut { table: self.table, id: parent }
    }

    pub fn nav(&mut self, path: &[Sym]) -> Option<EntryMut<'_>> {
        Some(EntryMut { id: self.table.nav(self.id, path)?, table: self.table })
    }

    pub fn new_entry(&mut self, kind: NodeKind) -> EntryMut<'_> {
        let id = self.table.new_entry(self.id, kind);
        self.with_id(id)
    }

    pub fn add_child(&mut self, name: Sym, child: Handle) -> Option<Handle> {
        self.table.add_child(self.id, name, child)
    }

    pub fn add_import(&mut self, name: Sym, path: Path) {
        self.table.add_import(self.id, name, path);
    }

    pub fn add_glob(&mut self, path: Path) {
        self.table.add_glob(self.id, path);
    }

    pub fn set_name(&mut self, name: Sym) -> Option<Sym> {
        self.table.set_name(self.id, name)
    }

    pub fn set_ty(&mut self, kind: TypeKind) -> Option<TypeKind> {
        self.table.set_ty(self.id, kind)
    }

    pub fn set_meta(&mut self, meta: Vec<Expr>) {
        self.table.set_meta(self.id, meta)
    }

    pub fn add_meta(&mut self, meta: Expr) {
        self.table.add_meta(self.id, meta)
    }

    pub fn set_impl_target(&mut self, target: Handle) -> Option<Handle> {
        self.table.set_impl_target(self.id, target)
    }

    pub fn mark_unchecked(&mut self) {
        self.table.mark_unchecked(self.id)
    }

    pub fn mark_impl_item(&mut self) {
        self.table.mark_impl_item(self.id)
    }

    pub fn mark_lang_item(&mut self, lang_item: &'static str) {
        self.table.mark_lang_item(lang_item, self.id)
    }
}

//! A [Handle] is an accessor for [Entries](Entry) in a [Table].
//!
//! There are two kinds of handle:
//! - [Handle]: Provides getters for an entry's fields, and an implementation of
//!   [Display](std::fmt::Display)
//! - [HandleMut]: Provides setters for an entry's fields, and an [`as_ref`](HandleMut::as_ref)
//!   method to demote to a [Handle].

use std::collections::HashMap;

use cl_ast::{Meta, PathPart, Sym};
use cl_structures::span::Span;

use crate::{
    handle::Handle,
    source::Source,
    table::{NodeKind, Table},
    type_kind::TypeKind,
};

pub mod display;

impl Handle {
    pub const fn to_entry<'t, 'a>(self, table: &'t Table<'a>) -> Entry<'t, 'a> {
        Entry { id: self, table }
    }
    pub fn to_entry_mut<'t, 'a>(self, table: &'t mut Table<'a>) -> EntryMut<'t, 'a> {
        EntryMut { id: self, table }
    }
}

#[derive(Debug)]
pub struct Entry<'t, 'a> {
    table: &'t Table<'a>,
    id: Handle,
}

impl<'t, 'a> Entry<'t, 'a> {
    pub const fn new(table: &'t Table<'a>, id: Handle) -> Self {
        Self { table, id }
    }

    pub const fn id(&self) -> Handle {
        self.id
    }

    pub fn inner(&self) -> &Table<'a> {
        self.table
    }

    pub const fn with_id(&self, id: Handle) -> Entry<'t, 'a> {
        Self { table: self.table, id }
    }

    pub fn nav(&self, path: &[PathPart]) -> Option<Entry<'t, 'a>> {
        Some(Entry { id: self.table.nav(self.id, path)?, table: self.table })
    }

    pub const fn root(&self) -> Handle {
        self.table.root()
    }

    pub fn kind(&self) -> Option<&NodeKind> {
        self.table.kind(self.id)
    }

    pub fn parent(&self) -> Option<&Handle> {
        self.table.parent(self.id)
    }

    pub fn children(&self) -> Option<&HashMap<Sym, Handle>> {
        self.table.children(self.id)
    }

    pub fn imports(&self) -> Option<&HashMap<Sym, Handle>> {
        self.table.imports(self.id)
    }

    pub fn ty(&self) -> Option<&TypeKind> {
        self.table.ty(self.id)
    }

    pub fn span(&self) -> Option<&Span> {
        self.table.span(self.id)
    }

    pub fn meta(&self) -> Option<&'a [Meta]> {
        self.table.meta(self.id)
    }

    pub fn source(&self) -> Option<&Source<'a>> {
        self.table.source(self.id)
    }

    pub fn impl_target(&self) -> Option<Handle> {
        self.table.impl_target(self.id)
    }

    pub fn selfty(&self) -> Option<Handle> {
        self.table.selfty(self.id)
    }

    pub fn name(&self) -> Option<Sym> {
        self.table.name(self.id)
    }
}

#[derive(Debug)]
pub struct EntryMut<'t, 'a> {
    table: &'t mut Table<'a>,
    id: Handle,
}

impl<'t, 'a> EntryMut<'t, 'a> {
    pub fn new(table: &'t mut Table<'a>, id: Handle) -> Self {
        Self { table, id }
    }

    pub fn as_ref(&self) -> Entry<'_, 'a> {
        Entry { table: self.table, id: self.id }
    }

    pub const fn id(&self) -> Handle {
        self.id
    }

    /// Constructs a new Handle with the provided parent [DefID]
    pub fn with_id(&mut self, parent: Handle) -> EntryMut<'_, 'a> {
        EntryMut { table: self.table, id: parent }
    }

    pub fn nav(&mut self, path: &[PathPart]) -> Option<EntryMut<'_, 'a>> {
        Some(EntryMut { id: self.table.nav(self.id, path)?, table: self.table })
    }

    pub fn new_entry(&mut self, kind: NodeKind) -> EntryMut<'_, 'a> {
        let id = self.table.new_entry(self.id, kind);
        self.with_id(id)
    }

    pub fn add_child(&mut self, name: Sym, child: Handle) -> Option<Handle> {
        self.table.add_child(self.id, name, child)
    }

    pub fn set_ty(&mut self, kind: TypeKind) -> Option<TypeKind> {
        self.table.set_ty(self.id, kind)
    }

    pub fn set_span(&mut self, span: Span) -> Option<Span> {
        self.table.set_span(self.id, span)
    }

    pub fn set_meta(&mut self, meta: &'a [Meta]) -> Option<&'a [Meta]> {
        self.table.set_meta(self.id, meta)
    }

    pub fn set_source(&mut self, source: Source<'a>) -> Option<Source<'a>> {
        self.table.set_source(self.id, source)
    }

    pub fn set_impl_target(&mut self, target: Handle) -> Option<Handle> {
        self.table.set_impl_target(self.id, target)
    }

    pub fn mark_use_item(&mut self) {
        self.table.mark_use_item(self.id)
    }

    pub fn mark_impl_item(&mut self) {
        self.table.mark_impl_item(self.id)
    }
}

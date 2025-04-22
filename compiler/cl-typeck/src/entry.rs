//! An [Entry] is an accessor for [nodes](Handle) in a [Table].
//!
//! There are two kinds of entry:
//! - [Entry]: Provides getters for an entry's fields, and an implementation of
//!   [Display](std::fmt::Display)
//! - [EntryMut]: Provides setters for an entry's fields, and an [`as_ref`](EntryMut::as_ref) method
//!   to demote to an [Entry].

use std::collections::HashMap;

use cl_ast::{Expr, Meta, PathPart, Sym};
use cl_structures::span::Span;

use crate::{
    handle::Handle,
    source::Source,
    stage::categorize as cat,
    table::{NodeKind, Table},
    type_expression::{self as tex, TypeExpression},
    type_kind::TypeKind,
};

mod display;

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

    pub fn inner(&self) -> &'t Table<'a> {
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

    pub fn kind(&self) -> Option<&'t NodeKind> {
        self.table.kind(self.id)
    }

    pub fn parent(&self) -> Option<Entry<'t, 'a>> {
        Some(Entry { id: *self.table.parent(self.id)?, ..*self })
    }

    pub fn children(&self) -> Option<&'t HashMap<Sym, Handle>> {
        self.table.children(self.id)
    }

    pub fn imports(&self) -> Option<&'t HashMap<Sym, Handle>> {
        self.table.imports(self.id)
    }

    pub fn bodies(&self) -> Option<&'a Expr> {
        self.table.body(self.id)
    }

    pub fn ty(&self) -> Option<&'t TypeKind> {
        self.table.ty(self.id)
    }

    pub fn span(&self) -> Option<&'t Span> {
        self.table.span(self.id)
    }

    pub fn meta(&self) -> Option<&'a [Meta]> {
        self.table.meta(self.id)
    }

    pub fn source(&self) -> Option<&'t Source<'a>> {
        self.table.source(self.id)
    }

    pub fn impl_target(&self) -> Option<Entry<'_, 'a>> {
        Some(Entry { id: self.table.impl_target(self.id)?, ..*self })
    }

    pub fn selfty(&self) -> Option<Entry<'_, 'a>> {
        Some(Entry { id: self.table.selfty(self.id)?, ..*self })
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

    /// Evaluates a [TypeExpression] in this entry's context
    pub fn evaluate<Out>(&mut self, ty: &impl TypeExpression<Out>) -> Result<Out, tex::Error> {
        let Self { table, id } = self;
        ty.evaluate(table, *id)
    }

    pub fn categorize(&mut self) -> Result<(), cat::Error> {
        cat::categorize(self.table, self.id)
    }

    /// Constructs a new Handle with the provided parent [Handle]
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

    pub fn set_body(&mut self, body: &'a Expr) -> Option<&'a Expr> {
        self.table.set_body(self.id, body)
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

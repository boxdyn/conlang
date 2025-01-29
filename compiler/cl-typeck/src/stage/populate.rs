//! The [Populator] populates entries in the sym table, including span info
use crate::{
    entry::EntryMut,
    handle::Handle,
    source::Source,
    table::{NodeKind, Table},
};
use cl_ast::{ast_visitor::Visit, ItemKind, Sym};

#[derive(Debug)]
pub struct Populator<'t, 'a> {
    inner: EntryMut<'t, 'a>,
    name: Option<Sym>, // this is a hack to get around the Visitor interface
}

impl<'t, 'a> Populator<'t, 'a> {
    pub fn new(table: &'t mut Table<'a>) -> Self {
        Self { inner: table.root_entry_mut(), name: None }
    }
    /// Constructs a new Populator with the provided parent Handle
    pub fn with_id(&mut self, parent: Handle) -> Populator<'_, 'a> {
        Populator { inner: self.inner.with_id(parent), name: None }
    }

    pub fn new_entry(&mut self, kind: NodeKind) -> Populator<'_, 'a> {
        Populator { inner: self.inner.new_entry(kind), name: None }
    }

    pub fn set_name(&mut self, name: Sym) {
        self.name = Some(name);
    }
}

impl<'a> Visit<'a> for Populator<'_, 'a> {
    fn visit_item(&mut self, i: &'a cl_ast::Item) {
        let cl_ast::Item { extents, attrs, vis, kind } = i;
        // TODO: this, better, better.
        let entry_kind = match kind {
            ItemKind::Alias(_) => NodeKind::Type,
            ItemKind::Enum(_) => NodeKind::Type,
            ItemKind::Struct(_) => NodeKind::Type,

            ItemKind::Const(_) => NodeKind::Const,
            ItemKind::Static(_) => NodeKind::Static,
            ItemKind::Function(_) => NodeKind::Function,

            ItemKind::Module(_) => NodeKind::Module,
            ItemKind::Impl(_) => NodeKind::Impl,
            ItemKind::Use(_) => NodeKind::Use,
        };

        let mut entry = self.new_entry(entry_kind);
        entry.inner.set_span(*extents);
        entry.inner.set_meta(&attrs.meta);

        entry.visit_span(extents);
        entry.visit_attrs(attrs);
        entry.visit_visibility(vis);
        entry.visit_item_kind(kind);

        if let (Some(name), child) = (entry.name, entry.inner.id()) {
            self.inner.add_child(name, child);
        }
    }

    fn visit_alias(&mut self, a: &'a cl_ast::Alias) {
        let cl_ast::Alias { to, from } = a;
        self.inner.set_source(Source::Alias(a));
        self.set_name(*to);

        if let Some(t) = from {
            self.visit_ty(t)
        }
    }

    fn visit_const(&mut self, c: &'a cl_ast::Const) {
        let cl_ast::Const { name, ty, init } = c;
        self.inner.set_source(Source::Const(c));
        self.set_name(*name);

        self.visit_ty(ty);
        self.visit_expr(init);
    }

    fn visit_static(&mut self, s: &'a cl_ast::Static) {
        let cl_ast::Static { mutable, name, ty, init } = s;
        self.inner.set_source(Source::Static(s));
        self.set_name(*name);

        self.visit_mutability(mutable);
        self.visit_ty(ty);
        self.visit_expr(init);
    }

    fn visit_module(&mut self, m: &'a cl_ast::Module) {
        let cl_ast::Module { name, kind } = m;
        self.inner.set_source(Source::Module(m));
        self.set_name(*name);

        self.visit_module_kind(kind);
    }

    fn visit_function(&mut self, f: &'a cl_ast::Function) {
        let cl_ast::Function { name, sign, bind, body } = f;
        self.inner.set_source(Source::Function(f));
        self.set_name(*name);

        self.visit_ty_fn(sign);
        bind.iter().for_each(|p| self.visit_param(p));
        if let Some(b) = body {
            self.visit_block(b)
        }
    }

    fn visit_struct(&mut self, s: &'a cl_ast::Struct) {
        let cl_ast::Struct { name, kind } = s;
        self.inner.set_source(Source::Struct(s));
        self.set_name(*name);

        self.visit_struct_kind(kind);
    }

    fn visit_enum(&mut self, e: &'a cl_ast::Enum) {
        let cl_ast::Enum { name, kind } = e;
        self.inner.set_source(Source::Enum(e));
        self.set_name(*name);

        self.visit_enum_kind(kind);
    }

    fn visit_impl(&mut self, i: &'a cl_ast::Impl) {
        let cl_ast::Impl { target, body } = i;
        self.inner.set_source(Source::Impl(i));
        self.inner.mark_impl_item();

        self.visit_impl_kind(target);
        self.visit_file(body);
    }

    fn visit_use(&mut self, u: &'a cl_ast::Use) {
        let cl_ast::Use { absolute: _, tree } = u;
        self.inner.set_source(Source::Use(u));
        self.inner.mark_use_item();

        self.visit_use_tree(tree);
    }

    fn visit_let(&mut self, l: &'a cl_ast::Let) {
        let cl_ast::Let { mutable, name: _, ty, init } = l;
        let mut entry = self.new_entry(NodeKind::Local);

        entry.inner.set_source(Source::Local(l));
        // entry.set_name(*name);

        entry.visit_mutability(mutable);
        if let Some(ty) = ty {
            entry.visit_ty(ty);
        }
        if let Some(init) = init {
            entry.visit_expr(init)
        }

        // let child = entry.inner.id();
        // self.inner.add_child(*name, child);
        todo!("Pattern destructuring in cl-typeck")
    }
}

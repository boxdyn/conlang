//! The [Populator] populates entries in the sym table, including span info
use crate::{
    entry::EntryMut,
    handle::Handle,
    source::Source,
    table::{NodeKind, Table},
};
use cl_ast::{ItemKind, Sym, ast_visitor::Visit};

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
        let cl_ast::Item { span, attrs, vis, kind } = i;
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
        entry.inner.set_span(*span);
        entry.inner.set_meta(&attrs.meta);

        entry.visit_span(span);
        entry.visit_attrs(attrs);
        entry.visit_visibility(vis);
        entry.visit_item_kind(kind);

        if let (Some(name), child) = (entry.name, entry.inner.id()) {
            self.inner.add_child(name, child);
        }
    }

    fn visit_alias(&mut self, a: &'a cl_ast::Alias) {
        let cl_ast::Alias { name, from } = a;
        self.inner.set_source(Source::Alias(a));
        self.set_name(*name);

        if let Some(t) = from {
            self.visit_ty(t)
        }
    }

    fn visit_const(&mut self, c: &'a cl_ast::Const) {
        let cl_ast::Const { name, ty, init } = c;
        self.inner.set_source(Source::Const(c));
        self.inner.set_body(init);
        self.set_name(*name);

        self.visit_ty(ty);
        self.visit_expr(init);
    }

    fn visit_static(&mut self, s: &'a cl_ast::Static) {
        let cl_ast::Static { mutable, name, ty, init } = s;
        self.inner.set_source(Source::Static(s));
        self.inner.set_body(init);
        self.set_name(*name);

        self.visit_mutability(mutable);
        self.visit_ty(ty);
        self.visit_expr(init);
    }

    fn visit_module(&mut self, m: &'a cl_ast::Module) {
        let cl_ast::Module { name, file } = m;
        self.inner.set_source(Source::Module(m));
        self.set_name(*name);

        if let Some(file) = file {
            self.visit_file(file);
        }
    }

    fn visit_function(&mut self, f: &'a cl_ast::Function) {
        let cl_ast::Function { name, gens: _, sign, bind, body } = f;
        // TODO: populate generics?
        self.inner.set_source(Source::Function(f));
        self.set_name(*name);

        self.visit_ty_fn(sign);
        self.visit_pattern(bind);
        if let Some(b) = body {
            self.inner.set_body(b);
            self.visit_expr(b)
        }
    }

    fn visit_struct(&mut self, s: &'a cl_ast::Struct) {
        let cl_ast::Struct { name, gens: _, kind } = s;
        // TODO: populate generics?
        self.inner.set_source(Source::Struct(s));
        self.set_name(*name);

        self.visit_struct_kind(kind);
    }

    fn visit_enum(&mut self, e: &'a cl_ast::Enum) {
        let cl_ast::Enum { name, gens: _, variants } = e;
        // TODO: populate generics?
        self.inner.set_source(Source::Enum(e));
        self.set_name(*name);

        if let Some(variants) = variants {
            variants.iter().for_each(|v| self.visit_variant(v));
        }
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
}

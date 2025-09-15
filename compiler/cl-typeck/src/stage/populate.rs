//! The [Populator] populates entries in the sym table, including span info
use crate::{
    entry::EntryMut,
    handle::Handle,
    source::Source,
    table::{NodeKind, Table},
    type_kind::TypeKind,
};
use cl_ast::{
    ItemKind, Literal, Meta, MetaKind, Sym,
    ast_visitor::{Visit, Walk},
};

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
        let cl_ast::Item { span, attrs, vis: _, kind } = i;
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

        for Meta { name, kind } in &attrs.meta {
            if let ("lang", MetaKind::Equals(Literal::String(s))) = (name.to_ref(), kind) {
                if let Ok(prim) = s.parse() {
                    entry.inner.set_ty(TypeKind::Primitive(prim));
                }
                entry.inner.mark_lang_item(Sym::from(s).to_ref());
            }
        }

        entry.visit_children(i);

        if let (Some(name), child) = (entry.name, entry.inner.id()) {
            self.inner.add_child(name, child);
        }
    }

    fn visit_generics(&mut self, value: &'a cl_ast::Generics) {
        let cl_ast::Generics { vars } = value;
        for var in vars {
            let mut entry = self.inner.new_entry(NodeKind::Type);
            entry.set_ty(TypeKind::Variable);

            let id = entry.id();
            self.inner.add_child(*var, id);
        }
    }

    fn visit_alias(&mut self, a: &'a cl_ast::Alias) {
        let cl_ast::Alias { name, from } = a;
        self.inner.set_source(Source::Alias(a));
        self.set_name(*name);

        self.visit(from);
    }

    fn visit_const(&mut self, c: &'a cl_ast::Const) {
        let cl_ast::Const { name, ty, init } = c;
        self.inner.set_source(Source::Const(c));
        self.inner.set_body(init);
        self.set_name(*name);

        self.visit(ty);
        self.visit(init);
    }

    fn visit_static(&mut self, s: &'a cl_ast::Static) {
        let cl_ast::Static { name, init, .. } = s;
        self.inner.set_source(Source::Static(s));
        self.inner.set_body(init);
        self.set_name(*name);

        s.children(self);
    }

    fn visit_function(&mut self, f: &'a cl_ast::Function) {
        let cl_ast::Function { name, gens, sign, bind, body } = f;
        self.inner.set_source(Source::Function(f));
        self.set_name(*name);

        self.visit(gens);
        self.visit(sign);
        self.visit(bind);

        if let Some(b) = body {
            self.inner.set_body(b);
            self.visit(b);
        }
    }

    fn visit_module(&mut self, m: &'a cl_ast::Module) {
        let cl_ast::Module { name, file } = m;
        self.inner.set_source(Source::Module(m));
        self.set_name(*name);

        self.visit(file);
    }

    fn visit_struct(&mut self, s: &'a cl_ast::Struct) {
        let cl_ast::Struct { name, gens, kind } = s;
        self.inner.set_source(Source::Struct(s));
        self.set_name(*name);

        self.visit(gens);
        self.visit(kind);
    }

    fn visit_enum(&mut self, e: &'a cl_ast::Enum) {
        let cl_ast::Enum { name, gens, variants } = e;
        self.inner.set_source(Source::Enum(e));
        self.set_name(*name);

        self.visit(gens);
        let mut children = Vec::new();
        for variant in variants.iter() {
            let mut entry = self.new_entry(NodeKind::Type);
            variant.visit_in(&mut entry);
            let child = entry.inner.id();
            children.push((variant.name, child));

            self.inner.add_child(variant.name, child);
        }
        self.inner
            .set_ty(TypeKind::Adt(crate::type_kind::Adt::Enum(children)));
    }

    fn visit_variant(&mut self, value: &'a cl_ast::Variant) {
        let cl_ast::Variant { name, kind, body } = value;
        self.inner.set_source(Source::Variant(value));
        self.set_name(*name);
        self.visit(kind);
        match (kind, body) {
            (cl_ast::StructKind::Empty, None) => {
                self.inner.set_ty(TypeKind::Inferred);
            }
            (cl_ast::StructKind::Empty, Some(body)) => {
                self.inner.set_body(body);
            }
            (cl_ast::StructKind::Tuple(_items), None) => {}
            (cl_ast::StructKind::Struct(_struct_members), None) => {}
            (_, Some(body)) => panic!("Unexpected body {body} in enum variant `{value}`"),
        }
    }

    fn visit_impl(&mut self, i: &'a cl_ast::Impl) {
        let cl_ast::Impl { gens, target: _, body } = i;
        self.inner.set_source(Source::Impl(i));
        self.inner.mark_impl_item();

        // We don't know if target is generic yet -- that's checked later.
        for generic in &gens.vars {
            let mut entry = self.new_entry(NodeKind::Type);
            entry.inner.set_ty(TypeKind::Inferred);

            let child = entry.inner.id();
            self.inner.add_child(*generic, child);
        }
        self.visit(body);
    }

    fn visit_use(&mut self, u: &'a cl_ast::Use) {
        let cl_ast::Use { absolute: _, tree } = u;
        self.inner.set_source(Source::Use(u));
        self.inner.mark_use_item();

        self.visit(tree);
    }
}

//! Performs step 1 of type checking: Collecting all the names of things into [Module] units
use crate::{
    definition::{Def, DefKind},
    handle::DefID,
    module::Module as Mod,
    node::{Node, NodeSource},
    project::Project as Prj,
};
use cl_ast::{ast_visitor::Visit, *};
use std::mem;

#[derive(Debug)]
pub struct NameCollector<'prj, 'a> {
    path: cl_ast::Path,
    prj: &'prj mut Prj<'a>,
    parent: DefID,
    retval: Option<DefID>,
}

impl<'prj, 'a> NameCollector<'prj, 'a> {
    /// Constructs a new [NameCollector] out of a [Project](Prj)
    pub fn new(prj: &'prj mut Prj<'a>) -> Self {
        Self { parent: prj.root, prj, path: Default::default(), retval: None }
    }
    /// Constructs a new [NameCollector] out of a [Project](Prj) and a parent [DefID]
    pub fn with_root(prj: &'prj mut Prj<'a>, parent: DefID) -> Self {
        Self { prj, parent, path: Default::default(), retval: None }
    }
    /// Runs the provided function with the given parent
    pub fn with_parent<F, N>(&mut self, parent: DefID, node: N, f: F)
    where F: FnOnce(&mut Self, N) {
        let parent = mem::replace(&mut self.parent, parent);
        f(self, node);
        self.parent = parent;
    }
    /// Extracts the return value from the provided function
    pub fn returns<F, N>(&mut self, node: N, f: F) -> Option<DefID>
    where F: FnOnce(&mut Self, N) {
        let out = self.retval.take();
        f(self, node);
        mem::replace(&mut self.retval, out)
    }
}

impl<'prj, 'a> Visit<'a> for NameCollector<'prj, 'a> {
    fn visit_item(&mut self, i: &'a Item) {
        let Item { extents: _, attrs, vis, kind } = i;
        if let Some(def) = self.returns(kind, Self::visit_item_kind) {
            self.prj[def].set_meta(&attrs.meta).set_vis(*vis);
        }
    }
    fn visit_module(&mut self, m: &'a Module) {
        let Self { prj, parent, path, retval: _ } = self;
        let Module { name, kind } = m;

        let def = Def {
            module: Mod::new(*parent),
            kind: DefKind::Undecided,
            node: Node::new(path.clone(), Some(NodeSource::Module(m))),
        };
        let id = prj.pool.insert(def);
        prj[*parent].module.insert_type(*name, id);
        self.path.push(PathPart::Ident(*name));
        self.with_parent(id, kind, Self::visit_module_kind);
        self.path.pop();
        self.retval = Some(id);
    }
    fn visit_alias(&mut self, a: &'a Alias) {
        let Self { prj, parent, path, retval: _ } = self;
        let Alias { to: name, from: _ } = a;

        let def = Def {
            module: Mod::new(*parent),
            kind: DefKind::Undecided,
            node: Node::new(path.clone(), Some(NodeSource::Alias(a))),
        };
        let id = prj.pool.insert(def);
        prj[*parent].module.insert_type(*name, id);

        self.retval = Some(id);
    }
    fn visit_enum(&mut self, e: &'a Enum) {
        let Self { prj, parent, path, retval: _ } = self;
        let Enum { name, kind } = e;

        let def = Def {
            module: Mod::new(*parent),
            kind: DefKind::Undecided,
            node: Node::new(path.clone(), Some(NodeSource::Enum(e))),
        };
        let id = prj.pool.insert(def);
        prj[*parent].module.insert_type(*name, id);

        self.with_parent(id, kind, Self::visit_enum_kind);
        self.retval = Some(id);
    }
    fn visit_variant(&mut self, v: &'a Variant) {
        let Self { path, prj, parent, retval: _ } = self;
        let Variant { name, kind } = v;

        let def = Def {
            module: Mod::new(*parent),
            kind: DefKind::Undecided,
            node: Node::new(path.clone(), Some(NodeSource::Variant(v))),
        };
        let id = prj.pool.insert(def);
        prj[*parent].module.insert_type(*name, id);

        self.with_parent(id, kind, Self::visit_variant_kind);
        self.retval = Some(id);
    }
    fn visit_struct(&mut self, s: &'a Struct) {
        let Self { prj, parent, path, retval: _ } = self;
        let Struct { name, kind } = s;

        let def = Def {
            module: Mod::new(*parent),
            kind: DefKind::Undecided,
            node: Node::new(path.clone(), Some(NodeSource::Struct(s))),
        };
        let id = prj.pool.insert(def);
        prj[*parent].module.insert_type(*name, id);

        self.with_parent(id, kind, Self::visit_struct_kind);
        self.retval = Some(id);
    }
    fn visit_const(&mut self, c: &'a Const) {
        let Self { prj, parent, path, retval: _ } = self;
        let Const { name, ty: _, init } = c;

        let def = Def {
            module: Mod::new(*parent),
            kind: DefKind::Undecided,
            node: Node::new(path.clone(), Some(NodeSource::Const(c))),
        };
        let id = prj.pool.insert(def);
        prj[*parent].module.insert_value(*name, id);

        self.with_parent(id, &**init, Self::visit_expr);
        self.retval = Some(id);
    }
    fn visit_static(&mut self, s: &'a Static) {
        let Self { prj, parent, path, retval: _ } = self;
        let Static { name, mutable: _, ty: _, init } = s;

        let def = Def {
            module: Mod::new(*parent),
            kind: DefKind::Undecided,
            node: Node::new(path.clone(), Some(NodeSource::Static(s))),
        };
        let id = prj.pool.insert(def);
        prj[*parent].module.insert_value(*name, id);

        self.with_parent(id, &**init, Self::visit_expr);
        self.retval = Some(id);
    }
    fn visit_function(&mut self, f: &'a Function) {
        let Self { prj, parent, path, retval: _ } = self;
        let Function { name, body, .. } = f;

        let def = Def {
            module: Mod::new(*parent),
            kind: DefKind::Undecided,
            node: Node::new(path.clone(), Some(NodeSource::Function(f))),
        };
        let id = prj.pool.insert(def);
        prj[*parent].module.insert_value(*name, id);

        if let Some(body) = body {
            self.with_parent(id, body, Self::visit_block);
        }
        self.retval = Some(id);
    }
    fn visit_impl(&mut self, i: &'a Impl) {
        let Self { prj, parent, path, retval: _ } = self;
        let Impl { target: _, body } = i;
        let def = Def {
            module: Mod::new(*parent),
            kind: DefKind::Undecided,
            node: Node::new(path.clone(), Some(NodeSource::Impl(i))),
        };
        let id = prj.pool.insert(def);

        // items will get reparented after name collection, when target is available
        self.with_parent(id, body, Self::visit_file);

        self.retval = Some(id);
    }
    fn visit_use(&mut self, u: &'a Use) {
        let Self { prj, parent, path, retval } = self;
        let def = Def {
            module: Mod::new(*parent),
            kind: DefKind::Use(*parent),
            node: Node::new(path.clone(), Some(NodeSource::Use(u))),
        };

        let id = prj.pool.insert(def);
        prj[*parent].module.imports.push(id);

        *retval = Some(id);
    }
    fn visit_let(&mut self, l: &'a Let) {
        let Self { prj, parent, path, retval: _ } = self;
        let Let { name, init, .. } = l;
        let def = Def {
            module: Mod::new(*parent),
            kind: DefKind::Undecided,
            node: Node::new(path.clone(), Some(NodeSource::Local(l))),
        };

        let id = prj.pool.insert(def);
        prj[*parent].module.insert_value(*name, id);
        if let Some(expr) = init {
            self.visit_expr(expr)
        }
        self.retval = Some(id)
    }
}

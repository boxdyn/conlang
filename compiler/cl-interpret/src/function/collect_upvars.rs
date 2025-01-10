//! Collects the "Upvars" of a function at the point of its creation, allowing variable capture
use crate::{convalue::ConValue, env::Environment};
use cl_ast::{ast_visitor::visit::*, Function, Let, Param, Path, PathPart, Sym};
use std::collections::{HashMap, HashSet};

pub fn collect_upvars(f: &Function, env: &Environment) -> super::Upvars {
    CollectUpvars::new(env).get_upvars(f)
}

#[derive(Clone, Debug)]
pub struct CollectUpvars<'env> {
    env: &'env Environment,
    upvars: HashMap<Sym, Option<ConValue>>,
    blacklist: HashSet<Sym>,
}

impl<'env> CollectUpvars<'env> {
    pub fn new(env: &'env Environment) -> Self {
        Self { upvars: HashMap::new(), blacklist: HashSet::new(), env }
    }
    pub fn get_upvars(mut self, f: &cl_ast::Function) -> HashMap<Sym, Option<ConValue>> {
        self.visit_function(f);
        self.upvars
    }

    pub fn add_upvar(&mut self, name: &Sym) {
        let Self { env, upvars, blacklist } = self;
        if blacklist.contains(name) || upvars.contains_key(name) {
            return;
        }
        if let Ok(upvar) = env.get(*name) {
            upvars.insert(*name, Some(upvar));
        }
    }

    pub fn bind_name(&mut self, name: &Sym) {
        self.blacklist.insert(*name);
    }
}

impl<'env, 'a> Visit<'a> for CollectUpvars<'env> {
    fn visit_block(&mut self, b: &'a cl_ast::Block) {
        let blacklist = self.blacklist.clone();

        // visit the block
        let cl_ast::Block { stmts } = b;
        stmts.iter().for_each(|s| self.visit_stmt(s));

        // restore the blacklist
        self.blacklist = blacklist;
    }

    fn visit_let(&mut self, l: &'a cl_ast::Let) {
        let Let { mutable, name, ty, init } = l;
        self.visit_mutability(mutable);
        if let Some(ty) = ty {
            self.visit_ty(ty);
        }
        // visit the initializer, which may use the bound name
        if let Some(init) = init {
            self.visit_expr(init)
        }
        // a bound name can never be an upvar
        self.blacklist.insert(*name);
    }

    fn visit_function(&mut self, f: &'a cl_ast::Function) {
        let Function { name: _, sign: _, bind, body } = f;
        // parameters can never be upvars
        for Param { mutability: _, name } in bind {
            self.bind_name(name);
        }
        if let Some(body) = body {
            self.visit_block(body);
        }
    }

    fn visit_for(&mut self, f: &'a cl_ast::For) {
        let cl_ast::For { bind, cond, pass, fail } = f;
        self.visit_expr(cond);
        self.visit_else(fail);
        self.bind_name(bind); // TODO: is bind only bound in the pass block?
        self.visit_block(pass);
    }

    fn visit_path(&mut self, p: &'a cl_ast::Path) {
        // TODO: path resolution in environments
        let Path { absolute: false, parts } = p else {
            return;
        };
        let [PathPart::Ident(name)] = parts.as_slice() else {
            return;
        };
        self.add_upvar(name);
    }

    fn visit_fielder(&mut self, f: &'a cl_ast::Fielder) {
        let cl_ast::Fielder { name, init } = f;
        if let Some(init) = init {
            self.visit_expr(init);
        } else {
            self.add_upvar(name); // fielder without init grabs from env
        }
    }
}

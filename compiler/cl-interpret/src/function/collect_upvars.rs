//! Collects the "Upvars" of a function at the point of its creation, allowing variable capture
use crate::{convalue::ConValue, env::Environment};
use cl_ast::{
    Function, Let, Path, PathPart, Pattern, Sym,
    ast_visitor::{visit::*, walk::Walk},
};
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
        if let Ok(upvar) = env.get_local(*name) {
            upvars.insert(*name, Some(upvar));
        }
    }

    pub fn bind_name(&mut self, name: &Sym) {
        self.blacklist.insert(*name);
    }
}

impl<'a> Visit<'a> for CollectUpvars<'_> {
    fn visit_block(&mut self, b: &'a cl_ast::Block) {
        let blacklist = self.blacklist.clone();

        // visit the block
        b.children(self);

        // restore the blacklist
        self.blacklist = blacklist;
    }

    fn visit_let(&mut self, l: &'a cl_ast::Let) {
        let Let { mutable, name, ty, init } = l;
        self.visit_mutability(mutable);

        ty.visit_in(self);
        // visit the initializer, which may use the bound name
        init.visit_in(self);
        // a bound name can never be an upvar
        self.visit_pattern(name);
    }

    fn visit_function(&mut self, f: &'a cl_ast::Function) {
        let Function { name: _, gens: _, sign: _, bind, body } = f;
        // parameters can never be upvars
        bind.visit_in(self);
        body.visit_in(self);
    }

    fn visit_for(&mut self, f: &'a cl_ast::For) {
        let cl_ast::For { bind, cond, pass, fail } = f;
        self.visit_expr(cond);
        self.visit_else(fail);
        self.visit_pattern(bind);
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

    fn visit_pattern(&mut self, p: &'a cl_ast::Pattern) {
        match p {
            Pattern::Name(name) => {
                self.bind_name(name);
            }
            Pattern::RangeExc(_, _) | Pattern::RangeInc(_, _) => {}
            _ => p.children(self),
        }
    }
}

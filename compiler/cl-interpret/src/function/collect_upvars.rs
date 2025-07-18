//! Collects the "Upvars" of a function at the point of its creation, allowing variable capture
use crate::env::Environment;
use cl_ast::{
    Function, Let, Path, PathPart, Pattern, Sym,
    ast_visitor::{visit::*, walk::Walk},
};
use std::collections::{HashMap, HashSet};

pub fn collect_upvars(f: &Function, env: &Environment) -> super::Upvars {
    CollectUpvars::new(env).visit(f).finish_copied()
}

#[derive(Clone, Debug)]
pub struct CollectUpvars<'env> {
    env: &'env Environment,
    upvars: HashMap<Sym, usize>,
    blacklist: HashSet<Sym>,
}

impl<'env> CollectUpvars<'env> {
    pub fn new(env: &'env Environment) -> Self {
        Self { upvars: HashMap::new(), blacklist: HashSet::new(), env }
    }

    pub fn finish(&mut self) -> HashMap<Sym, usize> {
        std::mem::take(&mut self.upvars)
    }

    pub fn finish_copied(&mut self) -> super::Upvars {
        let Self { env, upvars, blacklist: _ } = self;
        std::mem::take(upvars)
            .into_iter()
            .filter_map(|(k, v)| env.get_id(v).cloned().map(|v| (k, v)))
            .collect()
    }

    pub fn add_upvar(&mut self, name: &Sym) {
        let Self { env, upvars, blacklist } = self;
        if blacklist.contains(name) || upvars.contains_key(name) {
            return;
        }
        if let Ok(place) = env.id_of(*name) {
            upvars.insert(*name, place);
        }
    }

    pub fn bind_name(&mut self, name: &Sym) {
        self.blacklist.insert(*name);
    }

    pub fn scope(&mut self, f: impl Fn(&mut CollectUpvars<'env>)) {
        let blacklist = self.blacklist.clone();

        // visit the scope
        f(self);

        // restore the blacklist
        self.blacklist = blacklist;
    }
}

impl<'a> Visit<'a> for CollectUpvars<'_> {
    fn visit_block(&mut self, b: &'a cl_ast::Block) {
        self.scope(|cu| b.children(cu));
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

    fn visit_pattern(&mut self, value: &'a cl_ast::Pattern) {
        match value {
            Pattern::Name(name) => {
                self.bind_name(name);
            }
            Pattern::RangeExc(_, _) | Pattern::RangeInc(_, _) => {}
            _ => value.children(self),
        }
    }

    fn visit_match_arm(&mut self, value: &'a cl_ast::MatchArm) {
        // MatchArms bind variables with a very small local scope
        self.scope(|cu| value.children(cu));
    }
}

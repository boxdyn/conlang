//! The Resolver's job is to analyze all definitions (all places where a Pat::Name is bound)
//! and imports (all places where a Use::* is found), and construct a tree of modules.
//!
//! Then, that tree of modules can be used to replace all names with their definition ID.
#![expect(unused, reason = "WIP")]

use std::collections::HashMap;

use cl_ast::{
    Bind, DefaultTypes, Expr, Op, Pat, PatOp, Use,
    types::{Path, Symbol},
    visit::{Visit, Walk},
};

#[derive(Clone, Copy, Debug, Default)]
pub enum List<'parent, T> {
    Cons(&'parent List<'parent, T>, T),
    #[default]
    Nil,
}

impl<'parent, T> List<'parent, T> {
    pub fn new(value: T) -> List<'static, T> {
        List::Cons(&List::Nil, value)
    }

    pub fn enter<'a>(&'a self, value: T) -> List<'a, T>
    where T: 'a {
        List::Cons(self, value)
    }

    pub fn parent(&self) -> Option<&List<'parent, T>> {
        match self {
            Self::Cons(parent, _) => Some(parent),
            Self::Nil => None,
        }
    }
}

impl From<List<'_, Symbol>> for Path {
    fn from(value: List<'_, Symbol>) -> Self {
        fn inner(path: &List<'_, Symbol>, vec: &mut Vec<Symbol>) {
            match path {
                List::Nil => {}
                &List::Cons(nested, name) => {
                    inner(nested, vec);
                    vec.push(name);
                }
            }
        }

        let mut parts = vec![];
        inner(&value, &mut parts);
        Self { parts }
    }
}

pub struct Scope {
    pub parent: usize,
    pub path: Path,
    pub is_bound: bool,
    pub definitions: HashMap<Symbol, usize>,
    pub imports: HashMap<Symbol, Path>,
    pub globs: Vec<Path>,
}

impl Scope {
    pub fn new(parent: usize, path: Path) -> Self {
        Scope {
            parent,
            path,
            is_bound: Default::default(),
            definitions: Default::default(),
            imports: Default::default(),
            globs: Default::default(),
        }
    }

    pub fn bind(&mut self) {
        self.is_bound = true;
    }
}

pub struct Resolver<'parent> {
    pub path: List<'parent, Symbol>,
    pub stack: Vec<usize>,
    pub scopes: &'parent mut Vec<Scope>,
}

impl<'parent> Resolver<'parent> {
    pub fn new(modules: &'parent mut Vec<Scope>) -> Resolver<'parent> {
        Resolver { path: List::Nil, stack: Vec::new(), scopes: modules }
    }

    fn at(&self) -> usize {
        self.stack.last().copied().unwrap_or_default()
    }

    fn get(&mut self) -> &mut Scope {
        let at = self.at();
        &mut self.scopes[at]
    }

    fn enter_named(&mut self, symbol: Symbol) -> Resolver<'_> {
        let at = self.at();
        let Resolver { path, stack: _, scopes } = self;
        let next = match scopes[at].definitions.get(&symbol) {
            Some(&scope) => scope,
            None => {
                let next = scopes.len();
                scopes.push(Scope::new(at, Path::from(*path)));
                next
            }
        };
        Resolver { path: path.enter(symbol), stack: vec![next], scopes }
    }

    fn enter_block(&mut self) -> Resolver<'_> {
        let at = self.at();
        let Resolver { path, stack: _, scopes } = self;
        let next = scopes.len();
        scopes.push(Scope::new(at, Path::from(*path)));
        Resolver { path: *path, stack: vec![next], scopes }
    }
}

fn get_name(pat: &Pat<DefaultTypes>) -> Option<Symbol> {
    match pat {
        Pat::Ignore | Pat::Never | Pat::MetId(_) | Pat::Value(_) => None,
        Pat::Name(name) => Some(*name),
        Pat::Op(PatOp::TypePrefixed, pats) => match pats.as_slice() {
            [] => None,
            [Pat::Name(name), ..] => Some(*name),
            [pat, ..] => get_name(pat),
        },
        Pat::Op(PatOp::Generic, pats) => match pats.as_slice() {
            [Pat::Name(name), ..] => Some(*name),
            [..] => None,
        },
        Pat::Op(..) => None,
    }
}

impl<'a, 'parent> Visit<'a, DefaultTypes> for Resolver<'parent> {
    type Error = (); // TODO: error

    fn visit_expr(&mut self, expr: &'a Expr<DefaultTypes>) -> Result<(), Self::Error> {
        match expr {
            Expr::Omitted => Ok(()),
            Expr::Id(_) => Ok(()),
            Expr::MetId(_) => Ok(()),
            Expr::Lit(_) => Ok(()),
            Expr::Use(item) => self.visit_use(item),
            Expr::Bind(bind) => self.visit_bind(bind),
            Expr::Make(make) => self.visit_make(make),
            Expr::Op(Op::Block, exprs) => self.enter_block().visit(exprs),
            Expr::Op(op, annos) => todo!(),
        }
    }

    fn visit_bind(&mut self, item: &'a cl_ast::Bind<DefaultTypes>) -> Result<(), Self::Error> {
        let Bind(op, gens, pat, exprs) = item;

        exprs.visit_in(self)
    }

    fn visit_use(&mut self, item: &'a Use<DefaultTypes>) -> Result<(), Self::Error> {
        let module = self.at();
        let Scope { imports, globs, .. } = &mut self.scopes[module];
        let mut collector = User::new(self.path, imports, globs);
        collector.visit_use(item);

        Ok(())
    }
}

/// Imports [Use] items into a [Module] by their path
pub struct User<'parent> {
    path: List<'parent, Symbol>,
    imports: &'parent mut HashMap<Symbol, Path>,
    globs: &'parent mut Vec<Path>,
}

impl<'parent> User<'parent> {
    pub fn new(
        path: List<'parent, Symbol>,
        imports: &'parent mut HashMap<Symbol, Path>,
        globs: &'parent mut Vec<Path>,
    ) -> Self {
        Self { path, imports, globs }
    }

    pub fn visit_use(&mut self, item: &'parent Use) {
        let Self { path, imports, globs } = self;
        match item {
            Use::Glob => {
                globs.push((*path).into());
            }
            &Use::Name(name) => {
                let path: Path = path.enter(name).into();
                imports.insert(name, path);
            }
            &Use::Alias(name, alias) => {
                let path: Path = path.enter(name).into();
                imports.insert(alias, path);
            }
            Use::Path(name, rest) => {
                User { path: path.enter(*name), imports, globs }.visit_use(rest);
            }
            Use::Tree(items) => {
                items.iter().for_each(|item| self.visit_use(item));
            }
        }
    }
}

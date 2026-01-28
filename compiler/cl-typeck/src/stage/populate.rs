//! The [Populator] populates entries in the sym table, including span info
use std::collections::HashMap;

use crate::{
    entry::EntryMut,
    list::List,
    source::NameFinder,
    table::{NodeKind, Table},
};
use cl_ast::{
    At, Bind, BindOp, DefaultTypes, Expr, Op, Use,
    types::{Path, Symbol},
    visit::{Visit, Walk},
};

#[derive(Debug)]
pub struct Populator<'t, 'parent> {
    entry: EntryMut<'t>,
    meta: List<'parent, &'parent Expr>,
}

impl<'t> Populator<'t, '_> {
    pub fn new(table: &'t mut Table) -> Self {
        Self { entry: table.root_entry_mut(), meta: List::Nil }
    }

    pub fn with_meta<'p, 'e: 'p>(&'p mut self, expr: &'e Expr) -> Populator<'p, 'p> {
        let Self { entry, meta } = self;
        Populator { entry: entry.with_id(entry.id()), meta: meta.enter(expr) }
    }

    pub fn without_meta(&mut self) -> Populator<'_, '_> {
        let Self { entry, meta: _ } = self;
        Populator { entry: entry.with_id(entry.id()), meta: List::Nil }
    }

    pub fn new_entry(&mut self, kind: NodeKind) -> Populator<'_, '_> {
        let entry = self.entry.new_entry(kind);
        Populator { entry, meta: self.meta }
    }
}

impl Visit<'_, DefaultTypes> for Populator<'_, '_> {
    type Error = ();

    fn visit_expr(&mut self, expr: &Expr<DefaultTypes>) -> Result<(), Self::Error> {
        match expr {
            Expr::Omitted | Expr::Id(_) | Expr::MetId(_) | Expr::Lit(_) => Ok(()),
            Expr::Use(item) => self.visit_use(item),
            Expr::Bind(bind) => self.visit_bind(bind),
            Expr::Make(make) => self.visit_make(make),
            // outer meta is collected on the stack, and shared among all scoped items
            Expr::Op(Op::MetaOuter, exprs) => match exprs.as_slice() {
                [At(meta, ..), expr] => self.with_meta(meta).visit(expr),
                _ => unreachable!("MetaOuter is binary"),
            },
            // inner meta is immediately appended to the contextually current item
            Expr::Op(Op::MetaInner, exprs) => match exprs.as_slice() {
                [At(meta, ..), expr] => {
                    self.entry.add_meta(meta.clone());
                    self.visit(expr)
                }
                _ => unreachable!("MetaOuter is binary"),
            },
            Expr::Op(Op::Const, exprs) => self.visit(exprs),
            Expr::Op(Op::Static, exprs) => self.visit(exprs),
            Expr::Op(Op::Match, exprs) => self.new_entry(NodeKind::Scope).visit(exprs),
            Expr::Op(_, exprs) => self.visit(exprs),
        }
    }

    fn visit_bind(&mut self, item: &Bind<DefaultTypes>) -> Result<(), Self::Error> {
        let Bind(op, _ts, pat, _exprs) = item;
        let mut scope = self.new_entry(match op {
            BindOp::Let => NodeKind::Let,
            BindOp::Fn => NodeKind::Function,
            BindOp::Mod => NodeKind::Module,
            BindOp::Impl => NodeKind::Impl,
            BindOp::Type => NodeKind::Type,
            BindOp::Struct => NodeKind::Type,
            BindOp::Enum => NodeKind::Type,
            BindOp::For => NodeKind::Temporary,
            BindOp::Match => NodeKind::Temporary,
        });

        let mut outer: Vec<_> = scope.meta.iter().map(|&e| e.clone()).collect();
        outer.reverse();
        scope.entry.set_meta(outer);

        item.children(&mut scope.without_meta())?;

        match op {
            BindOp::Let | BindOp::For | BindOp::Match => {
                println!("TODO: {op}nodes bind multiple names: {pat}");
                return Ok(());
            }
            BindOp::Fn | BindOp::Mod => {}
            BindOp::Impl => {
                scope.entry.mark_impl_item();
                scope.entry.add_glob(Path::from("super"));
                let ty = match scope.entry.evaluate(pat) {
                    Ok(ty) => ty,
                    Err(_) => Err(())?,
                };

                scope.entry.set_impl_target(ty);
                return Ok(());
            }
            BindOp::Type | BindOp::Struct | BindOp::Enum => println!("TODO: {op}{pat}"),
        }

        if let Some(name) = NameFinder::get(pat) {
            scope.entry.set_name(name);
            let id = scope.entry.id();
            self.entry.add_child(name, id);
        }
        Ok(())
    }

    fn visit_use(&mut self, item: &Use<DefaultTypes>) -> Result<(), Self::Error> {
        let Self { entry: inner, .. } = self;
        let id = inner.id();
        let Table { lazy_imports, glob_imports, .. } = inner.inner_mut();

        User::new(
            List::Nil,
            lazy_imports.entry(id).or_default(),
            glob_imports.entry(id).or_default(),
        )
        .visit_use(item);

        Ok(())
    }
}

/// Imports [Use] items into a module by their path
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

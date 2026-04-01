//! The [Populator] populates entries in the sym table, including span info

use crate::{
    entry::EntryMut,
    list::List,
    table::{NodeKind, Table},
};
use cl_ast::{
    At, Bind, BindOp, DefaultTypes, Expr, Match, MatchArm, Op, Use,
    types::Path,
    visit::{Visit, Walk},
};

mod name_finder;
use name_finder::NameFinder;

mod user;
use user::User;

/// Populates a [Table] with the modules, imports and bindings described in
/// a [syntax tree](Walk).
#[derive(Debug)]
pub struct Populator<'t, 'parent> {
    entry: EntryMut<'t>,
    meta: List<'parent, &'parent Expr>,
}

impl<'t> Populator<'t, '_> {
    /// Constructs a new [Populator] with the given [Table]
    pub fn new(table: &'t mut Table) -> Self {
        Self { entry: table.root_entry_mut(), meta: List::Nil }
    }

    /// Adds an [outer meta attribute](cl_ast::Op::MetaOuter) to the current [List]
    pub fn with_meta<'p, 'e: 'p>(&'p mut self, expr: &'e Expr) -> Populator<'p, 'p> {
        let Self { entry, meta } = self;
        Populator { entry: entry.with_id(entry.id()), meta: meta.enter(expr) }
    }

    /// Creates a [Populator] with an empty [outer meta](cl_ast::Op::MetaOuter) [List]
    pub fn without_meta(&mut self) -> Populator<'_, '_> {
        let Self { entry, meta: _ } = self;
        Populator { entry: entry.with_id(entry.id()), meta: List::Nil }
    }

    /// Creates a populator at a brand-new entry in the [Table]
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
            Expr::Match(mtch) => self.visit_match(mtch),

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

    fn visit_match(&mut self, item: &Match<DefaultTypes>) -> Result<(), Self::Error> {
        let cl_ast::Match(scrutinee, arms) = item;
        scrutinee.visit_in(self)?;

        for arm in arms {
            self.new_entry(NodeKind::Scope).visit(arm)?;
        }

        Ok(())
    }

    fn visit_matcharm(&mut self, item: &MatchArm<DefaultTypes>) -> Result<(), Self::Error> {
        let cl_ast::MatchArm(pat, expr) = item;
        println!("TODO: MatchArm patterns may bind multiple items:\n{pat} => {expr}");
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

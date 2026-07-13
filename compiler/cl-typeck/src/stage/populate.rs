//! The [Populator] populates entries in the sym table, including span info

use crate::{
    entry::EntryMut,
    table::{NodeKind, Table},
};
use cl_ast::{
    At, Bind, BindOp, DefaultTypes, Expr, Match, MatchArm, Op, Pat, PatOp, Use,
    types::Path,
    visit::{Visit, Walk},
};
use cl_structures::{intern::interned::Interned, list::List};

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
    kind: NodeKind,
}

impl<'t, 'parent> Populator<'t, 'parent> {
    /// Constructs a new [Populator] with the given [Table]
    pub fn new(table: &'t mut Table) -> Self {
        Self { entry: table.root_entry_mut(), meta: List::Nil, kind: NodeKind::Root }
    }

    /// Adds an [outer meta attribute](cl_ast::Op::MetaOuter) to the current [List]
    pub fn with_meta<'p, 'e: 'p>(&'p mut self, expr: &'e Expr) -> Populator<'p, 'p> {
        let Self { entry, meta, kind } = self;
        Populator { entry: entry.with_id(entry.id()), meta: meta.enter(expr), kind: *kind }
    }

    /// Creates a [Populator] with an empty [outer meta](cl_ast::Op::MetaOuter) [List]
    pub fn without_meta(&mut self) -> Populator<'_, '_> {
        let Self { entry, meta: _, kind } = self;
        Populator { entry: entry.with_id(entry.id()), meta: List::Nil, kind: *kind }
    }

    pub fn with_kind(
        &mut self,
        kind: NodeKind,
        f: impl FnOnce(&mut Self) -> Result<(), ()>,
    ) -> Result<(), ()> {
        let old_kind = self.kind;
        self.kind = kind;
        let out = f(self);
        self.kind = old_kind;
        out
    }

    /// Creates a populator at a brand-new entry in the [Table]
    pub fn new_entry(&mut self, kind: NodeKind) -> Populator<'_, '_> {
        let mut entry = self.entry.new_entry(kind);

        let mut outer: Vec<_> = self.meta.iter().map(|&e| e.clone()).collect();
        outer.reverse();
        entry.set_meta(outer);
        Populator { entry, meta: self.meta, kind }
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
            Expr::Label(labl) => self.visit_label(labl),

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
        let Bind(op, _ts, pat, exprs) = item;

        let nodekind = match op {
            BindOp::Let => NodeKind::Let,
            BindOp::Fn => NodeKind::Function,
            BindOp::Mod => NodeKind::Module,
            BindOp::Impl => NodeKind::Impl,
            BindOp::Type => NodeKind::Type,
            BindOp::Struct => NodeKind::Type,
            BindOp::Enum => NodeKind::Type,
            BindOp::For => NodeKind::Scope,
        };

        match nodekind {
            NodeKind::Root | NodeKind::Const | NodeKind::Static => todo!("Root!"),
            NodeKind::Module | NodeKind::Function => {
                let mut scope = self.new_entry(nodekind);
                if let Some(name) = NameFinder::get(pat) {
                    scope.entry.set_name(name);
                    let id = scope.entry.id();

                    scope.entry.parent().unwrap().add_child(name, id);
                    exprs.visit_in(&mut scope.without_meta())?;
                }
            }
            NodeKind::Type | NodeKind::Let => {
                self.with_kind(nodekind, |scope| {
                    exprs.visit_in(&mut scope.without_meta())?;
                    pat.visit_in(scope)
                })?;
                return Ok(());
            }
            NodeKind::Temporary | NodeKind::Scope => {
                let mut scope = self.new_entry(nodekind);
                // TODO: proper eval order for For
                scope.entry.add_glob(Path::from("super"));
                pat.visit_in(&mut scope)?;
                exprs.visit_in(&mut scope.without_meta())?;
            }
            NodeKind::Impl => {
                let mut scope = self.new_entry(nodekind);

                scope.entry.mark_impl_item();
                scope.entry.add_glob(Path::from("super"));
                let ty = match scope.entry.evaluate(pat) {
                    Ok(ty) => ty,
                    Err(_) => Err(())?,
                };

                scope.entry.set_impl_target(ty);
                let se = format!("impl {}", pat);
                scope.entry.set_name(Interned::from(&*se));
                exprs.visit_in(&mut scope.without_meta())?;
            }
            NodeKind::Use => todo!(),
        }
        Ok(())
    }

    fn visit_pat(&mut self, item: &'_ cl_ast::Pat<DefaultTypes>) -> Result<(), Self::Error> {
        match item {
            Pat::Name(name) => {
                let mut scope = self.new_entry(self.kind);

                scope.entry.set_name(*name);
                let id = scope.entry.id();
                self.entry.add_child(*name, id);
                Ok(())
            }
            // TODO: record patterns, guard-let patterns, etc.
            Pat::Op(PatOp::Typed, b) if let [n, _ty] = b.as_slice() => n.visit_in(self),
            other => other.children(self),
        }
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

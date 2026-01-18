//! The [Populator] populates entries in the sym table, including span info
use crate::{
    entry::EntryMut,
    handle::Handle,
    table::{NodeKind, Table},
};
use cl_ast::{
    Bind, BindOp, DefaultTypes, Expr, Op, Use,
    types::Symbol,
    visit::{Visit, Walk},
};

#[derive(Debug)]
pub struct Populator<'t, 'a> {
    inner: EntryMut<'t, 'a>,
    name: Option<Symbol>, // this is a hack to get around the Visitor interface
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

    pub fn set_name(&mut self, name: Symbol) {
        self.name = Some(name);
    }
}

impl<'a> Visit<'a, DefaultTypes> for Populator<'_, 'a> {
    type Error = ();

    fn visit_expr(&mut self, expr: &'a Expr<DefaultTypes>) -> Result<(), Self::Error> {
        match expr {
            Expr::Omitted => Ok(()),
            Expr::Id(_) => Ok(()), // Maybe we could eagerly populate add all paths?
            Expr::MetId(_) => Ok(()),
            Expr::Lit(_) => Ok(()),
            Expr::Use(item) => self.visit_use(item),
            Expr::Bind(bind) => self.visit_bind(bind),
            Expr::Make(make) => self.visit_make(make),
            Expr::Op(Op::Meta, exprs) => self.visit(exprs),
            Expr::Op(Op::Const, exprs) => self.visit(exprs),
            Expr::Op(Op::Static, exprs) => self.visit(exprs),
            Expr::Op(_, exprs) => self.visit(exprs),
        }
    }

    fn visit_bind(&mut self, item: &'a Bind<DefaultTypes>) -> Result<(), Self::Error> {
        let Bind(op, _ts, _pat, _exprs) = item;

        match op {
            BindOp::Let => println!("TODO: {item}"),
            BindOp::Type => println!("TODO: {item}"),
            BindOp::Fn => println!("TODO: {item}"),
            BindOp::Mod => println!("TODO: {item}"),
            BindOp::Impl => println!("TODO: {item}"),
            BindOp::Struct => println!("TODO: {item}"),
            BindOp::Enum => println!("TODO: {item}"),
            BindOp::For => println!("TODO: {item}"),
            BindOp::Match => println!("TODO: {item}"),
        }
        item.children(self)
    }

    fn visit_use(&mut self, item: &'a Use<DefaultTypes>) -> Result<(), Self::Error> {
        todo!("Create lazy back-edges for each use-binding in {item}")
    }
}

impl<'a> Populator<'_, 'a> {}

//! A [TypeExpression] is a [syntactic](cl_ast) representation of a [TypeKind], and is used to
//! construct type bindings in a [Table]'s typing context.

use crate::{
    consteval::ConstEval,
    table::{Scope, Table},
    type_kind::TypeKind,
};
use cl_ast::{AstNode, AstTypes, At, Expr, Pat, PatOp, types::Symbol};

#[derive(Clone, Debug, PartialEq, Eq)] // TODO: impl Display and Error
pub enum Error {
    BadPath { parent: Scope, path: Vec<Symbol> },
    ConstEval { parent: Scope, eval: Box<At<Expr>> },
}

impl std::error::Error for Error {}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::BadPath { parent, path } => {
                write!(f, "No item at path {parent}")?;
                for part in path {
                    write!(f, "::{part}")?;
                }
            }
            Error::ConstEval { parent, eval } => {
                write!(f, "Failed to evaluate constant {eval} in {parent}")?;
            }
        }
        Ok(())
    }
}

/// A [TypeExpression] is a syntactic representation of a [TypeKind], and is used to construct
/// type bindings in a [Table]'s typing context.
pub trait TypeExpression<Out = Scope> {
    /// Evaluates a type expression, recursively creating intermediate bindings.
    fn evaluate(&self, table: &mut Table, node: Scope) -> Result<Out, Error>;
}

impl TypeExpression for Pat {
    fn evaluate(&self, table: &mut Table, node: Scope) -> Result<Scope, Error> {
        match self {
            Pat::Ignore => Ok(table.inferred_type()),
            Pat::Never => Ok(table.get_lang_item("never")),
            Pat::MetId(_) => todo!(),
            Pat::Name(name) => name.evaluate(table, node),
            Pat::Value(expr) => expr.0.evaluate(table, node),

            Pat::Op(PatOp::MetaInner | PatOp::MetaOuter, pats) if let [pat] = &pats[..] => {
                pat.evaluate(table, node)
            }
            Pat::Op(PatOp::Pub, pats) if let [pat] = &pats[..] => pat.evaluate(table, node),
            Pat::Op(PatOp::Mut, pats) if let [pat] = &pats[..] => pat.evaluate(table, node),
            Pat::Op(PatOp::Ref, pats) if let [pat] = &pats[..] => {
                let ty = pat.evaluate(table, node)?;
                Ok(table.anon_type(TypeKind::Ref(ty)))
            }
            Pat::Op(PatOp::Ptr, pats) if let [pat] = &pats[..] => {
                let ty = pat.evaluate(table, node)?;
                Ok(table.anon_type(TypeKind::Ptr(ty)))
            }
            Pat::Op(PatOp::Guard, pats) if let [pat, _g] = &pats[..] => pat.evaluate(table, node),
            Pat::Op(PatOp::Rest, _pats) => Ok(table.inferred_type()),
            Pat::Op(PatOp::RangeEx, _pats) => todo!(),
            Pat::Op(PatOp::RangeIn, _pats) => todo!(),
            Pat::Op(PatOp::Record, pats) => {
                todo!("Anonymous record destructuring {pats:?} in {self}")
            }
            Pat::Op(PatOp::Tuple, pats) => {
                let tys = pats.evaluate(table, node)?;
                Ok(table.anon_type(TypeKind::Tuple(tys)))
            }
            Pat::Op(PatOp::Slice, _) => todo!(""),
            Pat::Op(PatOp::ArRep, pats) if let [pat, rep] = &pats[..] => {
                let ty = pat.evaluate(table, node)?;
                let rep = match rep.value() {
                    Self::Value(expr) => expr
                        .const_eval()
                        .and_then(|v| v.uint())
                        .ok_or_else(|| Error::ConstEval { parent: node, eval: expr.clone() }),
                    _ => todo!("{rep} in array-repetition patterns"),
                }?;
                Ok(table.anon_type(TypeKind::Array(ty, rep as _)))
            }
            Pat::Op(PatOp::Typed, pats) if let [_, pat] = &pats[..] => {
                Ok(pat.evaluate(table, node)?)
            }
            Pat::Op(PatOp::TypePrefixed, pats) => todo!("TypePrefixed {pats:?}"),
            Pat::Op(PatOp::Generic, pats) if let [pat, ..] = &pats[..] => {
                Ok(pat.evaluate(table, node)?)
            }
            Pat::Op(PatOp::Fn, pats) => todo!("Fn {pats:?}"),
            Pat::Op(PatOp::Alt, pats) => todo!("Alt {pats:?}"),
            _ => unreachable!(),
        }
    }
}

impl TypeExpression for cl_ast::Expr {
    fn evaluate(&self, table: &mut Table, node: Scope) -> Result<Scope, Error> {
        match self {
            Self::Omitted => Ok(table.anon_type(TypeKind::Tuple(vec![]))),
            Self::Id(path) => path.evaluate(table, node),
            Self::MetId(_) => todo!("Metaidentifiers in resolver"),
            Self::Lit(lit) => todo!("Literals ({lit}) in type expressions!"),
            Self::Use(item) => todo!("Use-items ({item}) in type expressions!??!"),
            Self::Bind(bind) => todo!("Bind-items ({bind}) in type expressions!"),
            Self::Make(make) => todo!("Make-items ({make}) in type expressions!"),
            Self::Match(mtch) => todo!("Match-exprs ({mtch}) in type expressions!"),
            Self::Label(labl) => todo!("Label-exprs ({labl}) in type expressions!"),
            Self::Op(op, ats) => todo!("Op({op}, {ats:?})"),
        }
    }
}

impl TypeExpression for cl_ast::types::Path {
    fn evaluate(&self, table: &mut Table, node: Scope) -> Result<Scope, Error> {
        let Self { parts } = self;
        parts.evaluate(table, node)
    }
}

impl TypeExpression for [Symbol] {
    fn evaluate(&self, table: &mut Table, node: Scope) -> Result<Scope, Error> {
        table
            .nav(node, self)
            .ok_or_else(|| Error::BadPath { parent: node, path: self.to_owned() })
    }
}

impl TypeExpression for Symbol {
    fn evaluate(&self, table: &mut Table, node: Scope) -> Result<Scope, Error> {
        let path = [*self];
        table
            .nav(node, &path)
            .ok_or_else(|| Error::BadPath { parent: node, path: path.to_vec() })
    }
}

impl<T: TypeExpression<U>, U> TypeExpression<Vec<U>> for [T] {
    fn evaluate(&self, table: &mut Table, node: Scope) -> Result<Vec<U>, Error> {
        let mut out = Vec::with_capacity(self.len());
        for te in self {
            out.push(te.evaluate(table, node)?) // try_collect is unstable
        }
        Ok(out)
    }
}

impl<T: TypeExpression<U> + AstNode, U, A: AstTypes> TypeExpression<U> for At<T, A> {
    fn evaluate(&self, table: &mut Table, node: Scope) -> Result<U, Error> {
        self.0.evaluate(table, node)
    }
}

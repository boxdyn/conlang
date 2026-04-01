//! A [TypeExpression] is a [syntactic](cl_ast) representation of a [TypeKind], and is used to
//! construct type bindings in a [Table]'s typing context.

use crate::{consteval::ConstEval, handle::Handle, table::Table, type_kind::TypeKind};
use cl_ast::{At, Expr, Pat, PatOp, types::Symbol};

#[derive(Clone, Debug, PartialEq, Eq)] // TODO: impl Display and Error
pub enum Error {
    BadPath { parent: Handle, path: Vec<Symbol> },
    ConstEval { parent: Handle, eval: Box<At<Expr>> },
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
pub trait TypeExpression<Out = Handle> {
    /// Evaluates a type expression, recursively creating intermediate bindings.
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Out, Error>;
}

impl TypeExpression for Pat {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Handle, Error> {
        match self {
            Pat::Ignore => Ok(table.inferred_type()),
            Pat::Never => Ok(table.get_lang_item("never")),
            Pat::MetId(_) => todo!(),
            Pat::Name(name) => name.evaluate(table, node),
            Pat::Value(expr) => expr.0.evaluate(table, node),

            Pat::Op(PatOp::Pub, pats) => match pats.as_slice() {
                [pat] => pat.evaluate(table, node),
                _ => unreachable!(),
            },
            Pat::Op(PatOp::Mut, pats) => match pats.as_slice() {
                [pat] => pat.evaluate(table, node),
                _ => unreachable!(),
            },
            Pat::Op(PatOp::Ref, pats) => match pats.as_slice() {
                [pat] => {
                    let ty = pat.evaluate(table, node)?;
                    Ok(table.anon_type(TypeKind::Ref(ty)))
                }
                _ => unreachable!(),
            },
            Pat::Op(PatOp::Ptr, pats) => match pats.as_slice() {
                [pat] => {
                    let ty = pat.evaluate(table, node)?;
                    Ok(table.anon_type(TypeKind::Ptr(ty)))
                }
                _ => unreachable!(),
            },
            Pat::Op(PatOp::Rest, _pats) => Ok(table.inferred_type()),
            Pat::Op(PatOp::RangeEx, _pats) => todo!(),
            Pat::Op(PatOp::RangeIn, _pats) => todo!(),
            Pat::Op(PatOp::Record, pats) => {
                let tys = pats.evaluate(table, node)?;
                todo!("Anonymous record destructuring {tys:?} in {self}")
            }
            Pat::Op(PatOp::Tuple, pats) => {
                let tys = pats.evaluate(table, node)?;
                Ok(table.anon_type(TypeKind::Tuple(tys)))
            }
            Pat::Op(PatOp::Slice, _) => todo!(""),
            Pat::Op(PatOp::ArRep, pats) => match pats.as_slice() {
                [pat, rep] => {
                    let ty = pat.evaluate(table, node)?;
                    let rep = match rep {
                        Self::Value(at) => at
                            .const_eval()
                            .ok_or_else(|| Error::ConstEval { parent: node, eval: at.clone() }),
                        _ => todo!("{rep} in array-repetition patterns"),
                    }?;
                    Ok(table.anon_type(TypeKind::Array(ty, rep as _)))
                }
                _ => unreachable!(),
            },
            Pat::Op(PatOp::Typed, pats) => match pats.as_slice() {
                [_, pat] => Ok(pat.evaluate(table, node)?),
                _ => unreachable!(),
            },
            Pat::Op(PatOp::TypePrefixed, pats) => todo!("TypePrefixed {pats:?}"),
            Pat::Op(PatOp::Generic, pats) => match pats.as_slice() {
                [pat, ..] => Ok(pat.evaluate(table, node)?),
                _ => unreachable!(),
            },
            Pat::Op(PatOp::Fn, pats) => todo!("Fn {pats:?}"),
            Pat::Op(PatOp::Alt, pats) => todo!("Alt {pats:?}"),
        }
    }
}

impl TypeExpression for cl_ast::Expr {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Handle, Error> {
        match self {
            Self::Omitted => Ok(table.anon_type(TypeKind::Tuple(vec![]))),
            Self::Id(path) => path.evaluate(table, node),
            Self::MetId(_) => todo!("Metaidentifiers in resolver"),
            Self::Lit(lit) => todo!("Literals ({lit}) in type expressions!"),
            Self::Use(item) => todo!("Use-items ({item}) in type expressions!??!"),
            Self::Bind(bind) => todo!("Bind-items ({bind}) in type expressions!"),
            Self::Make(make) => todo!("Make-items ({make}) in type expressions!"),
            Self::Match(mtch) => todo!("Match-exprs ({mtch}) in type expressions!"),
            Self::Op(op, ats) => todo!("Op({op}, {ats:?})"),
        }
    }
}

impl TypeExpression for cl_ast::types::Path {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Handle, Error> {
        let Self { parts } = self;
        parts.evaluate(table, node)
    }
}

impl TypeExpression for [Symbol] {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Handle, Error> {
        table
            .nav(node, self)
            .ok_or_else(|| Error::BadPath { parent: node, path: self.to_owned() })
    }
}

impl TypeExpression for Symbol {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Handle, Error> {
        let path = [*self];
        table
            .nav(node, &path)
            .ok_or_else(|| Error::BadPath { parent: node, path: path.to_vec() })
    }
}

impl<T: TypeExpression<U>, U> TypeExpression<Vec<U>> for [T] {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Vec<U>, Error> {
        let mut out = Vec::with_capacity(self.len());
        for te in self {
            out.push(te.evaluate(table, node)?) // try_collect is unstable
        }
        Ok(out)
    }
}

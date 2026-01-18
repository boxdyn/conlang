//! A [TypeExpression] is a [syntactic](cl_ast) representation of a [TypeKind], and is used to
//! construct type bindings in a [Table]'s typing context.

use crate::{handle::Handle, table::Table};
use cl_ast::{Pat, PatOp, types::Symbol};

#[derive(Clone, Debug, PartialEq, Eq)] // TODO: impl Display and Error
pub enum Error {
    BadPath { parent: Handle, path: Vec<Symbol> },
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
            Pat::Value(_) => todo!("Evaluate expressions as type expressions in {self}"),

            Pat::Op(PatOp::Pub, _pats) => todo!(),
            Pat::Op(PatOp::Mut, _pats) => todo!(),
            Pat::Op(PatOp::Ref, _pats) => todo!(),
            Pat::Op(PatOp::Ptr, _pats) => todo!(),
            Pat::Op(PatOp::Rest, _pats) => todo!(),
            Pat::Op(PatOp::RangeEx, _pats) => todo!(),
            Pat::Op(PatOp::RangeIn, _pats) => todo!(),
            Pat::Op(PatOp::Record, _pats) => todo!(),
            Pat::Op(PatOp::Tuple, _pats) => todo!(),
            Pat::Op(PatOp::Slice, _pats) => todo!(),
            Pat::Op(PatOp::ArRep, _pats) => todo!(),
            Pat::Op(PatOp::Typed, _pats) => todo!(),
            Pat::Op(PatOp::TypePrefixed, _pats) => todo!(),
            Pat::Op(PatOp::Generic, _pats) => todo!(),
            Pat::Op(PatOp::Fn, _pats) => todo!(),
            Pat::Op(PatOp::Alt, _pats) => todo!(),
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

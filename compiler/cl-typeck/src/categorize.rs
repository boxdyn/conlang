//! Categorizes all top-level entries in a table

use crate::{
    handle::Handle,
    source::Source,
    table::Table,
    type_expression::{Error as TypeEval, TypeExpression},
    type_kind::TypeKind,
};
use cl_ast::*;

/// Ensures a type entry exists for the provided handle in the table
pub fn categorize(table: &mut Table, node: Handle) -> CatResult<()> {
    let Some(source) = table.source(node) else {
        Err(Error::NoSource)?
    };

    match source {
        Source::Root => {}
        Source::Module(_) => {}
        Source::Alias(a) => categorize_alias(table, node, a)?,
        Source::Enum(_) => todo!(),
        Source::Variant(_) => todo!(),
        Source::Struct(_) => todo!(),
        Source::Const(_) => todo!(),
        Source::Static(_) => todo!(),
        Source::Function(_) => todo!(),
        Source::Local(_) => todo!(),
        Source::Impl(_) => todo!(),
        Source::Use(_) => todo!(),
        Source::Ty(ty) => {
            ty.evaluate(table, node)?;
        }
    }

    todo!("categorize {node} in {table:?}")
}

pub fn categorize_alias(table: &mut Table, node: Handle, a: &Alias) -> CatResult<()> {
    let kind = match &a.from {
        Some(ty) => TypeKind::Alias(ty.evaluate(table, node)?),
        None => TypeKind::Empty,
    };
    table.set_ty(node, kind);

    Ok(())
}

pub fn categorize_const(table: &mut Table, node: Handle, c: &Const) -> CatResult<()> {
    let kind = TypeKind::Alias(c.ty.evaluate(table, node)?);

    table.set_ty(node, kind);
    Ok(())
}

type CatResult<T> = Result<T, Error>;

pub enum Error {
    NoSource,
    BadMeta(Meta),
    Recursive(Handle),
    TypeEval(TypeEval),
}

impl From<TypeEval> for Error {
    fn from(value: TypeEval) -> Self {
        Error::TypeEval(value)
    }
}

//! A [TypeExpression] is a [syntactic](cl_ast) representation of a [TypeKind], and is used to
//! construct type bindings in a [Table]'s typing context.

use crate::{handle::Handle, table::Table, type_kind::TypeKind};
use cl_ast::{PathPart, Sym, Ty, TyArray, TyFn, TyKind, TyPtr, TyRef, TySlice, TyTuple};

#[derive(Clone, Debug, PartialEq, Eq)] // TODO: impl Display and Error
pub enum Error {
    BadPath { parent: Handle, path: Vec<PathPart> },
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

impl TypeExpression for Ty {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Handle, Error> {
        self.kind.evaluate(table, node)
    }
}

impl TypeExpression for TyKind {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Handle, Error> {
        match self {
            TyKind::Never => Ok(table.get_lang_item("never")),
            TyKind::Infer => Ok(table.inferred_type()),
            TyKind::Path(p) => p.evaluate(table, node),
            TyKind::Array(a) => a.evaluate(table, node),
            TyKind::Slice(s) => s.evaluate(table, node),
            TyKind::Tuple(t) => t.evaluate(table, node),
            TyKind::Ref(r) => r.evaluate(table, node),
            TyKind::Ptr(r) => r.evaluate(table, node),
            TyKind::Fn(f) => f.evaluate(table, node),
        }
    }
}

impl TypeExpression for cl_ast::Path {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Handle, Error> {
        let Self { absolute, parts } = self;
        parts.evaluate(table, if *absolute { table.root() } else { node })
    }
}

impl TypeExpression for [PathPart] {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Handle, Error> {
        table
            .nav(node, self)
            .ok_or_else(|| Error::BadPath { parent: node, path: self.to_owned() })
    }
}

impl TypeExpression for Sym {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Handle, Error> {
        let path = [PathPart::Ident(*self)];
        table
            .nav(node, &path)
            .ok_or_else(|| Error::BadPath { parent: node, path: path.to_vec() })
    }
}

impl TypeExpression for TyArray {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Handle, Error> {
        let Self { ty, count } = self;
        let kind = TypeKind::Array(ty.evaluate(table, node)?, *count);
        Ok(table.anon_type(kind))
    }
}

impl TypeExpression for TySlice {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Handle, Error> {
        let Self { ty } = self;
        let kind = TypeKind::Slice(ty.evaluate(table, node)?);
        Ok(table.anon_type(kind))
    }
}

impl TypeExpression for TyTuple {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Handle, Error> {
        let Self { types } = self;
        let kind = TypeKind::Tuple(types.evaluate(table, node)?);
        Ok(table.anon_type(kind))
    }
}

impl TypeExpression for TyRef {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Handle, Error> {
        let Self { mutable: _, count, to } = self;
        let mut t = to.evaluate(table, node)?;
        for _ in 0..*count {
            let kind = TypeKind::Ref(t);
            t = table.anon_type(kind)
        }
        Ok(t)
    }
}

impl TypeExpression for TyPtr {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Handle, Error> {
        let Self { to } = self;
        let mut t = to.evaluate(table, node)?;
        t = table.anon_type(TypeKind::Ptr(t));
        Ok(t)
    }
}

impl TypeExpression for TyFn {
    fn evaluate(&self, table: &mut Table, node: Handle) -> Result<Handle, Error> {
        let Self { args, rety } = self;
        let kind = TypeKind::FnSig {
            args: args.evaluate(table, node)?,
            rety: rety.evaluate(table, node)?,
        };
        Ok(table.anon_type(kind))
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

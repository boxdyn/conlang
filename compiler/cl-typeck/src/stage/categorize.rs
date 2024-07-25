//! Categorizes an entry in a table according to its embedded type information

use crate::{
    handle::Handle,
    source::Source,
    table::{NodeKind, Table},
    type_expression::{Error as TypeEval, TypeExpression},
    type_kind::{Adt, TypeKind},
};
use cl_ast::*;

/// Ensures a type entry exists for the provided handle in the table
pub fn categorize(table: &mut Table, node: Handle) -> CatResult<()> {
    if let Some(meta) = table.meta(node) {
        for meta @ Meta { name, kind } in meta {
            if let ("intrinsic", MetaKind::Equals(Literal::String(s))) = (&**name, kind) {
                let kind =
                    TypeKind::Intrinsic(s.parse().map_err(|_| Error::BadMeta(meta.clone()))?);
                table.set_ty(node, kind);
                return Ok(());
            }
        }
    }

    let Some(source) = table.source(node) else {
        return Ok(());
    };

    match source {
        Source::Root => Ok(()),
        Source::Module(_) => Ok(()),
        Source::Alias(a) => cat_alias(table, node, a),
        Source::Enum(e) => cat_enum(table, node, e),
        Source::Variant(_) => Ok(()),
        Source::Struct(s) => cat_struct(table, node, s),
        Source::Const(c) => cat_const(table, node, c),
        Source::Static(s) => cat_static(table, node, s),
        Source::Function(f) => cat_function(table, node, f),
        Source::Local(l) => cat_local(table, node, l),
        Source::Impl(i) => cat_impl(table, node, i),
        Source::Use(_) => Ok(()),
        Source::Ty(ty) => ty
            .evaluate(table, node)
            .map_err(|e| Error::TypeEval(e, " while categorizing a type"))
            .map(drop),
    }
}

fn parent(table: &Table, node: Handle) -> Handle {
    table.parent(node).copied().unwrap_or(node)
}

fn cat_alias(table: &mut Table, node: Handle, a: &Alias) -> CatResult<()> {
    let parent = parent(table, node);
    let kind = match &a.from {
        Some(ty) => TypeKind::Instance(
            ty.evaluate(table, parent)
                .map_err(|e| Error::TypeEval(e, " while categorizing an alias"))?,
        ),
        None => TypeKind::Empty,
    };
    table.set_ty(node, kind);

    Ok(())
}

fn cat_struct(table: &mut Table, node: Handle, s: &Struct) -> CatResult<()> {
    let parent = parent(table, node);
    let Struct { name: _, kind } = s;
    let kind = match kind {
        StructKind::Empty => TypeKind::Adt(Adt::UnitStruct),
        StructKind::Tuple(types) => {
            let mut out = vec![];
            for ty in types {
                out.push((Visibility::Public, ty.evaluate(table, parent)?))
            }
            TypeKind::Adt(Adt::TupleStruct(out))
        }
        StructKind::Struct(members) => {
            let mut out = vec![];
            for m in members {
                out.push(cat_member(table, node, m)?)
            }
            TypeKind::Adt(Adt::Struct(out))
        }
    };

    table.set_ty(node, kind);
    Ok(())
}

fn cat_member(
    table: &mut Table,
    node: Handle,
    m: &StructMember,
) -> CatResult<(Sym, Visibility, Handle)> {
    let StructMember { vis, name, ty } = m;
    Ok((*name, *vis, ty.evaluate(table, node)?))
}

fn cat_enum<'a>(table: &mut Table<'a>, node: Handle, e: &'a Enum) -> CatResult<()> {
    let Enum { name: _, kind } = e;
    let kind = match kind {
        EnumKind::NoVariants => TypeKind::Adt(Adt::Enum(vec![])),
        EnumKind::Variants(variants) => {
            let mut out_vars = vec![];
            for v in variants {
                out_vars.push(cat_variant(table, node, v)?)
            }
            TypeKind::Adt(Adt::Enum(out_vars))
        }
    };

    table.set_ty(node, kind);
    Ok(())
}

fn cat_variant<'a>(
    table: &mut Table<'a>,
    node: Handle,
    v: &'a Variant,
) -> CatResult<(Sym, Option<Handle>)> {
    let parent = parent(table, node);
    let Variant { name, kind } = v;
    match kind {
        VariantKind::Plain => Ok((*name, None)),
        VariantKind::CLike(c) => todo!("enum-variant constant {c}"),
        VariantKind::Tuple(ty) => {
            let ty = ty
                .evaluate(table, parent)
                .map_err(|e| Error::TypeEval(e, " while categorizing a variant"))?;
            Ok((*name, Some(ty)))
        }
        VariantKind::Struct(members) => {
            let mut out = vec![];
            for m in members {
                out.push(cat_member(table, node, m)?)
            }
            let kind = TypeKind::Adt(Adt::Struct(out));

            let mut h = node.to_entry_mut(table);
            let mut variant = h.new_entry(NodeKind::Type);
            variant.set_source(Source::Variant(v));
            variant.set_ty(kind);
            Ok((*name, Some(variant.id())))
        }
    }
}

fn cat_const(table: &mut Table, node: Handle, c: &Const) -> CatResult<()> {
    let parent = parent(table, node);
    let kind = TypeKind::Instance(
        c.ty.evaluate(table, parent)
            .map_err(|e| Error::TypeEval(e, " while categorizing a const"))?,
    );
    table.set_ty(node, kind);
    Ok(())
}

fn cat_static(table: &mut Table, node: Handle, s: &Static) -> CatResult<()> {
    let parent = parent(table, node);
    let kind = TypeKind::Instance(
        s.ty.evaluate(table, parent)
            .map_err(|e| Error::TypeEval(e, " while categorizing a static"))?,
    );
    table.set_ty(node, kind);
    Ok(())
}

fn cat_function(table: &mut Table, node: Handle, f: &Function) -> CatResult<()> {
    let parent = parent(table, node);
    let kind = TypeKind::Instance(
        f.sign
            .evaluate(table, parent)
            .map_err(|e| Error::TypeEval(e, " while categorizing a function"))?,
    );
    table.set_ty(node, kind);
    Ok(())
}

fn cat_local(table: &mut Table, node: Handle, l: &Let) -> CatResult<()> {
    let parent = parent(table, node);
    if let Some(ty) = &l.ty {
        let kind = ty
            .evaluate(table, parent)
            .map_err(|e| Error::TypeEval(e, " while categorizing a let binding"))?;
        table.set_ty(node, TypeKind::Instance(kind));
    }
    Ok(())
}

fn cat_impl(table: &mut Table, node: Handle, i: &Impl) -> CatResult<()> {
    let parent = parent(table, node);
    let Impl { target, body: _ } = i;
    let target = match target {
        ImplKind::Type(t) => t.evaluate(table, parent),
        ImplKind::Trait { impl_trait: _, for_type: t } => t.evaluate(table, parent),
    }?;

    table.set_impl_target(node, target);
    Ok(())
}

type CatResult<T> = Result<T, Error>;

#[derive(Clone, Debug)]
pub enum Error {
    BadMeta(Meta),
    Recursive(Handle),
    TypeEval(TypeEval, &'static str),
}

impl From<TypeEval> for Error {
    fn from(value: TypeEval) -> Self {
        Error::TypeEval(value, "")
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::BadMeta(meta) => write!(f, "Unknown meta attribute: #[{meta}]"),
            Error::Recursive(id) => {
                write!(f, "Encountered recursive type without indirection: {id}")
            }
            Error::TypeEval(e, during) => write!(f, "{e}{during}"),
        }
    }
}

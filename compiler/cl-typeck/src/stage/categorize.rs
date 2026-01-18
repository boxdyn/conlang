//! Categorizes an entry in a table according to its embedded type information
#![allow(unused)]
use crate::{
    entry::EntryMut,
    handle::Handle,
    source::Source,
    table::{NodeKind, Table},
    type_expression::{Error as TypeEval, TypeExpression},
    type_kind::{Adt, TypeKind},
};
use cl_ast::*;

/// Ensures a type entry exists for the provided handle in the table
pub fn categorize(table: &mut Table, node: Handle) -> CatResult<()> {
    let Some(source) = table.source(node) else {
        return Ok(());
    };

    match source {
        Source::Root => {}
        Source::Binding(item) => todo!("Categorize {item}"),
        Source::Use(import) => todo!("Categorize {import}"),
        Source::Ty(pat) => todo!("Categorize {pat}"),
    }
    Ok(())
}

fn parent(table: &Table, node: Handle) -> Handle {
    table.parent(node).copied().unwrap_or(node)
}

// fn cat_struct(table: &mut Table, node: Handle, s: &Struct) -> CatResult<()> {
//     let Struct { name: _, gens: _, kind } = s;
//     // TODO: Generics
//     let kind = match kind {
//         StructKind::Empty => TypeKind::Adt(Adt::UnitStruct),
//         StructKind::Tuple(types) => {
//             let mut out = vec![];
//             for ty in types {
//                 out.push((Visibility::Public, ty.evaluate(table, node)?))
//             }
//             TypeKind::Adt(Adt::TupleStruct(out))
//         }
//         StructKind::Struct(members) => {
//             let mut out = vec![];
//             for m in members {
//                 out.push(cat_member(table, node, m)?)
//             }
//             TypeKind::Adt(Adt::Struct(out))
//         }
//     };

//     table.set_ty(node, kind);
//     Ok(())
// }

// fn cat_member(
//     table: &mut Table,
//     node: Handle,
//     m: &StructMember,
// ) -> CatResult<(Sym, Visibility, Handle)> {
//     let StructMember { vis, name, ty } = m;
//     Ok((*name, *vis, ty.evaluate(table, node)?))
// }

// fn cat_enum<'a>(_table: &mut Table<'a>, _node: Handle, e: &'a Enum) -> CatResult<()> {
//     let Enum { name: _, gens: _, variants: _ } = e;
//     // table.set_ty(node, kind);
//     Ok(())
// }

// fn cat_variant<'a>(table: &mut Table<'a>, node: Handle, v: &'a Variant) -> CatResult<()> {
//     let Variant { name, kind, body } = v;
//     let parent = table.parent(node).copied().unwrap_or(table.root());
//     match (kind) {
//         (StructKind::Empty) => Ok(()),
//         (StructKind::Empty) => Ok(()),
//         (StructKind::Tuple(ty)) => {
//             let ty = TypeKind::Adt(Adt::TupleStruct(
//                 ty.iter()
//                     .map(|ty| ty.evaluate(table, node).map(|ty| (Visibility::Public, ty)))
//                     .collect::<Result<_, _>>()?,
//             ));
//             table.set_ty(node, ty);
//             Ok(())
//         }
//         (StructKind::Struct(members)) => {
//             let mut out = vec![];
//             for StructMember { vis, name, ty } in members {
//                 let ty = ty.evaluate(table, node)?;
//                 out.push((*name, *vis, ty));

//                 let mut this = node.to_entry_mut(table);
//                 let mut child = this.new_entry(NodeKind::Type);
//                 child.set_source(Source::Variant(v));
//                 child.set_ty(TypeKind::Instance(ty));

//                 let child = child.id();
//                 this.add_child(*name, child);
//             }

//             table.set_ty(node, TypeKind::Adt(Adt::Struct(out)));
//             Ok(())
//         }
//     }
// }

// fn cat_const(table: &mut Table, node: Handle, c: &Const) -> CatResult<()> {
//     let parent = parent(table, node);
//     let kind = TypeKind::Instance(
//         c.ty.evaluate(table, parent)
//             .map_err(|e| Error::TypeEval(e, " while categorizing a const"))?,
//     );
//     table.set_ty(node, kind);
//     Ok(())
// }

// fn cat_static(table: &mut Table, node: Handle, s: &Static) -> CatResult<()> {
//     let parent = parent(table, node);
//     let kind = TypeKind::Instance(
//         s.ty.evaluate(table, parent)
//             .map_err(|e| Error::TypeEval(e, " while categorizing a static"))?,
//     );
//     table.set_ty(node, kind);
//     Ok(())
// }

// fn cat_function(table: &mut Table, node: Handle, f: &Function) -> CatResult<()> {
//     let kind = TypeKind::Instance(
//         f.sign
//             .evaluate(table, node)
//             .map_err(|e| Error::TypeEval(e, " while categorizing a function"))?,
//     );
//     table.set_ty(node, kind);
//     Ok(())
// }

// fn cat_local(table: &mut Table, node: Handle, l: &Let) -> CatResult<()> {
//     let parent = parent(table, node);
//     if let Some(ty) = &l.ty {
//         let kind = ty
//             .evaluate(table, parent)
//             .map_err(|e| Error::TypeEval(e, " while categorizing a let binding"))?;
//         table.set_ty(node, TypeKind::Instance(kind));
//     }
//     Ok(())
// }

// fn cat_impl(table: &mut Table, node: Handle, i: &Impl) -> CatResult<()> {
//     let parent = parent(table, node);
//     let Impl { gens, target, body: _ } = i;
//     let target = match target {
//         ImplKind::Type(t) => t.evaluate(table, parent),
//         ImplKind::Trait { impl_trait: _, for_type: t } => t.evaluate(table, parent),
//     }?;

//     table.set_impl_target(node, target);
//     Ok(())
// }

type CatResult<T> = Result<T, Error>;

#[derive(Clone, Debug)]
pub enum Error {
    BadMeta(Expr),
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
            Error::BadMeta(meta) => write!(f, "Unknown attribute: #[{meta}]"),
            Error::TypeEval(e, during) => write!(f, "{e}{during}"),
        }
    }
}

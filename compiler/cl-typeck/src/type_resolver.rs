//! Performs step 2 of type checking: Evaluating type definitions

use crate::{
    definition::{Adt, Def, DefKind, TypeKind, ValueKind},
    key::DefID,
    node::{Node, NodeSource},
    project::{evaluate::EvaluableTypeExpression, Project as Prj},
};
use cl_ast::*;

/// Evaluate a single ID
pub fn resolve(prj: &mut Prj, id: DefID) -> Result<(), &'static str> {
    let Def { node: Node { kind: Some(source), meta, .. }, kind: DefKind::Undecided, .. } = prj[id]
    else {
        return Ok(());
    };

    let kind = match &source {
        NodeSource::Root => "root",
        NodeSource::Alias(_) => "type",
        NodeSource::Module(_) => "mod",
        NodeSource::Enum(_) => "enum",
        NodeSource::Variant(_) => "variant",
        NodeSource::Struct(_) => "struct",
        NodeSource::Const(_) => "const",
        NodeSource::Static(_) => "static",
        NodeSource::Function(_) => "fn",
        NodeSource::Impl(_) => "impl",
        NodeSource::Use(_) => "use",
        NodeSource::Local(_) => "let",
        NodeSource::Ty(_) => "ty",
    };
    let name = prj[id].name().unwrap_or("".into());

    eprintln!("Resolver: \x1b[32mEvaluating\x1b[0m \"\x1b[36m{kind} {name}\x1b[0m\" (`{id:?}`)");

    for Meta { name: Identifier(name), kind } in meta {
        if let ("intrinsic", MetaKind::Equals(Literal::String(s))) = (&**name, kind) {
            prj[id].kind = DefKind::Type(TypeKind::Intrinsic(
                s.parse().map_err(|_| "Failed to parse intrinsic")?,
            ));
        }
    }
    if DefKind::Undecided == prj[id].kind {
        prj[id].kind = source.resolve_type(prj, id)?;
    }

    eprintln!("\x1b[33m=> {}\x1b[0m", prj[id].kind);

    Ok(())
}

/// Resolves a given node
pub trait TypeResolvable<'a> {
    /// The return type upon success
    type Out;
    /// Resolves type expressions within this node
    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str>;
}

impl<'a> TypeResolvable<'a> for NodeSource<'a> {
    type Out = DefKind;

    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        match self {
            NodeSource::Root => Ok(DefKind::Type(TypeKind::Module)),
            NodeSource::Module(v) => v.resolve_type(prj, id),
            NodeSource::Alias(v) => v.resolve_type(prj, id),
            NodeSource::Enum(v) => v.resolve_type(prj, id),
            NodeSource::Variant(v) => v.resolve_type(prj, id),
            NodeSource::Struct(v) => v.resolve_type(prj, id),
            NodeSource::Const(v) => v.resolve_type(prj, id),
            NodeSource::Static(v) => v.resolve_type(prj, id),
            NodeSource::Function(v) => v.resolve_type(prj, id),
            NodeSource::Local(v) => v.resolve_type(prj, id),
            NodeSource::Impl(v) => v.resolve_type(prj, id),
            NodeSource::Use(v) => v.resolve_type(prj, id),
            NodeSource::Ty(v) => v.resolve_type(prj, id),
        }
    }
}

impl<'a> TypeResolvable<'a> for &'a Meta {
    type Out = DefKind;

    #[allow(unused_variables)]
    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let Meta { name: Identifier(name), kind } = self;
        match (name.as_ref(), kind) {
            ("intrinsic", MetaKind::Equals(Literal::String(intrinsic))) => Ok(DefKind::Type(
                TypeKind::Intrinsic(intrinsic.parse().map_err(|_| "unknown intrinsic type")?),
            )),
            (_, MetaKind::Plain) => Ok(DefKind::Type(TypeKind::Intrinsic(
                name.parse().map_err(|_| "Unknown intrinsic type")?,
            ))),
            _ => Err("Unknown meta attribute"),
        }
    }
}

impl<'a> TypeResolvable<'a> for &'a Module {
    type Out = DefKind;
    #[allow(unused_variables)]
    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        Ok(DefKind::Type(TypeKind::Module))
    }
}

impl<'a> TypeResolvable<'a> for &'a Alias {
    type Out = DefKind;

    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let parent = prj.parent_of(id).unwrap_or(id);
        let alias = if let Some(ty) = &self.from {
            Some(
                ty.evaluate(prj, parent)
                    .or_else(|_| ty.evaluate(prj, id))
                    .map_err(|_| "Unresolved type in alias")?,
            )
        } else {
            None
        };

        Ok(DefKind::Type(TypeKind::Alias(alias)))
    }
}

impl<'a> TypeResolvable<'a> for &'a Enum {
    type Out = DefKind;

    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let Enum { name: _, kind } = self;
        let EnumKind::Variants(v) = kind else {
            return Ok(DefKind::Type(TypeKind::Adt(Adt::FieldlessEnum)));
        };
        let mut fields = vec![];
        for Variant { name: Identifier(name), kind: _ } in v {
            let id = prj[id].module.get_type(*name);
            fields.push((*name, id))
        }
        Ok(DefKind::Type(TypeKind::Adt(Adt::Enum(fields))))
    }
}

impl<'a> TypeResolvable<'a> for &'a Variant {
    type Out = DefKind;

    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        // Get the grandparent of this node, for name resolution
        let parent = prj.parent_of(id).unwrap_or(id);
        let grandparent = prj.parent_of(parent).unwrap_or(parent);
        let Variant { name: _, kind } = self;

        Ok(DefKind::Type(match kind {
            VariantKind::Plain => return Ok(DefKind::Type(TypeKind::Empty)),
            VariantKind::CLike(_) => return Ok(DefKind::Undecided),
            VariantKind::Tuple(ty) => match &ty.kind {
                TyKind::Empty => TypeKind::Tuple(vec![]),
                TyKind::Tuple(TyTuple { types }) => {
                    TypeKind::Tuple(types.evaluate(prj, grandparent).map_err(|e| {
                        eprintln!("{e}");
                        ""
                    })?)
                }
                _ => Err("Unexpected TyKind in tuple variant")?,
            },
            VariantKind::Struct(members) => {
                TypeKind::Adt(Adt::Struct(members.resolve_type(prj, parent)?))
            }
        }))
    }
}

impl<'a> TypeResolvable<'a> for &'a Struct {
    type Out = DefKind;
    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let parent = prj.parent_of(id).unwrap_or(id);
        let Struct { name: _, kind } = self;
        Ok(match kind {
            StructKind::Empty => DefKind::Type(TypeKind::Empty),
            StructKind::Tuple(types) => DefKind::Type(TypeKind::Adt(Adt::TupleStruct({
                let mut out = vec![];
                for ty in types {
                    out.push((
                        Visibility::Public,
                        ty.evaluate(prj, parent)
                            .map_err(|_| "Unresolved type in tuple-struct member")?,
                    ));
                }
                out
            }))),
            StructKind::Struct(members) => {
                DefKind::Type(TypeKind::Adt(Adt::Struct(members.resolve_type(prj, id)?)))
            }
        })
    }
}

impl<'a> TypeResolvable<'a> for &'a StructMember {
    type Out = (Sym, Visibility, DefID);

    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let parent = prj.parent_of(id).unwrap_or(id);
        let StructMember { name: Identifier(name), vis, ty } = self;

        let ty = ty
            .evaluate(prj, parent)
            .map_err(|_| "Invalid type while resolving StructMember")?;

        Ok((*name, *vis, ty))
    }
}

impl<'a> TypeResolvable<'a> for &'a Const {
    type Out = DefKind;

    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let Const { ty, .. } = self;
        let ty = ty
            .evaluate(prj, id)
            .map_err(|_| "Invalid type while resolving const")?;
        Ok(DefKind::Value(ValueKind::Const(ty)))
    }
}

impl<'a> TypeResolvable<'a> for &'a Static {
    type Out = DefKind;

    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let parent = prj.parent_of(id).unwrap_or(id);
        let Static { ty, .. } = self;
        let ty = ty
            .evaluate(prj, parent)
            .map_err(|_| "Invalid type while resolving static")?;
        Ok(DefKind::Value(ValueKind::Static(ty)))
    }
}

impl<'a> TypeResolvable<'a> for &'a Function {
    type Out = DefKind;

    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let parent = prj.parent_of(id).unwrap_or(id);
        let Function { sign, .. } = self;
        let sign = sign
            .evaluate(prj, parent)
            .map_err(|_| "Invalid type in function signature")?;
        Ok(DefKind::Value(ValueKind::Fn(sign)))
    }
}

impl<'a> TypeResolvable<'a> for &'a Let {
    type Out = DefKind;

    #[allow(unused)]
    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let Let { mutable, name, ty, init } = self;
        Ok(DefKind::Undecided)
    }
}

impl<'a> TypeResolvable<'a> for &'a Impl {
    type Out = DefKind;

    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let parent = prj.parent_of(id).unwrap_or(id);

        let target = match &self.target {
            ImplKind::Type(t) => t.evaluate(prj, parent),
            ImplKind::Trait { for_type, .. } => for_type.evaluate(prj, parent),
        }
        .map_err(|_| "Unresolved type in impl target")?;

        prj[id].module.parent = Some(target);

        Ok(DefKind::Impl(target))
    }
}

impl<'a> TypeResolvable<'a> for &'a Use {
    type Out = DefKind;

    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        todo!("Resolve types for {self} with ID {id} in {prj:?}")
    }
}

impl<'a> TypeResolvable<'a> for &'a TyKind {
    type Out = DefKind;

    #[allow(unused)]
    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        todo!()
    }
}

impl<'a, T> TypeResolvable<'a> for &'a [T]
where &'a T: TypeResolvable<'a>
{
    type Out = Vec<<&'a T as TypeResolvable<'a>>::Out>;

    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let mut members = vec![];
        for member in self {
            members.push(member.resolve_type(prj, id)?);
        }
        Ok(members)
    }
}

impl<'a, T> TypeResolvable<'a> for Option<&'a T>
where &'a T: TypeResolvable<'a>
{
    type Out = Option<<&'a T as TypeResolvable<'a>>::Out>;
    fn resolve_type(self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        match self {
            Some(t) => Some(t.resolve_type(prj, id)).transpose(),
            None => Ok(None),
        }
    }
}

//! Performs step 2 of type checking: Evaluating type definitions

use crate::{
    definition::{Adt, Def, DefKind, TypeKind, ValueKind},
    key::DefID,
    module,
    project::{evaluate::EvaluableTypeExpression, Project as Prj},
};
use cl_ast::*;

/// Evaluate a single ID
pub fn resolve(prj: &mut Prj, id: DefID) -> Result<(), &'static str> {
    let (DefKind::Undecided, Some(source)) = (&prj[id].kind, prj[id].source) else {
        return Ok(());
    };
    let kind = match &source.kind {
        ItemKind::Alias(_) => "type",
        ItemKind::Module(_) => "mod",
        ItemKind::Enum(_) => "enum",
        ItemKind::Struct(_) => "struct",
        ItemKind::Const(_) => "const",
        ItemKind::Static(_) => "static",
        ItemKind::Function(_) => "fn",
        ItemKind::Impl(_) => "impl",
        ItemKind::Use(_) => "use",
    };
    eprintln!(
        "Resolver: \x1b[32mEvaluating\x1b[0m \"\x1b[36m{kind} {}\x1b[0m\" (`{id:?}`)",
        prj[id].name
    );

    prj[id].kind = source.resolve_type(prj, id)?;

    eprintln!("\x1b[33m=> {}\x1b[0m", prj[id].kind);

    Ok(())
}

/// Resolves a given node
pub trait TypeResolvable<'a> {
    /// The return type upon success
    type Out;
    /// Resolves type expressions within this node
    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str>;
}

impl<'a> TypeResolvable<'a> for Item {
    type Out = DefKind;
    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let Self { attrs: Attrs { meta }, kind, .. } = self;
        for meta in meta {
            if let Ok(def) = meta.resolve_type(prj, id) {
                return Ok(def);
            }
        }
        kind.resolve_type(prj, id)
    }
}

impl<'a> TypeResolvable<'a> for Meta {
    type Out = DefKind;

    #[allow(unused_variables)]
    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let Self { name: Identifier(name), kind } = self;
        let name = name.get().unwrap_or_default();
        match (name.as_str(), kind) {
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

impl<'a> TypeResolvable<'a> for ItemKind {
    type Out = DefKind;
    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        if prj[id].source.map(|s| &s.kind as *const _) != Some(self as *const _) {
            return Err("id is not self!");
        }
        match self {
            ItemKind::Module(i) => i.resolve_type(prj, id),
            ItemKind::Impl(i) => i.resolve_type(prj, id),
            ItemKind::Alias(i) => i.resolve_type(prj, id),
            ItemKind::Enum(i) => i.resolve_type(prj, id),
            ItemKind::Struct(i) => i.resolve_type(prj, id),
            ItemKind::Const(i) => i.resolve_type(prj, id),
            ItemKind::Static(i) => i.resolve_type(prj, id),
            ItemKind::Function(i) => i.resolve_type(prj, id),
            ItemKind::Use(i) => i.resolve_type(prj, id),
        }
    }
}

impl<'a> TypeResolvable<'a> for Module {
    type Out = DefKind;
    #[allow(unused_variables)]
    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        Ok(DefKind::Type(TypeKind::Module))
    }
}

impl<'a> TypeResolvable<'a> for Impl {
    type Out = DefKind;

    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
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

impl<'a> TypeResolvable<'a> for Use {
    type Out = DefKind;

    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        todo!("Resolve types for {self} with ID {id} in {prj:?}")
    }
}

impl<'a> TypeResolvable<'a> for Alias {
    type Out = DefKind;

    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
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

impl<'a> TypeResolvable<'a> for Enum {
    type Out = DefKind;

    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let Self { name: _, kind } = self;
        let EnumKind::Variants(v) = kind else {
            return Ok(DefKind::Type(TypeKind::Adt(Adt::FieldlessEnum)));
        };
        let mut fields = vec![];
        for v @ Variant { name: Identifier(name), kind: _ } in v {
            let id = v.resolve_type(prj, id)?;
            fields.push((*name, id))
        }
        Ok(DefKind::Type(TypeKind::Adt(Adt::Enum(fields))))
    }
}

impl<'a> TypeResolvable<'a> for Variant {
    type Out = Option<DefID>;

    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let parent = prj.parent_of(id).unwrap_or(id);
        let Self { name: Identifier(name), kind } = self;

        let adt = match kind {
            VariantKind::Plain => return Ok(None),
            VariantKind::CLike(_) => todo!("Resolve variant info for C-like enums"),
            VariantKind::Tuple(ty) => {
                return ty
                    .evaluate(prj, parent)
                    .map_err(|_| "Unresolved type in enum tuple variant")
                    .map(Some)
            }
            VariantKind::Struct(members) => Adt::Struct(members.resolve_type(prj, id)?),
        };

        let def = Def {
            name: *name,
            kind: DefKind::Type(TypeKind::Adt(adt)),
            module: module::Module::new(id),
            ..Default::default()
        };

        let new_id = prj.pool.insert(def);
        // Insert the struct variant type into the enum's namespace
        prj[id].module.insert_type(*name, new_id);

        Ok(Some(new_id))
    }
}

impl<'a> TypeResolvable<'a> for Struct {
    type Out = DefKind;
    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let parent = prj.parent_of(id).unwrap_or(id);
        let Self { name: _, kind } = self;
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

impl<'a> TypeResolvable<'a> for StructMember {
    type Out = (Sym, Visibility, DefID);

    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let parent = prj.parent_of(id).unwrap_or(id);
        let Self { name: Identifier(name), vis, ty } = self;

        let ty = ty
            .evaluate(prj, parent)
            .map_err(|_| "Invalid type while resolving StructMember")?;

        Ok((*name, *vis, ty))
    }
}

impl<'a> TypeResolvable<'a> for Const {
    type Out = DefKind;

    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let Self { ty, .. } = self;
        let ty = ty
            .evaluate(prj, id)
            .map_err(|_| "Invalid type while resolving const")?;
        Ok(DefKind::Value(ValueKind::Const(ty)))
    }
}
impl<'a> TypeResolvable<'a> for Static {
    type Out = DefKind;

    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let parent = prj.parent_of(id).unwrap_or(id);
        let Self { ty, .. } = self;
        let ty = ty
            .evaluate(prj, parent)
            .map_err(|_| "Invalid type while resolving static")?;
        Ok(DefKind::Value(ValueKind::Static(ty)))
    }
}

impl<'a> TypeResolvable<'a> for Function {
    type Out = DefKind;

    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let parent = prj.parent_of(id).unwrap_or(id);
        let Self { sign, .. } = self;
        let sign = sign
            .evaluate(prj, parent)
            .map_err(|_| "Invalid type in function signature")?;
        Ok(DefKind::Value(ValueKind::Fn(sign)))
    }
}

impl<'a, T: TypeResolvable<'a>> TypeResolvable<'a> for [T] {
    type Out = Vec<T::Out>;

    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        let mut members = vec![];
        for member in self {
            members.push(member.resolve_type(prj, id)?);
        }
        Ok(members)
    }
}
impl<'a, T: TypeResolvable<'a>> TypeResolvable<'a> for Option<T> {
    type Out = Option<T::Out>;

    fn resolve_type(&'a self, prj: &mut Prj<'a>, id: DefID) -> Result<Self::Out, &'static str> {
        match self {
            Some(t) => Some(t.resolve_type(prj, id)).transpose(),
            None => Ok(None),
        }
    }
}

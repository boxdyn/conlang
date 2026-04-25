//! The pattern separator separates patterns into their Value and Type components.

use std::{convert::Infallible, mem::replace};

use crate::{
    AstTypes, At, Bind, DefaultTypes, Pat, PatOp,
    fold::{Fold, Foldable},
};

fn take(At(pat, span): &mut At<Pat>) -> At<Pat> {
    At(replace(pat, Pat::Ignore), *span)
}

pub fn bubble_types(pat: At<Pat>, in_enum: bool) -> (At<Pat>, Option<At<Pat>>) {
    //! Bubbles up type annotations from within a pattern to the top level.
    let (op, mut pats, span) = match pat {
        At(Pat::Op(op, pats), span) => (op, pats, span),
        _ => return (pat, None),
    };

    match (op, &mut pats[..]) {
        (PatOp::Typed, [pat, ty]) => {
            let (value, _ty2) = bubble_types(take(pat), in_enum);
            // TODO: unify ty, ty2
            (value, Some(take(ty)))
        }
        (PatOp::TypePrefixed, [prefix, pat]) => {
            let (pat, ty) = bubble_types(take(pat), in_enum);
            let ty = match (ty, in_enum) {
                (Some(At(ty, span)), false) => {
                    Pat::Op(op, vec![prefix.clone(), ty.at(span)]).at(span)
                }
                (Some(At(ty, span)), true) => {
                    Pat::Op(op, vec![Pat::Ignore.at(span), ty.at(span)]).at(span)
                }
                (None, _) => prefix.clone(),
            };
            let value = Pat::Op(op, vec![take(prefix), pat]).at(span);
            (value, Some(ty))
        }
        (PatOp::MetaInner | PatOp::MetaOuter, [meta, pat]) => {
            let (value, ty) = bubble_types(take(pat), in_enum);
            (Pat::Op(op, vec![take(meta), value]).at(span), ty)
        }
        (PatOp::Pub | PatOp::Mut | PatOp::Ref | PatOp::Ptr, [pat]) => {
            let (value, ty) = bubble_types(take(pat), in_enum);
            (Pat::Op(op, vec![value]).at(span), ty)
        }
        (PatOp::Record, ..) => {
            let (mut values, mut types) = (vec![], vec![]);
            for At(pat, span) in pats {
                // records use `Typed` nodes for value decomposition
                let (name, body) = bubble_types(pat.at(span), false);

                // but enums do not.
                if in_enum {
                    values.push(body.unwrap_or_else(|| name.clone()));
                    types.push(name);
                    continue;
                }

                // those `Typed` nodes must be further bubbled
                let body = body.unwrap_or(Pat::Ignore.at(name.1));
                let (body, ty) = bubble_types(body, false);
                let ty = ty.unwrap_or(Pat::Ignore.at(body.1));

                values.push(Pat::Op(PatOp::Typed, vec![name.clone(), body]).at(span));
                types.push(Pat::Op(PatOp::Typed, vec![name, ty]).at(span));
            }
            let (value, ty) = (Pat::Op(op, values).at(span), Pat::Op(op, types).at(span));
            (value, Some(ty))
        }
        (PatOp::ArRep, [pat, rep]) => {
            let (pat, ty) = bubble_types(take(pat), in_enum);
            let ty = ty.unwrap_or(Pat::Ignore.at(pat.1));
            (
                Pat::Op(op, vec![pat, Pat::Ignore.at(span)]).at(span),
                Some(Pat::Op(op, vec![ty, take(rep)]).at(span)),
            )
        }
        (PatOp::Generic, [pat, ..]) => {
            let (pat, ty) = bubble_types(take(pat), in_enum);
            pats[0] = ty.unwrap_or(Pat::Ignore.at(pat.1));
            (pat, Some(Pat::Op(op, pats).at(span)))
        }
        (PatOp::Fn, [arg, ret]) => {
            let (pat, ty) = bubble_types(take(arg), in_enum);
            let ty = ty.unwrap_or(Pat::Ignore.at(pat.1));
            (pat, Some(Pat::Op(op, vec![ty, take(ret)]).at(span)))
        }
        _ => {
            let (mut values, mut tys) = (vec![], vec![]);
            for pat in pats {
                let (value, ty) = bubble_types(pat, in_enum);
                tys.push(ty.unwrap_or(Pat::Ignore.at(value.1)));
                values.push(value);
            }
            let (value, ty) = (Pat::Op(op, values).at(span), Pat::Op(op, tys).at(span));
            (value, Some(ty))
        }
    }
}

/// The [Bubbler]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct Bubbler(pub bool);

impl Fold<DefaultTypes, DefaultTypes> for Bubbler {
    type Error = Infallible;

    fn fold_annotation(
        &mut self,
        anno: <DefaultTypes as AstTypes>::Annotation,
    ) -> Result<<DefaultTypes as AstTypes>::Annotation, Self::Error> {
        Ok(anno)
    }

    fn fold_macro_id(
        &mut self,
        name: <DefaultTypes as AstTypes>::MacroId,
    ) -> Result<<DefaultTypes as AstTypes>::MacroId, Self::Error> {
        Ok(name)
    }

    fn fold_symbol(
        &mut self,
        name: <DefaultTypes as AstTypes>::Symbol,
    ) -> Result<<DefaultTypes as AstTypes>::Symbol, Self::Error> {
        Ok(name)
    }

    fn fold_path(
        &mut self,
        path: <DefaultTypes as AstTypes>::Path,
    ) -> Result<<DefaultTypes as AstTypes>::Path, Self::Error> {
        Ok(path)
    }

    fn fold_literal(
        &mut self,
        lit: <DefaultTypes as AstTypes>::Literal,
    ) -> Result<<DefaultTypes as AstTypes>::Literal, Self::Error> {
        Ok(lit)
    }

    fn fold_at_pat(
        &mut self,
        pat: At<Pat<DefaultTypes>, DefaultTypes>,
    ) -> Result<At<Pat<DefaultTypes>, DefaultTypes>, Self::Error> {
        Ok(match bubble_types(pat, self.0) {
            (value @ At(_, span), Some(ty)) => Pat::Op(PatOp::Typed, vec![value, ty]).at(span),
            (value, None) => value,
        })
    }

    fn fold_bind(&mut self, bind: Bind<DefaultTypes>) -> Result<Bind<DefaultTypes>, Self::Error> {
        let mut bubbler = Bubbler(bind.0 == crate::BindOp::Enum);
        bind.children(&mut bubbler)
    }
}

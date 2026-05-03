//! A folder (implementer of the [`Fold`] trait) maps ASTs to ASTs
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::wildcard_imports, clippy::missing_errors_doc)]

use super::*;

/// Deconstructs an entire AST, and reconstructs it from parts.
///
/// Each function acts as a customization point.
///
/// Aside from [`AstTypes`], each node in the AST implements the [`Foldable`] trait,
/// which provides double dispatch.
pub trait Fold<From: AstTypes, To: AstTypes = From> {
    type Error;

    /// Consumes an Annotation in A, possibly transforms it, and produces a replacement Annotation
    /// in B
    fn fold_annotation(&mut self, anno: From::Annotation) -> Result<To::Annotation, Self::Error>;

    /// Consumes a `MacroId` in A, possibly transforms it, and produces a replacement `MacroId` in B
    fn fold_macro_id(&mut self, name: From::MacroId) -> Result<To::MacroId, Self::Error>;

    /// Consumes a `Symbol` in A, possibly transforms it, and produces a replacement `Symbol` in B
    fn fold_symbol(&mut self, name: From::Symbol) -> Result<To::Symbol, Self::Error>;

    /// Consumes a `Path` in A, possibly transforms it, and produces a replacement `Path` in B
    fn fold_path(&mut self, path: From::Path) -> Result<To::Path, Self::Error>;

    /// Consumes a `Literal` in A, possibly transforms it, and produces a replacement `Literal` in B
    fn fold_literal(&mut self, lit: From::Literal) -> Result<To::Literal, Self::Error>;

    /// Folds an annotated expression, so the expression and annotation can be seen at once.
    fn fold_at_expr(
        &mut self,
        expr: At<Expr<From>, From>,
    ) -> Result<At<Expr<To>, To>, Self::Error> {
        expr.children(self)
    }

    /// Consumes an [`Expr`], possibly transforms it, and produces a replacement [`Expr`]
    fn fold_expr(&mut self, expr: Expr<From>) -> Result<Expr<To>, Self::Error> {
        expr.children(self)
    }

    /// Consumes a [`Use`], possibly transforms it, and produces a replacement [`Use`]
    fn fold_use(&mut self, item: Use<From>) -> Result<Use<To>, Self::Error> {
        item.children(self)
    }

    /// Folds an annotated pattern, so the pattern and annotation can be seen at once.
    fn fold_at_pat(&mut self, pat: At<Pat<From>, From>) -> Result<At<Pat<To>, To>, Self::Error> {
        pat.children(self)
    }

    /// Consumes a [`Pat`], possibly transforms it, and produces a replacement [`Pat`]
    fn fold_pat(&mut self, pat: Pat<From>) -> Result<Pat<To>, Self::Error> {
        pat.children(self)
    }

    /// Consumes a [`Bind`], possibly transforms it, and produces a replacement [`Bind`]
    fn fold_bind(&mut self, bind: Bind<From>) -> Result<Bind<To>, Self::Error> {
        bind.children(self)
    }

    /// Consumes a [`Make`], possibly transforms it, and produces a replacement [`Make`]
    fn fold_make(&mut self, make: Make<From>) -> Result<Make<To>, Self::Error> {
        make.children(self)
    }

    /// Consumes a [`MakeArm`], possibly transforms it, and produces a replacement [`MakeArm`]
    fn fold_makearm(&mut self, arm: MakeArm<From>) -> Result<MakeArm<To>, Self::Error> {
        arm.children(self)
    }

    /// Consumes a [`Make`], possibly transforms it, and produces a replacement [`Make`]
    fn fold_match(&mut self, mtch: Match<From>) -> Result<Match<To>, Self::Error> {
        mtch.children(self)
    }

    /// Consumes a [`MakeArm`], possibly transforms it, and produces a replacement [`MakeArm`]
    fn fold_matcharm(&mut self, arm: MatchArm<From>) -> Result<MatchArm<To>, Self::Error> {
        arm.children(self)
    }
}

/// ```ignore
/// pub macro impl_default_fold($Src: ty, $Dst: ty)
/// ```
///  Implements [`Into`]-based defaults for the required [`Fold`] members:
/// - [`Fold::fold_annotation`] `where Src::Annotation: Into<Dst::Annotation>`
/// - [`Fold::fold_macro_id`] `where Src::MacroId: Into<Dst::MacroId>`
/// - [`Fold::fold_symbol`] `where Src::Symbol: Into<Dst::Symbol>`
/// - [`Fold::fold_path`] `where Src::Path: Into<Dst::Path>`
/// - [`Fold::fold_literal`] `where Src::Literal: Into<Dst::Literal>`
///
/// # Examples:
/// Implements an "identity" folder
/// ```rust
/// # use cl_ast::ast::{AstTypes, Annotation};
/// # use cl_ast::ast::fold::{Fold, impl_default_fold};
/// struct IdentityFold;
/// impl<A: AstTypes> Fold<A, A> for IdentityFold {
///     type Error = std::convert::Infallible;
///     impl_default_fold!(A, A);
/// }
/// ```
pub macro impl_default_fold($Src: ty, $Dst: ty) {
    fn fold_annotation(
        &mut self,
        anno: <$Src as AstTypes>::Annotation,
    ) -> Result<<$Dst as AstTypes>::Annotation, Self::Error> {
        Ok(anno.into())
    }
    fn fold_macro_id(
        &mut self,
        name: <$Src as AstTypes>::MacroId,
    ) -> Result<<$Dst as AstTypes>::MacroId, Self::Error> {
        Ok(name.into())
    }
    fn fold_symbol(
        &mut self,
        name: <$Src as AstTypes>::Symbol,
    ) -> Result<<$Dst as AstTypes>::Symbol, Self::Error> {
        Ok(name.into())
    }
    fn fold_path(
        &mut self,
        path: <$Src as AstTypes>::Path,
    ) -> Result<<$Dst as AstTypes>::Path, Self::Error> {
        Ok(path.into())
    }
    fn fold_literal(
        &mut self,
        lit: <$Src as AstTypes>::Literal,
    ) -> Result<<$Dst as AstTypes>::Literal, Self::Error> {
        Ok(lit.into())
    }
}

/// Implements depth-first traversal for folders
pub trait Foldable<A: AstTypes, B: AstTypes>: Sized {
    /// The return type of the associated [Fold] function
    type Out;

    /// Calls `Self`'s appropriate [Folder](Fold) function(s)
    fn fold_in<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error>;

    /// Destructures `self`, calling [`Foldable::fold_in`] on all foldable members,
    /// and rebuilds a `Self` out of the results.
    #[allow(unused_variables)]
    fn children<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error>;
}

impl<A: AstTypes, B: AstTypes> Foldable<A, B> for Expr<A> {
    type Out = Expr<B>;

    fn fold_in<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        folder.fold_expr(self)
    }

    fn children<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        Ok(match self {
            Self::Omitted => Expr::Omitted,
            Self::Id(path) => Expr::Id(folder.fold_path(path)?),
            Self::MetId(id) => Expr::MetId(folder.fold_macro_id(id)?),
            Self::Lit(lit) => Expr::Lit(folder.fold_literal(lit)?),
            Self::Use(item) => Expr::Use(item.fold_in(folder)?),
            Self::Bind(bind) => Expr::Bind(bind.fold_in(folder)?),
            Self::Make(make) => Expr::Make(make.fold_in(folder)?),
            Self::Match(mtch) => Expr::Match(mtch.fold_in(folder)?),
            Self::Op(op, annos) => Expr::Op(op, annos.fold_in(folder)?),
        })
    }
}

impl<A: AstTypes, B: AstTypes> Foldable<A, B> for Use<A> {
    type Out = Use<B>;

    fn fold_in<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        folder.fold_use(self)
    }

    fn children<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        Ok(match self {
            Self::Glob => Use::Glob,
            Self::Name(name) => Use::Name(folder.fold_symbol(name)?),
            Self::Alias(name, alias) => {
                Use::Alias(folder.fold_symbol(name)?, folder.fold_symbol(alias)?)
            }
            Self::Path(name, rest) => Use::Path(folder.fold_symbol(name)?, rest.fold_in(folder)?),
            Self::Tree(items) => Use::Tree(items.fold_in(folder)?),
        })
    }
}

impl<A: AstTypes, B: AstTypes> Foldable<A, B> for Pat<A> {
    type Out = Pat<B>;

    fn fold_in<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        folder.fold_pat(self)
    }

    fn children<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        Ok(match self {
            Self::Ignore => Pat::Ignore,
            Self::Never => Pat::Never,
            Self::MetId(name) => Pat::MetId(folder.fold_macro_id(name)?),
            Self::Name(name) => Pat::Name(folder.fold_symbol(name)?),
            Self::Value(expr) => Pat::Value(expr.fold_in(folder)?),
            Self::Op(op, pats) => Pat::Op(op, pats.fold_in(folder)?),
        })
    }
}

impl<A: AstTypes, B: AstTypes> Foldable<A, B> for Bind<A> {
    type Out = Bind<B>;

    fn fold_in<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        folder.fold_bind(self)
    }

    fn children<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        let Self(op, gens, pat, exprs) = self;
        Ok(Bind(
            op,
            gens.into_iter()
                .map(|g| folder.fold_path(g))
                .collect::<Result<_, _>>()?,
            pat.fold_in(folder)?,
            exprs.fold_in(folder)?,
        ))
    }
}

impl<A: AstTypes, B: AstTypes> Foldable<A, B> for Make<A> {
    type Out = Make<B>;

    fn fold_in<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        folder.fold_make(self)
    }

    fn children<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        let Self(expr, arms) = self;
        Ok(Make(expr.fold_in(folder)?, arms.fold_in(folder)?))
    }
}

impl<A: AstTypes, B: AstTypes> Foldable<A, B> for MakeArm<A> {
    type Out = MakeArm<B>;

    fn fold_in<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        folder.fold_makearm(self)
    }

    fn children<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        let Self(name, expr) = self;
        Ok(MakeArm(folder.fold_symbol(name)?, expr.fold_in(folder)?))
    }
}

impl<A: AstTypes, B: AstTypes> Foldable<A, B> for Match<A> {
    type Out = Match<B>;

    fn fold_in<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        folder.fold_match(self)
    }

    fn children<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        let Self(scrutinee, arms) = self;
        Ok(Match(scrutinee.fold_in(folder)?, arms.fold_in(folder)?))
    }
}

impl<A: AstTypes, B: AstTypes> Foldable<A, B> for MatchArm<A> {
    type Out = MatchArm<B>;

    fn fold_in<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        folder.fold_matcharm(self)
    }

    fn children<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        let Self(pat, expr) = self;
        Ok(MatchArm(pat.fold_in(folder)?, expr.fold_in(folder)?))
    }
}

impl<A: AstTypes, B: AstTypes> Foldable<A, B> for At<Expr<A>, A> {
    type Out = At<Expr<B>, B>;

    fn fold_in<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        folder.fold_at_expr(self)
    }

    fn children<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        let Self(expr, anno) = self;
        Ok(At(expr.children(folder)?, folder.fold_annotation(anno)?))
    }
}

impl<A: AstTypes, B: AstTypes> Foldable<A, B> for At<Pat<A>, A> {
    type Out = At<Pat<B>, B>;

    fn fold_in<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        folder.fold_at_pat(self)
    }

    fn children<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        let Self(pat, anno) = self;
        Ok(At(pat.children(folder)?, folder.fold_annotation(anno)?))
    }
}

//////////////////////////////////////////////
//  GENERIC IMPLEMENTATIONS ON COLLECTIONS  //
//////////////////////////////////////////////

// Maps the value in the box across `f()` without deallocating
// fn box_try_map<T, E>(boxed: Box<T>, f: impl FnOnce(T) -> Result<T, E>) -> Result<Box<T>, E> {
//     // TODO: replace with Box::take when it stabilizes.
//     let rawbox = Box::into_raw(boxed);

//     // Safety: `rawbox` came from a Box, so it is aligned and initialized.
//     //     To prevent further reuse and deallocate on failure, rawbox is
//     //     shadowed by a Box<MaybeUninit<T>>.
//     // Safety: MaybeUninit<T> has the same size and alignment as T.
//     let (value, rawbox) = (unsafe { rawbox.read() }, unsafe {
//         Box::from_raw(rawbox.cast::<MaybeUninit<T>>())
//     });

//     // rawbox is reinitialized with f(value)
//     Ok(Box::write(rawbox, f(value)?))
// }

impl<T, A, B> Foldable<A, B> for Box<T>
where
    T: Foldable<A, B>,
    A: AstTypes,
    B: AstTypes,
{
    type Out = Box<T::Out>;

    fn fold_in<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        let value = *self;
        Ok(Box::new(value.fold_in(folder)?))
    }

    fn children<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        let value = *self;
        Ok(Box::new(value.children(folder)?))
    }
}

impl<T: Foldable<A, B>, A: AstTypes, B: AstTypes> Foldable<A, B> for Vec<T> {
    type Out = Vec<T::Out>;

    fn fold_in<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        self.children(folder)
    }

    fn children<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        // TODO: is this correct for generic data structures?
        self.into_iter().map(|e| e.fold_in(folder)).collect()
    }
}

impl<T: Foldable<A, B>, A: AstTypes, B: AstTypes> Foldable<A, B> for Option<T> {
    type Out = Option<T::Out>;

    fn fold_in<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        Ok(match self {
            Self::Some(value) => Some(value.fold_in(folder)?),
            Self::None => None,
        })
    }

    fn children<F: Fold<A, B> + ?Sized>(self, folder: &mut F) -> Result<Self::Out, F::Error> {
        Ok(match self {
            Self::Some(value) => Some(value.children(folder)?),
            Self::None => None,
        })
    }
}

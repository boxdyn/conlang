//! AST visitor
#![warn(clippy::all, clippy::pedantic)]
#![allow(clippy::wildcard_imports, clippy::missing_errors_doc)]
use super::*;

pub trait Visit<'a, A: AstTypes> {
    type Error;

    fn visit<W: Walk<'a, A> + ?Sized>(&mut self, walk: &'a W) -> Result<(), Self::Error> {
        walk.visit_in(self)
    }
    fn visit_literal(&mut self, lit: &'a A::Literal) -> Result<(), Self::Error> {
        let _ = lit;
        Ok(())
    }
    fn visit_macro_id(&mut self, name: &'a A::MacroId) -> Result<(), Self::Error> {
        let _ = name;
        Ok(())
    }
    fn visit_symbol(&mut self, name: &'a A::Symbol) -> Result<(), Self::Error> {
        let _ = name;
        Ok(())
    }
    fn visit_path(&mut self, path: &'a A::Path) -> Result<(), Self::Error> {
        let _ = path;
        Ok(())
    }
    fn visit_expr(&mut self, expr: &'a Expr<A>) -> Result<(), Self::Error> {
        expr.children(self)
    }
    fn visit_use(&mut self, item: &'a Use<A>) -> Result<(), Self::Error> {
        item.children(self)
    }
    fn visit_pat(&mut self, item: &'a Pat<A>) -> Result<(), Self::Error> {
        item.children(self)
    }
    fn visit_bind(&mut self, item: &'a Bind<A>) -> Result<(), Self::Error> {
        item.children(self)
    }
    fn visit_make(&mut self, item: &'a Make<A>) -> Result<(), Self::Error> {
        item.children(self)
    }
    fn visit_makearm(&mut self, item: &'a MakeArm<A>) -> Result<(), Self::Error> {
        item.children(self)
    }
    fn visit_match(&mut self, item: &'a Match<A>) -> Result<(), Self::Error> {
        item.children(self)
    }
    fn visit_matcharm(&mut self, item: &'a MatchArm<A>) -> Result<(), Self::Error> {
        item.children(self)
    }
}

pub trait Walk<'a, A: AstTypes> {
    #[inline]
    fn children<V: Visit<'a, A> + ?Sized>(&'a self, _v: &mut V) -> Result<(), V::Error> {
        Ok(())
    }
    fn visit_in<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error>;
}

impl<'a, A: AstTypes> Walk<'a, A> for Expr<A> {
    fn children<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        match self {
            Self::Omitted => Ok(()),
            Self::Id(path) => v.visit_path(path),
            Self::MetId(id) => v.visit_macro_id(id),
            Self::Lit(lit) => v.visit_literal(lit),
            Self::Use(u) => u.visit_in(v),
            Self::Bind(bind) => bind.visit_in(v),
            Self::Make(make) => make.visit_in(v),
            Self::Match(mtch) => mtch.visit_in(v),
            Self::Op(_op, exprs) => exprs.visit_in(v),
        }
    }

    #[inline]
    fn visit_in<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        v.visit_expr(self)
    }
}

impl<'a, A: AstTypes> Walk<'a, A> for Use<A> {
    fn children<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        match self {
            Self::Glob => Ok(()),
            Self::Name(name) => v.visit_symbol(name),
            Self::Alias(name, alias) => {
                v.visit_symbol(name)?;
                v.visit_symbol(alias)
            }
            Self::Path(name, rest) => {
                v.visit_symbol(name)?;
                rest.visit_in(v)
            }
            Self::Tree(items) => items.visit_in(v),
        }
    }

    #[inline]
    fn visit_in<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        v.visit_use(self)
    }
}

impl<'a, A: AstTypes> Walk<'a, A> for Pat<A> {
    fn children<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        match self {
            Self::Ignore | Self::Never => Ok(()),
            Self::MetId(id) => v.visit_macro_id(id),
            Self::Name(name) => v.visit_symbol(name),
            Self::Value(literal) => literal.visit_in(v),
            Self::Op(_, pats) => pats.visit_in(v),
        }
    }

    #[inline]
    fn visit_in<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        v.visit_pat(self)
    }
}

impl<'a, A: AstTypes> Walk<'a, A> for Bind<A> {
    fn children<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        let Self(_kind, gens, pat, exprs) = self;
        gens.iter().try_for_each(|g| v.visit_path(g))?;
        pat.visit_in(v)?;
        exprs.visit_in(v)
    }

    #[inline]
    fn visit_in<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        v.visit_bind(self)
    }
}

impl<'a, A: AstTypes> Walk<'a, A> for Make<A> {
    fn children<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        let Self(expr, arms) = self;
        expr.visit_in(v)?;
        arms.visit_in(v)
    }

    #[inline]
    fn visit_in<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        v.visit_make(self)
    }
}

impl<'a, A: AstTypes> Walk<'a, A> for MakeArm<A> {
    fn children<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        let Self(name, expr) = self;
        v.visit_symbol(name)?;
        expr.visit_in(v)
    }

    #[inline]
    fn visit_in<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        v.visit_makearm(self)
    }
}

impl<'a, A: AstTypes> Walk<'a, A> for Match<A> {
    fn children<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        let Self(expr, arms) = self;
        expr.visit_in(v)?;
        arms.visit_in(v)
    }

    #[inline]
    fn visit_in<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        v.visit_match(self)
    }
}

impl<'a, A: AstTypes> Walk<'a, A> for MatchArm<A> {
    fn children<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        let Self(name, expr) = self;
        name.visit_in(v)?;
        expr.visit_in(v)
    }

    #[inline]
    fn visit_in<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        v.visit_matcharm(self)
    }
}

impl<'a, T: Annotation + Walk<'a, A>, A: AstTypes> Walk<'a, A> for At<T, A> {
    #[inline]
    fn children<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        self.0.children(v)
    }

    #[inline]
    fn visit_in<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        self.0.visit_in(v)
    }
}

//////////////////////////////////////////////
//  GENERIC IMPLEMENTATIONS ON COLLECTIONS  //
//////////////////////////////////////////////

impl<'a, T: Walk<'a, A>, A: AstTypes> Walk<'a, A> for [T] {
    fn children<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        for item in self {
            item.visit_in(v)?;
        }
        Ok(())
    }

    fn visit_in<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        for item in self {
            item.visit_in(v)?;
        }
        Ok(())
    }
}

impl<'a, T: Walk<'a, A>, A: AstTypes> Walk<'a, A> for Vec<T> {
    fn children<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        self.as_slice().children(v)
    }

    fn visit_in<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        self.as_slice().visit_in(v)
    }
}

impl<'a, T: Walk<'a, A>, A: AstTypes> Walk<'a, A> for Option<T> {
    fn children<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        match self {
            Some(t) => t.children(v),
            _ => Ok(()),
        }
    }
    fn visit_in<V: Visit<'a, A> + ?Sized>(&'a self, v: &mut V) -> Result<(), V::Error> {
        match self {
            Some(t) => t.visit_in(v),
            _ => Ok(()),
        }
    }
}

// pub trait VisitMut<'a> {
//     fn visit_mut(&mut self, walk: &'a mut impl WalkMut<'a>) {
//         walk.visit_in_mut(self);
//     }
//     fn visit_ident_mut(&mut self, name: &'a mut str) {
//         name.children_mut(self);
//     }
//     fn visit_path_mut(&mut self, walk: &'a FqPath) {
//         walk.children_mut(self);
//     }
//     fn visit_literal_mut(&mut self, walk: &'a Literal) {
//         walk.children_mut(self);
//     }
//     fn visit_use_mut(&mut self, walk: &'a Use) {
//         walk.children_mut(self);
//     }
//     fn visit_pat_mut(&mut self, walk: &'a Pat) {
//         walk.children_mut(self);
//     }
//     fn visit_bind_mut(&mut self, walk: &'a Bind) {
//         walk.children_mut(self);
//     }
//     fn visit_make_mut(&mut self, walk: &'a Make) {
//         walk.children_mut(self);
//     }
//     fn visit_makearm_mut(&mut self, walk: &'a MakeArm) {
//         walk.children_mut(self);
//     }
//     fn visit_typedef_mut(&mut self, walk: &'a Typedef) {
//         walk.children_mut(self);
//     }
//     fn visit_expr_mut(&mut self, walk: &'a Expr) {
//         walk.children_mut(self);
//     }
// }
// pub trait WalkMut<'a> {
//     fn children_mut<V: VisitMut<'a> + ?Sized>(&'a mut self, v: &mut V);
//     fn visit_in_mut<V: VisitMut<'a> + ?Sized>(&'a mut self, v: &mut V);
// }

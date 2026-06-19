//! Implements pattern matching

use super::*;
use std::collections::HashMap;

/// Stores a substitution from meta-identifiers to values
#[derive(Clone, Debug)]
pub struct Subst<A: AstTypes> {
    pub exp: HashMap<A::MacroId, Expr<A>>,
    pub pat: HashMap<A::MacroId, Pat<A>>,
}

impl<A: AstTypes> Default for Subst<A> {
    fn default() -> Self {
        Self { exp: Default::default(), pat: Default::default() }
    }
}
impl<A: AstTypes> Subst<A> {
    fn add_pat(&mut self, name: A::MacroId, pat: &Pat<A>) -> bool {
        if self.exp.contains_key(&name) {
            return false;
        }
        if let Some(entry) = self.pat.get(&name) {
            return entry == pat;
        }
        self.pat.insert(name, pat.clone()).is_none()
    }
    fn add_expr(&mut self, name: A::MacroId, exp: &Expr<A>) -> bool {
        if self.pat.contains_key(&name) {
            return false;
        }
        if let Some(entry) = self.exp.get(&name) {
            return entry == exp;
        }
        self.exp.insert(name, exp.clone()).is_none()
    }
}

pub trait Match<A: AstTypes> {
    /// Applies a substitution rule from `pat` to `template` on `self`
    fn apply_rule(&mut self, pat: &Self, template: &Self) -> bool
    where Self: Sized + Clone {
        let Some(sub) = self.match_with(pat) else {
            return false;
        };

        *self = template.clone();
        self.apply(&sub);

        true
    }

    /// With self as the pattern, recursively applies the Subst
    fn apply(&mut self, sub: &Subst<A>);

    /// Implements recursive Subst-building for Self
    fn recurse(sub: &mut Subst<A>, pat: &Self, expr: &Self) -> bool;

    /// Matches self against the provided pattern
    fn match_with(&self, pat: &Self) -> Option<Subst<A>> {
        let mut sub = Subst::default();
        Match::recurse(&mut sub, pat, self).then_some(sub)
    }
}

impl<M: Match<A> + AstNode, A: AstTypes> Match<A> for At<M, A> {
    fn recurse(sub: &mut Subst<A>, pat: &Self, expr: &Self) -> bool {
        Match::recurse(sub, &pat.0, &expr.0)
    }

    fn apply(&mut self, sub: &Subst<A>) {
        self.0.apply(sub);
    }
}

impl<A: AstTypes> Match<A> for Bind<A> {
    fn recurse(sub: &mut Subst<A>, pat: &Self, expr: &Self) -> bool {
        let (Self(pat_kind, _, pat_pat, pat_expr), Self(expr_kind, _, expr_pat, expr_expr)) =
            (pat, expr);
        pat_kind == expr_kind
            && Match::recurse(sub, pat_pat, expr_pat)
            && Match::recurse(sub, pat_expr, expr_expr)
    }

    fn apply(&mut self, sub: &Subst<A>) {
        let Self(_, _, pat, expr) = self;
        pat.apply(sub);
        expr.apply(sub);
    }
}

impl<A: AstTypes> Match<A> for Expr<A> {
    fn recurse(sub: &mut Subst<A>, pat: &Self, expr: &Self) -> bool {
        match (pat, expr) {
            (Expr::Omitted, Expr::Omitted) => true,
            (Expr::Omitted, _) => false,
            (Expr::MetId(name), _) if name.as_ref() == "_" => true,
            (Expr::MetId(name), _) => sub.add_expr(name.clone(), expr),
            (Expr::Id(pat), Expr::Id(expr)) => pat == expr,
            (Expr::Id(_), _) => false,
            (Expr::Lit(pat), Expr::Lit(expr)) => pat == expr,
            (Expr::Lit(_), _) => false,
            (Expr::Use(_), Expr::Use(_)) => true,
            (Expr::Use(_), _) => false,
            (Expr::Bind(pat), Expr::Bind(expr)) => Match::recurse(sub, pat, expr),
            (Expr::Bind(..), _) => false,
            (Expr::Make(pat), Expr::Make(expr)) => Match::recurse(sub, pat, expr),
            (Expr::Make(..), _) => false,
            (Expr::Match(pat), Expr::Match(expr)) => Match::recurse(sub, pat, expr),
            (Expr::Match(..), _) => false,
            (Expr::Label(pat), Expr::Label(expr)) => Match::recurse(sub, pat, expr),
            (Expr::Label(..), _) => false,
            (Expr::Op(pat_op, pat_exprs), Expr::Op(expr_op, expr_exprs)) => {
                Match::recurse(sub, pat_op, expr_op) && Match::recurse(sub, pat_exprs, expr_exprs)
            }
            (Expr::Op(..), _) => false,
        }
    }

    fn apply(&mut self, sub: &Subst<A>) {
        match self {
            Expr::MetId(id) => {
                if let Some(expr) = sub.exp.get(id) {
                    *self = expr.clone();
                }
            }
            Expr::Omitted | Expr::Id(_) | Expr::Lit(_) | Expr::Use(_) => {}
            Expr::Bind(expr) => expr.apply(sub),
            Expr::Make(expr) => expr.apply(sub),
            Expr::Match(expr) => expr.apply(sub),
            Expr::Label(expr) => expr.apply(sub),
            Expr::Op(op, exprs) => {
                op.apply(sub);
                exprs.apply(sub);
            }
        }
    }
}

impl<A: AstTypes> Match<A> for crate::ast::Label<A> {
    fn recurse(sub: &mut Subst<A>, pat: &Self, expr: &Self) -> bool {
        let (Label(pat_label, pat_expr), Label(expr_label, expr_expr)) = (pat, expr);
        pat_label == expr_label && Match::recurse(sub, pat_expr, expr_expr)
    }

    fn apply(&mut self, sub: &Subst<A>) {
        let Label(_, expr) = self;
        expr.apply(sub);
    }
}

impl<A: AstTypes> Match<A> for crate::ast::Make<A> {
    fn recurse(sub: &mut Subst<A>, pat: &Self, expr: &Self) -> bool {
        let (Make(pat, pat_arms), Make(expr, expr_arms)) = (pat, expr);
        Match::recurse(sub, pat, expr) && Match::recurse(sub, pat_arms, expr_arms)
    }

    fn apply(&mut self, sub: &Subst<A>) {
        let Make(expr, make_arms) = self;
        expr.apply(sub);
        make_arms.apply(sub);
    }
}

impl<A: AstTypes> Match<A> for MakeArm<A> {
    // TODO: order-independent matching for MakeArm specifically.
    fn recurse(sub: &mut Subst<A>, pat: &Self, expr: &Self) -> bool {
        pat.0 == expr.0 && Match::recurse(sub, &pat.1, &expr.1)
    }

    fn apply(&mut self, sub: &Subst<A>) {
        let Self(_, expr) = self;
        expr.apply(sub);
    }
}

impl<A: AstTypes> Match<A> for super::Match<A> {
    fn recurse(sub: &mut Subst<A>, pat: &Self, expr: &Self) -> bool {
        let (Self(pat_scr, pat_arms), Self(expr_scr, expr_arms)) = (pat, expr);
        Match::recurse(sub, pat_scr, expr_scr) && Match::recurse(sub, pat_arms, expr_arms)
    }

    fn apply(&mut self, sub: &Subst<A>) {
        let Self(scrutinee, arms) = self;
        scrutinee.apply(sub);
        arms.apply(sub);
    }
}

impl<A: AstTypes> Match<A> for MatchArm<A> {
    fn recurse(sub: &mut Subst<A>, pat: &Self, expr: &Self) -> bool {
        let (Self(pat_pat, pat_expr), Self(expr_pat, expr_expr)) = (pat, expr);
        Match::recurse(sub, pat_pat, expr_pat) && Match::recurse(sub, pat_expr, expr_expr)
    }

    fn apply(&mut self, sub: &Subst<A>) {
        let Self(pat, expr) = self;
        pat.apply(sub);
        expr.apply(sub);
    }
}

impl<A: AstTypes> Match<A> for Pat<A> {
    fn recurse(sub: &mut Subst<A>, pat: &Self, expr: &Self) -> bool {
        match (pat, expr) {
            (Pat::MetId(name), _) if name.as_ref() == "_" => true,
            (Pat::MetId(name), _) => sub.add_pat(name.clone(), expr),
            (Pat::Ignore, Pat::Ignore) => true,
            (Pat::Ignore, _) => false,
            (Pat::Never, Pat::Never) => true,
            (Pat::Never, _) => false,
            (Pat::Name(pat), Pat::Name(expr)) => pat == expr,
            (Pat::Name(_), _) => false,
            (Pat::Value(pat), Pat::Value(expr)) => pat == expr,
            (Pat::Value(_), _) => false,
            (Pat::Op(_, pat), Pat::Op(_, expr)) => Match::recurse(sub, pat, expr),
            (Pat::Op(..), _) => false,
        }
    }

    fn apply(&mut self, sub: &Subst<A>) {
        match self {
            Pat::Ignore | Pat::Never | Pat::Name(_) => {}
            Pat::Value(expr) => expr.apply(sub),
            Pat::MetId(id) => {
                if let Some(expr) = sub.pat.get(id) {
                    *self = expr.clone();
                }
            }
            Pat::Op(_, pats) => pats.apply(sub),
        }
    }
}

impl<A: AstTypes> Match<A> for Op {
    fn recurse(_: &mut Subst<A>, pat: &Self, expr: &Self) -> bool {
        pat == expr
    }

    fn apply(&mut self, _sub: &Subst<A>) {}
}

impl<A: AstTypes, T: Match<A>> Match<A> for [T] {
    fn recurse(sub: &mut Subst<A>, pat: &Self, expr: &Self) -> bool {
        if pat.len() != expr.len() {
            return false;
        }
        for (pat, expr) in pat.iter().zip(expr.iter()) {
            if !Match::recurse(sub, pat, expr) {
                return false;
            }
        }
        true
    }

    fn apply(&mut self, sub: &Subst<A>) {
        for item in self {
            item.apply(sub);
        }
    }
}

impl<A: AstTypes, T: Match<A>> Match<A> for Box<T> {
    fn recurse(sub: &mut Subst<A>, pat: &Self, expr: &Self) -> bool {
        Match::recurse(sub, pat.as_ref(), expr.as_ref())
    }

    fn apply(&mut self, sub: &Subst<A>) {
        self.as_mut().apply(sub);
    }
}

impl<A: AstTypes, T: Match<A>> Match<A> for Vec<T> {
    fn recurse(sub: &mut Subst<A>, pat: &Self, expr: &Self) -> bool {
        Match::recurse(sub, pat.as_slice(), expr.as_slice())
    }

    fn apply(&mut self, sub: &Subst<A>) {
        self.as_mut_slice().apply(sub);
    }
}

impl<A: AstTypes, T: Match<A>> Match<A> for Option<T> {
    fn recurse(sub: &mut Subst<A>, pat: &Self, expr: &Self) -> bool {
        match (pat, expr) {
            (Some(pat), Some(expr)) => Match::recurse(sub, pat, expr),
            (None, None) => true,
            _ => false,
        }
    }

    fn apply(&mut self, sub: &Subst<A>) {
        self.as_mut_slice().apply(sub);
    }
}

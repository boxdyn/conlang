//! Squashes group expressions
use crate::{ast::*, fold::*};

/// Squashes group expressions
pub struct SquashGroups;

impl<A: AstTypes> Fold<A, A> for SquashGroups {
    type Error = ();
    impl_default_fold!(A, A);

    fn fold_at_expr(&mut self, expr: At<Expr<A>, A>) -> Result<At<Expr<A>, A>, Self::Error> {
        let expr = expr.children(self)?;

        let At(Expr::Op(Op::Group, mut args), span) = expr else {
            return Ok(expr);
        };

        if args.len() != 1 {
            // TODO: should this be an error? Should we match on args.pop() instead?
            return Ok(Expr::Op(Op::Group, args).at(span));
        }

        Ok(args.pop().unwrap())
    }
}

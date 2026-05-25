//! Desugars `while {...} else` expressions
//! into `loop if {...} else break` expressions

use crate::{
    ast::*,
    fold::{Fold, Foldable, impl_default_fold},
};

/// Desugars while-else expressions
/// into loop-if-else-break expressions
pub struct WhileElseDesugar;

impl<A: AstTypes> Fold<A, A> for WhileElseDesugar {
    type Error = ();
    impl_default_fold!(A, A);

    fn fold_at_expr(&mut self, expr: At<Expr<A>, A>) -> Result<At<Expr<A>, A>, Self::Error> {
        let expr = expr.children(self)?;
        let At(Expr::Op(Op::While, mut parts), span) = expr else {
            return Ok(expr);
        };
        if parts.len() != 3 {
            std::hint::cold_path();
            panic!("`while` must have exactly 3 branches")
        }
        let fail = parts.pop().unwrap();
        let pass = parts.pop().unwrap();
        let cond = parts.pop().unwrap();
        let fail_span = fail.1;
        let fail = Expr::Op(Op::Break, vec![fail]).at(fail_span);
        let body = Expr::Op(Op::If, vec![cond, pass, fail]).at(span);
        let expr = Expr::Op(Op::Loop, vec![body]).at(span);

        Ok(expr)
    }
}

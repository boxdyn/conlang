//! Desugars `while {...} else` expressions
//! into `loop if {...} else break` expressions

use crate::{
    ast::*,
    fold::{Fold, Foldable},
};

/// Desugars while-else expressions
/// into loop-if-else-break expressions
pub struct WhileElseDesugar;

impl<A: AstTypes> Fold<A, A> for WhileElseDesugar {
    type Error = ();
    fn fold_annotation(
        &mut self,
        anno: <A as AstTypes>::Annotation,
    ) -> Result<<A as AstTypes>::Annotation, Self::Error> {
        Ok(anno)
    }
    fn fold_macro_id(
        &mut self,
        name: <A as AstTypes>::MacroId,
    ) -> Result<<A as AstTypes>::MacroId, Self::Error> {
        Ok(name)
    }
    fn fold_symbol(
        &mut self,
        name: <A as AstTypes>::Symbol,
    ) -> Result<<A as AstTypes>::Symbol, Self::Error> {
        Ok(name)
    }
    fn fold_path(
        &mut self,
        path: <A as AstTypes>::Path,
    ) -> Result<<A as AstTypes>::Path, Self::Error> {
        Ok(path)
    }
    fn fold_literal(
        &mut self,
        lit: <A as AstTypes>::Literal,
    ) -> Result<<A as AstTypes>::Literal, Self::Error> {
        Ok(lit)
    }

    fn fold_at_expr(&mut self, expr: At<Expr<A>, A>) -> Result<At<Expr<A>, A>, Self::Error> {
        let expr = expr.children(self)?;
        let At(Expr::Op(Op::While, mut parts), span) = expr else {
            return Ok(expr);
        };
        assert_eq!(parts.len(), 3, "`while` must have exactly 3 branches");
        let fail = parts.pop().unwrap();
        let pass = parts.pop().unwrap();
        let cond = parts.pop().unwrap();
        let fail_span = fail.1.clone();
        let fail = Expr::Op(Op::Break, vec![fail]).at(fail_span);
        let body = Expr::Op(Op::If, vec![cond, pass, fail]).at(span.clone());
        let expr = Expr::Op(Op::Loop, vec![body]).at(span);

        Ok(expr)
    }
}

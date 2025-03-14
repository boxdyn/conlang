//! Desugars `while {...} else` expressions
//! into `loop if {...} else break` expressions

use crate::{ast::*, ast_visitor::fold::Fold};
use cl_structures::span::Span;

/// Desugars while-else expressions
/// into loop-if-else-break expressions
pub struct WhileElseDesugar;

impl Fold for WhileElseDesugar {
    fn fold_expr(&mut self, e: Expr) -> Expr {
        let Expr { span, kind } = e;
        let kind = desugar_while(span, kind);
        Expr { span: self.fold_span(span), kind: self.fold_expr_kind(kind) }
    }
}

/// Desugars while(-else) expressions into loop-if-else-break expressions
fn desugar_while(span: Span, kind: ExprKind) -> ExprKind {
    match kind {
        // work backwards: fail -> break -> if -> loop
        ExprKind::While(While { cond, pass, fail: Else { body } }) => {
            // Preserve the else-expression's span, if present, or use the parent's span
            let fail_span = body.as_ref().map(|body| body.span).unwrap_or(span);
            let break_expr = Expr { span: fail_span, kind: ExprKind::Break(Break { body }) };

            let loop_body = If { cond, pass, fail: Else { body: Some(Box::new(break_expr)) } };
            let loop_body = ExprKind::If(loop_body);
            ExprKind::Unary(Unary {
                kind: UnaryKind::Loop,
                tail: Box::new(Expr { span, kind: loop_body }),
            })
        }
        _ => kind,
    }
}

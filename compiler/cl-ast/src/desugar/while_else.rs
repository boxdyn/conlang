//! Desugars `while {...} else` expressions
//! into `loop if {...} else break` expressions

use crate::{ast::*, ast_visitor::fold::Fold};
use cl_structures::span::Span;

/// Desugars while-else expressions
/// into loop-if-else-break expressions
pub struct WhileElseDesugar;

impl Fold for WhileElseDesugar {
    fn fold_expr(&mut self, e: Expr) -> Expr {
        let Expr { extents, kind } = e;
        let kind = desugar_while(extents, kind);
        Expr { extents: self.fold_span(extents), kind: self.fold_expr_kind(kind) }
    }
}

/// Desugars while(-else) expressions into loop-if-else-break expressions
fn desugar_while(extents: Span, kind: ExprKind) -> ExprKind {
    match kind {
        // work backwards: fail -> break -> if -> loop
        ExprKind::While(While { cond, pass, fail: Else { body } }) => {
            let Expr { extents, kind } = body
                .map(|b| *b)
                .unwrap_or(Expr { extents, kind: ExprKind::Empty });
            // Preserve the else-expression's extents, if present, or use the parent's extents
            let kind = ExprKind::Unary(Unary { kind: UnaryKind::Break, tail: kind.into() });
            let break_expr = Expr { extents, kind };

            let loop_body = If { cond, pass, fail: Else { body: Some(Box::new(break_expr)) } };
            let loop_body = ExprKind::If(loop_body);
            ExprKind::Unary(Unary { kind: UnaryKind::Loop, tail: Box::new(loop_body) })
        }
        _ => kind,
    }
}

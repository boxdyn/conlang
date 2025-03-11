//! Squashes group expressions
use crate::{ast::*, ast_visitor::fold::*};

/// Squashes group expressions
pub struct SquashGroups;

impl Fold for SquashGroups {
    fn fold_expr_kind(&mut self, kind: ExprKind) -> ExprKind {
        match kind {
            ExprKind::Group(Group { expr }) => self.fold_expr(*expr).kind,
            _ => or_fold_expr_kind(self, kind),
        }
    }
}

use tune_hir::expr::{Expr, ExprKind};

use super::{LowerContext, PlanOp};

impl LowerContext<'_> {
    pub(super) fn lower_assignment(&self, target: &Expr, value: &Expr, ops: &mut Vec<PlanOp>) {
        match &target.kind {
            ExprKind::Name(_) => {
                self.lower_expr(value, ops);
                ops.push(PlanOp::BindingSet {
                    target: self.name_target(target.id),
                });
            }
            ExprKind::Field { base, name } => {
                self.lower_expr(base, ops);
                self.lower_expr(value, ops);
                let field = name.clone().unwrap_or_default();
                ops.push(PlanOp::FieldSet {
                    member: self.field_member(base, &field),
                    field,
                    base: self.field_base_target(base),
                    span: target.span,
                });
            }
            ExprKind::Index { base, index } => {
                self.lower_expr(base, ops);
                self.lower_expr(index, ops);
                self.lower_expr(value, ops);
                ops.push(PlanOp::SequenceSet {
                    checked: true,
                    index_member: self.index_member(base),
                    base: self.field_base_target(base),
                });
            }
            _ => {
                self.lower_expr(target, ops);
                self.lower_expr(value, ops);
                ops.push(PlanOp::Assign);
            }
        }
    }
}

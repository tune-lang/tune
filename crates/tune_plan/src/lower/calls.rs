use tune_hir::expr::{Expr, ExprKind};
use tune_resolve::NameTarget;

use super::LowerContext;
use super::values::task_join_base;
use crate::PlanOp;

impl LowerContext<'_> {
    pub(super) fn lower_call(&self, callee: &Expr, args: &[Expr], ops: &mut Vec<PlanOp>) {
        if let Some(base) = task_join_base(callee, args) {
            self.lower_expr(base, ops);
            ops.push(PlanOp::TaskJoin);
            return;
        }

        if let ExprKind::Field { base, .. } = &callee.kind {
            self.lower_expr(base, ops);
            for arg in args {
                self.lower_expr(arg, ops);
            }
            ops.push(self.call_op(callee, args.len()));
            return;
        }

        if !self.static_call_target(callee) {
            self.lower_expr(callee, ops);
        }
        for arg in args {
            self.lower_expr(arg, ops);
        }
        ops.push(self.call_op(callee, args.len()));
    }

    fn call_op(&self, callee: &Expr, arg_count: usize) -> PlanOp {
        if let ExprKind::Field { base, name } = &callee.kind {
            let name = name.clone().unwrap_or_default();
            return PlanOp::MemberCall {
                member: self.callable_member(base, &name),
                name,
                arg_count,
            };
        }

        match self.name_target(callee.id) {
            Some(NameTarget::TopLevel(target)) => PlanOp::DirectCall { target, arg_count },
            Some(NameTarget::Variant(variant)) => PlanOp::VariantConstruct { variant, arg_count },
            _ => PlanOp::BoundCall,
        }
    }

    fn static_call_target(&self, callee: &Expr) -> bool {
        matches!(
            self.name_target(callee.id),
            Some(NameTarget::TopLevel(_) | NameTarget::Variant(_))
        )
    }
}

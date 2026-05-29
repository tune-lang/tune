use tune_hir::expr::{Expr, ExprKind};
use tune_resolve::NameTarget;
use tune_shape::Shape;

use super::LowerContext;
use super::StructuralWitnessKind;
use super::values::task_join_base;
use crate::PlanOp;

impl LowerContext<'_> {
    pub(super) fn lower_call(
        &self,
        expr: tune_hir::ExprId,
        callee: &Expr,
        args: &[Expr],
        ops: &mut Vec<PlanOp>,
    ) {
        if let Some(base) = task_join_base(callee, args)
            && matches!(self.expr_shape(base), Some(Shape::Task(_)))
        {
            self.lower_expr(base, ops);
            ops.push(PlanOp::TaskJoin { span: callee.span });
            return;
        }

        if self.lower_structural_witness_call(callee, args, ops) {
            return;
        }

        if let ExprKind::Field { base, name } = &callee.kind {
            if args.is_empty()
                && name.as_deref() == Some("len")
                && matches!(self.expr_shape(base), Some(Shape::String))
            {
                self.lower_expr(base, ops);
                ops.push(PlanOp::StringLen { span: callee.span });
                return;
            }
            self.lower_expr(base, ops);
            for arg in args {
                self.lower_expr(arg, ops);
            }
            ops.push(self.call_op(expr, callee, args.len()));
            return;
        }

        if !self.static_call_target(callee) {
            self.lower_expr(callee, ops);
        }
        for arg in args {
            self.lower_expr(arg, ops);
        }
        ops.push(self.call_op(expr, callee, args.len()));
    }

    fn lower_structural_witness_call(
        &self,
        callee: &Expr,
        args: &[Expr],
        ops: &mut Vec<PlanOp>,
    ) -> bool {
        let Some(witness) = self.structural_witness_for_expr(callee) else {
            return false;
        };
        if witness.kind != StructuralWitnessKind::Callable {
            return false;
        }

        ops.push(PlanOp::BindingGet {
            source: Some(witness.source),
        });
        for arg in args {
            self.lower_expr(arg, ops);
        }
        ops.push(PlanOp::MemberCall {
            member: Some(witness.member),
            name: witness.name.clone(),
            arg_count: args.len(),
            span: callee.span,
        });
        true
    }

    fn call_op(&self, expr: tune_hir::ExprId, callee: &Expr, arg_count: usize) -> PlanOp {
        if let ExprKind::Field { base, name } = &callee.kind {
            let name = name.clone().unwrap_or_default();
            return PlanOp::MemberCall {
                member: self.callable_member(base, &name),
                name,
                arg_count,
                span: callee.span,
            };
        }

        match self.name_target(callee.id) {
            Some(NameTarget::TopLevel(target)) if self.is_callable_decl(target) => {
                PlanOp::DirectCall {
                    target,
                    arg_count,
                    type_args: self.call_type_args(expr),
                    span: callee.span,
                }
            }
            Some(NameTarget::Variant(variant)) => PlanOp::VariantConstruct {
                variant,
                arg_count,
                span: callee.span,
            },
            Some(NameTarget::TopLevel(_)) => PlanOp::BoundCall {
                arg_count,
                span: callee.span,
            },
            _ => PlanOp::BoundCall {
                arg_count,
                span: callee.span,
            },
        }
    }

    fn is_callable_decl(&self, target: tune_hir::HirId) -> bool {
        let Some(module) = self.module else {
            return true;
        };
        module
            .items
            .iter()
            .find(|item| item.id == target)
            .is_some_and(|item| item.kind == tune_hir::item::ItemKind::CallableDecl)
    }

    fn call_type_args(&self, expr: tune_hir::ExprId) -> Vec<Shape> {
        self.analysis
            .and_then(|analysis| analysis.calls.iter().find(|call| call.expr == expr))
            .map_or_else(Vec::new, |call| call.type_args.clone())
    }

    fn static_call_target(&self, callee: &Expr) -> bool {
        match self.name_target(callee.id) {
            Some(NameTarget::TopLevel(target)) => self.is_callable_decl(target),
            Some(NameTarget::Variant(_)) => true,
            _ => false,
        }
    }
}

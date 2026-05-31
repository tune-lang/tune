use tune_hir::expr::{Expr, ExprKind};
use tune_resolve::NameTarget;
use tune_shape::{CallTarget, Shape};

use super::LowerContext;
use crate::PlanOp;

impl LowerContext<'_> {
    pub(super) fn lower_call(
        &self,
        expr: tune_hir::ExprId,
        callee: &Expr,
        args: &[Expr],
        ops: &mut Vec<PlanOp>,
    ) {
        if matches!(self.call_target(expr), Some(CallTarget::TaskJoin))
            && let ExprKind::Field { base, .. } = &callee.kind
        {
            self.lower_expr(base, ops);
            ops.push(PlanOp::TaskJoin { span: callee.span });
            return;
        }

        if matches!(self.call_target(expr), Some(CallTarget::StringLen))
            && let ExprKind::Field { base, .. } = &callee.kind
        {
            self.lower_expr(base, ops);
            ops.push(PlanOp::StringLen { span: callee.span });
            return;
        }

        if matches!(self.call_target(expr), Some(CallTarget::SequenceLen))
            && let ExprKind::Field { base, .. } = &callee.kind
        {
            self.lower_expr(base, ops);
            ops.push(PlanOp::SequenceLen { span: callee.span });
            return;
        }

        let resolved_static = match self.name_target(callee.id) {
            Some(NameTarget::Variant(_)) => true,
            Some(NameTarget::TopLevel(target)) => {
                self.host_symbol(target).is_some() || self.is_callable_decl(target)
            }
            _ => false,
        };
        if resolved_static {
            self.lower_call_args(expr, args, ops);
            ops.push(self.call_op(expr, callee, args.len()));
            return;
        }

        if let ExprKind::Field { base, .. } = &callee.kind {
            self.lower_expr(base, ops);
            self.lower_call_args(expr, args, ops);
            ops.push(self.call_op(expr, callee, args.len()));
            return;
        }

        if !self.static_call_target(callee) {
            self.lower_expr(callee, ops);
        }
        self.lower_call_args(expr, args, ops);
        ops.push(self.call_op(expr, callee, args.len()));
    }

    fn call_op(&self, expr: tune_hir::ExprId, callee: &Expr, arg_count: usize) -> PlanOp {
        if let Some(op) = match self.name_target(callee.id) {
            Some(NameTarget::TopLevel(target)) => {
                if let Some(symbol) = self.host_symbol(target) {
                    Some(PlanOp::HostCall {
                        symbol,
                        task_safe: self.host_function_task_safe(target),
                        arg_count,
                        span: callee.span,
                    })
                } else if self.is_callable_decl(target) {
                    Some(PlanOp::DirectCall {
                        target,
                        arg_count,
                        type_args: self.call_type_args(expr),
                        span: callee.span,
                    })
                } else if matches!(callee.kind, ExprKind::Field { .. }) {
                    None
                } else {
                    Some(PlanOp::BoundCall {
                        arg_count,
                        span: callee.span,
                    })
                }
            }
            Some(NameTarget::Variant(variant)) => Some(PlanOp::VariantConstruct {
                variant,
                arg_count,
                span: callee.span,
            }),
            _ => None,
        } {
            return op;
        }

        if let ExprKind::Field { base, name } = &callee.kind {
            let name = name.clone().unwrap_or_default();
            return PlanOp::MemberCall {
                member: self.callable_member(base, &name),
                name,
                arg_count,
                span: callee.span,
            };
        }

        PlanOp::BoundCall {
            arg_count,
            span: callee.span,
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

    fn host_symbol(&self, target: tune_hir::HirId) -> Option<tune_host::HostSymbolId> {
        let item = self.module?.items.iter().find(|item| item.id == target)?;
        match item.external.as_ref()? {
            tune_hir::item::ExternalItem::HostFunction { symbol, .. } => {
                Some(tune_host::HostSymbolId(symbol.0))
            }
            tune_hir::item::ExternalItem::HostValueType { .. }
            | tune_hir::item::ExternalItem::HostResourceType { .. }
            | tune_hir::item::ExternalItem::ModuleNamespace { .. } => None,
        }
    }

    fn host_function_task_safe(&self, target: tune_hir::HirId) -> bool {
        let Some(item) = self
            .module
            .and_then(|module| module.items.iter().find(|item| item.id == target))
        else {
            return false;
        };
        match item.external.as_ref() {
            Some(tune_hir::item::ExternalItem::HostFunction { task_safe, .. }) => *task_safe,
            _ => false,
        }
    }

    fn call_type_args(&self, expr: tune_hir::ExprId) -> Vec<Shape> {
        self.analysis
            .and_then(|analysis| analysis.calls.iter().find(|call| call.expr == expr))
            .map_or_else(Vec::new, |call| call.type_args.clone())
    }

    fn lower_call_args(&self, expr: tune_hir::ExprId, args: &[Expr], ops: &mut Vec<PlanOp>) {
        let params = self
            .analysis
            .and_then(|analysis| analysis.calls.iter().find(|call| call.expr == expr))
            .map(|call| call.params.as_slice())
            .unwrap_or(&[]);
        for (index, arg) in args.iter().enumerate() {
            self.lower_expr_for_shape(arg, params.get(index).cloned(), ops);
        }
    }

    fn call_target(&self, expr: tune_hir::ExprId) -> Option<CallTarget> {
        self.analysis
            .and_then(|analysis| analysis.calls.iter().find(|call| call.expr == expr))
            .map(|call| call.target)
    }

    fn static_call_target(&self, callee: &Expr) -> bool {
        match self.name_target(callee.id) {
            Some(NameTarget::TopLevel(target)) if self.host_symbol(target).is_some() => true,
            Some(NameTarget::TopLevel(target)) => self.is_callable_decl(target),
            Some(NameTarget::Variant(_)) => true,
            _ => false,
        }
    }
}

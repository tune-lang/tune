use tune_hir::expr::{Expr, ExprKind, LiteralKind};
use tune_shape::{BindingKey, LiteralFact, Shape};

use super::LowerContext;
use crate::PlanOp;

impl LowerContext<'_> {
    pub(super) fn lower_materialized_numeric_expr(
        &self,
        expr: &Expr,
        ops: &mut Vec<PlanOp>,
    ) -> bool {
        let Some(text) = numeric_literal_text(expr) else {
            return false;
        };
        let target = self
            .analysis
            .and_then(|analysis| {
                analysis
                    .materializations
                    .iter()
                    .find(|materialization| materialization.expr == expr.id)
            })
            .map(|materialization| &materialization.plan.target);
        target.is_some_and(|target| self.lower_numeric_literal(text, Some(target), ops))
    }

    pub(super) fn lower_numeric_expr_for_target(
        &self,
        expr: &Expr,
        target: &Shape,
        ops: &mut Vec<PlanOp>,
    ) -> bool {
        numeric_literal_text(expr)
            .is_some_and(|text| self.lower_numeric_literal(text, Some(target), ops))
    }

    pub(super) fn lower_numeric_binding_for_target(
        &self,
        expr: &Expr,
        target: &Shape,
        ops: &mut Vec<PlanOp>,
    ) -> bool {
        self.numeric_binding_text(expr)
            .is_some_and(|text| self.lower_numeric_literal(text, Some(target), ops))
    }

    fn numeric_binding_text<'a>(&'a self, expr: &Expr) -> Option<&'a str> {
        let ExprKind::Name(_) = expr.kind else {
            return None;
        };
        let key = match self.name_target(expr.id)? {
            tune_resolve::NameTarget::Local(local) => BindingKey::Local(local),
            tune_resolve::NameTarget::Param(param) => BindingKey::Param(param),
            tune_resolve::NameTarget::TopLevel(item) => BindingKey::TopLevel(item),
            tune_resolve::NameTarget::SelfValue | tune_resolve::NameTarget::Variant(_) => {
                return None;
            }
        };
        let binding = self.analysis?.frame.get(key)?;
        if binding.storage_shape != Shape::Hole {
            return None;
        }
        match binding.literal_fact.as_ref()? {
            LiteralFact::Numeric { text } => Some(text.as_str()),
            _ => None,
        }
    }

    pub(super) fn lower_numeric_literal(
        &self,
        text: &str,
        expected: Option<&Shape>,
        ops: &mut Vec<PlanOp>,
    ) -> bool {
        match expected {
            Some(Shape::Float) => parse_float(text).is_some_and(|value| {
                ops.push(PlanOp::ConstFloat {
                    bits: value.to_bits(),
                });
                true
            }),
            Some(Shape::Size) => parse_unsigned(text).is_some_and(|value| {
                if let Ok(value) = u64::try_from(value) {
                    ops.push(PlanOp::ConstSize { value });
                    true
                } else {
                    false
                }
            }),
            Some(Shape::Byte) => parse_unsigned(text).is_some_and(|value| {
                if let Ok(value) = u8::try_from(value) {
                    ops.push(PlanOp::ConstByte { value });
                    true
                } else {
                    false
                }
            }),
            _ => {
                if let Ok(value) = text.replace('_', "").parse::<i64>() {
                    ops.push(PlanOp::ConstInt { value });
                    true
                } else {
                    false
                }
            }
        }
    }
}

fn numeric_literal_text(expr: &Expr) -> Option<&str> {
    match &expr.kind {
        ExprKind::Literal(LiteralKind::Int(text) | LiteralKind::Float(text)) => Some(text),
        _ => None,
    }
}

fn parse_unsigned(text: &str) -> Option<u128> {
    text.replace('_', "").parse::<u128>().ok()
}

fn parse_float(text: &str) -> Option<f64> {
    text.replace('_', "")
        .parse::<f64>()
        .ok()
        .filter(|value| value.is_finite())
}

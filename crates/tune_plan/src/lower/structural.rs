use tune_hir::expr::{Expr, ExprKind};
use tune_hir::pattern::PatternKind;
use tune_resolve::NameTarget;

use super::{LowerContext, PlanOp, StructuralWitness, StructuralWitnessKind};

impl LowerContext<'_> {
    pub(super) fn with_struct_state(&self, struct_state: super::StructStatePlan) -> Self {
        Self {
            resolved: self.resolved,
            module: self.module,
            analysis: self.analysis,
            self_shape: self.self_shape.clone(),
            struct_state,
            structural_witnesses: self.structural_witnesses.clone(),
            param_shapes: self.param_shapes.clone(),
        }
    }

    pub(super) fn structural_witness_for_expr(&self, expr: &Expr) -> Option<&StructuralWitness> {
        let local = match self.name_target(expr.id)? {
            NameTarget::Local(local) => local,
            _ => return None,
        };
        self.structural_witnesses
            .iter()
            .rev()
            .find(|witness| witness.local == local)
    }

    pub(super) fn lower_structural_witness_get(&self, expr: &Expr, ops: &mut Vec<PlanOp>) -> bool {
        let Some(witness) = self.structural_witness_for_expr(expr) else {
            return false;
        };
        if witness.kind != StructuralWitnessKind::Field {
            return false;
        }
        ops.push(PlanOp::BindingGet {
            source: Some(witness.source),
        });
        ops.push(PlanOp::FieldGet {
            field: witness.name.clone(),
            member: Some(witness.member),
            span: expr.span,
        });
        true
    }

    pub(super) fn lower_structural_match(
        &self,
        scrutinee: &Expr,
        arms: &[tune_hir::expr::MatchArm],
        ops: &mut Vec<PlanOp>,
    ) -> bool {
        if !arms
            .iter()
            .any(|arm| matches!(arm.pattern.kind, PatternKind::StructuralShape(_)))
        {
            return false;
        }
        let Some(source) = self.scrutinee_source(scrutinee) else {
            return false;
        };
        let Some(struct_name) = self
            .expr_shape(scrutinee)
            .and_then(|shape| self.struct_shape_name(&shape).map(str::to_owned))
        else {
            return false;
        };

        for arm in arms {
            match &arm.pattern.kind {
                PatternKind::StructuralShape(requirements)
                    if self.struct_satisfies_requirements(&struct_name, requirements) =>
                {
                    let witnesses =
                        self.structural_witnesses_for(source, &struct_name, &arm.pattern);
                    let context = self.with_structural_witnesses(witnesses);
                    context.lower_expr(&arm.body, ops);
                    return true;
                }
                PatternKind::StructuralShape(_) => {}
                PatternKind::Else => {
                    self.lower_expr(&arm.body, ops);
                    return true;
                }
                _ => return false,
            }
        }

        false
    }

    fn with_structural_witnesses(&self, structural_witnesses: Vec<StructuralWitness>) -> Self {
        let mut combined = self.structural_witnesses.clone();
        combined.extend(structural_witnesses);
        Self {
            resolved: self.resolved,
            module: self.module,
            analysis: self.analysis,
            self_shape: self.self_shape.clone(),
            struct_state: self.struct_state,
            structural_witnesses: combined,
            param_shapes: self.param_shapes.clone(),
        }
    }

    fn scrutinee_source(&self, scrutinee: &Expr) -> Option<NameTarget> {
        let ExprKind::Name(_) = &scrutinee.kind else {
            return None;
        };
        self.name_target(scrutinee.id)
    }
}

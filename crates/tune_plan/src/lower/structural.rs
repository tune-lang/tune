use tune_hir::expr::{Expr, ExprKind};
use tune_hir::item::ItemKind;
use tune_hir::pattern::PatternKind;
use tune_resolve::NameTarget;

use super::values::expr_produces_value;
use super::{LowerContext, PlanOp, StructEscapeReason, StructuralWitness, StructuralWitnessKind};
use crate::PlanIfBranch;

impl LowerContext<'_> {
    pub(super) fn clone_context(&self) -> Self {
        Self {
            resolved: self.resolved,
            module: self.module,
            analysis: self.analysis,
            self_shape: self.self_shape.clone(),
            struct_escape: self.struct_escape,
            structural_witnesses: self.structural_witnesses.clone(),
            param_shapes: self.param_shapes.clone(),
            captured_locals: self.captured_locals.clone(),
        }
    }

    pub(super) fn with_struct_escape(&self, struct_escape: StructEscapeReason) -> Self {
        Self {
            resolved: self.resolved,
            module: self.module,
            analysis: self.analysis,
            self_shape: self.self_shape.clone(),
            struct_escape,
            structural_witnesses: self.structural_witnesses.clone(),
            param_shapes: self.param_shapes.clone(),
            captured_locals: self.captured_locals.clone(),
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
            return self.lower_unknown_structural_match(scrutinee, arms, source, ops, false);
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

    pub(super) fn lower_structural_return_match(
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
            return self.lower_unknown_structural_match(scrutinee, arms, source, ops, true);
        };

        for arm in arms {
            match &arm.pattern.kind {
                PatternKind::StructuralShape(requirements)
                    if self.struct_satisfies_requirements(&struct_name, requirements) =>
                {
                    let witnesses =
                        self.structural_witnesses_for(source, &struct_name, &arm.pattern);
                    let context = self.with_structural_witnesses(witnesses);
                    context.lower_return_expr(&arm.body, ops);
                    return true;
                }
                PatternKind::StructuralShape(_) => {}
                PatternKind::Else => {
                    self.lower_return_expr(&arm.body, ops);
                    return true;
                }
                _ => return false,
            }
        }

        false
    }

    fn lower_unknown_structural_match(
        &self,
        scrutinee: &Expr,
        arms: &[tune_hir::expr::MatchArm],
        source: NameTarget,
        ops: &mut Vec<PlanOp>,
        _returns: bool,
    ) -> bool {
        let Some(module) = self.module else {
            return false;
        };
        let mut branches = Vec::new();
        let mut else_ops = Vec::new();

        for arm in arms {
            match &arm.pattern.kind {
                PatternKind::StructuralShape(requirements) => {
                    for item in module
                        .items
                        .iter()
                        .filter(|item| item.kind == ItemKind::Struct)
                    {
                        let Some(struct_name) = item.name.as_deref() else {
                            continue;
                        };
                        if !self.struct_satisfies_requirements(struct_name, requirements) {
                            continue;
                        }
                        let witnesses =
                            self.structural_witnesses_for(source, struct_name, &arm.pattern);
                        let context = self.with_structural_witnesses(witnesses);
                        let body_ops = context.lower_expr_to_ops(&arm.body);
                        branches.push(PlanIfBranch {
                            condition: scrutinee.id,
                            body: arm.body.id,
                            condition_ops: vec![
                                PlanOp::BindingGet {
                                    source: Some(source),
                                },
                                PlanOp::StructIs {
                                    item: item.id,
                                    span: arm.pattern.span,
                                },
                            ],
                            body_ops,
                        });
                    }
                }
                PatternKind::Else => {
                    else_ops = self.lower_expr_to_ops(&arm.body);
                    break;
                }
                _ => return false,
            }
        }

        if branches.is_empty() {
            return false;
        }
        ops.push(PlanOp::If {
            branches,
            else_body: None,
            else_ops,
            produces_value: arms.iter().all(|arm| expr_produces_value(&arm.body)),
            span: scrutinee.span,
        });
        true
    }

    fn with_structural_witnesses(&self, structural_witnesses: Vec<StructuralWitness>) -> Self {
        let mut combined = self.structural_witnesses.clone();
        combined.extend(structural_witnesses);
        Self {
            structural_witnesses: combined,
            ..self.clone_context()
        }
    }

    fn scrutinee_source(&self, scrutinee: &Expr) -> Option<NameTarget> {
        let ExprKind::Name(_) = &scrutinee.kind else {
            return None;
        };
        self.name_target(scrutinee.id)
    }
}

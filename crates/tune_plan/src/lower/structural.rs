use tune_hir::expr::{Expr, ExprKind};
use tune_hir::item::ItemKind;
use tune_hir::pattern::PatternKind;
use tune_resolve::NameTarget;
use tune_shape::{NominalShape, Shape};

use super::values::expr_produces_value;
use super::{LowerContext, PlanOp, StructEscapeReason};
use crate::PlanIfBranch;

impl LowerContext<'_> {
    pub(super) fn clone_context(&self) -> Self {
        Self {
            resolved: self.resolved,
            module: self.module,
            analysis: self.analysis,
            self_shape: self.self_shape.clone(),
            struct_escape: self.struct_escape,
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
            param_shapes: self.param_shapes.clone(),
            captured_locals: self.captured_locals.clone(),
        }
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
                    self.lower_expr(&arm.body, ops);
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
                    self.lower_return_expr(&arm.body, ops);
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
                        let context = self.with_structural_source_shape(source, item);
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

    fn scrutinee_source(&self, scrutinee: &Expr) -> Option<NameTarget> {
        let ExprKind::Name(_) = &scrutinee.kind else {
            return None;
        };
        self.name_target(scrutinee.id)
    }

    fn with_structural_source_shape(
        &self,
        source: NameTarget,
        item: &tune_hir::item::Item,
    ) -> Self {
        let mut context = self.clone_context();
        let Some(name) = item.name.as_ref() else {
            return context;
        };
        if let NameTarget::Param(param) = source {
            context.param_shapes.to_mut().push((
                param,
                Shape::Struct(NominalShape::new(item.id, name.clone())),
            ));
        }
        context
    }
}

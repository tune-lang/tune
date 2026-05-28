use tune_hir::expr::Expr;
use tune_hir::item::StructMember;
use tune_hir::pattern::{Pattern, PatternKind, StructuralRequirementKind};

use crate::{MemberRequirement, Shape, lower_resolved_hir_shape};

use super::Analyzer;

impl Analyzer<'_> {
    pub(in crate::analyze) fn apply_structural_pattern(
        &mut self,
        scrutinee: &Expr,
        pattern: &Pattern,
        scrutinee_shape: &Shape,
    ) {
        let PatternKind::StructuralShape(requirements) = &pattern.kind else {
            return;
        };
        let structural = Shape::Structural(
            requirements
                .iter()
                .map(|requirement| match &requirement.kind {
                    StructuralRequirementKind::Field { name, shape } => MemberRequirement::Field {
                        name: name.clone(),
                        shape: shape
                            .as_ref()
                            .map(|shape| self.lower_structural_shape(shape)),
                    },
                    StructuralRequirementKind::Callable { name, params, ret } => {
                        MemberRequirement::Callable {
                            name: name.clone(),
                            params: params
                                .iter()
                                .map(|shape| self.lower_structural_shape(shape))
                                .collect(),
                            ret: ret.as_ref().map(|shape| self.lower_structural_shape(shape)),
                        }
                    }
                })
                .collect(),
        );
        if !self.structural_pattern_can_match(scrutinee_shape, &structural) {
            return;
        }
        if let Some(key) = self.binding_key(scrutinee)
            && let Some(binding) = self.frame.get_mut(key)
        {
            binding.narrow_current(structural);
        }
    }

    pub(super) fn lower_structural_shape(&mut self, shape: &tune_hir::shape::ShapeExpr) -> Shape {
        let lowered = lower_resolved_hir_shape(shape, &self.resolved.scope);
        self.diagnostics.extend(lowered.diagnostics);
        lowered.shape
    }

    fn structural_pattern_can_match(
        &mut self,
        scrutinee_shape: &Shape,
        structural: &Shape,
    ) -> bool {
        matches!(scrutinee_shape, Shape::Hole)
            || structural.accepts(scrutinee_shape)
            || self.struct_satisfies_structural(scrutinee_shape, structural)
    }

    fn struct_satisfies_structural(&mut self, scrutinee_shape: &Shape, structural: &Shape) -> bool {
        let Some(struct_name) = struct_shape_name(scrutinee_shape) else {
            return false;
        };
        let Shape::Structural(requirements) = structural else {
            return false;
        };
        requirements
            .iter()
            .all(|requirement| self.struct_satisfies_requirement(struct_name, requirement))
    }

    fn struct_satisfies_requirement(
        &mut self,
        struct_name: &str,
        requirement: &MemberRequirement,
    ) -> bool {
        let Some(item) = self.struct_item(struct_name) else {
            return false;
        };
        let members = item.struct_members.clone();
        match requirement {
            MemberRequirement::Field { name, shape } => members.iter().any(|member| {
                let StructMember::Field(field) = member else {
                    return false;
                };
                if field.name.as_deref() != Some(name) {
                    return false;
                }
                let actual = field
                    .shape
                    .as_ref()
                    .map(|shape| self.lower_structural_shape(shape));
                optional_shape_accepts(shape.as_ref(), actual.as_ref())
            }),
            MemberRequirement::Callable { name, params, ret } => members.iter().any(|member| {
                let StructMember::Callable(callable) = member else {
                    return false;
                };
                if callable.name.as_deref() != Some(name) || callable.params.len() != params.len() {
                    return false;
                }
                let params_match = params
                    .iter()
                    .zip(&callable.params)
                    .all(|(expected, actual)| {
                        actual
                            .shape
                            .as_ref()
                            .map(|shape| self.lower_structural_shape(shape))
                            .is_none_or(|actual| expected.accepts(&actual))
                    });
                let actual_ret = callable
                    .shape
                    .as_ref()
                    .map(|shape| self.lower_structural_shape(shape));
                params_match && optional_shape_accepts(ret.as_ref(), actual_ret.as_ref())
            }),
        }
    }
}

fn struct_shape_name(shape: &Shape) -> Option<&str> {
    match shape {
        Shape::Struct(name) | Shape::Apply { name, .. } => Some(name),
        _ => None,
    }
}

fn optional_shape_accepts(expected: Option<&Shape>, actual: Option<&Shape>) -> bool {
    match (expected, actual) {
        (None, _) | (_, None) => true,
        (Some(expected), Some(actual)) => expected.accepts(actual),
    }
}

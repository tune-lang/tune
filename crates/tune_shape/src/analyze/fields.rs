use tune_hir::HirId;
use tune_hir::expr::{Expr, ExprKind, StructFieldInit};
use tune_hir::item::{Field, StructMember};
use tune_resolve::BindingKind;

use super::{
    Analyzer,
    generics::{
        collect_generic_shape_constraints, item_type_param_solution, shape_has_type_params,
        substitute_generic_params,
    },
    item_shapes::lower_item_shape_expr,
};
use crate::{MemberRequirement, NominalShape, Shape};

impl Analyzer<'_> {
    pub(super) fn analyze_struct_literal(
        &mut self,
        name: &str,
        fields: &[StructFieldInit],
    ) -> Shape {
        let owner_hint = self.expected_struct_literal_shape(name);
        let mut solved = owner_hint
            .as_ref()
            .and_then(|shape| {
                let Shape::Apply { args, .. } = shape else {
                    return None;
                };
                let struct_id = shape.nominal()?.id?;
                Some(item_type_param_solution(self.struct_item(struct_id)?, args))
            })
            .unwrap_or_default();

        for field in fields {
            let raw_expected = self.struct_field_shape(name, &field.name);
            let expected = owner_hint.as_ref().map_or_else(
                || raw_expected.clone(),
                |owner| self.struct_field_shape_for_owner(owner, &field.name),
            );
            let check_expected = if shape_has_type_params(&expected) {
                Shape::Hole
            } else {
                expected.clone()
            };
            let actual = self.analyze_expr_expected(&field.value, &check_expected);
            if !matches!(check_expected, Shape::Hole) {
                self.constrain_expr_to_shape(&field.value, &check_expected);
                self.check_value_against(&check_expected, &actual, field.value.span);
            }
            collect_generic_shape_constraints(&raw_expected, &actual, &mut solved);
        }

        self.struct_literal_shape_with_solution(name, &solved)
            .unwrap_or_else(|| self.struct_literal_shape(name))
    }

    pub(super) fn struct_field_shape(&mut self, struct_name: &str, field_name: &str) -> Shape {
        let Some(struct_id) = self
            .resolved
            .scope
            .get(struct_name)
            .filter(|binding| binding.kind == tune_resolve::BindingKind::Struct)
            .map(|binding| binding.id)
        else {
            return Shape::Hole;
        };
        self.struct_field_shape_by_id(struct_id, field_name)
    }

    pub(super) fn struct_field_shape_for_owner(
        &mut self,
        owner_shape: &Shape,
        field_name: &str,
    ) -> Shape {
        let Some(struct_id) = struct_shape_id(owner_shape) else {
            return Shape::Hole;
        };
        let shape = self.struct_field_shape_by_id(struct_id, field_name);
        let Shape::Apply { args, .. } = owner_shape else {
            return shape;
        };
        let Some(item) = self.struct_item(struct_id).cloned() else {
            return shape;
        };
        let solved = item_type_param_solution(&item, args);
        substitute_generic_params(&shape, &solved)
    }

    fn struct_field_shape_by_id(&mut self, struct_id: HirId, field_name: &str) -> Shape {
        let field = self.struct_field(struct_id, field_name);

        field
            .as_ref()
            .map_or(Shape::Hole, |field| self.shape_for_field(field))
    }

    pub(super) fn struct_field(&self, struct_id: HirId, field_name: &str) -> Option<Field> {
        self.struct_item(struct_id).and_then(|item| {
            item.struct_members.iter().find_map(|member| {
                let StructMember::Field(field) = member else {
                    return None;
                };
                (field.name.as_deref() == Some(field_name)).then(|| field.clone())
            })
        })
    }

    fn shape_for_field(&mut self, field: &Field) -> Shape {
        if let Some(shape) = &field.shape {
            let Some(item) = self.struct_item(field.id.owner).cloned() else {
                return Shape::Hole;
            };
            let lowered = lower_item_shape_expr(shape, &item, &self.resolved.scope);
            self.diagnostics.extend(lowered.diagnostics);
            return lowered.shape;
        }

        field
            .default
            .as_ref()
            .map_or(Shape::Hole, |default| self.analyze_expr(default))
    }

    pub(super) fn field_shape(&mut self, base: &Expr, expr: &Expr) -> Shape {
        if self.binding_key(expr).is_some() {
            return self.name_shape(expr);
        }
        let base_shape = self.analyze_expr(base);
        let ExprKind::Field { name, .. } = &expr.kind else {
            return Shape::Hole;
        };
        let Some(field_name) = name.as_deref() else {
            return Shape::Hole;
        };
        if let Some(shape) = structural_field_shape(&base_shape, field_name) {
            return shape;
        }
        let Some(struct_id) = struct_shape_id(&base_shape) else {
            return Shape::Hole;
        };
        let shape = self.struct_field_shape_by_id(struct_id, field_name);
        let Shape::Apply { args, .. } = &base_shape else {
            return shape;
        };
        let Some(item) = self.struct_item(struct_id).cloned() else {
            return shape;
        };
        let solved = item_type_param_solution(&item, args);
        substitute_generic_params(&shape, &solved)
    }

    fn struct_literal_shape(&self, name: &str) -> Shape {
        match self.resolved.scope.get(name) {
            Some(binding) if binding.kind == BindingKind::Struct => {
                Shape::Struct(NominalShape::new(binding.id, name))
            }
            _ => Shape::Hole,
        }
    }

    fn expected_struct_literal_shape(&self, name: &str) -> Option<Shape> {
        let binding = self.resolved.scope.get(name)?;
        if binding.kind != BindingKind::Struct {
            return None;
        }
        let expected = self.expected_shape()?;
        match expected {
            Shape::Struct(nominal) | Shape::Apply { nominal, .. }
                if nominal.id == Some(binding.id) =>
            {
                Some(expected.clone())
            }
            _ => None,
        }
    }

    fn struct_literal_shape_with_solution(
        &self,
        name: &str,
        solved: &[(String, Shape)],
    ) -> Option<Shape> {
        let binding = self.resolved.scope.get(name)?;
        if binding.kind != BindingKind::Struct {
            return None;
        }
        let nominal = NominalShape::new(binding.id, name);
        if binding.generic_arity == 0 {
            return Some(Shape::Struct(nominal));
        }
        let item = self.struct_item(binding.id)?;
        Some(Shape::Apply {
            nominal,
            args: item
                .type_params
                .iter()
                .map(|param| {
                    param
                        .name
                        .as_ref()
                        .and_then(|name| {
                            solved
                                .iter()
                                .rev()
                                .find(|(param, _)| param == name)
                                .map(|(_, shape)| shape.clone())
                        })
                        .unwrap_or(Shape::Hole)
                })
                .collect(),
        })
    }
}

fn structural_field_shape(shape: &Shape, field_name: &str) -> Option<Shape> {
    let Shape::Structural(requirements) = shape else {
        return None;
    };
    requirements.iter().find_map(|requirement| {
        let MemberRequirement::Field { name, shape } = requirement else {
            return None;
        };
        (name == field_name).then(|| shape.clone().unwrap_or(Shape::Hole))
    })
}

fn struct_shape_id(shape: &Shape) -> Option<HirId> {
    match shape {
        Shape::Struct(nominal) | Shape::Apply { nominal, .. } => nominal.id,
        _ => None,
    }
}

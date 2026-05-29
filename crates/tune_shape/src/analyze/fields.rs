use tune_hir::HirId;
use tune_hir::expr::{Expr, ExprKind};
use tune_hir::item::{Field, ItemKind, StructMember};

use super::Analyzer;
use crate::{MemberRequirement, Shape, lower_resolved_hir_shape};

impl Analyzer<'_> {
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

    fn struct_field_shape_by_id(&mut self, struct_id: HirId, field_name: &str) -> Shape {
        let field = self
            .module
            .items
            .iter()
            .find(|item| item.kind == ItemKind::Struct && item.id == struct_id)
            .and_then(|item| {
                item.struct_members.iter().find_map(|member| {
                    let StructMember::Field(field) = member else {
                        return None;
                    };
                    (field.name.as_deref() == Some(field_name)).then(|| field.clone())
                })
            });

        field
            .as_ref()
            .map_or(Shape::Hole, |field| self.shape_for_field(field))
    }

    fn shape_for_field(&mut self, field: &Field) -> Shape {
        if let Some(shape) = &field.shape {
            let lowered = lower_resolved_hir_shape(shape, &self.resolved.scope);
            self.diagnostics.extend(lowered.diagnostics);
            return lowered.shape;
        }

        field
            .default
            .as_ref()
            .map_or(Shape::Hole, |default| self.analyze_expr(default))
    }

    pub(super) fn field_shape(&mut self, base: &Expr, expr: &Expr) -> Shape {
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
        self.struct_field_shape_by_id(struct_id, field_name)
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

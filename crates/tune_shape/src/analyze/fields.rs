use tune_hir::expr::{Expr, ExprKind};
use tune_hir::item::{ItemKind, StructMember};

use super::Analyzer;
use crate::{Shape, lower_resolved_hir_shape};

impl Analyzer<'_> {
    pub(super) fn struct_field_shape(&mut self, struct_name: &str, field_name: &str) -> Shape {
        self.module
            .items
            .iter()
            .find(|item| item.kind == ItemKind::Struct && item.name.as_deref() == Some(struct_name))
            .and_then(|item| {
                item.struct_members.iter().find_map(|member| {
                    let StructMember::Field(field) = member else {
                        return None;
                    };
                    (field.name.as_deref() == Some(field_name)).then(|| {
                        field
                            .shape
                            .as_ref()
                            .map(|shape| lower_resolved_hir_shape(shape, &self.resolved.scope))
                            .map_or(Shape::Hole, |lowered| {
                                self.diagnostics.extend(lowered.diagnostics);
                                lowered.shape
                            })
                    })
                })
            })
            .unwrap_or(Shape::Hole)
    }

    pub(super) fn field_shape(&mut self, base: &Expr, expr: &Expr) -> Shape {
        let base_shape = self.analyze_expr(base);
        let ExprKind::Field { name, .. } = &expr.kind else {
            return Shape::Hole;
        };
        let Some(field_name) = name.as_deref() else {
            return Shape::Hole;
        };
        let Some(struct_name) = struct_shape_name(&base_shape) else {
            return Shape::Hole;
        };
        self.struct_field_shape(struct_name, field_name)
    }
}

fn struct_shape_name(shape: &Shape) -> Option<&str> {
    match shape {
        Shape::Struct(name) | Shape::Apply { name, .. } => Some(name),
        _ => None,
    }
}

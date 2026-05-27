use tune_hir::MemberId;
use tune_hir::expr::{Expr, ExprKind};
use tune_hir::item::{Item, StructMember};
use tune_resolve::NameTarget;
use tune_shape::{Shape, lower_resolved_hir_shape};

use super::LowerContext;

impl LowerContext<'_> {
    pub(super) fn field_member(&self, base: &Expr, field: &str) -> Option<MemberId> {
        let shape = self.expr_shape(base)?;
        let name = self.struct_shape_name(&shape)?;
        self.struct_item(name)?
            .struct_members
            .iter()
            .find_map(|member| match member {
                StructMember::Field(member) if member.name.as_deref() == Some(field) => {
                    Some(member.id)
                }
                StructMember::Callable(member) if member.name.as_deref() == Some(field) => {
                    Some(member.id)
                }
                _ => None,
            })
    }

    pub(super) fn len_member(&self, base: &Expr) -> Option<MemberId> {
        let shape = self.expr_shape(base)?;
        let name = self.struct_shape_name(&shape)?;
        self.struct_item(name)?
            .struct_members
            .iter()
            .find_map(|member| match member {
                StructMember::Callable(member) if member.name.as_deref() == Some("len") => {
                    Some(member.id)
                }
                _ => None,
            })
    }

    pub(super) fn index_member(&self, base: &Expr) -> Option<MemberId> {
        let shape = self.expr_shape(base)?;
        let name = self.struct_shape_name(&shape)?;
        self.struct_item(name)?
            .struct_members
            .iter()
            .find_map(|member| match member {
                StructMember::IndexAccess(member) => Some(member.id),
                _ => None,
            })
    }

    pub(super) fn lower_shape(&self, shape: Option<&tune_hir::shape::ShapeExpr>) -> Option<Shape> {
        let resolved = self.resolved?;
        Some(lower_resolved_hir_shape(shape?, &resolved.scope).shape)
    }

    fn expr_shape(&self, expr: &Expr) -> Option<Shape> {
        if let Some(shape) = self.analysis_expr_shape(expr) {
            return Some(shape);
        }
        match &expr.kind {
            ExprKind::Name(_) => self.name_shape(expr),
            ExprKind::Sequence(_) => Some(Shape::Sequence(Box::new(Shape::Hole))),
            ExprKind::Call { .. } => {
                let module = self.module?;
                let resolved = self.resolved?;
                tune_shape::expr_shape_fact(expr, module, resolved)
            }
            _ => None,
        }
    }

    fn name_shape(&self, expr: &Expr) -> Option<Shape> {
        let target = self.name_target(expr.id)?;
        let module = self.module?;
        match target {
            NameTarget::TopLevel(id) => module
                .items
                .iter()
                .find(|item| item.id == id)
                .and_then(|item| self.lower_shape(item.shape.as_ref())),
            NameTarget::Param(id) => module
                .items
                .iter()
                .flat_map(|item| item.params.iter())
                .find(|param| param.id == id)
                .and_then(|param| self.lower_shape(param.shape.as_ref())),
            NameTarget::Local(_) | NameTarget::SelfValue | NameTarget::Variant(_) => None,
        }
    }

    fn analysis_expr_shape(&self, expr: &Expr) -> Option<Shape> {
        self.analysis?
            .expr_shapes
            .iter()
            .rev()
            .find(|shape| shape.expr == expr.id)
            .map(|shape| shape.shape.clone())
    }

    fn struct_shape_name<'shape>(&self, shape: &'shape Shape) -> Option<&'shape str> {
        match shape {
            Shape::Struct(name) | Shape::Apply { name, .. } => Some(name),
            _ => None,
        }
    }

    fn struct_item(&self, name: &str) -> Option<&Item> {
        self.module?.items.iter().find(|item| {
            item.kind == tune_hir::item::ItemKind::Struct && item.name.as_deref() == Some(name)
        })
    }
}

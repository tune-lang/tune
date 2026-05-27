use tune_hir::MemberId;
use tune_hir::expr::{Expr, ExprKind};
use tune_hir::shape::{ShapeExpr, ShapeExprKind};

use crate::locals::NameTarget;

use super::BodyResolver;

impl BodyResolver<'_> {
    pub(super) fn variant_for_expected_enum(
        &self,
        variant_name: &str,
        expected: Option<&ShapeExpr>,
    ) -> Option<MemberId> {
        let enum_name = expected_enum_name(expected?)?;
        self.items
            .iter()
            .find(|item| {
                item.kind == tune_hir::item::ItemKind::Enum
                    && item.name.as_deref() == Some(enum_name)
            })
            .and_then(|item| {
                item.variants
                    .iter()
                    .find(|variant| variant.name.as_deref() == Some(variant_name))
            })
            .map(|variant| variant.id)
    }

    pub(super) fn expected_shape_for_expr(&self, expr: &Expr) -> Option<ShapeExpr> {
        let ExprKind::Name(name) = &expr.kind else {
            return None;
        };
        let Some(NameTarget::TopLevel(item_id)) = self.lookup_local(name).or_else(|| {
            self.resolved
                .scope
                .get(name)
                .map(|binding| NameTarget::TopLevel(binding.id))
        }) else {
            return None;
        };
        self.items
            .iter()
            .find(|item| item.id == item_id)
            .and_then(|item| item.shape.clone())
    }
}

fn expected_enum_name(expected: &ShapeExpr) -> Option<&str> {
    match &expected.kind {
        ShapeExprKind::Named(name) | ShapeExprKind::Generic { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

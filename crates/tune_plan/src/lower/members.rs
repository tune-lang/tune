use tune_hir::MemberId;
use tune_hir::expr::{Expr, ExprKind};
use tune_hir::item::{Item, StructMember};
use tune_hir::pattern::{StructuralRequirement, StructuralRequirementKind};
use tune_resolve::NameTarget;
use tune_shape::{Shape, lower_resolved_hir_shape};

use super::LowerContext;

impl LowerContext<'_> {
    pub(super) fn struct_item_id(&self, name: &str) -> Option<tune_hir::HirId> {
        Some(self.struct_item(name)?.id)
    }

    pub(super) fn struct_field_inits(
        &self,
        name: &str,
        fields: &[tune_hir::expr::StructFieldInit],
    ) -> Vec<(MemberId, Expr)> {
        let Some(item) = self.struct_item(name) else {
            return Vec::new();
        };
        item.struct_members
            .iter()
            .filter_map(|member| {
                let StructMember::Field(field) = member else {
                    return None;
                };
                let field_name = field.name.as_deref()?;
                let value = fields
                    .iter()
                    .find(|init| init.name == field_name)
                    .map(|init| init.value.clone())
                    .or_else(|| field.default.clone())?;
                Some((field.id, value))
            })
            .collect()
    }

    pub(super) fn struct_field_shape(
        &self,
        name: &str,
        field: MemberId,
    ) -> Option<tune_shape::Shape> {
        let scope = &self.resolved?.scope;
        self.struct_item(name)?
            .struct_members
            .iter()
            .find_map(|member| match member {
                StructMember::Field(member) if member.id == field => member.shape.as_ref(),
                _ => None,
            })
            .map(|shape| lower_resolved_hir_shape(shape, scope).shape)
            .or_else(|| {
                self.struct_item(name)?
                    .struct_members
                    .iter()
                    .find_map(|member| match member {
                        StructMember::Field(member) if member.id == field => {
                            member.default.as_ref()
                        }
                        _ => None,
                    })
                    .and_then(|default| self.expr_shape(default))
            })
    }

    pub(super) fn enum_variant_id(
        &self,
        base: &Expr,
        variant_name: &str,
    ) -> Option<tune_resolve::VariantId> {
        let NameTarget::TopLevel(item_id) = self.name_target(base.id)? else {
            return None;
        };
        let item = self
            .module?
            .items
            .iter()
            .find(|item| item.id == item_id)?;
        if item.kind != tune_hir::item::ItemKind::Enum {
            return None;
        }
        item.variants.iter().find_map(|variant| {
            (variant.name.as_deref() == Some(variant_name))
                .then_some(tune_resolve::VariantId::Member(variant.id))
        })
    }

    pub(super) fn field_member(&self, base: &Expr, field: &str) -> Option<MemberId> {
        let shape = self.expr_shape(base)?;
        self.struct_item_for_shape(&shape)?
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

    fn struct_item_for_shape(&self, shape: &Shape) -> Option<&Item> {
        let nominal = shape.nominal()?;
        if let Some(id) = nominal.id {
            return self.module?.items.iter().find(|item| item.id == id);
        }
        self.struct_item(&nominal.name)
    }

    pub(super) fn field_base_target(&self, base: &Expr) -> Option<NameTarget> {
        let ExprKind::Name(_) = &base.kind else {
            return None;
        };
        self.name_target(base.id)
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

    pub(super) fn sequence_materializer(&self, shape: &Shape) -> Option<MemberId> {
        let name = self.struct_shape_name(shape)?;
        self.struct_item(name)?
            .struct_members
            .iter()
            .find_map(|member| match member {
                StructMember::SequenceMaterializer(member) => Some(member.id),
                _ => None,
            })
    }

    pub(super) fn callable_member(&self, base: &Expr, member_name: &str) -> Option<MemberId> {
        let shape = self.expr_shape(base)?;
        let name = self.struct_shape_name(&shape)?;
        self.struct_item(name)?
            .struct_members
            .iter()
            .find_map(|member| match member {
                StructMember::Callable(member) if member.name.as_deref() == Some(member_name) => {
                    Some(member.id)
                }
                _ => None,
            })
    }

    pub(super) fn struct_satisfies_requirements(
        &self,
        struct_name: &str,
        requirements: &[StructuralRequirement],
    ) -> bool {
        requirements.iter().all(|requirement| {
            self.struct_member_for_requirement(struct_name, requirement)
                .is_some()
        })
    }

    pub(super) fn lower_shape(&self, shape: Option<&tune_hir::shape::ShapeExpr>) -> Option<Shape> {
        let resolved = self.resolved?;
        Some(lower_resolved_hir_shape(shape?, &resolved.scope).shape)
    }

    pub(super) fn expr_shape(&self, expr: &Expr) -> Option<Shape> {
        if let ExprKind::Name(_) = &expr.kind
            && let Some(NameTarget::Param(id)) = self.name_target(expr.id)
            && let Some(shape) = self.param_shape(id)
        {
            return Some(shape);
        }
        if let Some(shape) = self.analysis_expr_shape(expr)
            && shape != Shape::Hole
        {
            return Some(shape);
        }
        match &expr.kind {
            ExprKind::Name(_) => self.name_shape(expr),
            ExprKind::Sequence(_) => Some(Shape::Sequence(Box::new(Shape::Hole))),
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
                .and_then(|param| {
                    self.param_shape(id)
                        .or_else(|| self.lower_shape(param.shape.as_ref()))
                }),
            NameTarget::SelfValue => self.self_shape.clone(),
            NameTarget::Local(_) | NameTarget::Variant(_) => None,
        }
    }

    fn param_shape(&self, id: MemberId) -> Option<Shape> {
        self.param_shapes
            .iter()
            .rev()
            .find(|(param, _)| *param == id)
            .map(|(_, shape)| shape.clone())
    }

    fn analysis_expr_shape(&self, expr: &Expr) -> Option<Shape> {
        self.analysis?
            .expr_shapes
            .iter()
            .rev()
            .find(|shape| shape.expr == expr.id)
            .map(|shape| shape.shape.clone())
    }

    pub(super) fn struct_shape_name<'shape>(&self, shape: &'shape Shape) -> Option<&'shape str> {
        shape.nominal_name()
    }

    pub(super) fn struct_item(&self, name: &str) -> Option<&Item> {
        self.module?.items.iter().find(|item| {
            item.kind == tune_hir::item::ItemKind::Struct && item.name.as_deref() == Some(name)
        })
    }

    fn struct_member_for_requirement(
        &self,
        struct_name: &str,
        requirement: &StructuralRequirement,
    ) -> Option<MemberId> {
        let item = self.struct_item(struct_name)?;
        item.struct_members
            .iter()
            .find_map(|member| match (&requirement.kind, member) {
                (StructuralRequirementKind::Field { name, .. }, StructMember::Field(field))
                    if field.name.as_deref() == Some(name.as_str()) =>
                {
                    Some(field.id)
                }
                (
                    StructuralRequirementKind::Callable { name, params, .. },
                    StructMember::Callable(callable),
                ) if callable.name.as_deref() == Some(name.as_str())
                    && callable.params.len() == params.len() =>
                {
                    Some(callable.id)
                }
                _ => None,
            })
    }
}

use tune_hir::MemberId;
use tune_hir::expr::{Expr, ExprKind};
use tune_hir::item::{Item, StructMember};
use tune_hir::pattern::{Pattern, PatternKind, StructuralRequirement, StructuralRequirementKind};
use tune_resolve::NameTarget;
use tune_shape::{Shape, lower_resolved_hir_shape};

use super::{FiniteForContractKind, LowerContext, StructuralWitness, StructuralWitnessKind};

impl LowerContext<'_> {
    pub(super) fn struct_item_id(&self, name: &str) -> Option<tune_hir::HirId> {
        Some(self.struct_item(name)?.id)
    }

    pub(super) fn struct_field_inits<'expr>(
        &self,
        name: &str,
        fields: &'expr [tune_hir::expr::StructFieldInit],
    ) -> Vec<(MemberId, &'expr Expr)> {
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
                let init = fields.iter().find(|init| init.name == field_name)?;
                Some((field.id, &init.value))
            })
            .collect()
    }

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

    pub(super) fn field_base_target(&self, base: &Expr) -> Option<NameTarget> {
        let ExprKind::Name(_) = &base.kind else {
            return None;
        };
        self.name_target(base.id)
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

    pub(super) fn finite_for_contract_kind(&self, base: &Expr) -> FiniteForContractKind {
        match self.expr_shape(base) {
            Some(Shape::Range(_)) => FiniteForContractKind::Range,
            Some(Shape::Sequence(_))
            | Some(Shape::Literal(tune_shape::LiteralFact::Sequence { .. })) => {
                FiniteForContractKind::Sequence
            }
            Some(Shape::Struct(_) | Shape::Apply { .. }) => FiniteForContractKind::MemberAccess,
            _ => FiniteForContractKind::Unknown,
        }
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

    pub(super) fn structural_witnesses_for(
        &self,
        source: NameTarget,
        struct_name: &str,
        pattern: &Pattern,
    ) -> Vec<StructuralWitness> {
        let PatternKind::StructuralShape(requirements) = &pattern.kind else {
            return Vec::new();
        };
        requirements
            .iter()
            .filter_map(|requirement| {
                let local = self.local_for_expr(requirement.id)?;
                let (name, kind) = match &requirement.kind {
                    StructuralRequirementKind::Field { name, .. } => {
                        (name.clone(), StructuralWitnessKind::Field)
                    }
                    StructuralRequirementKind::Callable { name, .. } => {
                        (name.clone(), StructuralWitnessKind::Callable)
                    }
                };
                Some(StructuralWitness {
                    local,
                    source,
                    member: self.struct_member_for_requirement(struct_name, requirement)?,
                    name,
                    kind,
                })
            })
            .collect()
    }

    pub(super) fn lower_shape(&self, shape: Option<&tune_hir::shape::ShapeExpr>) -> Option<Shape> {
        let resolved = self.resolved?;
        Some(lower_resolved_hir_shape(shape?, &resolved.scope).shape)
    }

    pub(super) fn expr_shape(&self, expr: &Expr) -> Option<Shape> {
        if let Some(shape) = self.analysis_expr_shape(expr)
            && shape != Shape::Hole
        {
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
                .and_then(|param| self.lower_shape(param.shape.as_ref()))
                .or_else(|| self.param_shape(id)),
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
        match shape {
            Shape::Struct(name) | Shape::Apply { name, .. } => Some(name),
            _ => None,
        }
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

use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::Expr;
use tune_hir::item::{Item, ItemKind, StructMember};
use tune_hir::pattern::{Pattern, PatternKind, StructuralRequirementKind};
use tune_hir::{ExprId, MemberId};
use tune_resolve::{LocalId, NameTarget};

use super::{Analyzer, ExprShape, FiniteForContractKind, MaterializerCheck};
use crate::lower_resolved_hir_shape;
use crate::{BindingKey, BindingState, LiteralFact, Shape, expr_shape_fact};
mod effects;
mod structural;

use effects::{expr_assigns_binding, expr_has_materializer_effect};

impl Analyzer<'_> {
    pub(super) fn check_materializer(&mut self, expected: &Shape, span: Option<Span>) {
        let materializer = self.sequence_materializer(expected);
        if materializer.is_none() && !matches!(expected, Shape::Hole | Shape::Sequence(_)) {
            self.diagnostics.push(
                Diagnostic::error(
                    codes::MATERIALIZATION_FAILED,
                    "sequence literal has no materializer for target shape",
                    span.unwrap_or_else(Span::synthetic),
                    "this target must be a sequence shape or define a sequence materializer",
                )
                .build(),
            );
        } else if materializer
            .and_then(|id| self.materializer_body(id))
            .is_some_and(expr_has_materializer_effect)
        {
            self.diagnostics.push(
                Diagnostic::error(
                    codes::MATERIALIZATION_FAILED,
                    "sequence materializer is not pure",
                    span.unwrap_or_else(Span::synthetic),
                    "materializers must not mutate state, spawn tasks, panic, or propagate errors",
                )
                .build(),
            );
        }
        self.materializers.push(MaterializerCheck {
            target: expected.clone(),
            materializer,
            span,
        });
    }

    pub(super) fn check_match_exhaustive(
        &mut self,
        expr: &Expr,
        scrutinee_shape: &Shape,
        arms: &[tune_hir::expr::MatchArm],
    ) {
        let Some(nominal) = scrutinee_shape.nominal() else {
            return;
        };
        if arms
            .iter()
            .any(|arm| matches!(arm.pattern.kind, PatternKind::Else))
        {
            return;
        }
        let Some(id) = nominal.id else {
            return;
        };
        let Some(item) = self.enum_item(id) else {
            return;
        };
        let covered = self.covered_variant_ids(item, arms);
        let missing = item.variants.iter().any(|variant| {
            variant
                .name
                .as_ref()
                .is_some_and(|_| !covered.contains(&variant.id))
        });
        if missing {
            self.diagnostics.push(
                Diagnostic::error(
                    codes::MATCH_NOT_EXHAUSTIVE,
                    "match is not exhaustive",
                    expr.span.unwrap_or_else(Span::synthetic),
                    "not every known enum variant is covered",
                )
                .with_help("add the missing variant arms or an `else` arm")
                .build(),
            );
        }
    }

    pub(super) fn iteration_contract(
        &mut self,
        shape: &Shape,
        span: Option<Span>,
    ) -> (FiniteForContractKind, Option<MemberId>, Option<MemberId>) {
        match shape {
            Shape::Hole => (FiniteForContractKind::Unknown, None, None),
            Shape::Sequence(_) | Shape::Literal(LiteralFact::Sequence { .. }) => {
                (FiniteForContractKind::Sequence, None, None)
            }
            Shape::Range(_) => (FiniteForContractKind::Range, None, None),
            Shape::Struct(nominal) | Shape::Apply { nominal, .. } => {
                let Some(id) = nominal.id else {
                    return (FiniteForContractKind::Unknown, None, None);
                };
                let len = self.callable_member(id, "len");
                let index = self.index_member(id);
                if len.is_none() || !self.len_member_returns_size(id) {
                    self.diagnostics.push(iter_diag(
                        codes::ITERATION_LEN_MISSING,
                        "finite `for` source has no `len(): Size` contract",
                        span,
                    ));
                }
                if index.is_none() || !self.index_member_accepts_size(id) {
                    self.diagnostics.push(iter_diag(
                        codes::ITERATION_INDEX_MISSING,
                        "finite `for` source has no indexed access contract",
                        span,
                    ));
                }
                (FiniteForContractKind::MemberAccess, len, index)
            }
            _ => {
                self.diagnostics.push(iter_diag(
                    codes::ITERATION_LEN_MISSING,
                    "finite `for` source has no `len(): Size` contract",
                    span,
                ));
                self.diagnostics.push(iter_diag(
                    codes::ITERATION_INDEX_MISSING,
                    "finite `for` source has no indexed access contract",
                    span,
                ));
                (FiniteForContractKind::Unknown, None, None)
            }
        }
    }

    pub(super) fn name_shape(&self, expr: &Expr) -> Shape {
        let Some(key) = self.binding_key(expr) else {
            return expr_shape_fact(expr, self.module, self.resolved).unwrap_or(Shape::Hole);
        };
        if let Some(binding) = self.frame.get(key) {
            return binding.current_shape.clone();
        }
        match key {
            BindingKey::TopLevel(item_id) => self
                .module
                .items
                .iter()
                .find(|item| item.id == item_id)
                .and_then(|item| item.shape.as_ref())
                .map(|shape| lower_resolved_hir_shape(shape, &self.resolved.scope).shape)
                .unwrap_or(Shape::Hole),
            _ => Shape::Hole,
        }
    }

    pub(super) fn bind_pattern(&mut self, pattern: &Pattern, shape: Shape) {
        match &pattern.kind {
            PatternKind::Binding(name) => self.bind_named_pattern(name, pattern, shape),
            PatternKind::Tuple(items) => {
                for item in items {
                    self.bind_pattern(item, Shape::Hole);
                }
            }
            PatternKind::Variant { args, .. } => {
                for arg in args {
                    self.bind_pattern(arg, Shape::Hole);
                }
            }
            PatternKind::StructuralShape(requirements) => {
                for requirement in requirements {
                    let (name, shape) = match &requirement.kind {
                        StructuralRequirementKind::Field { name, shape } => (
                            name,
                            shape
                                .as_ref()
                                .map(|shape| self.lower_structural_shape(shape))
                                .unwrap_or(Shape::Hole),
                        ),
                        StructuralRequirementKind::Callable { name, params, ret } => (
                            name,
                            Shape::Callable {
                                params: params
                                    .iter()
                                    .map(|shape| self.lower_structural_shape(shape))
                                    .collect(),
                                ret: Box::new(
                                    ret.as_ref()
                                        .map(|shape| self.lower_structural_shape(shape))
                                        .unwrap_or(Shape::Hole),
                                ),
                            },
                        ),
                    };
                    self.bind_named_pattern_id(name, requirement.id, requirement.span, shape);
                }
            }
            PatternKind::Hole | PatternKind::Unit | PatternKind::Else => {}
        }
    }

    pub(super) fn check_iteration_source_mutation(
        &mut self,
        iterable: &Expr,
        body: &Expr,
        span: Option<Span>,
    ) {
        let Some(source) = self.binding_key(iterable) else {
            return;
        };
        if !expr_assigns_binding(body, source, self) {
            return;
        }
        self.diagnostics.push(
            Diagnostic::warning(
                codes::ITERATION_SOURCE_MUTATED,
                "finite `for` source is mutated during iteration",
                span.unwrap_or_else(Span::synthetic),
                "the source length and indexed access contract must remain stable while iterating",
            )
            .with_help("iterate over a stable snapshot or move the mutation after the loop")
            .build(),
        );
    }

    pub(super) fn binding_key(&self, expr: &Expr) -> Option<BindingKey> {
        self.resolved
            .name_refs
            .iter()
            .find(|name_ref| name_ref.expr == expr.id)
            .and_then(|name_ref| match name_ref.target {
                NameTarget::TopLevel(id) => Some(BindingKey::TopLevel(id)),
                NameTarget::Param(id) => Some(BindingKey::Param(id)),
                NameTarget::Local(id) => Some(BindingKey::Local(id)),
                NameTarget::SelfValue => Some(BindingKey::SelfValue),
                NameTarget::Variant(_) => None,
            })
    }

    pub(super) fn local_for_expr(&self, expr: ExprId) -> Option<LocalId> {
        self.resolved
            .locals
            .iter()
            .find(|local| local.expr == Some(expr))
            .map(|local| local.id)
    }

    pub(super) fn callable_param_local(&self, name: &str, span: Option<Span>) -> Option<LocalId> {
        self.resolved
            .locals
            .iter()
            .find(|local| {
                local.kind == tune_resolve::LocalKind::CallableParam
                    && local.name == name
                    && local.span == span
            })
            .map(|local| local.id)
    }

    pub(super) fn local_name(&self, id: LocalId) -> Option<String> {
        self.resolved
            .locals
            .iter()
            .find(|local| local.id == id)
            .map(|local| local.name.clone())
    }

    pub(super) fn record_expr_shape(&mut self, expr: ExprId, shape: Shape) {
        self.expr_shapes.push(ExprShape { expr, shape });
    }

    fn bind_named_pattern(&mut self, name: &str, pattern: &Pattern, shape: Shape) {
        self.bind_named_pattern_id(name, pattern.id, pattern.span, shape);
    }

    fn bind_named_pattern_id(
        &mut self,
        name: &str,
        expr: ExprId,
        span: Option<Span>,
        shape: Shape,
    ) {
        if let Some(local) = self
            .resolved
            .locals
            .iter()
            .find(|local| local.name == name && local.expr == Some(expr) && local.span == span)
        {
            self.frame.define(BindingState::new(
                BindingKey::Local(local.id),
                Some(name.to_owned()),
                shape.clone(),
                shape,
                span,
            ));
        }
    }

    fn enum_item(&self, id: tune_hir::HirId) -> Option<&Item> {
        self.module
            .items
            .iter()
            .find(|item| item.kind == ItemKind::Enum && item.id == id)
    }

    pub(super) fn struct_item(&self, id: tune_hir::HirId) -> Option<&Item> {
        self.module
            .items
            .iter()
            .find(|item| item.kind == ItemKind::Struct && item.id == id)
    }

    fn callable_member(&self, struct_id: tune_hir::HirId, member_name: &str) -> Option<MemberId> {
        self.struct_item(struct_id)?
            .struct_members
            .iter()
            .find_map(|member| match member {
                StructMember::Callable(callable)
                    if callable.name.as_deref() == Some(member_name) =>
                {
                    Some(callable.id)
                }
                _ => None,
            })
    }

    fn index_member(&self, struct_id: tune_hir::HirId) -> Option<MemberId> {
        self.struct_item(struct_id)?
            .struct_members
            .iter()
            .find_map(|member| match member {
                StructMember::IndexAccess(access) => Some(access.id),
                _ => None,
            })
    }

    fn len_member_returns_size(&mut self, struct_id: tune_hir::HirId) -> bool {
        self.struct_item(struct_id)
            .and_then(|item| {
                item.struct_members.iter().find_map(|member| match member {
                    StructMember::Callable(callable) if callable.name.as_deref() == Some("len") => {
                        callable.shape.as_ref()
                    }
                    _ => None,
                })
            })
            .map(|shape| lower_resolved_hir_shape(shape, &self.resolved.scope).shape == Shape::Size)
            .unwrap_or(false)
    }

    fn index_member_accepts_size(&mut self, struct_id: tune_hir::HirId) -> bool {
        self.struct_item(struct_id)
            .and_then(|item| {
                item.struct_members.iter().find_map(|member| match member {
                    StructMember::IndexAccess(access) => access.index_shape.as_ref(),
                    _ => None,
                })
            })
            .map(|shape| lower_resolved_hir_shape(shape, &self.resolved.scope).shape == Shape::Size)
            .unwrap_or(false)
    }

    pub(super) fn sequence_materializer(&self, shape: &Shape) -> Option<MemberId> {
        let id = match shape {
            Shape::Struct(nominal) | Shape::Apply { nominal, .. } => nominal.id?,
            _ => return None,
        };
        self.struct_item(id)?
            .struct_members
            .iter()
            .find_map(|member| match member {
                StructMember::SequenceMaterializer(materializer) => Some(materializer.id),
                _ => None,
            })
    }

    fn materializer_body(&self, id: MemberId) -> Option<&Expr> {
        self.module
            .items
            .iter()
            .flat_map(|item| item.struct_members.iter())
            .find_map(|member| match member {
                StructMember::SequenceMaterializer(materializer) if materializer.id == id => {
                    materializer.body.as_ref()
                }
                _ => None,
            })
    }

    fn covered_variant_ids(&self, item: &Item, arms: &[tune_hir::expr::MatchArm]) -> Vec<MemberId> {
        arms.iter()
            .filter_map(|arm| pattern_variant_name(&arm.pattern))
            .filter_map(|name| {
                item.variants
                    .iter()
                    .find(|variant| variant.name.as_deref() == Some(name))
                    .map(|variant| variant.id)
            })
            .collect()
    }
}

fn iter_diag(
    code: tune_diagnostics::DiagnosticCode,
    title: &str,
    span: Option<Span>,
) -> Diagnostic {
    Diagnostic::error(
        code,
        title,
        span.unwrap_or_else(Span::synthetic),
        "finite `for` only lowers over sources with known length and indexed access",
    )
    .build()
}

fn pattern_variant_name(pattern: &Pattern) -> Option<&str> {
    match &pattern.kind {
        PatternKind::Variant { name, .. } | PatternKind::Binding(name) => Some(name),
        _ => None,
    }
}

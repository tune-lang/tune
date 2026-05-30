use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::Expr;
use tune_hir::item::{Item, ItemKind, StructMember};
use tune_hir::pattern::{Pattern, PatternKind};
use tune_hir::{ExprId, MemberId};
use tune_resolve::{LocalId, NameTarget};

use super::generics::{item_type_param_solution, substitute_generic_params};
use super::{Analyzer, ExprShape, FiniteForContractKind, MaterializerCheck};
use crate::lower_resolved_hir_shape;
use crate::{
    BindingKey, BindingState, LiteralFact, Shape, can_materialize, expr_literal_fact,
    expr_shape_fact,
};
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
        if arms
            .iter()
            .any(|arm| matches!(arm.pattern.kind, PatternKind::Else))
        {
            return;
        }
        match scrutinee_shape {
            Shape::Result { .. } => {
                self.check_result_match_exhaustive(expr, arms);
                return;
            }
            Shape::Optional(_) => {
                self.check_optional_match_exhaustive(expr, arms);
                return;
            }
            _ => {}
        }
        let Some(nominal) = scrutinee_shape.nominal() else {
            return;
        };
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

    fn check_result_match_exhaustive(&mut self, expr: &Expr, arms: &[tune_hir::expr::MatchArm]) {
        let mut covers_ok = false;
        let mut covers_error = false;
        for arm in arms {
            match self.pattern_variant(arm.pattern.id) {
                Some(tune_resolve::VariantId::Prelude(tune_resolve::PreludeVariant::Ok)) => {
                    covers_ok = true;
                }
                Some(tune_resolve::VariantId::Prelude(tune_resolve::PreludeVariant::Error)) => {
                    covers_error = true;
                }
                _ => {}
            }
        }
        if covers_ok && covers_error {
            return;
        }
        self.diagnostics.push(
            Diagnostic::error(
                codes::MATCH_NOT_EXHAUSTIVE,
                "match is not exhaustive",
                expr.span.unwrap_or_else(Span::synthetic),
                "not every Result variant is covered",
            )
            .with_help("add the missing `Ok`/`Error` arms or an `else` arm")
            .build(),
        );
    }

    fn check_optional_match_exhaustive(&mut self, expr: &Expr, arms: &[tune_hir::expr::MatchArm]) {
        let covers_none = arms
            .iter()
            .any(|arm| matches!(arm.pattern.kind, PatternKind::None));
        let covers_present = arms
            .iter()
            .any(|arm| matches!(arm.pattern.kind, PatternKind::Binding(_)));
        if covers_none && covers_present {
            return;
        }
        self.diagnostics.push(
            Diagnostic::error(
                codes::MATCH_NOT_EXHAUSTIVE,
                "match is not exhaustive",
                expr.span.unwrap_or_else(Span::synthetic),
                "not every optional case is covered",
            )
            .with_help("add both a `none` arm and a present-value arm, or add an `else` arm")
            .build(),
        );
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
                .top_level_shapes
                .get(&item_id)
                .cloned()
                .or_else(|| {
                    self.module
                        .items
                        .iter()
                        .find(|item| item.id == item_id)
                        .map(|item| self.top_level_current_shape(item))
                })
                .unwrap_or(Shape::Hole),
            _ => Shape::Hole,
        }
    }

    fn top_level_current_shape(&self, item: &Item) -> Shape {
        let declared = item
            .shape
            .as_ref()
            .map(|shape| lower_resolved_hir_shape(shape, &self.resolved.scope).shape)
            .unwrap_or(Shape::Hole);
        let Some(body) = item.body.as_ref() else {
            return if matches!(declared, Shape::Optional(_)) {
                Shape::Literal(LiteralFact::None)
            } else {
                declared
            };
        };
        if let Some(literal) = expr_literal_fact(body) {
            return top_level_literal_current_shape(&declared, &literal);
        }
        let actual =
            expr_shape_fact(body, self.module, self.resolved).unwrap_or_else(|| declared.clone());
        top_level_current_shape_from_actual(&declared, &actual)
    }

    pub(super) fn bind_pattern(&mut self, pattern: &Pattern, shape: Shape) {
        match &pattern.kind {
            PatternKind::Binding(name) => {
                let shape = match shape {
                    Shape::Optional(inner) => inner.as_ref().clone(),
                    shape => shape,
                };
                self.bind_named_pattern(name, pattern, shape);
            }
            PatternKind::Tuple(items) => {
                let shapes = match shape {
                    Shape::Tuple(shapes) => shapes,
                    Shape::Unit => Vec::new(),
                    _ => Vec::new(),
                };
                for (index, item) in items.iter().enumerate() {
                    self.bind_pattern(item, shapes.get(index).cloned().unwrap_or(Shape::Hole));
                }
            }
            PatternKind::Variant { args, .. } => {
                let payload_shapes = self.pattern_variant_payload_shapes(pattern, &shape);
                for (index, arg) in args.iter().enumerate() {
                    self.bind_pattern(
                        arg,
                        payload_shapes.get(index).cloned().unwrap_or(Shape::Hole),
                    );
                }
            }
            PatternKind::StructuralShape(_) => {}
            PatternKind::Hole | PatternKind::None | PatternKind::Unit | PatternKind::Else => {}
        }
    }

    fn pattern_variant_payload_shapes(&mut self, pattern: &Pattern, shape: &Shape) -> Vec<Shape> {
        let Some(variant) = self.pattern_variant(pattern.id) else {
            return Vec::new();
        };
        match variant {
            tune_resolve::VariantId::Prelude(tune_resolve::PreludeVariant::Ok) => match shape {
                Shape::Result { ok, .. } => vec![ok.as_ref().clone()],
                _ => vec![Shape::Hole],
            },
            tune_resolve::VariantId::Prelude(tune_resolve::PreludeVariant::Error) => match shape {
                Shape::Result { err, .. } => vec![err.as_ref().clone()],
                _ => vec![Shape::Hole],
            },
            tune_resolve::VariantId::Member(id) => {
                let Some(item) = self.enum_item(id.owner).cloned() else {
                    return Vec::new();
                };
                let Some(variant) = item.variants.iter().find(|variant| variant.id == id) else {
                    return Vec::new();
                };
                let mut payloads = Vec::new();
                for payload in &variant.payload {
                    let lowered = super::item_shapes::lower_item_shape_expr(
                        payload,
                        &item,
                        &self.resolved.scope,
                    );
                    payloads.push(lowered.shape);
                    self.diagnostics.extend(lowered.diagnostics);
                }
                let Shape::Apply { nominal, args } = shape else {
                    return payloads;
                };
                if nominal.id != Some(item.id) {
                    return payloads;
                }
                let solved = item_type_param_solution(&item, args);
                payloads
                    .iter()
                    .map(|payload| substitute_generic_params(payload, &solved))
                    .collect()
            }
        }
    }

    fn pattern_variant(&self, pattern: ExprId) -> Option<tune_resolve::VariantId> {
        self.resolved
            .variant_pattern_refs
            .iter()
            .find(|variant_ref| variant_ref.pattern == pattern)
            .map(|variant_ref| variant_ref.variant)
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

fn top_level_literal_current_shape(storage: &Shape, literal: &LiteralFact) -> Shape {
    match (storage, literal) {
        (Shape::Optional(_), LiteralFact::None) => Shape::Literal(LiteralFact::None),
        (Shape::Optional(inner), literal) if can_materialize(literal, inner) => {
            inner.as_ref().clone()
        }
        (Shape::Hole, _) => Shape::Literal(literal.clone()),
        (storage, _) => storage.clone(),
    }
}

fn top_level_current_shape_from_actual(storage: &Shape, actual: &Shape) -> Shape {
    match (storage, actual) {
        (Shape::Hole, actual) => actual.clone(),
        (Shape::Optional(_), Shape::Literal(LiteralFact::None)) => {
            Shape::Literal(LiteralFact::None)
        }
        (Shape::Optional(inner), actual) if inner.accepts(actual) => actual.clone(),
        (storage, _) => storage.clone(),
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
        PatternKind::None => None,
        _ => None,
    }
}

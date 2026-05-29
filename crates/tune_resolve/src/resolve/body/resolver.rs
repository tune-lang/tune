use std::collections::HashMap;

use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::{Expr, ExprKind, LiteralKind, StringPart};
use tune_hir::item::Item;
use tune_hir::pattern::{Pattern, PatternKind, StructuralRequirementKind};
use tune_hir::shape::ShapeExpr;
use tune_hir::{HirId, MemberId};

use crate::locals::{LocalBinding, LocalId, LocalKind, NameTarget};
use crate::prelude::VariantId;

use super::super::ResolvedModule;

mod expected;
mod names;
mod validation;

pub(super) struct BodyResolver<'resolved> {
    resolved: &'resolved mut ResolvedModule,
    items: &'resolved [Item],
    owner: HirId,
    scopes: Vec<HashMap<String, NameTarget>>,
}

impl<'resolved> BodyResolver<'resolved> {
    pub(super) fn new(
        resolved: &'resolved mut ResolvedModule,
        items: &'resolved [Item],
        owner: HirId,
    ) -> Self {
        Self {
            resolved,
            items,
            owner,
            scopes: vec![HashMap::new()],
        }
    }

    pub(super) fn resolve_expr_names(&mut self, expr: &Expr) {
        self.resolve_expr_names_with_expected(expr, None);
    }

    pub(super) fn resolve_expr_names_with_expected(
        &mut self,
        expr: &Expr,
        expected: Option<&ShapeExpr>,
    ) {
        match &expr.kind {
            ExprKind::Missing => {}
            ExprKind::Literal(LiteralKind::String(literal)) => {
                for part in &literal.parts {
                    if let StringPart::Interpolation(expr) = part {
                        self.resolve_expr_names(expr);
                    }
                }
            }
            ExprKind::Literal(_) => {}
            ExprKind::Tuple(elements) | ExprKind::Sequence(elements) => {
                for element in elements {
                    self.resolve_expr_names(element);
                }
            }
            ExprKind::Struct { name, fields } => {
                self.resolve_struct_name_ref(name, expr);
                for field in fields {
                    self.resolve_expr_names_with_expected(
                        &field.value,
                        self.expected_struct_field_shape(name, &field.name).as_ref(),
                    );
                }
            }
            ExprKind::Name(name) => {
                if !self.resolve_expected_variant_name(name, expr, expected) {
                    self.resolve_name_ref(name, expr);
                }
            }
            ExprKind::CallableValue { params, body } => {
                self.with_scope(|this| {
                    for param in params {
                        if let Some(name) = &param.name {
                            this.bind_local(name, LocalKind::CallableParam, None, param.span);
                        }
                    }
                    this.resolve_expr_names(body);
                });
            }
            ExprKind::Call { callee, args } => {
                if !self.resolve_expected_variant_callee(callee, expected) {
                    self.resolve_expr_names(callee);
                }
                let arg_shapes = self.expected_arg_shapes_for_call(callee);
                for (index, arg) in args.iter().enumerate() {
                    self.resolve_expr_names_with_expected(
                        arg,
                        arg_shapes.get(index).and_then(Option::as_ref),
                    );
                }
            }
            ExprKind::Field { base, .. } => self.resolve_expr_names(base),
            ExprKind::Index { base, index } => {
                self.resolve_expr_names(base);
                self.resolve_expr_names(index);
            }
            ExprKind::Let {
                name, shape, value, ..
            } => {
                if let Some(value) = value {
                    self.resolve_expr_names_with_expected(value, shape.as_ref());
                }
                if let Some(name) = name {
                    self.bind_local(name, LocalKind::Let, Some(expr.id), expr.span);
                }
            }
            ExprKind::Assign { target, value } => {
                self.resolve_assignment_target(target);
                self.resolve_expr_names(value);
            }
            ExprKind::Unary { expr, .. } => self.resolve_expr_names(expr),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.resolve_expr_names(lhs);
                self.resolve_expr_names(rhs);
            }
            ExprKind::Spawn(inner) | ExprKind::Propagate(inner) => {
                self.resolve_expr_names(inner);
            }
            ExprKind::If {
                branches,
                else_branch,
            } => {
                for branch in branches {
                    self.resolve_expr_names(&branch.condition);
                    self.with_scope(|this| this.resolve_expr_names(&branch.body));
                }
                if let Some(else_branch) = else_branch {
                    self.with_scope(|this| this.resolve_expr_names(else_branch));
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.resolve_expr_names(scrutinee);
                let expected = self.expected_shape_for_expr(scrutinee);
                for arm in arms {
                    self.validate_match_pattern(&arm.pattern);
                    self.with_scope(|this| {
                        this.bind_pattern_names_with_expected(&arm.pattern, expected.as_ref());
                        this.resolve_expr_names(&arm.body);
                    });
                }
            }
            ExprKind::While { condition, body } => {
                self.resolve_expr_names(condition);
                self.with_scope(|this| this.resolve_expr_names(body));
            }
            ExprKind::Loop(body) => {
                self.with_scope(|this| this.resolve_expr_names(body));
            }
            ExprKind::Break | ExprKind::Continue => {}
            ExprKind::Return(inner) => {
                if let Some(inner) = inner {
                    self.resolve_expr_names(inner);
                }
            }
            ExprKind::Panic(args) => {
                for arg in args {
                    self.resolve_expr_names(arg);
                }
            }
            ExprKind::For {
                pattern,
                iterable,
                body,
            } => {
                self.resolve_expr_names(iterable);
                self.with_scope(|this| {
                    this.bind_pattern_names(pattern);
                    this.resolve_expr_names(body);
                });
            }
            ExprKind::Block(exprs) => {
                self.with_scope(|this| {
                    for expr in exprs {
                        this.resolve_expr_names(expr);
                    }
                });
            }
        }
    }

    pub(super) fn bind_param(&mut self, name: &str, id: MemberId) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_owned(), NameTarget::Param(id));
        }
    }

    pub(super) fn bind_local(
        &mut self,
        name: &str,
        kind: LocalKind,
        expr: Option<tune_hir::ExprId>,
        span: Option<Span>,
    ) -> Option<LocalId> {
        self.validate_user_name(name, span, "local");
        let id = LocalId(u32::try_from(self.resolved.locals.len()).ok()?);
        self.resolved.locals.push(LocalBinding {
            id,
            owner: self.owner,
            kind,
            name: name.to_owned(),
            expr,
            span,
        });

        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_owned(), NameTarget::Local(id));
        }

        Some(id)
    }

    fn resolve_assignment_target(&mut self, target: &Expr) {
        match &target.kind {
            ExprKind::Name(name) => self.resolve_name_ref(name, target),
            ExprKind::Field { base, .. } => self.resolve_expr_names(base),
            ExprKind::Index { base, index } => {
                self.resolve_expr_names(base);
                self.resolve_expr_names(index);
            }
            _ => {
                self.resolve_expr_names(target);
                self.resolved.diagnostics.push(
                    Diagnostic::error(
                        codes::INVALID_ASSIGNMENT_TARGET,
                        "invalid assignment target",
                        target.span.unwrap_or_else(Span::synthetic),
                        "assignment target must be a name, field, or indexed access",
                    )
                    .build(),
                );
            }
        }
    }

    fn bind_pattern_names(&mut self, pattern: &Pattern) {
        self.bind_pattern_names_with_expected(pattern, None);
    }

    fn bind_pattern_names_with_expected(
        &mut self,
        pattern: &Pattern,
        expected: Option<&ShapeExpr>,
    ) {
        match &pattern.kind {
            PatternKind::Binding(name) => {
                self.bind_local(name, LocalKind::Pattern, Some(pattern.id), pattern.span);
            }
            PatternKind::Tuple(patterns) => {
                for pattern in patterns {
                    self.bind_pattern_names_with_expected(pattern, None);
                }
            }
            PatternKind::Variant { name, args } => {
                self.resolve_variant_pattern(pattern.id, name, pattern.span, expected);
                for pattern in args {
                    self.bind_pattern_names_with_expected(pattern, None);
                }
            }
            PatternKind::StructuralShape(requirements) => {
                for requirement in requirements {
                    let name = match &requirement.kind {
                        StructuralRequirementKind::Callable { name, .. }
                        | StructuralRequirementKind::Field { name, .. } => name,
                    };
                    self.bind_local(
                        name,
                        LocalKind::Pattern,
                        Some(requirement.id),
                        requirement.span,
                    );
                }
            }
            PatternKind::Hole | PatternKind::Unit | PatternKind::Else => {}
        }
    }

    fn resolve_variant_pattern(
        &mut self,
        pattern: tune_hir::ExprId,
        name: &str,
        span: Option<Span>,
        expected: Option<&ShapeExpr>,
    ) {
        if let Some(variant) = self
            .variant_for_expected_enum(name, expected)
            .map(VariantId::Member)
            .or_else(|| self.variant_by_name(name))
        {
            self.resolved
                .variant_pattern_refs
                .push(crate::VariantPatternRef {
                    pattern,
                    variant,
                    span,
                });
            return;
        }

        if self.resolved.variants.is_ambiguous(name) {
            self.resolved.diagnostics.push(
                Diagnostic::error(
                    codes::UNRESOLVED_NAME,
                    format!("ambiguous variant pattern `{name}`"),
                    span.unwrap_or_else(Span::synthetic),
                    "this variant name is provided by more than one enum",
                )
                .build(),
            );
            return;
        }

        self.resolved.diagnostics.push(
            Diagnostic::error(
                codes::UNRESOLVED_NAME,
                format!("unresolved variant pattern `{name}`"),
                span.unwrap_or_else(Span::synthetic),
                "this variant pattern has no matching enum variant",
            )
            .build(),
        );
    }

    fn with_scope(&mut self, f: impl FnOnce(&mut Self)) {
        self.scopes.push(HashMap::new());
        f(self);
        self.scopes.pop();
    }
}

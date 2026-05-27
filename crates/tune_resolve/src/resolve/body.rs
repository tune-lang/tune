use std::collections::HashMap;

use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::{Expr, ExprKind};
use tune_hir::item::{Item, StructMember};
use tune_hir::pattern::{Pattern, PatternKind};
use tune_hir::{HirId, MemberId};

use crate::locals::{LocalBinding, LocalId, LocalKind, NameRef, NameTarget};
use crate::prelude::VariantId;

use super::ResolvedModule;

pub(super) fn resolve_item_body(resolved: &mut ResolvedModule, item: &Item) {
    if item.tags.iter().any(|tag| !tag.args.is_empty()) {
        let mut resolver = BodyResolver {
            resolved,
            owner: item.id,
            scopes: vec![HashMap::new()],
        };
        for tag in &item.tags {
            for arg in &tag.args {
                resolver.resolve_expr_names(&arg.value);
            }
        }
    }

    if let Some(body) = &item.body {
        let mut resolver = BodyResolver {
            resolved,
            owner: item.id,
            scopes: vec![HashMap::new()],
        };

        for param in &item.params {
            if let Some(name) = &param.name {
                resolver.bind_param(name, param.id);
            }
        }

        resolver.resolve_expr_names(body);
    }

    for member in &item.struct_members {
        resolve_struct_member_body(resolved, item, member);
    }
}

fn resolve_struct_member_body(resolved: &mut ResolvedModule, item: &Item, member: &StructMember) {
    match member {
        StructMember::Callable(callable) => {
            let Some(body) = &callable.body else {
                return;
            };
            let mut resolver = BodyResolver {
                resolved,
                owner: item.id,
                scopes: vec![HashMap::new()],
            };
            for param in &callable.params {
                if let Some(name) = &param.name {
                    resolver.bind_param(name, param.id);
                }
            }
            resolver.resolve_expr_names(body);
        }
        StructMember::SequenceMaterializer(materializer) => {
            let Some(body) = &materializer.body else {
                return;
            };
            let mut resolver = BodyResolver {
                resolved,
                owner: item.id,
                scopes: vec![HashMap::new()],
            };
            if let Some(name) = &materializer.param_name {
                resolver.bind_local(name, LocalKind::Pattern, None, materializer.span);
            }
            resolver.resolve_expr_names(body);
        }
        StructMember::IndexAccess(access) => {
            let Some(body) = &access.body else {
                return;
            };
            let mut resolver = BodyResolver {
                resolved,
                owner: item.id,
                scopes: vec![HashMap::new()],
            };
            if let Some(name) = &access.index_param_name {
                resolver.bind_local(name, LocalKind::Pattern, None, access.span);
            }
            resolver.resolve_expr_names(body);
        }
        StructMember::Field(_) => {}
    }
}

struct BodyResolver<'resolved> {
    resolved: &'resolved mut ResolvedModule,
    owner: HirId,
    scopes: Vec<HashMap<String, NameTarget>>,
}

impl BodyResolver<'_> {
    fn resolve_expr_names(&mut self, expr: &Expr) {
        match &expr.kind {
            ExprKind::Missing | ExprKind::Literal(_) => {}
            ExprKind::Sequence(elements) => {
                for element in elements {
                    self.resolve_expr_names(element);
                }
            }
            ExprKind::Name(name) => self.resolve_name_ref(name, expr),
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
                self.resolve_expr_names(callee);
                for arg in args {
                    self.resolve_expr_names(arg);
                }
            }
            ExprKind::Field { base, .. } => self.resolve_expr_names(base),
            ExprKind::Index { base, index } => {
                self.resolve_expr_names(base);
                self.resolve_expr_names(index);
            }
            ExprKind::Let { name, value, .. } => {
                if let Some(value) = value {
                    self.resolve_expr_names(value);
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
                for arm in arms {
                    self.with_scope(|this| {
                        this.bind_pattern_names(&arm.pattern);
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

    fn resolve_name_ref(&mut self, name: &str, expr: &Expr) {
        let target = if name == "self" {
            Some(NameTarget::SelfValue)
        } else {
            self.lookup_local(name).or_else(|| {
                self.resolved
                    .scope
                    .get(name)
                    .map(|binding| NameTarget::TopLevel(binding.id))
                    .or_else(|| self.variant_by_name(name).map(NameTarget::Variant))
            })
        };

        if let Some(target) = target {
            self.resolved.name_refs.push(NameRef {
                expr: expr.id,
                target,
                span: expr.span,
            });
            return;
        }

        self.resolved.diagnostics.push(
            Diagnostic::error(
                codes::UNRESOLVED_NAME,
                format!("unresolved name `{name}`"),
                expr.span.unwrap_or_else(Span::synthetic),
                "this name is not in scope",
            )
            .build(),
        );
    }

    fn bind_param(&mut self, name: &str, id: MemberId) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.to_owned(), NameTarget::Param(id));
        }
    }

    fn bind_local(
        &mut self,
        name: &str,
        kind: LocalKind,
        expr: Option<tune_hir::ExprId>,
        span: Option<Span>,
    ) -> Option<LocalId> {
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

    fn bind_pattern_names(&mut self, pattern: &Pattern) {
        match &pattern.kind {
            PatternKind::Binding(name) => {
                self.bind_local(name, LocalKind::Pattern, None, pattern.span);
            }
            PatternKind::Tuple(patterns) => {
                for pattern in patterns {
                    self.bind_pattern_names(pattern);
                }
            }
            PatternKind::Variant { name, args } => {
                self.resolve_variant_pattern(name, pattern.span);
                for pattern in args {
                    self.bind_pattern_names(pattern);
                }
            }
            PatternKind::Hole
            | PatternKind::Unit
            | PatternKind::StructuralShape
            | PatternKind::Else => {}
        }
    }

    fn resolve_variant_pattern(&mut self, name: &str, span: Option<Span>) {
        if let Some(variant) = self.variant_by_name(name) {
            self.resolved
                .variant_pattern_refs
                .push(crate::VariantPatternRef { variant, span });
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

    fn variant_by_name(&self, name: &str) -> Option<VariantId> {
        self.resolved
            .variants
            .get(name)
            .or_else(|| self.resolved.prelude.variant(name).map(VariantId::Prelude))
    }

    fn lookup_local(&self, name: &str) -> Option<NameTarget> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn with_scope(&mut self, f: impl FnOnce(&mut Self)) {
        self.scopes.push(HashMap::new());
        f(self);
        self.scopes.pop();
    }
}

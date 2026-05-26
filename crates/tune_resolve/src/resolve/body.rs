use std::collections::HashMap;

use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::{Expr, ExprKind};
use tune_hir::item::Item;
use tune_hir::pattern::{Pattern, PatternKind};
use tune_hir::{HirId, MemberId};

use crate::locals::{LocalBinding, LocalId, LocalKind, NameRef, NameTarget};

use super::ResolvedModule;

pub(super) fn resolve_item_body(resolved: &mut ResolvedModule, item: &Item) {
    let Some(body) = &item.body else {
        return;
    };

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
                            this.bind_local(name, LocalKind::CallableParam, param.span);
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
                    self.bind_local(name, LocalKind::Let, expr.span);
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
            ExprKind::Return(inner) => {
                if let Some(inner) = inner {
                    self.resolve_expr_names(inner);
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

    fn bind_local(&mut self, name: &str, kind: LocalKind, span: Option<Span>) -> Option<LocalId> {
        let id = LocalId(u32::try_from(self.resolved.locals.len()).ok()?);
        self.resolved.locals.push(LocalBinding {
            id,
            owner: self.owner,
            kind,
            name: name.to_owned(),
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
                self.bind_local(name, LocalKind::Pattern, pattern.span);
            }
            PatternKind::Tuple(patterns) => {
                for pattern in patterns {
                    self.bind_pattern_names(pattern);
                }
            }
            PatternKind::Variant { args, .. } => {
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

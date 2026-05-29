use tune_hir::expr::{Expr, ExprKind, LiteralKind, StringPart};
use tune_resolve::{LocalId, LocalKind, NameTarget};

use super::LowerContext;
use crate::{Capture, CaptureMode, CaptureSource};

impl LowerContext<'_> {
    pub(super) fn captured_locals_in_callable_values(&self, body: &Expr) -> Vec<LocalId> {
        let mut captures = Vec::new();
        collect_callable_value_captures(body, self, &mut captures);
        captures
    }

    pub(super) fn callable_value_captures(&self, body: &Expr) -> Vec<Capture> {
        let mut declared = Vec::new();
        collect_declared_locals(body, self, &mut declared);

        let mut captures = Vec::new();
        collect_captured_locals(body, body, self, &declared, &mut captures);
        captures
    }
}

fn collect_callable_value_captures(
    expr: &Expr,
    context: &LowerContext<'_>,
    captures: &mut Vec<LocalId>,
) {
    if let ExprKind::CallableValue { body, .. } = &expr.kind {
        for capture in context.callable_value_captures(body) {
            let CaptureSource::Local(capture) = capture.source else {
                continue;
            };
            if !captures.contains(&capture) {
                captures.push(capture);
            }
        }
    }

    walk_expr(expr, &mut |child| {
        collect_callable_value_captures(child, context, captures);
    });
}

fn collect_captured_locals(
    body: &Expr,
    expr: &Expr,
    context: &LowerContext<'_>,
    declared: &[LocalId],
    captures: &mut Vec<Capture>,
) {
    if let ExprKind::Name(_) = expr.kind {
        match context.name_target(expr.id) {
            Some(NameTarget::Local(local))
                if !declared.contains(&local)
                    && context.local_kind(local) == Some(LocalKind::Let) =>
            {
                let capture = capture_for(body, context, CaptureSource::Local(local));
                if !captures
                    .iter()
                    .any(|candidate| candidate.source == capture.source)
                {
                    captures.push(capture);
                }
            }
            Some(NameTarget::Param(param)) => {
                let capture = capture_for(body, context, CaptureSource::Param(param));
                if !captures
                    .iter()
                    .any(|candidate| candidate.source == capture.source)
                {
                    captures.push(capture);
                }
            }
            Some(NameTarget::TopLevel(item)) if context.top_level_is_value_binding(item) => {
                let capture = capture_for(body, context, CaptureSource::TopLevel(item));
                if !captures
                    .iter()
                    .any(|candidate| candidate.source == capture.source)
                {
                    captures.push(capture);
                }
            }
            _ => {}
        }
    }

    walk_expr(expr, &mut |child| {
        collect_captured_locals(body, child, context, declared, captures);
    });
}

fn capture_for(body: &Expr, context: &LowerContext<'_>, source: CaptureSource) -> Capture {
    let mode = if capture_is_mutated(body, context, source) {
        CaptureMode::PrivateSnapshot
    } else {
        CaptureMode::Reference
    };
    Capture { source, mode }
}

fn capture_is_mutated(body: &Expr, context: &LowerContext<'_>, source: CaptureSource) -> bool {
    let mut mutated = false;
    visit_until(body, &mut |expr| {
        if mutated {
            return;
        }
        match &expr.kind {
            ExprKind::Assign { target, .. } => {
                mutated =
                    target_capture_source(target, context).is_some_and(|target| target == source);
            }
            ExprKind::Call { callee, args } => {
                mutated =
                    member_receiver_source(callee, context).is_some_and(|target| target == source);
                mutated |= args.iter().any(|arg| {
                    call_arg_capture_source(arg, context).is_some_and(|target| target == source)
                });
            }
            _ => {}
        }
    });
    mutated
}

fn target_capture_source(expr: &Expr, context: &LowerContext<'_>) -> Option<CaptureSource> {
    match &expr.kind {
        ExprKind::Name(_) => expr_capture_source(expr, context),
        ExprKind::Field { base, .. } | ExprKind::Index { base, .. } => {
            target_capture_source(base, context)
        }
        _ => None,
    }
}

fn member_receiver_source(expr: &Expr, context: &LowerContext<'_>) -> Option<CaptureSource> {
    let ExprKind::Field { base, .. } = &expr.kind else {
        return None;
    };
    expr_capture_source(base, context)
}

fn call_arg_capture_source(expr: &Expr, context: &LowerContext<'_>) -> Option<CaptureSource> {
    target_capture_source(expr, context)
}

fn expr_capture_source(expr: &Expr, context: &LowerContext<'_>) -> Option<CaptureSource> {
    match context.name_target(expr.id) {
        Some(NameTarget::Local(local)) if context.local_kind(local) == Some(LocalKind::Let) => {
            Some(CaptureSource::Local(local))
        }
        Some(NameTarget::Param(param)) => Some(CaptureSource::Param(param)),
        Some(NameTarget::TopLevel(item)) if context.top_level_is_value_binding(item) => {
            Some(CaptureSource::TopLevel(item))
        }
        _ => None,
    }
}

fn collect_declared_locals(expr: &Expr, context: &LowerContext<'_>, declared: &mut Vec<LocalId>) {
    if let ExprKind::Let { .. } = expr.kind
        && let Some(local) = context.local_for_expr(expr.id)
        && !declared.contains(&local)
    {
        declared.push(local);
    }

    walk_expr(expr, &mut |child| {
        collect_declared_locals(child, context, declared);
    });
}

fn walk_expr(expr: &Expr, visit: &mut impl FnMut(&Expr)) {
    match &expr.kind {
        ExprKind::Tuple(items)
        | ExprKind::Sequence(items)
        | ExprKind::Block(items)
        | ExprKind::Panic(items) => {
            for item in items {
                visit(item);
            }
        }
        ExprKind::Struct { fields, .. } => {
            for field in fields {
                visit(&field.value);
            }
        }
        ExprKind::CallableValue { body, .. }
        | ExprKind::Spawn(body)
        | ExprKind::Propagate(body)
        | ExprKind::Loop(body) => visit(body),
        ExprKind::Call { callee, args } => {
            visit(callee);
            for arg in args {
                visit(arg);
            }
        }
        ExprKind::Field { base, .. } => visit(base),
        ExprKind::Index { base, index }
        | ExprKind::Binary {
            lhs: base,
            rhs: index,
            ..
        } => {
            visit(base);
            visit(index);
        }
        ExprKind::Let { value, .. } => {
            if let Some(value) = value {
                visit(value);
            }
        }
        ExprKind::Assign { target, value } => {
            visit(target);
            visit(value);
        }
        ExprKind::Unary { expr, .. } => visit(expr),
        ExprKind::If {
            branches,
            else_branch,
        } => {
            for branch in branches {
                visit(&branch.condition);
                visit(&branch.body);
            }
            if let Some(else_branch) = else_branch {
                visit(else_branch);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            visit(scrutinee);
            for arm in arms {
                visit(&arm.body);
            }
        }
        ExprKind::While { condition, body } => {
            visit(condition);
            visit(body);
        }
        ExprKind::Return(inner) => {
            if let Some(inner) = inner {
                visit(inner);
            }
        }
        ExprKind::For { iterable, body, .. } => {
            visit(iterable);
            visit(body);
        }
        ExprKind::Literal(LiteralKind::String(literal)) => {
            for part in &literal.parts {
                if let StringPart::Interpolation(expr) = part {
                    visit(expr);
                }
            }
        }
        ExprKind::Missing
        | ExprKind::Literal(_)
        | ExprKind::Name(_)
        | ExprKind::Break
        | ExprKind::Continue => {}
    }
}

fn visit_until(expr: &Expr, visit: &mut impl FnMut(&Expr)) {
    visit(expr);
    walk_expr(expr, &mut |child| visit_until(child, visit));
}

use tune_hir::expr::{Expr, ExprKind, LiteralKind, StringPart};
use tune_hir::item::{Item, StructMember};
use tune_hir::pattern::{Pattern, PatternKind, StructuralRequirement};

pub(super) fn item_exprs(item: &Item) -> Vec<&Expr> {
    let mut exprs = Vec::new();
    for tag in &item.tags {
        for arg in &tag.args {
            collect_exprs(&arg.value, &mut exprs);
        }
    }
    for field in &item.fields {
        if let Some(default) = &field.default {
            collect_exprs(default, &mut exprs);
        }
    }
    for member in &item.struct_members {
        match member {
            StructMember::Field(field) => {
                if let Some(default) = &field.default {
                    collect_exprs(default, &mut exprs);
                }
            }
            StructMember::Callable(callable) => {
                if let Some(body) = &callable.body {
                    collect_exprs(body, &mut exprs);
                }
            }
            StructMember::SequenceMaterializer(member) => {
                if let Some(body) = &member.body {
                    collect_exprs(body, &mut exprs);
                }
            }
            StructMember::IndexAccess(member) => {
                if let Some(body) = &member.body {
                    collect_exprs(body, &mut exprs);
                }
            }
        }
    }
    if let Some(body) = &item.body {
        collect_exprs(body, &mut exprs);
    }
    exprs
}

fn collect_exprs<'a>(expr: &'a Expr, out: &mut Vec<&'a Expr>) {
    out.push(expr);
    match &expr.kind {
        ExprKind::Tuple(items) | ExprKind::Sequence(items) | ExprKind::Panic(items) => {
            for item in items {
                collect_exprs(item, out);
            }
        }
        ExprKind::Struct { fields, .. } => {
            for field in fields {
                collect_exprs(&field.value, out);
            }
        }
        ExprKind::CallableValue { body, .. }
        | ExprKind::Spawn(body)
        | ExprKind::Propagate(body)
        | ExprKind::Loop(body) => collect_exprs(body, out),
        ExprKind::Call { callee, args } => {
            collect_exprs(callee, out);
            for arg in args {
                collect_exprs(arg, out);
            }
        }
        ExprKind::Field { base, .. } => collect_exprs(base, out),
        ExprKind::Index { base, index } => {
            collect_exprs(base, out);
            collect_exprs(index, out);
        }
        ExprKind::Let { value, .. } => {
            if let Some(value) = value {
                collect_exprs(value, out);
            }
        }
        ExprKind::Assign { target, value }
        | ExprKind::Binary {
            lhs: target,
            rhs: value,
            ..
        } => {
            collect_exprs(target, out);
            collect_exprs(value, out);
        }
        ExprKind::Unary { expr, .. } => collect_exprs(expr, out),
        ExprKind::If {
            branches,
            else_branch,
        } => {
            for branch in branches {
                collect_exprs(&branch.condition, out);
                collect_exprs(&branch.body, out);
            }
            if let Some(else_branch) = else_branch {
                collect_exprs(else_branch, out);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_exprs(scrutinee, out);
            for arm in arms {
                collect_pattern(&arm.pattern, out);
                collect_exprs(&arm.body, out);
            }
        }
        ExprKind::While { condition, body } => {
            collect_exprs(condition, out);
            collect_exprs(body, out);
        }
        ExprKind::Return(value) => {
            if let Some(value) = value {
                collect_exprs(value, out);
            }
        }
        ExprKind::For {
            pattern,
            iterable,
            body,
        } => {
            collect_pattern(pattern, out);
            collect_exprs(iterable, out);
            collect_exprs(body, out);
        }
        ExprKind::Block(exprs) => {
            for expr in exprs {
                collect_exprs(expr, out);
            }
        }
        ExprKind::Literal(LiteralKind::String(string)) => {
            for part in &string.parts {
                if let StringPart::Interpolation(expr) = part {
                    collect_exprs(expr, out);
                }
            }
        }
        ExprKind::Literal(_)
        | ExprKind::Missing
        | ExprKind::Name(_)
        | ExprKind::Break
        | ExprKind::Continue => {}
    }
}

fn collect_pattern<'a>(pattern: &'a Pattern, out: &mut Vec<&'a Expr>) {
    match &pattern.kind {
        PatternKind::Tuple(items) | PatternKind::Variant { args: items, .. } => {
            for item in items {
                collect_pattern(item, out);
            }
        }
        PatternKind::StructuralShape(requirements) => {
            for requirement in requirements {
                collect_structural_requirement(requirement, out);
            }
        }
        PatternKind::Hole
        | PatternKind::Binding(_)
        | PatternKind::None
        | PatternKind::Unit
        | PatternKind::Else => {}
    }
}

fn collect_structural_requirement<'a>(
    requirement: &'a StructuralRequirement,
    _out: &mut Vec<&'a Expr>,
) {
    let _ = requirement;
}

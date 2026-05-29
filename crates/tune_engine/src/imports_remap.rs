use tune_hir::expr::{
    Expr, ExprKind, IfBranch, LiteralKind, MatchArm, StringPart, StructFieldInit,
};
use tune_hir::item::{Item, StructMember, TagArg};
use tune_hir::module::Module;
use tune_hir::pattern::{Pattern, PatternKind, StructuralRequirement};
use tune_hir::{ExprId, HirId, MemberId};

pub(crate) fn remap_item(item: &mut Item, old: HirId, new: HirId, expr_offset: u64) {
    item.id = new;
    for tag in &mut item.tags {
        for arg in &mut tag.args {
            remap_tag_arg(arg, expr_offset);
        }
    }
    for type_param in &mut item.type_params {
        remap_member_id(&mut type_param.id, old, new);
    }
    for param in &mut item.params {
        remap_member_id(&mut param.id, old, new);
    }
    for field in &mut item.fields {
        remap_member_id(&mut field.id, old, new);
        if let Some(default) = &mut field.default {
            remap_expr(default, expr_offset);
        }
    }
    for member in &mut item.struct_members {
        remap_struct_member(member, old, new, expr_offset);
    }
    for variant in &mut item.variants {
        remap_member_id(&mut variant.id, old, new);
    }
    if let Some(body) = &mut item.body {
        remap_expr(body, expr_offset);
    }
}

pub(crate) fn next_expr_id(module: &Module) -> u64 {
    module
        .items
        .iter()
        .flat_map(item_expr_ids)
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

fn remap_struct_member(member: &mut StructMember, old: HirId, new: HirId, expr_offset: u64) {
    match member {
        StructMember::Field(field) => {
            remap_member_id(&mut field.id, old, new);
            if let Some(default) = &mut field.default {
                remap_expr(default, expr_offset);
            }
        }
        StructMember::Callable(callable) => {
            remap_member_id(&mut callable.id, old, new);
            for param in &mut callable.params {
                remap_member_id(&mut param.id, old, new);
            }
            if let Some(body) = &mut callable.body {
                remap_expr(body, expr_offset);
            }
        }
        StructMember::SequenceMaterializer(materializer) => {
            remap_member_id(&mut materializer.id, old, new);
            if let Some(body) = &mut materializer.body {
                remap_expr(body, expr_offset);
            }
        }
        StructMember::IndexAccess(access) => {
            remap_member_id(&mut access.id, old, new);
            remap_member_id(&mut access.index_param_id, old, new);
            if let Some(body) = &mut access.body {
                remap_expr(body, expr_offset);
            }
        }
    }
}

fn remap_member_id(id: &mut MemberId, old: HirId, new: HirId) {
    if id.owner == old {
        id.owner = new;
    }
}

fn remap_tag_arg(arg: &mut TagArg, expr_offset: u64) {
    remap_expr(&mut arg.value, expr_offset);
}

fn remap_expr(expr: &mut Expr, offset: u64) {
    expr.id = ExprId(expr.id.0.saturating_add(offset));
    match &mut expr.kind {
        ExprKind::Literal(LiteralKind::String(value)) => {
            for part in &mut value.parts {
                if let StringPart::Interpolation(expr) = part {
                    remap_expr(expr, offset);
                }
            }
        }
        ExprKind::Tuple(items)
        | ExprKind::Sequence(items)
        | ExprKind::Panic(items)
        | ExprKind::Block(items) => {
            for item in items {
                remap_expr(item, offset);
            }
        }
        ExprKind::Struct { fields, .. } => {
            for StructFieldInit { value, .. } in fields {
                remap_expr(value, offset);
            }
        }
        ExprKind::CallableValue { body, .. }
        | ExprKind::Spawn(body)
        | ExprKind::Propagate(body)
        | ExprKind::Loop(body) => remap_expr(body, offset),
        ExprKind::Call { callee, args } => {
            remap_expr(callee, offset);
            for arg in args {
                remap_expr(arg, offset);
            }
        }
        ExprKind::Field { base, .. } => remap_expr(base, offset),
        ExprKind::Index { base, index } => {
            remap_expr(base, offset);
            remap_expr(index, offset);
        }
        ExprKind::Let { value, .. } => {
            if let Some(value) = value {
                remap_expr(value, offset);
            }
        }
        ExprKind::Assign { target, value }
        | ExprKind::Binary {
            lhs: target,
            rhs: value,
            ..
        } => {
            remap_expr(target, offset);
            remap_expr(value, offset);
        }
        ExprKind::Unary { expr, .. } => remap_expr(expr, offset),
        ExprKind::If {
            branches,
            else_branch,
        } => {
            for IfBranch { condition, body } in branches {
                remap_expr(condition, offset);
                remap_expr(body, offset);
            }
            if let Some(else_branch) = else_branch {
                remap_expr(else_branch, offset);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            remap_expr(scrutinee, offset);
            for MatchArm { pattern, body } in arms {
                remap_pattern(pattern, offset);
                remap_expr(body, offset);
            }
        }
        ExprKind::While { condition, body } => {
            remap_expr(condition, offset);
            remap_expr(body, offset);
        }
        ExprKind::Return(value) => {
            if let Some(value) = value {
                remap_expr(value, offset);
            }
        }
        ExprKind::For {
            pattern,
            iterable,
            body,
        } => {
            remap_pattern(pattern, offset);
            remap_expr(iterable, offset);
            remap_expr(body, offset);
        }
        ExprKind::Missing
        | ExprKind::Literal(_)
        | ExprKind::Name(_)
        | ExprKind::Break
        | ExprKind::Continue => {}
    }
}

fn remap_pattern(pattern: &mut Pattern, offset: u64) {
    pattern.id = ExprId(pattern.id.0.saturating_add(offset));
    match &mut pattern.kind {
        PatternKind::Tuple(items) => {
            for item in items {
                remap_pattern(item, offset);
            }
        }
        PatternKind::Variant { args, .. } => {
            for arg in args {
                remap_pattern(arg, offset);
            }
        }
        PatternKind::StructuralShape(requirements) => {
            for requirement in requirements {
                remap_requirement(requirement, offset);
            }
        }
        PatternKind::Hole
        | PatternKind::Binding(_)
        | PatternKind::None
        | PatternKind::Unit
        | PatternKind::Else => {}
    }
}

fn remap_requirement(requirement: &mut StructuralRequirement, offset: u64) {
    requirement.id = ExprId(requirement.id.0.saturating_add(offset));
}

fn item_expr_ids(item: &Item) -> Vec<u64> {
    let mut ids = Vec::new();
    for tag in &item.tags {
        for arg in &tag.args {
            collect_expr_ids(&arg.value, &mut ids);
        }
    }
    for field in &item.fields {
        if let Some(default) = &field.default {
            collect_expr_ids(default, &mut ids);
        }
    }
    for member in &item.struct_members {
        collect_member_expr_ids(member, &mut ids);
    }
    if let Some(body) = &item.body {
        collect_expr_ids(body, &mut ids);
    }
    ids
}

fn collect_member_expr_ids(member: &StructMember, ids: &mut Vec<u64>) {
    match member {
        StructMember::Field(field) => {
            if let Some(default) = &field.default {
                collect_expr_ids(default, ids);
            }
        }
        StructMember::Callable(callable) => {
            if let Some(body) = &callable.body {
                collect_expr_ids(body, ids);
            }
        }
        StructMember::SequenceMaterializer(materializer) => {
            if let Some(body) = &materializer.body {
                collect_expr_ids(body, ids);
            }
        }
        StructMember::IndexAccess(access) => {
            if let Some(body) = &access.body {
                collect_expr_ids(body, ids);
            }
        }
    }
}

fn collect_expr_ids(expr: &Expr, ids: &mut Vec<u64>) {
    ids.push(expr.id.0);
    match &expr.kind {
        ExprKind::Literal(LiteralKind::String(value)) => {
            for part in &value.parts {
                if let StringPart::Interpolation(expr) = part {
                    collect_expr_ids(expr, ids);
                }
            }
        }
        ExprKind::Tuple(items)
        | ExprKind::Sequence(items)
        | ExprKind::Panic(items)
        | ExprKind::Block(items) => {
            for item in items {
                collect_expr_ids(item, ids);
            }
        }
        ExprKind::Struct { fields, .. } => {
            for field in fields {
                collect_expr_ids(&field.value, ids);
            }
        }
        ExprKind::CallableValue { body, .. }
        | ExprKind::Spawn(body)
        | ExprKind::Propagate(body)
        | ExprKind::Loop(body) => collect_expr_ids(body, ids),
        ExprKind::Call { callee, args } => {
            collect_expr_ids(callee, ids);
            for arg in args {
                collect_expr_ids(arg, ids);
            }
        }
        ExprKind::Field { base, .. } => collect_expr_ids(base, ids),
        ExprKind::Index { base, index } => {
            collect_expr_ids(base, ids);
            collect_expr_ids(index, ids);
        }
        ExprKind::Let { value, .. } => {
            if let Some(value) = value {
                collect_expr_ids(value, ids);
            }
        }
        ExprKind::Assign { target, value }
        | ExprKind::Binary {
            lhs: target,
            rhs: value,
            ..
        } => {
            collect_expr_ids(target, ids);
            collect_expr_ids(value, ids);
        }
        ExprKind::Unary { expr, .. } => collect_expr_ids(expr, ids),
        ExprKind::If {
            branches,
            else_branch,
        } => {
            for branch in branches {
                collect_expr_ids(&branch.condition, ids);
                collect_expr_ids(&branch.body, ids);
            }
            if let Some(else_branch) = else_branch {
                collect_expr_ids(else_branch, ids);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            collect_expr_ids(scrutinee, ids);
            for arm in arms {
                collect_pattern_ids(&arm.pattern, ids);
                collect_expr_ids(&arm.body, ids);
            }
        }
        ExprKind::While { condition, body } => {
            collect_expr_ids(condition, ids);
            collect_expr_ids(body, ids);
        }
        ExprKind::Return(value) => {
            if let Some(value) = value {
                collect_expr_ids(value, ids);
            }
        }
        ExprKind::For {
            pattern,
            iterable,
            body,
        } => {
            collect_pattern_ids(pattern, ids);
            collect_expr_ids(iterable, ids);
            collect_expr_ids(body, ids);
        }
        ExprKind::Missing
        | ExprKind::Literal(_)
        | ExprKind::Name(_)
        | ExprKind::Break
        | ExprKind::Continue => {}
    }
}

fn collect_pattern_ids(pattern: &Pattern, ids: &mut Vec<u64>) {
    ids.push(pattern.id.0);
    match &pattern.kind {
        PatternKind::Tuple(items) => {
            for item in items {
                collect_pattern_ids(item, ids);
            }
        }
        PatternKind::Variant { args, .. } => {
            for arg in args {
                collect_pattern_ids(arg, ids);
            }
        }
        PatternKind::StructuralShape(requirements) => {
            for requirement in requirements {
                ids.push(requirement.id.0);
            }
        }
        PatternKind::Hole
        | PatternKind::Binding(_)
        | PatternKind::None
        | PatternKind::Unit
        | PatternKind::Else => {}
    }
}

use std::collections::HashMap;

use tune_hir::HirId;
use tune_hir::expr::{
    Expr, ExprKind, IfBranch, LiteralKind, MatchArm, StringPart, StructFieldInit,
};
use tune_hir::item::{Item, StructMember, TagArg};
use tune_hir::pattern::{Pattern, PatternKind, StructuralRequirement};
use tune_hir::shape::{ShapeExpr, ShapeExprKind, StructuralShapeRequirementKind};
use tune_resolve::{NameTarget, ResolvedModule};

#[derive(Debug, Default)]
pub(crate) struct ImportInternalNames {
    by_item: HashMap<HirId, String>,
    by_name: HashMap<String, String>,
}

impl ImportInternalNames {
    #[must_use]
    pub(crate) fn for_closure(
        imported: &tune_hir::module::Module,
        path: &str,
        selected: &[HirId],
        closure: &[HirId],
    ) -> Self {
        let mut names = Self::default();
        for item_id in closure {
            if selected.contains(item_id) {
                continue;
            }
            let Some(item) = imported.items.iter().find(|item| item.id == *item_id) else {
                continue;
            };
            let Some(name) = item.name.as_deref() else {
                continue;
            };
            let internal = internal_name(path, name, *item_id);
            names.by_item.insert(*item_id, internal.clone());
            names.by_name.insert(name.to_owned(), internal);
        }
        names
    }

    fn item_name(&self, item: HirId) -> Option<&str> {
        self.by_item.get(&item).map(String::as_str)
    }

    fn shape_name(&self, name: &mut String) {
        if let Some(internal) = self.by_name.get(name.as_str()) {
            *name = internal.clone();
        }
    }
}

pub(crate) fn internalized_import_item(
    item: &Item,
    resolved: &ResolvedModule,
    names: &ImportInternalNames,
) -> Item {
    let mut item = item.clone();
    if let Some(internal) = names.item_name(item.id) {
        item.name = Some(internal.to_owned());
        item.span = None;
    }

    let expr_names = resolved
        .name_refs
        .iter()
        .filter_map(|name_ref| {
            let NameTarget::TopLevel(target) = name_ref.target else {
                return None;
            };
            Some((name_ref.expr.0, names.item_name(target)?.to_owned()))
        })
        .collect::<HashMap<_, _>>();

    rewrite_item(&mut item, &expr_names, names);
    item
}

fn internal_name(path: &str, name: &str, item: HirId) -> String {
    let mut out = String::from("__import_");
    for ch in path.chars().chain(std::iter::once('_')).chain(name.chars()) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out.push('_');
    out.push_str(&item.0.to_string());
    out
}

fn rewrite_item(item: &mut Item, expr_names: &HashMap<u64, String>, names: &ImportInternalNames) {
    for tag in &mut item.tags {
        names.shape_name(&mut tag.name);
        for arg in &mut tag.args {
            rewrite_tag_arg(arg, expr_names, names);
        }
    }
    for type_param in &mut item.type_params {
        if let Some(shape) = &mut type_param.constraint {
            rewrite_shape(shape, names);
        }
    }
    for param in &mut item.params {
        if let Some(shape) = &mut param.shape {
            rewrite_shape(shape, names);
        }
    }
    for field in &mut item.fields {
        if let Some(shape) = &mut field.shape {
            rewrite_shape(shape, names);
        }
        if let Some(default) = &mut field.default {
            rewrite_expr(default, expr_names, names);
        }
    }
    for member in &mut item.struct_members {
        rewrite_struct_member(member, expr_names, names);
    }
    for variant in &mut item.variants {
        for payload in &mut variant.payload {
            rewrite_shape(payload, names);
        }
    }
    if let Some(shape) = &mut item.shape {
        rewrite_shape(shape, names);
    }
    if let Some(body) = &mut item.body {
        rewrite_expr(body, expr_names, names);
    }
}

fn rewrite_struct_member(
    member: &mut StructMember,
    expr_names: &HashMap<u64, String>,
    names: &ImportInternalNames,
) {
    match member {
        StructMember::Field(field) => {
            if let Some(shape) = &mut field.shape {
                rewrite_shape(shape, names);
            }
            if let Some(default) = &mut field.default {
                rewrite_expr(default, expr_names, names);
            }
        }
        StructMember::Callable(callable) => {
            for param in &mut callable.params {
                if let Some(shape) = &mut param.shape {
                    rewrite_shape(shape, names);
                }
            }
            if let Some(shape) = &mut callable.shape {
                rewrite_shape(shape, names);
            }
            if let Some(body) = &mut callable.body {
                rewrite_expr(body, expr_names, names);
            }
        }
        StructMember::SequenceMaterializer(materializer) => {
            if let Some(body) = &mut materializer.body {
                rewrite_expr(body, expr_names, names);
            }
        }
        StructMember::IndexAccess(access) => {
            if let Some(shape) = &mut access.index_shape {
                rewrite_shape(shape, names);
            }
            if let Some(shape) = &mut access.result_shape {
                rewrite_shape(shape, names);
            }
            if let Some(body) = &mut access.body {
                rewrite_expr(body, expr_names, names);
            }
        }
    }
}

fn rewrite_tag_arg(
    arg: &mut TagArg,
    expr_names: &HashMap<u64, String>,
    names: &ImportInternalNames,
) {
    rewrite_expr(&mut arg.value, expr_names, names);
}

fn rewrite_expr(expr: &mut Expr, expr_names: &HashMap<u64, String>, names: &ImportInternalNames) {
    match &mut expr.kind {
        ExprKind::Name(name) => {
            if let Some(internal) = expr_names.get(&expr.id.0) {
                *name = internal.clone();
            }
        }
        ExprKind::Literal(LiteralKind::String(value)) => {
            for part in &mut value.parts {
                if let StringPart::Interpolation(expr) = part {
                    rewrite_expr(expr, expr_names, names);
                }
            }
        }
        ExprKind::Tuple(items)
        | ExprKind::Sequence(items)
        | ExprKind::Panic(items)
        | ExprKind::Block(items) => {
            for item in items {
                rewrite_expr(item, expr_names, names);
            }
        }
        ExprKind::Struct { name, fields } => {
            names.shape_name(name);
            for StructFieldInit { value, .. } in fields {
                rewrite_expr(value, expr_names, names);
            }
        }
        ExprKind::CallableValue { body, .. }
        | ExprKind::Spawn(body)
        | ExprKind::Propagate(body)
        | ExprKind::Loop(body) => rewrite_expr(body, expr_names, names),
        ExprKind::Call { callee, args } => {
            rewrite_expr(callee, expr_names, names);
            for arg in args {
                rewrite_expr(arg, expr_names, names);
            }
        }
        ExprKind::Field { base, .. } => rewrite_expr(base, expr_names, names),
        ExprKind::Index { base, index } => {
            rewrite_expr(base, expr_names, names);
            rewrite_expr(index, expr_names, names);
        }
        ExprKind::Let { shape, value, .. } => {
            if let Some(shape) = shape {
                rewrite_shape(shape, names);
            }
            if let Some(value) = value {
                rewrite_expr(value, expr_names, names);
            }
        }
        ExprKind::Assign { target, value }
        | ExprKind::Binary {
            lhs: target,
            rhs: value,
            ..
        } => {
            rewrite_expr(target, expr_names, names);
            rewrite_expr(value, expr_names, names);
        }
        ExprKind::Unary { expr, .. } => rewrite_expr(expr, expr_names, names),
        ExprKind::If {
            branches,
            else_branch,
        } => {
            for IfBranch { condition, body } in branches {
                rewrite_expr(condition, expr_names, names);
                rewrite_expr(body, expr_names, names);
            }
            if let Some(else_branch) = else_branch {
                rewrite_expr(else_branch, expr_names, names);
            }
        }
        ExprKind::Match { scrutinee, arms } => {
            rewrite_expr(scrutinee, expr_names, names);
            for MatchArm { pattern, body } in arms {
                rewrite_pattern(pattern, names);
                rewrite_expr(body, expr_names, names);
            }
        }
        ExprKind::While { condition, body } => {
            rewrite_expr(condition, expr_names, names);
            rewrite_expr(body, expr_names, names);
        }
        ExprKind::Return(value) => {
            if let Some(value) = value {
                rewrite_expr(value, expr_names, names);
            }
        }
        ExprKind::For {
            pattern,
            iterable,
            body,
        } => {
            rewrite_pattern(pattern, names);
            rewrite_expr(iterable, expr_names, names);
            rewrite_expr(body, expr_names, names);
        }
        ExprKind::Missing | ExprKind::Literal(_) | ExprKind::Break | ExprKind::Continue => {}
    }
}

fn rewrite_pattern(pattern: &mut Pattern, names: &ImportInternalNames) {
    match &mut pattern.kind {
        PatternKind::Tuple(items) => {
            for item in items {
                rewrite_pattern(item, names);
            }
        }
        PatternKind::Variant { name, args } => {
            names.shape_name(name);
            for arg in args {
                rewrite_pattern(arg, names);
            }
        }
        PatternKind::StructuralShape(requirements) => {
            for requirement in requirements {
                rewrite_requirement(requirement, names);
            }
        }
        PatternKind::Hole
        | PatternKind::Binding(_)
        | PatternKind::None
        | PatternKind::Unit
        | PatternKind::Else => {}
    }
}

fn rewrite_requirement(requirement: &mut StructuralRequirement, names: &ImportInternalNames) {
    match &mut requirement.kind {
        tune_hir::pattern::StructuralRequirementKind::Field { shape, .. } => {
            if let Some(shape) = shape {
                rewrite_shape(shape, names);
            }
        }
        tune_hir::pattern::StructuralRequirementKind::Callable { params, ret, .. } => {
            for param in params {
                rewrite_shape(param, names);
            }
            if let Some(ret) = ret {
                rewrite_shape(ret, names);
            }
        }
    }
}

fn rewrite_shape(shape: &mut ShapeExpr, names: &ImportInternalNames) {
    match &mut shape.kind {
        ShapeExprKind::Named(name) => names.shape_name(name),
        ShapeExprKind::Generic { name, args } => {
            names.shape_name(name);
            for arg in args {
                rewrite_shape(arg, names);
            }
        }
        ShapeExprKind::Sequence(inner) | ShapeExprKind::Optional(inner) => {
            rewrite_shape(inner, names);
        }
        ShapeExprKind::Union(items) | ShapeExprKind::Tuple(items) => {
            for item in items {
                rewrite_shape(item, names);
            }
        }
        ShapeExprKind::Structural(requirements) => {
            for requirement in requirements {
                rewrite_structural_shape_requirement(requirement, names);
            }
        }
        ShapeExprKind::Callable { params, ret } => {
            for param in params {
                rewrite_shape(param, names);
            }
            rewrite_shape(ret, names);
        }
        ShapeExprKind::Missing => {}
    }
}

fn rewrite_structural_shape_requirement(
    requirement: &mut tune_hir::shape::StructuralShapeRequirement,
    names: &ImportInternalNames,
) {
    match &mut requirement.kind {
        StructuralShapeRequirementKind::Field { shape } => {
            if let Some(shape) = shape {
                rewrite_shape(shape, names);
            }
        }
        StructuralShapeRequirementKind::Callable { params, ret } => {
            for param in params {
                rewrite_shape(param, names);
            }
            if let Some(ret) = ret {
                rewrite_shape(ret, names);
            }
        }
    }
}

use tune_hir::expr::{Expr, ExprKind, ExprParam, LiteralKind, StringPart};
use tune_hir::item::{Item, StructMember};
use tune_hir::module::Module;
use tune_resolve::ResolvedModule;
use tune_shape::Shape;

use super::LowerContext;
use super::module::captured_locals_for_body;
use crate::plan::{PlanFunction, PlanOp};

pub(super) fn lower_callable_value_functions<'a>(
    module: &'a Module,
    resolved: &'a ResolvedModule,
    analyses: &'a [tune_shape::ShapeAnalysis],
    param_shapes: &'a [(tune_hir::MemberId, Shape)],
) -> impl Iterator<Item = PlanFunction> + 'a {
    module
        .items
        .iter()
        .zip(analyses)
        .flat_map(move |(item, analysis)| {
            item.body
                .iter()
                .chain(item.struct_members.iter().filter_map(struct_member_body))
                .flat_map(move |body| {
                    let context = CallableFunctionContext {
                        module,
                        resolved,
                        owner: item,
                        analysis,
                        param_shapes,
                    };
                    let mut functions = Vec::new();
                    collect_callable_value_functions(&context, body, &mut functions);
                    functions
                })
        })
}

struct CallableFunctionContext<'a> {
    module: &'a Module,
    resolved: &'a ResolvedModule,
    owner: &'a Item,
    analysis: &'a tune_shape::ShapeAnalysis,
    param_shapes: &'a [(tune_hir::MemberId, Shape)],
}

fn struct_member_body(member: &StructMember) -> Option<&Expr> {
    match member {
        StructMember::Callable(callable) => callable.body.as_ref(),
        StructMember::SequenceMaterializer(materializer) => materializer.body.as_ref(),
        StructMember::IndexAccess(access) => access.body.as_ref(),
        StructMember::Field(_) => None,
    }
}

fn collect_callable_value_functions(
    context: &CallableFunctionContext<'_>,
    expr: &Expr,
    functions: &mut Vec<PlanFunction>,
) {
    if let ExprKind::CallableValue { params, body } = &expr.kind {
        functions.push(lower_callable_value_function(context, expr, params, body));
        collect_callable_value_functions(context, body, functions);
        return;
    }

    walk_expr(expr, &mut |child| {
        collect_callable_value_functions(context, child, functions);
    });
}

fn lower_callable_value_function(
    context: &CallableFunctionContext<'_>,
    expr: &Expr,
    params: &[ExprParam],
    body: &Expr,
) -> PlanFunction {
    let lower = LowerContext {
        resolved: Some(context.resolved),
        module: Some(context.module),
        analysis: Some(context.analysis),
        self_shape: None,
        struct_escape: crate::StructEscapeReason::Local,
        structural_witnesses: Vec::new(),
        param_shapes: context.param_shapes.to_vec(),
        captured_locals: captured_locals_for_body(context.resolved, body),
    };
    let mut plan = PlanFunction {
        owner: None,
        member: None,
        callable: Some(expr.id),
        name: format!("<callable:{}>", expr.id.0),
        type_params: Vec::new(),
        span: expr.span,
        params: Vec::new(),
        local_params: params
            .iter()
            .filter_map(|param| callable_param_local(context.resolved, context.owner, param))
            .collect(),
        captures: lower.callable_value_captures(body),
        module_bindings: Vec::new(),
        struct_layouts: super::struct_layouts(context.module),
        ops: Vec::new(),
    };
    lower.lower_return_expr(body, &mut plan.ops);
    if super::falls_through(body, Some(context.analysis)) {
        plan.ops.push(PlanOp::Return);
    }
    plan
}

fn callable_param_local(
    resolved: &ResolvedModule,
    owner: &Item,
    param: &ExprParam,
) -> Option<tune_resolve::LocalId> {
    let name = param.name.as_deref()?;
    resolved
        .locals
        .iter()
        .find(|local| {
            local.owner == owner.id
                && local.kind == tune_resolve::LocalKind::CallableParam
                && local.name == name
                && local.span == param.span
        })
        .map(|local| local.id)
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

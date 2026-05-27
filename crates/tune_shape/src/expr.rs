use tune_hir::expr::{Expr, ExprKind, LiteralKind};
use tune_hir::module::Module;
use tune_resolve::{NameTarget, PreludeVariant, ResolvedModule, VariantId};

use crate::{LiteralFact, Shape};

#[must_use]
pub fn expr_literal_fact(expr: &Expr) -> Option<LiteralFact> {
    match &expr.kind {
        ExprKind::Literal(literal) => literal_fact(literal),
        ExprKind::Sequence(elements) => elements
            .iter()
            .map(expr_literal_fact)
            .collect::<Option<Vec<_>>>()
            .map(|elements| LiteralFact::Sequence { elements }),
        _ => None,
    }
}

fn literal_fact(literal: &LiteralKind) -> Option<LiteralFact> {
    match literal {
        LiteralKind::Int(text) | LiteralKind::Float(text) => {
            Some(LiteralFact::Numeric { text: text.clone() })
        }
        LiteralKind::String(text) => Some(LiteralFact::String {
            segments: vec![string_literal_body(text).to_owned()],
        }),
        LiteralKind::Bool(value) => Some(LiteralFact::Bool(*value)),
        LiteralKind::None => Some(LiteralFact::None),
    }
}

#[must_use]
pub fn expr_shape_fact(expr: &Expr, module: &Module, resolved: &ResolvedModule) -> Option<Shape> {
    match &expr.kind {
        ExprKind::Name(_) => {
            let target = name_target(expr, resolved)?;
            variant_shape(target, None, module)
        }
        ExprKind::Call { callee, args } => {
            variant_constructor_shape(callee, args, module, resolved)
        }
        ExprKind::Propagate(inner) => result_ok_shape(expr_shape_fact(inner, module, resolved)?),
        _ => None,
    }
}

fn variant_constructor_shape(
    callee: &Expr,
    args: &[Expr],
    module: &Module,
    resolved: &ResolvedModule,
) -> Option<Shape> {
    let arg_shape = args
        .first()
        .map(|arg| value_shape_hint(arg, module, resolved));

    variant_shape(name_target(callee, resolved)?, arg_shape, module)
}

#[must_use]
pub fn expr_result_constructor_shape_fact(
    expr: &Expr,
    module: &Module,
    resolved: &ResolvedModule,
) -> Option<Shape> {
    let mut collector = ResultConstructorCollector::default();
    collector.collect_value(expr, module, resolved);
    collector.finish()
}

#[must_use]
pub fn expr_propagated_error_shape_fact(
    expr: &Expr,
    module: &Module,
    resolved: &ResolvedModule,
) -> Option<Shape> {
    let mut collector = PropagatedErrorCollector::default();
    collector.collect(expr, module, resolved);
    collector.finish()
}

#[derive(Debug, Default)]
struct ResultConstructorCollector {
    ok: Vec<Shape>,
    err: Vec<Shape>,
}

impl ResultConstructorCollector {
    fn collect_value(&mut self, expr: &Expr, module: &Module, resolved: &ResolvedModule) {
        if let Some(shape) = expr_shape_fact(expr, module, resolved) {
            self.add_shape(shape);
            return;
        }

        match &expr.kind {
            ExprKind::Block(exprs) => {
                for expr in exprs {
                    self.collect_return(expr, module, resolved);
                }
                if let Some(last) = exprs.last() {
                    self.collect_value(last, module, resolved);
                }
            }
            ExprKind::If {
                branches,
                else_branch,
            } => {
                for branch in branches {
                    self.collect_value(&branch.body, module, resolved);
                }
                if let Some(else_branch) = else_branch {
                    self.collect_value(else_branch, module, resolved);
                }
            }
            ExprKind::Match { arms, .. } => {
                for arm in arms {
                    self.collect_value(&arm.body, module, resolved);
                }
            }
            ExprKind::Return(inner) => {
                self.collect_return_expr(inner.as_deref(), module, resolved);
            }
            _ => {}
        }
    }

    fn collect_return(&mut self, expr: &Expr, module: &Module, resolved: &ResolvedModule) {
        match &expr.kind {
            ExprKind::Return(inner) => self.collect_return_expr(inner.as_deref(), module, resolved),
            ExprKind::Block(exprs) => {
                for expr in exprs {
                    self.collect_return(expr, module, resolved);
                }
            }
            ExprKind::If {
                branches,
                else_branch,
            } => {
                for branch in branches {
                    self.collect_return(&branch.body, module, resolved);
                }
                if let Some(else_branch) = else_branch {
                    self.collect_return(else_branch, module, resolved);
                }
            }
            ExprKind::Match { arms, .. } => {
                for arm in arms {
                    self.collect_return(&arm.body, module, resolved);
                }
            }
            ExprKind::CallableValue { .. } => {}
            _ => {}
        }
    }

    fn collect_return_expr(
        &mut self,
        expr: Option<&Expr>,
        module: &Module,
        resolved: &ResolvedModule,
    ) {
        if let Some(expr) = expr {
            self.collect_value(expr, module, resolved);
        }
    }

    fn add_shape(&mut self, shape: Shape) {
        if let Shape::Result { ok, err } = shape {
            if *ok != Shape::Hole {
                self.ok.push(*ok);
            }
            if *err != Shape::Hole {
                self.err.push(*err);
            }
        }
    }

    fn finish(self) -> Option<Shape> {
        (!self.ok.is_empty() || !self.err.is_empty()).then(|| Shape::Result {
            ok: Box::new(union_shapes(self.ok)),
            err: Box::new(union_shapes(self.err)),
        })
    }
}

#[derive(Debug, Default)]
struct PropagatedErrorCollector {
    errors: Vec<Shape>,
}

impl PropagatedErrorCollector {
    fn collect(&mut self, expr: &Expr, module: &Module, resolved: &ResolvedModule) {
        match &expr.kind {
            ExprKind::Propagate(inner) => {
                if let Some(Shape::Result { err, .. }) = expr_shape_fact(inner, module, resolved) {
                    self.errors.push(*err);
                }
                self.collect(inner, module, resolved);
            }
            ExprKind::CallableValue { .. } => {}
            ExprKind::Spawn(body) | ExprKind::Loop(body) => self.collect(body, module, resolved),
            ExprKind::Sequence(elements) | ExprKind::Block(elements) => {
                for element in elements {
                    self.collect(element, module, resolved);
                }
            }
            ExprKind::Call { callee, args } => {
                self.collect(callee, module, resolved);
                for arg in args {
                    self.collect(arg, module, resolved);
                }
            }
            ExprKind::Field { base, .. } => self.collect(base, module, resolved),
            ExprKind::Index { base, index } => {
                self.collect(base, module, resolved);
                self.collect(index, module, resolved);
            }
            ExprKind::Let { value, .. } => {
                if let Some(value) = value {
                    self.collect(value, module, resolved);
                }
            }
            ExprKind::Assign { target, value } => {
                self.collect(target, module, resolved);
                self.collect(value, module, resolved);
            }
            ExprKind::Unary { expr, .. } => self.collect(expr, module, resolved),
            ExprKind::Binary { lhs, rhs, .. } => {
                self.collect(lhs, module, resolved);
                self.collect(rhs, module, resolved);
            }
            ExprKind::If {
                branches,
                else_branch,
            } => {
                for branch in branches {
                    self.collect(&branch.condition, module, resolved);
                    self.collect(&branch.body, module, resolved);
                }
                if let Some(else_branch) = else_branch {
                    self.collect(else_branch, module, resolved);
                }
            }
            ExprKind::Match { scrutinee, arms } => {
                self.collect(scrutinee, module, resolved);
                for arm in arms {
                    self.collect(&arm.body, module, resolved);
                }
            }
            ExprKind::While { condition, body } => {
                self.collect(condition, module, resolved);
                self.collect(body, module, resolved);
            }
            ExprKind::Return(inner) => {
                if let Some(inner) = inner {
                    self.collect(inner, module, resolved);
                }
            }
            ExprKind::Panic(args) => {
                for arg in args {
                    self.collect(arg, module, resolved);
                }
            }
            ExprKind::For { iterable, body, .. } => {
                self.collect(iterable, module, resolved);
                self.collect(body, module, resolved);
            }
            ExprKind::Missing
            | ExprKind::Literal(_)
            | ExprKind::Name(_)
            | ExprKind::Break
            | ExprKind::Continue => {}
        }
    }

    fn finish(self) -> Option<Shape> {
        (!self.errors.is_empty()).then(|| union_shapes(self.errors))
    }
}

fn value_shape_hint(expr: &Expr, module: &Module, resolved: &ResolvedModule) -> Shape {
    expr_shape_fact(expr, module, resolved).unwrap_or_else(|| {
        expr_literal_fact(expr)
            .map(Shape::Literal)
            .unwrap_or(Shape::Hole)
    })
}

fn string_literal_body(text: &str) -> &str {
    if let Some(body) = text
        .strip_prefix("\"\"\"")
        .and_then(|s| s.strip_suffix("\"\"\""))
    {
        body
    } else {
        text.strip_prefix('"')
            .and_then(|s| s.strip_suffix('"'))
            .unwrap_or(text)
    }
}

fn union_shapes(shapes: Vec<Shape>) -> Shape {
    let mut flattened = Vec::new();
    for shape in shapes {
        match shape {
            Shape::Union(items) => flattened.extend(items),
            other => flattened.push(other),
        }
    }

    let mut unique = Vec::new();
    for shape in flattened {
        if !unique.contains(&shape) {
            unique.push(shape);
        }
    }

    match unique.as_slice() {
        [] => Shape::Hole,
        [shape] => shape.clone(),
        _ => Shape::Union(unique),
    }
}

fn name_target(expr: &Expr, resolved: &ResolvedModule) -> Option<NameTarget> {
    resolved
        .name_refs
        .iter()
        .find(|name_ref| name_ref.expr == expr.id)
        .map(|name_ref| name_ref.target)
}

fn variant_shape(target: NameTarget, arg_shape: Option<Shape>, module: &Module) -> Option<Shape> {
    match target {
        NameTarget::Variant(tune_resolve::VariantId::Prelude(PreludeVariant::Ok)) => {
            Some(Shape::Result {
                ok: Box::new(arg_shape.unwrap_or(Shape::Hole)),
                err: Box::new(Shape::Hole),
            })
        }
        NameTarget::Variant(tune_resolve::VariantId::Prelude(PreludeVariant::Error)) => {
            Some(Shape::Result {
                ok: Box::new(Shape::Hole),
                err: Box::new(arg_shape.unwrap_or(Shape::Hole)),
            })
        }
        NameTarget::Variant(VariantId::Member(variant)) => module
            .items
            .iter()
            .find(|item| item.id == variant.owner)
            .and_then(|item| item.name.as_ref())
            .map(|name| Shape::Enum(name.clone())),
        _ => None,
    }
}

fn result_ok_shape(shape: Shape) -> Option<Shape> {
    match shape {
        Shape::Result { ok, .. } => Some(*ok),
        _ => None,
    }
}

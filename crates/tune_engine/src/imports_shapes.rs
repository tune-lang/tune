use tune_hir::shape::{ShapeExpr, ShapeExprKind};
use tune_shape::Shape;

pub(crate) fn shape_expr(shape: &Shape) -> ShapeExpr {
    ShapeExpr {
        kind: shape_expr_kind(shape),
        span: None,
    }
}

fn shape_expr_kind(shape: &Shape) -> ShapeExprKind {
    match shape {
        Shape::Hole => ShapeExprKind::Missing,
        Shape::Never => ShapeExprKind::Named("Never".into()),
        Shape::Unit => ShapeExprKind::Named("Unit".into()),
        Shape::Int => ShapeExprKind::Named("Int".into()),
        Shape::Float => ShapeExprKind::Named("Float".into()),
        Shape::Size => ShapeExprKind::Named("Size".into()),
        Shape::Byte => ShapeExprKind::Named("Byte".into()),
        Shape::Bool => ShapeExprKind::Named("Bool".into()),
        Shape::String => ShapeExprKind::Named("String".into()),
        Shape::Sequence(inner) => ShapeExprKind::Sequence(Box::new(shape_expr(inner))),
        Shape::Tuple(items) => ShapeExprKind::Tuple(items.iter().map(shape_expr).collect()),
        Shape::Optional(inner) => ShapeExprKind::Optional(Box::new(shape_expr(inner))),
        Shape::Union(items) => ShapeExprKind::Union(items.iter().map(shape_expr).collect()),
        Shape::Callable { params, ret } => ShapeExprKind::Callable {
            params: params.iter().map(shape_expr).collect(),
            ret: Box::new(shape_expr(ret)),
        },
        Shape::Result { ok, err } => ShapeExprKind::Generic {
            name: "Result".into(),
            args: vec![shape_expr(ok), shape_expr(err)],
        },
        Shape::Task(inner) => ShapeExprKind::Generic {
            name: "Task".into(),
            args: vec![shape_expr(inner)],
        },
        Shape::Apply { nominal, args } => ShapeExprKind::Generic {
            name: nominal.name.clone(),
            args: args.iter().map(shape_expr).collect(),
        },
        Shape::Struct(nominal) | Shape::Enum(nominal) => ShapeExprKind::Named(nominal.name.clone()),
        Shape::Range(inner) => ShapeExprKind::Generic {
            name: "Range".into(),
            args: vec![shape_expr(inner)],
        },
        Shape::Literal(_) | Shape::Param(_) | Shape::Structural(_) => ShapeExprKind::Missing,
    }
}

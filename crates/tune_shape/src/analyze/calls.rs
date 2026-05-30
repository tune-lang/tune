use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::{Expr, ExprKind};
use tune_hir::item::{Item, ItemKind, StructMember, Variant};
use tune_resolve::{NameTarget, PreludeVariant, VariantId};

use super::{
    Analyzer, CallCheck, CallSignature, CallTarget,
    generics::{item_type_param_solution, solve_generic_call_signature, substitute_generic_params},
};
use crate::{MemberRequirement, NominalShape, Shape, expr_shape_fact};

impl Analyzer<'_> {
    pub(super) fn analyze_call(&mut self, expr: &Expr, callee: &Expr, args: &[Expr]) -> Shape {
        if !matches!(callee.kind, ExprKind::Field { .. }) {
            self.analyze_expr(callee);
        }
        let signature = self.call_signature(callee);
        let arg_shapes = match signature.as_ref() {
            Some(signature) if !signature_contains_type_params(signature) => args
                .iter()
                .enumerate()
                .map(|(index, arg)| {
                    if let Some(param) = signature.params.get(index) {
                        self.analyze_expr_expected(arg, param)
                    } else {
                        self.analyze_expr(arg)
                    }
                })
                .collect::<Vec<_>>(),
            _ => args
                .iter()
                .map(|arg| self.analyze_expr(arg))
                .collect::<Vec<_>>(),
        };
        let signature = signature.map(|signature| {
            solve_generic_call_signature(signature, &arg_shapes, self.expected_shape())
        });
        let ret = signature.as_ref().map_or_else(
            || expr_shape_fact(expr, self.module, self.resolved).unwrap_or(Shape::Hole),
            |signature| {
                self.check_call_args(expr, signature, &arg_shapes);
                self.check_generic_call_solved(expr, signature);
                if matches!(signature.target, CallTarget::Variant(_)) {
                    expr_shape_fact(expr, self.module, self.resolved)
                        .unwrap_or_else(|| signature.ret.clone())
                } else {
                    signature.ret.clone()
                }
            },
        );
        self.calls.push(CallCheck {
            expr: expr.id,
            target: signature
                .as_ref()
                .map_or(CallTarget::Unknown, |signature| signature.target),
            args: arg_shapes,
            params: signature
                .as_ref()
                .map_or_else(Vec::new, |signature| signature.params.clone()),
            ret: ret.clone(),
            type_args: signature
                .as_ref()
                .map_or_else(Vec::new, |signature| signature.type_args.clone()),
            receiver: signature
                .as_ref()
                .and_then(|signature| signature.receiver.clone()),
            span: expr.span,
        });
        ret
    }

    fn check_call_args(&mut self, expr: &Expr, signature: &CallSignature, args: &[Shape]) {
        if signature.params.len() != args.len() {
            self.diagnostics.push(
                Diagnostic::error(
                    codes::CALLABLE_MISMATCH,
                    "call argument count does not match callable signature",
                    expr.span.or(signature.span).unwrap_or_else(Span::synthetic),
                    format!(
                        "expected {} argument(s), got {}",
                        signature.params.len(),
                        args.len()
                    ),
                )
                .build(),
            );
            return;
        }

        for (index, (expected, actual)) in signature.params.iter().zip(args).enumerate() {
            if !expected.accepts(actual) && !self.structural_pattern_can_match(actual, expected) {
                self.diagnostics.push(
                    Diagnostic::error(
                        codes::CALLABLE_MISMATCH,
                        "call argument does not match callable parameter shape",
                        expr.span.or(signature.span).unwrap_or_else(Span::synthetic),
                        format!(
                            "argument {} expected `{expected:?}`, got `{actual:?}`",
                            index + 1
                        ),
                    )
                    .build(),
                );
            }
        }
    }

    fn check_generic_call_solved(&mut self, expr: &Expr, signature: &CallSignature) {
        if signature.type_params.is_empty()
            || !matches!(
                signature.target,
                CallTarget::TopLevel(_) | CallTarget::Variant(_)
            )
            || !signature_contains_type_params(signature)
        {
            return;
        }
        let unsolved = signature
            .type_params
            .iter()
            .enumerate()
            .filter_map(|(index, param)| {
                let solved = signature
                    .type_args
                    .get(index)
                    .is_some_and(|shape| *shape != Shape::Hole);
                (!solved).then_some(param.as_str())
            })
            .collect::<Vec<_>>();
        if unsolved.is_empty() {
            return;
        }
        self.diagnostics.push(
            Diagnostic::error(
                codes::CALLABLE_MISMATCH,
                "generic call type arguments could not be inferred",
                expr.span.or(signature.span).unwrap_or_else(Span::synthetic),
                format!("unsolved generic parameter(s): {}", unsolved.join(", ")),
            )
            .with_help("add enough argument or return context for Tune to solve the generic shape")
            .build(),
        );
    }

    fn call_signature(&mut self, callee: &Expr) -> Option<CallSignature> {
        match self.name_target(callee) {
            Some(NameTarget::TopLevel(id)) => return self.top_level_signature(id),
            Some(NameTarget::Variant(variant)) => return self.variant_signature(variant),
            Some(NameTarget::Local(_) | NameTarget::Param(_) | NameTarget::SelfValue) | None => {}
        }
        match &callee.kind {
            ExprKind::Name(_) => self.name_call_signature(callee),
            ExprKind::Field { base, name } => self.member_call_signature(base, name.as_deref()),
            _ => None,
        }
    }

    fn name_call_signature(&mut self, callee: &Expr) -> Option<CallSignature> {
        match self.name_target(callee)? {
            NameTarget::TopLevel(id) => self.top_level_signature(id),
            NameTarget::Variant(variant) => self.variant_signature(variant),
            NameTarget::Local(_) | NameTarget::Param(_) | NameTarget::SelfValue => {
                let shape = self.name_shape(callee);
                if let Shape::Callable { params, ret } = shape {
                    return Some(CallSignature {
                        target: CallTarget::Bound,
                        params,
                        param_type_params: Vec::new(),
                        ret: *ret,
                        type_params: Vec::new(),
                        type_args: Vec::new(),
                        receiver: None,
                        span: callee.span,
                    });
                }
                if shape != Shape::Hole {
                    self.diagnostics
                        .push(non_callable_call(&shape, callee.span));
                }
                None
            }
        }
    }

    fn member_call_signature(
        &mut self,
        base: &Expr,
        member_name: Option<&str>,
    ) -> Option<CallSignature> {
        let base_shape = self.analyze_expr(base);
        if base_shape == Shape::String && member_name == Some("len") {
            return Some(CallSignature {
                target: CallTarget::StringLen,
                params: Vec::new(),
                param_type_params: Vec::new(),
                ret: Shape::Size,
                type_params: Vec::new(),
                type_args: Vec::new(),
                receiver: Some(base_shape),
                span: base.span,
            });
        }
        if matches!(base_shape, Shape::Sequence(_)) && member_name == Some("len") {
            return Some(CallSignature {
                target: CallTarget::Bound,
                params: Vec::new(),
                param_type_params: Vec::new(),
                ret: Shape::Size,
                type_params: Vec::new(),
                type_args: Vec::new(),
                receiver: Some(base_shape),
                span: base.span,
            });
        }
        if let Shape::Task(inner) = &base_shape
            && member_name == Some("join")
        {
            return Some(CallSignature {
                target: CallTarget::TaskJoin,
                params: Vec::new(),
                param_type_params: Vec::new(),
                ret: inner.as_ref().clone(),
                type_params: Vec::new(),
                type_args: Vec::new(),
                receiver: Some(base_shape),
                span: base.span,
            });
        }
        if let Some(signature) =
            structural_member_call_signature(&base_shape, member_name, base.span)
        {
            return Some(signature);
        }
        let struct_name = struct_shape_name(&base_shape)?;
        let (item, callable) = self
            .module
            .items
            .iter()
            .find(|item| item.kind == ItemKind::Struct && item.name.as_deref() == Some(struct_name))
            .and_then(|item| {
                item.struct_members.iter().find_map(|member| match member {
                    StructMember::Callable(callable) if callable.name.as_deref() == member_name => {
                        Some((item, callable))
                    }
                    _ => None,
                })
            })?;
        let mut params = callable
            .params
            .iter()
            .map(|param| self.lower_item_shape_or_hole(item, param.shape.as_ref()))
            .collect::<Vec<_>>();
        let mut ret = self.lower_item_shape_or_hole(item, callable.shape.as_ref());
        if let Shape::Apply { args, .. } = &base_shape {
            let solved = item_type_param_solution(item, args);
            params = params
                .iter()
                .map(|param| substitute_generic_params(param, &solved))
                .collect();
            ret = substitute_generic_params(&ret, &solved);
        }
        Some(CallSignature {
            target: CallTarget::Member(callable.id),
            params,
            param_type_params: Vec::new(),
            ret,
            type_params: item
                .type_params
                .iter()
                .filter_map(|param| param.name.clone())
                .collect(),
            type_args: Vec::new(),
            receiver: Some(base_shape),
            span: callable.span,
        })
    }

    fn top_level_signature(&mut self, id: tune_hir::HirId) -> Option<CallSignature> {
        let item = self.module.items.iter().find(|item| item.id == id)?;
        if item.kind != ItemKind::CallableDecl {
            return None;
        }
        Some(CallSignature {
            target: CallTarget::TopLevel(id),
            params: item
                .params
                .iter()
                .map(|param| self.lower_item_shape_or_hole(item, param.shape.as_ref()))
                .collect(),
            param_type_params: item
                .params
                .iter()
                .map(|param| direct_type_param_name(item, param.shape.as_ref()))
                .collect(),
            ret: self.lower_item_shape_or_hole(item, item.shape.as_ref()),
            type_params: item
                .type_params
                .iter()
                .filter_map(|param| param.name.clone())
                .collect(),
            type_args: Vec::new(),
            receiver: None,
            span: item.span,
        })
    }

    fn variant_signature(&mut self, variant: VariantId) -> Option<CallSignature> {
        match variant {
            VariantId::Prelude(PreludeVariant::Ok | PreludeVariant::Error) => Some(CallSignature {
                target: CallTarget::Variant(variant),
                params: vec![Shape::Hole],
                param_type_params: Vec::new(),
                ret: Shape::Result {
                    ok: Box::new(Shape::Hole),
                    err: Box::new(Shape::Hole),
                },
                type_params: Vec::new(),
                type_args: Vec::new(),
                receiver: None,
                span: None,
            }),
            VariantId::Member(id) => {
                let item = self
                    .module
                    .items
                    .iter()
                    .find(|item| item.id == id.owner)?
                    .clone();
                let variant_item = item
                    .variants
                    .iter()
                    .find(|variant| variant.id == id)?
                    .clone();
                let mut params = Vec::new();
                for payload in &variant_item.payload {
                    let lowered = super::item_shapes::lower_item_shape_expr(
                        payload,
                        &item,
                        &self.resolved.scope,
                    );
                    params.push(lowered.shape);
                    self.diagnostics.extend(lowered.diagnostics);
                }
                Some(CallSignature {
                    target: CallTarget::Variant(variant),
                    params,
                    param_type_params: Vec::new(),
                    ret: variant_return_shape(&item, &variant_item),
                    type_params: item
                        .type_params
                        .iter()
                        .filter_map(|param| param.name.clone())
                        .collect(),
                    type_args: Vec::new(),
                    receiver: None,
                    span: variant_item.span,
                })
            }
        }
    }

    fn name_target(&self, expr: &Expr) -> Option<NameTarget> {
        self.resolved
            .name_refs
            .iter()
            .find(|name_ref| name_ref.expr == expr.id)
            .map(|name_ref| name_ref.target)
    }
}

fn signature_contains_type_params(signature: &CallSignature) -> bool {
    signature.param_type_params.iter().any(Option::is_some)
        || signature.params.iter().any(shape_contains_type_param)
        || shape_contains_type_param(&signature.ret)
}

fn direct_type_param_name(
    item: &Item,
    shape: Option<&tune_hir::shape::ShapeExpr>,
) -> Option<String> {
    let Some(tune_hir::shape::ShapeExpr {
        kind: tune_hir::shape::ShapeExprKind::Named(name),
        ..
    }) = shape
    else {
        return None;
    };
    item.type_params
        .iter()
        .any(|param| param.name.as_deref() == Some(name.as_str()))
        .then(|| name.clone())
}

fn shape_contains_type_param(shape: &Shape) -> bool {
    match shape {
        Shape::Param(_) => true,
        Shape::Sequence(inner)
        | Shape::Range(inner)
        | Shape::Optional(inner)
        | Shape::Task(inner) => shape_contains_type_param(inner),
        Shape::Tuple(items) | Shape::Union(items) => items.iter().any(shape_contains_type_param),
        Shape::Callable { params, ret } => {
            params.iter().any(shape_contains_type_param) || shape_contains_type_param(ret)
        }
        Shape::Result { ok, err } => {
            shape_contains_type_param(ok) || shape_contains_type_param(err)
        }
        Shape::Apply { args, .. } => args.iter().any(shape_contains_type_param),
        Shape::Structural(requirements) => {
            requirements.iter().any(|requirement| match requirement {
                MemberRequirement::Field { shape, .. } => {
                    shape.as_ref().is_some_and(shape_contains_type_param)
                }
                MemberRequirement::Callable { params, ret, .. } => {
                    params.iter().any(shape_contains_type_param)
                        || ret.as_ref().is_some_and(shape_contains_type_param)
                }
            })
        }
        _ => false,
    }
}

fn struct_shape_name(shape: &Shape) -> Option<&str> {
    shape.nominal_name()
}

fn structural_member_call_signature(
    base_shape: &Shape,
    member_name: Option<&str>,
    span: Option<Span>,
) -> Option<CallSignature> {
    let Shape::Structural(requirements) = base_shape else {
        return None;
    };
    requirements.iter().find_map(|requirement| {
        let MemberRequirement::Callable { name, params, ret } = requirement else {
            return None;
        };
        (Some(name.as_str()) == member_name).then(|| CallSignature {
            target: CallTarget::Bound,
            params: params.clone(),
            param_type_params: Vec::new(),
            ret: ret.clone().unwrap_or(Shape::Hole),
            type_params: Vec::new(),
            type_args: Vec::new(),
            receiver: Some(base_shape.clone()),
            span,
        })
    })
}

fn variant_return_shape(item: &Item, variant: &Variant) -> Shape {
    let Some(name) = item.name.as_ref() else {
        return Shape::Hole;
    };
    let nominal = NominalShape::new(item.id, name);
    if item.type_params.is_empty() {
        return Shape::Enum(nominal);
    }
    Shape::Apply {
        nominal,
        args: item
            .type_params
            .iter()
            .map(|param| param.name.clone().map_or(Shape::Hole, Shape::Param))
            .zip(variant.payload.iter())
            .map(|(param, _)| param)
            .collect(),
    }
}

fn non_callable_call(shape: &Shape, span: Option<Span>) -> Diagnostic {
    Diagnostic::error(
        codes::CALLABLE_MISMATCH,
        "called value is not callable",
        span.unwrap_or_else(Span::synthetic),
        format!("this value has shape `{shape:?}`, which cannot be called"),
    )
    .build()
}

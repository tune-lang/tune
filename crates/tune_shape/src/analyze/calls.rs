use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::expr::{Expr, ExprKind};
use tune_hir::item::{Item, ItemKind, StructMember, Variant};
use tune_hir::shape::{ShapeExpr, ShapeExprKind, StructuralShapeRequirementKind};
use tune_resolve::{NameTarget, PreludeVariant, VariantId};

use super::{Analyzer, CallCheck, CallSignature, CallTarget};
use crate::{MemberRequirement, NominalShape, Shape, builtin::builtin_shape, expr_shape_fact};

impl Analyzer<'_> {
    pub(super) fn analyze_call(&mut self, expr: &Expr, callee: &Expr, args: &[Expr]) -> Shape {
        if !matches!(callee.kind, ExprKind::Field { .. }) {
            self.analyze_expr(callee);
        }
        let arg_shapes = args
            .iter()
            .map(|arg| self.analyze_expr(arg))
            .collect::<Vec<_>>();
        let signature = self.call_signature(callee);
        let ret = signature.as_ref().map_or_else(
            || expr_shape_fact(expr, self.module, self.resolved).unwrap_or(Shape::Hole),
            |signature| {
                self.check_call_args(expr, signature, &arg_shapes);
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

    fn call_signature(&mut self, callee: &Expr) -> Option<CallSignature> {
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
                        ret: *ret,
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
                ret: Shape::Size,
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
        Some(CallSignature {
            target: CallTarget::Member(callable.id),
            params: callable
                .params
                .iter()
                .map(|param| self.lower_item_shape_or_hole(item, param.shape.as_ref()))
                .collect(),
            ret: self.lower_item_shape_or_hole(item, callable.shape.as_ref()),
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
            ret: self.lower_item_shape_or_hole(item, item.shape.as_ref()),
            receiver: None,
            span: item.span,
        })
    }

    fn variant_signature(&mut self, variant: VariantId) -> Option<CallSignature> {
        match variant {
            VariantId::Prelude(PreludeVariant::Ok | PreludeVariant::Error) => Some(CallSignature {
                target: CallTarget::Variant(variant),
                params: vec![Shape::Hole],
                ret: Shape::Result {
                    ok: Box::new(Shape::Hole),
                    err: Box::new(Shape::Hole),
                },
                receiver: None,
                span: None,
            }),
            VariantId::Member(id) => {
                let item = self.module.items.iter().find(|item| item.id == id.owner)?;
                let variant_item = item.variants.iter().find(|variant| variant.id == id)?;
                Some(CallSignature {
                    target: CallTarget::Variant(variant),
                    params: variant_item
                        .payload
                        .iter()
                        .map(|payload| lower_payload_shape(payload, item))
                        .collect(),
                    ret: variant_return_shape(item, variant_item),
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
            ret: ret.clone().unwrap_or(Shape::Hole),
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

fn lower_payload_shape(shape: &ShapeExpr, item: &Item) -> Shape {
    match &shape.kind {
        ShapeExprKind::Named(name)
            if item
                .type_params
                .iter()
                .any(|param| param.name.as_deref() == Some(name.as_str())) =>
        {
            Shape::Hole
        }
        ShapeExprKind::Named(name) => named_payload_shape(name),
        ShapeExprKind::Sequence(element) => {
            Shape::Sequence(Box::new(lower_payload_shape(element, item)))
        }
        ShapeExprKind::Tuple(items) => Shape::Tuple(
            items
                .iter()
                .map(|item_shape| lower_payload_shape(item_shape, item))
                .collect(),
        ),
        ShapeExprKind::Optional(inner) => {
            Shape::Optional(Box::new(lower_payload_shape(inner, item)))
        }
        ShapeExprKind::Union(items) => Shape::Union(
            items
                .iter()
                .map(|item_shape| lower_payload_shape(item_shape, item))
                .collect(),
        ),
        ShapeExprKind::Structural(requirements) => Shape::Structural(
            requirements
                .iter()
                .map(|requirement| match &requirement.kind {
                    StructuralShapeRequirementKind::Field { shape } => MemberRequirement::Field {
                        name: requirement.name.clone(),
                        shape: shape.as_ref().map(|shape| lower_payload_shape(shape, item)),
                    },
                    StructuralShapeRequirementKind::Callable { params, ret } => {
                        MemberRequirement::Callable {
                            name: requirement.name.clone(),
                            params: params
                                .iter()
                                .map(|param| lower_payload_shape(param, item))
                                .collect(),
                            ret: ret.as_ref().map(|ret| lower_payload_shape(ret, item)),
                        }
                    }
                })
                .collect(),
        ),
        ShapeExprKind::Generic { .. } => Shape::Hole,
        ShapeExprKind::Callable { params, ret } => Shape::Callable {
            params: params
                .iter()
                .map(|param| lower_payload_shape(param, item))
                .collect(),
            ret: Box::new(lower_payload_shape(ret, item)),
        },
        ShapeExprKind::Missing => Shape::Hole,
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

fn named_payload_shape(name: &str) -> Shape {
    builtin_shape(name).unwrap_or(Shape::Hole)
}

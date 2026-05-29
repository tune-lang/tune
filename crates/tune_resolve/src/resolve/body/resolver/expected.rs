use tune_hir::MemberId;
use tune_hir::expr::{Expr, ExprKind, LiteralKind, StringPart};
use tune_hir::item::{Item, StructMember};
use tune_hir::shape::{ShapeExpr, ShapeExprKind};

use crate::locals::NameTarget;
use crate::prelude::{PreludeVariant, VariantId};

use super::BodyResolver;

impl BodyResolver<'_> {
    pub(super) fn expected_struct_field_shape(
        &self,
        struct_name: &str,
        field_name: &str,
    ) -> Option<ShapeExpr> {
        self.items
            .iter()
            .find(|item| {
                item.kind == tune_hir::item::ItemKind::Struct
                    && item.name.as_deref() == Some(struct_name)
            })
            .and_then(|item| {
                item.struct_members.iter().find_map(|member| {
                    let StructMember::Field(field) = member else {
                        return None;
                    };
                    (field.name.as_deref() == Some(field_name))
                        .then(|| field.shape.clone())
                        .flatten()
                })
            })
    }

    pub(super) fn expected_arg_shapes_for_call(&self, callee: &Expr) -> Vec<Option<ShapeExpr>> {
        let ExprKind::Name(name) = &callee.kind else {
            return Vec::new();
        };

        let Some(NameTarget::TopLevel(item_id)) = self.lookup_local(name).or_else(|| {
            self.resolved
                .scope
                .get(name)
                .map(|binding| NameTarget::TopLevel(binding.id))
        }) else {
            return Vec::new();
        };

        self.items
            .iter()
            .find(|item| item.id == item_id)
            .map(|item| {
                item.params
                    .iter()
                    .map(|param| param.shape.clone())
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(super) fn expected_arg_shapes_for_variant_callee(
        &self,
        callee: &Expr,
    ) -> Option<Vec<Option<ShapeExpr>>> {
        let variant = self
            .resolved
            .name_refs
            .iter()
            .find(|name_ref| name_ref.expr == callee.id)
            .and_then(|name_ref| match name_ref.target {
                NameTarget::Variant(variant) => Some(variant),
                _ => None,
            })?;
        let crate::VariantId::Member(member) = variant else {
            return None;
        };
        self.items
            .iter()
            .find(|item| item.id == member.owner)
            .and_then(|item| item.variants.iter().find(|variant| variant.id == member))
            .map(|variant| variant.payload.iter().cloned().map(Some).collect())
    }

    pub(super) fn variant_for_expected_enum(
        &self,
        variant_name: &str,
        expected: Option<&ShapeExpr>,
    ) -> Option<MemberId> {
        let enum_name = expected_enum_name(expected?)?;
        self.items
            .iter()
            .find(|item| {
                item.kind == tune_hir::item::ItemKind::Enum
                    && item.name.as_deref() == Some(enum_name)
            })
            .and_then(|item| {
                item.variants
                    .iter()
                    .find(|variant| variant.name.as_deref() == Some(variant_name))
            })
            .map(|variant| variant.id)
    }

    pub(super) fn variant_for_expected_shape(
        &self,
        variant_name: &str,
        expected: Option<&ShapeExpr>,
    ) -> Option<VariantId> {
        self.variant_for_expected_result(variant_name, expected)
            .map(VariantId::Prelude)
            .or_else(|| {
                self.variant_for_expected_enum(variant_name, expected)
                    .map(VariantId::Member)
            })
    }

    fn variant_for_expected_result(
        &self,
        variant_name: &str,
        expected: Option<&ShapeExpr>,
    ) -> Option<PreludeVariant> {
        if !is_result_shape(expected?) {
            return None;
        }
        self.resolved.prelude.variant(variant_name)
    }

    pub(super) fn expected_shape_for_expr(&self, expr: &Expr) -> Option<ShapeExpr> {
        let target = match &expr.kind {
            ExprKind::Name(name) => self.lookup_local(name).or_else(|| {
                self.resolved
                    .scope
                    .get(name)
                    .map(|binding| NameTarget::TopLevel(binding.id))
            })?,
            ExprKind::Call { callee, .. } => return self.call_return_shape(callee),
            _ => return None,
        };
        match target {
            NameTarget::TopLevel(item_id) => self
                .items
                .iter()
                .find(|item| item.id == item_id)
                .and_then(|item| item.shape.clone()),
            NameTarget::Param(param) => self.shape_for_param(param),
            NameTarget::Local(local) => self
                .resolved
                .locals
                .iter()
                .find(|binding| binding.id == local)
                .and_then(|binding| binding.expr)
                .and_then(|expr| self.shape_for_local_expr(expr)),
            NameTarget::SelfValue | NameTarget::Variant(_) => None,
        }
    }

    fn call_return_shape(&self, callee: &Expr) -> Option<ShapeExpr> {
        let ExprKind::Name(name) = &callee.kind else {
            return None;
        };
        let NameTarget::TopLevel(item_id) = self.lookup_local(name).or_else(|| {
            self.resolved
                .scope
                .get(name)
                .map(|binding| NameTarget::TopLevel(binding.id))
        })?
        else {
            return None;
        };
        self.items
            .iter()
            .find(|item| item.id == item_id)
            .and_then(|item| item.shape.clone())
    }

    pub(super) fn expected_result_for_propagate(
        &self,
        expected: Option<&ShapeExpr>,
    ) -> Option<ShapeExpr> {
        let expected = expected?;
        let span = expected.span;
        let missing = || ShapeExpr {
            kind: ShapeExprKind::Missing,
            span,
        };
        let result = |ok: ShapeExpr, err: ShapeExpr| ShapeExpr {
            kind: ShapeExprKind::Generic {
                name: "Result".to_owned(),
                args: vec![ok, err],
            },
            span,
        };

        match &expected.kind {
            ShapeExprKind::Named(name) if name == "Result" => Some(result(missing(), missing())),
            ShapeExprKind::Generic { name, args } if name == "Result" => {
                let err = args.get(1).cloned().unwrap_or_else(missing);
                Some(result(missing(), err))
            }
            _ => Some(result(expected.clone(), missing())),
        }
    }

    fn shape_for_param(&self, param: MemberId) -> Option<ShapeExpr> {
        self.items
            .iter()
            .find_map(|item| shape_for_item_param(item, param))
    }

    fn shape_for_local_expr(&self, expr: tune_hir::ExprId) -> Option<ShapeExpr> {
        self.items.iter().find_map(|item| {
            item.body
                .as_ref()
                .and_then(|body| shape_for_let_expr(body, expr))
                .or_else(|| {
                    item.struct_members.iter().find_map(|member| {
                        let StructMember::Callable(callable) = member else {
                            return None;
                        };
                        callable
                            .body
                            .as_ref()
                            .and_then(|body| shape_for_let_expr(body, expr))
                    })
                })
        })
    }
}

fn expected_enum_name(expected: &ShapeExpr) -> Option<&str> {
    match &expected.kind {
        ShapeExprKind::Named(name) | ShapeExprKind::Generic { name, .. } => Some(name.as_str()),
        _ => None,
    }
}

fn is_result_shape(expected: &ShapeExpr) -> bool {
    match &expected.kind {
        ShapeExprKind::Named(name) | ShapeExprKind::Generic { name, .. } => name == "Result",
        _ => false,
    }
}

fn shape_for_item_param(item: &Item, param: MemberId) -> Option<ShapeExpr> {
    item.params
        .iter()
        .find(|candidate| candidate.id == param)
        .and_then(|candidate| candidate.shape.clone())
        .or_else(|| {
            item.struct_members.iter().find_map(|member| {
                let StructMember::Callable(callable) = member else {
                    return None;
                };
                callable
                    .params
                    .iter()
                    .find(|candidate| candidate.id == param)
                    .and_then(|candidate| candidate.shape.clone())
            })
        })
}

fn shape_for_let_expr(expr: &Expr, target: tune_hir::ExprId) -> Option<ShapeExpr> {
    match &expr.kind {
        ExprKind::Let { shape, .. } if expr.id == target => shape.clone(),
        ExprKind::Tuple(elements) | ExprKind::Sequence(elements) | ExprKind::Block(elements) => {
            elements
                .iter()
                .find_map(|element| shape_for_let_expr(element, target))
        }
        ExprKind::Struct { fields, .. } => fields
            .iter()
            .find_map(|field| shape_for_let_expr(&field.value, target)),
        ExprKind::CallableValue { body, .. }
        | ExprKind::Spawn(body)
        | ExprKind::Propagate(body)
        | ExprKind::Loop(body) => shape_for_let_expr(body, target),
        ExprKind::Call { callee, args } => shape_for_let_expr(callee, target)
            .or_else(|| args.iter().find_map(|arg| shape_for_let_expr(arg, target))),
        ExprKind::Field { base, .. } => shape_for_let_expr(base, target),
        ExprKind::Index { base, index }
        | ExprKind::Binary {
            lhs: base,
            rhs: index,
            ..
        } => shape_for_let_expr(base, target).or_else(|| shape_for_let_expr(index, target)),
        ExprKind::Let { value, .. } => value
            .as_ref()
            .and_then(|value| shape_for_let_expr(value, target)),
        ExprKind::Assign {
            target: assign_target,
            value,
        } => {
            shape_for_let_expr(assign_target, target).or_else(|| shape_for_let_expr(value, target))
        }
        ExprKind::Unary { expr, .. } => shape_for_let_expr(expr, target),
        ExprKind::If {
            branches,
            else_branch,
        } => branches
            .iter()
            .find_map(|branch| {
                shape_for_let_expr(&branch.condition, target)
                    .or_else(|| shape_for_let_expr(&branch.body, target))
            })
            .or_else(|| {
                else_branch
                    .as_ref()
                    .and_then(|branch| shape_for_let_expr(branch, target))
            }),
        ExprKind::Match { scrutinee, arms } => {
            shape_for_let_expr(scrutinee, target).or_else(|| {
                arms.iter()
                    .find_map(|arm| shape_for_let_expr(&arm.body, target))
            })
        }
        ExprKind::While { condition, body } => {
            shape_for_let_expr(condition, target).or_else(|| shape_for_let_expr(body, target))
        }
        ExprKind::Return(inner) => inner
            .as_ref()
            .and_then(|inner| shape_for_let_expr(inner, target)),
        ExprKind::Panic(args) => args.iter().find_map(|arg| shape_for_let_expr(arg, target)),
        ExprKind::For { iterable, body, .. } => {
            shape_for_let_expr(iterable, target).or_else(|| shape_for_let_expr(body, target))
        }
        ExprKind::Literal(LiteralKind::String(literal)) => {
            literal.parts.iter().find_map(|part| match part {
                StringPart::Text(_) => None,
                StringPart::Interpolation(expr) => shape_for_let_expr(expr, target),
            })
        }
        ExprKind::Missing
        | ExprKind::Literal(_)
        | ExprKind::Name(_)
        | ExprKind::Break
        | ExprKind::Continue => None,
    }
}

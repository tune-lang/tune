use tune_diagnostics::{ByteOffset, Span};
mod walk;

use tune_hir::expr::{Expr, ExprKind};
use tune_hir::item::{Item, StructMember};
use tune_hir::{ExprId, HirId, MemberId};
use tune_resolve::{FactOwner, LocalId, NameTarget, VariantId};
use tune_shape::{BindingKey, CallCheck, Shape};

use crate::{FileId, ModuleAnalysis, TuneDb};
use walk::item_exprs;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticCursor {
    pub file: FileId,
    pub offset: ByteOffset,
    pub owner: Option<HirId>,
    pub owner_fact: Option<FactOwner>,
    pub expr: Option<SemanticExpr>,
    pub reference: Option<SemanticReference>,
    pub call: Option<SemanticCallContext>,
    pub scope: Vec<SemanticBinding>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticExpr {
    pub id: ExprId,
    pub span: Option<Span>,
    pub shape: Option<Shape>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticExprSpan {
    pub id: ExprId,
    pub span: Option<Span>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticReference {
    pub expr: ExprId,
    pub span: Option<Span>,
    pub target: NameTarget,
    pub definition: Option<SemanticDefinition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticDefinition {
    pub name: Option<String>,
    pub span: Option<Span>,
    pub owner: Option<FactOwner>,
    pub target: NameTarget,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticCallContext {
    pub expr: ExprId,
    pub span: Option<Span>,
    pub callee: ExprId,
    pub active_arg: Option<usize>,
    pub check: Option<CallCheck>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticBinding {
    pub name: String,
    pub target: NameTarget,
    pub definition: Option<SemanticDefinition>,
    pub shape: Option<Shape>,
}

#[must_use]
pub fn semantic_at(db: &TuneDb, file: FileId, offset: ByteOffset) -> Option<SemanticCursor> {
    let analysis = db.analyze_file(file)?;
    let owner = item_at_offset(&analysis, offset).map(|item| item.id);
    let owner_fact = owner.map(FactOwner::Item);
    let expr = expr_at_offset(&analysis, offset).map(|expr| SemanticExpr {
        id: expr.id,
        span: expr.span,
        shape: expr_shape(&analysis, expr.id),
    });
    let reference = reference_at_offset(&analysis, offset, expr.as_ref().map(|expr| expr.id));
    let call = call_at_offset(&analysis, offset);
    let scope = owner
        .and_then(|owner| scope_for_owner(&analysis, owner, offset))
        .unwrap_or_default();

    Some(SemanticCursor {
        file,
        offset,
        owner,
        owner_fact,
        expr,
        reference,
        call,
        scope,
    })
}

#[must_use]
pub fn semantic_exprs(db: &TuneDb, file: FileId) -> Option<Vec<SemanticExprSpan>> {
    let analysis = db.analyze_file(file)?;
    Some(
        analysis
            .module
            .items
            .iter()
            .flat_map(item_exprs)
            .map(|expr| SemanticExprSpan {
                id: expr.id,
                span: expr.span,
            })
            .collect(),
    )
}

fn item_at_offset(analysis: &ModuleAnalysis, offset: ByteOffset) -> Option<&Item> {
    let mut best = None;
    for item in &analysis.module.items {
        consider_span(&mut best, item.span, item, offset);
    }
    best.map(|(_, item)| item)
}

fn expr_at_offset(analysis: &ModuleAnalysis, offset: ByteOffset) -> Option<&Expr> {
    let mut best = None;
    for item in &analysis.module.items {
        for expr in item_exprs(item) {
            consider_span(&mut best, expr.span, expr, offset);
        }
    }
    best.map(|(_, expr)| expr)
}

fn call_at_offset(analysis: &ModuleAnalysis, offset: ByteOffset) -> Option<SemanticCallContext> {
    let mut best = None;
    for item in &analysis.module.items {
        for expr in item_exprs(item) {
            let ExprKind::Call { callee, args } = &expr.kind else {
                continue;
            };
            consider_span(&mut best, expr.span, (expr, callee.as_ref(), args), offset);
        }
    }
    let (_, (expr, callee, args)) = best?;
    let active_arg = args
        .iter()
        .position(|arg| arg.span.is_some_and(|span| span.contains(offset)));
    Some(SemanticCallContext {
        expr: expr.id,
        span: expr.span,
        callee: callee.id,
        active_arg,
        check: call_check(analysis, expr.id).cloned(),
    })
}

fn reference_at_offset(
    analysis: &ModuleAnalysis,
    offset: ByteOffset,
    expr: Option<ExprId>,
) -> Option<SemanticReference> {
    let name_ref = analysis.resolved.name_refs.iter().find(|name_ref| {
        name_ref.span.is_some_and(|span| span.contains(offset)) || Some(name_ref.expr) == expr
    })?;
    Some(SemanticReference {
        expr: name_ref.expr,
        span: name_ref.span,
        target: name_ref.target,
        definition: definition_for_target(analysis, name_ref.target),
    })
}

fn scope_for_owner(
    analysis: &ModuleAnalysis,
    owner: HirId,
    offset: ByteOffset,
) -> Option<Vec<SemanticBinding>> {
    let item = analysis.module.items.iter().find(|item| item.id == owner)?;
    let mut bindings = Vec::new();

    for item in &analysis.module.items {
        let Some(name) = item.name.clone() else {
            continue;
        };
        let target = NameTarget::TopLevel(item.id);
        bindings.push(SemanticBinding {
            name,
            target,
            definition: definition_for_target(analysis, target),
            shape: item_current_shape(analysis, item.id),
        });
    }

    for param in &item.params {
        push_member_binding(analysis, &mut bindings, param.id, param.name.clone());
    }
    for member in &item.struct_members {
        if let StructMember::Callable(callable) = member {
            for param in &callable.params {
                push_member_binding(analysis, &mut bindings, param.id, param.name.clone());
            }
        }
    }

    for local in &analysis.resolved.locals {
        if local.owner != owner {
            continue;
        }
        let Some(span) = local.span else {
            continue;
        };
        if span.start > offset {
            continue;
        }
        let target = NameTarget::Local(local.id);
        bindings.push(SemanticBinding {
            name: local.name.clone(),
            target,
            definition: definition_for_target(analysis, target),
            shape: binding_shape(analysis, BindingKey::Local(local.id)),
        });
    }

    Some(bindings)
}

fn push_member_binding(
    analysis: &ModuleAnalysis,
    bindings: &mut Vec<SemanticBinding>,
    member: MemberId,
    name: Option<String>,
) {
    let Some(name) = name else {
        return;
    };
    let target = NameTarget::Param(member);
    bindings.push(SemanticBinding {
        name,
        target,
        definition: definition_for_target(analysis, target),
        shape: binding_shape(analysis, BindingKey::Param(member)),
    });
}

fn definition_for_target(
    analysis: &ModuleAnalysis,
    target: NameTarget,
) -> Option<SemanticDefinition> {
    match target {
        NameTarget::TopLevel(id) => {
            let item = analysis.module.items.iter().find(|item| item.id == id)?;
            Some(SemanticDefinition {
                name: item.name.clone(),
                span: item.span,
                owner: Some(FactOwner::Item(id)),
                target,
            })
        }
        NameTarget::Param(id) => member_definition(analysis, id, target),
        NameTarget::Local(id) => local_definition(analysis, id, target),
        NameTarget::Variant(VariantId::Member(id)) => member_definition(analysis, id, target),
        NameTarget::Variant(VariantId::Prelude(_)) | NameTarget::SelfValue => None,
    }
}

fn local_definition(
    analysis: &ModuleAnalysis,
    id: LocalId,
    target: NameTarget,
) -> Option<SemanticDefinition> {
    let local = analysis
        .resolved
        .locals
        .iter()
        .find(|local| local.id == id)?;
    Some(SemanticDefinition {
        name: Some(local.name.clone()),
        span: local.span,
        owner: None,
        target,
    })
}

fn member_definition(
    analysis: &ModuleAnalysis,
    id: MemberId,
    target: NameTarget,
) -> Option<SemanticDefinition> {
    let (name, span) = find_member(analysis, id)?;
    Some(SemanticDefinition {
        name,
        span,
        owner: Some(FactOwner::Member(id)),
        target,
    })
}

fn find_member(analysis: &ModuleAnalysis, id: MemberId) -> Option<(Option<String>, Option<Span>)> {
    for item in &analysis.module.items {
        for param in &item.params {
            if param.id == id {
                return Some((param.name.clone(), param.span));
            }
        }
        for param in &item.type_params {
            if param.id == id {
                return Some((param.name.clone(), param.span));
            }
        }
        for field in &item.fields {
            if field.id == id {
                return Some((field.name.clone(), field.span));
            }
        }
        for variant in &item.variants {
            if variant.id == id {
                return Some((variant.name.clone(), variant.span));
            }
        }
        for member in &item.struct_members {
            match member {
                StructMember::Field(field) if field.id == id => {
                    return Some((field.name.clone(), field.span));
                }
                StructMember::Callable(callable) if callable.id == id => {
                    return Some((callable.name.clone(), callable.span));
                }
                StructMember::Callable(callable) => {
                    for param in &callable.params {
                        if param.id == id {
                            return Some((param.name.clone(), param.span));
                        }
                    }
                }
                StructMember::SequenceMaterializer(member) if member.id == id => {
                    return Some((member.param_name.clone(), member.span));
                }
                StructMember::IndexAccess(member) if member.id == id => {
                    return Some((member.receiver_name.clone(), member.span));
                }
                StructMember::IndexAccess(member) if member.index_param_id == id => {
                    return Some((member.index_param_name.clone(), member.span));
                }
                _ => {}
            }
        }
    }
    None
}

fn item_current_shape(analysis: &ModuleAnalysis, id: HirId) -> Option<Shape> {
    analysis
        .module
        .items
        .iter()
        .position(|item| item.id == id)
        .and_then(|index| analysis.shape.get(index))
        .map(|shape| shape.item_current_shape.clone())
}

fn expr_shape(analysis: &ModuleAnalysis, id: ExprId) -> Option<Shape> {
    analysis
        .shape
        .iter()
        .flat_map(|shape| shape.expr_shapes.iter())
        .find(|shape| shape.expr == id)
        .map(|shape| shape.shape.clone())
}

fn binding_shape(analysis: &ModuleAnalysis, key: BindingKey) -> Option<Shape> {
    analysis
        .shape
        .iter()
        .find_map(|shape| shape.frame.get(key))
        .map(|binding| binding.current_shape.clone())
}

fn call_check(analysis: &ModuleAnalysis, expr: ExprId) -> Option<&CallCheck> {
    analysis
        .shape
        .iter()
        .flat_map(|shape| shape.calls.iter())
        .find(|call| call.expr == expr)
}

fn consider_span<T>(best: &mut Option<(u32, T)>, span: Option<Span>, value: T, offset: ByteOffset) {
    let Some(span) = span else {
        return;
    };
    if !span.contains(offset) {
        return;
    }
    let len = span.len();
    if best.as_ref().is_none_or(|(best_len, _)| len < *best_len) {
        *best = Some((len, value));
    }
}

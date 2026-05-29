use tune_ast::AstNode;

use crate::item::{
    CallableMember, Field, IndexAccess, Param, SequenceMaterializer, StructMember, Variant,
};
use crate::{HirId, MemberId, MemberKind};

use super::exprs::ExprLowerer;
use super::shapes::lower_shape;

pub(super) fn lower_params(source: &str, node: tune_ast::nodes::LetDecl<'_>) -> Vec<Param> {
    node.params()
        .into_iter()
        .flat_map(|params| params.params())
        .enumerate()
        .filter_map(|(index, param)| {
            Some(Param {
                id: member_id(index, MemberKind::Param)?,
                name: param.name(source).map(str::to_owned),
                span: param.syntax().span,
                shape: param
                    .shape_annotation()
                    .map(|shape| lower_shape(source, shape)),
            })
        })
        .collect()
}

pub(super) fn lower_fields(
    source: &str,
    fields: Vec<tune_ast::nodes::DocumentedField<'_>>,
    exprs: &mut ExprLowerer,
) -> Vec<Field> {
    fields
        .into_iter()
        .enumerate()
        .filter_map(|(index, documented)| {
            Some(Field {
                id: member_id(index, MemberKind::Field)?,
                name: documented.field.name(source).map(str::to_owned),
                span: documented.field.syntax().span,
                doc: documented.doc_text(source),
                shape: documented
                    .field
                    .shape_annotation()
                    .map(|shape| lower_shape(source, shape)),
                default: documented
                    .field
                    .default_expr()
                    .map(|expr| exprs.lower(source, expr)),
            })
        })
        .collect()
}

pub(super) fn lower_struct_members(
    source: &str,
    members: Vec<tune_ast::nodes::DocumentedStructMember<'_>>,
    exprs: &mut ExprLowerer,
) -> Vec<StructMember> {
    members
        .into_iter()
        .enumerate()
        .filter_map(|(index, documented)| {
            let id = member_id_for_struct_member(index, documented.member)?;
            let doc = documented.doc_text(source);
            match documented.member {
                tune_ast::nodes::StructMember::Field(field) => Some(StructMember::Field(Field {
                    id,
                    name: field.name(source).map(str::to_owned),
                    span: field.syntax().span,
                    doc,
                    shape: field
                        .shape_annotation()
                        .map(|shape| lower_shape(source, shape)),
                    default: field.default_expr().map(|expr| exprs.lower(source, expr)),
                })),
                tune_ast::nodes::StructMember::Callable(callable) => {
                    Some(StructMember::Callable(CallableMember {
                        id,
                        name: callable.name(source).map(str::to_owned),
                        span: callable.syntax().span,
                        doc,
                        params: lower_member_params(source, callable.params()),
                        shape: callable
                            .shape_annotation()
                            .map(|shape| lower_shape(source, shape)),
                        body: callable.body_expr().map(|body| exprs.lower(source, body)),
                    }))
                }
                tune_ast::nodes::StructMember::SequenceMaterializer(materializer) => {
                    Some(StructMember::SequenceMaterializer(SequenceMaterializer {
                        id,
                        param_name: materializer.param_name(source).map(str::to_owned),
                        span: materializer.syntax().span,
                        doc,
                        body: materializer
                            .body_expr()
                            .map(|body| exprs.lower(source, body)),
                    }))
                }
                tune_ast::nodes::StructMember::IndexAccess(access) => {
                    let shapes = access
                        .shapes()
                        .into_iter()
                        .map(|shape| lower_shape(source, shape))
                        .collect::<Vec<_>>();
                    Some(StructMember::IndexAccess(IndexAccess {
                        id,
                        index_param_id: member_id(0, MemberKind::Param)?,
                        receiver_name: access.receiver_name(source).map(str::to_owned),
                        index_param_name: access.index_param_name(source).map(str::to_owned),
                        span: access.syntax().span,
                        doc,
                        index_shape: shapes.first().cloned(),
                        result_shape: shapes.get(1).cloned(),
                        body: access.body_expr().map(|body| exprs.lower(source, body)),
                    }))
                }
            }
        })
        .collect()
}

pub(super) fn lower_variants(
    source: &str,
    variants: Vec<tune_ast::nodes::DocumentedVariant<'_>>,
) -> Vec<Variant> {
    variants
        .into_iter()
        .enumerate()
        .filter_map(|(index, documented)| {
            Some(Variant {
                id: member_id(index, MemberKind::Variant)?,
                name: documented.variant.name(source).map(str::to_owned),
                span: documented.variant.syntax().span,
                doc: documented.doc_text(source),
                payload: documented
                    .variant
                    .payload_shapes()
                    .into_iter()
                    .map(|shape| lower_shape(source, shape))
                    .collect(),
            })
        })
        .collect()
}

fn lower_member_params(source: &str, params: Option<tune_ast::nodes::ParamList<'_>>) -> Vec<Param> {
    params
        .into_iter()
        .flat_map(|params| params.params())
        .enumerate()
        .filter_map(|(index, param)| {
            Some(Param {
                id: member_id(index, MemberKind::Param)?,
                name: param.name(source).map(str::to_owned),
                span: param.syntax().span,
                shape: param
                    .shape_annotation()
                    .map(|shape| lower_shape(source, shape)),
            })
        })
        .collect()
}

fn member_id_for_struct_member(
    index: usize,
    member: tune_ast::nodes::StructMember<'_>,
) -> Option<MemberId> {
    let kind = match member {
        tune_ast::nodes::StructMember::Field(_) => MemberKind::Field,
        tune_ast::nodes::StructMember::Callable(_) => MemberKind::Callable,
        tune_ast::nodes::StructMember::SequenceMaterializer(_) => MemberKind::SequenceMaterializer,
        tune_ast::nodes::StructMember::IndexAccess(_) => MemberKind::IndexAccess,
    };
    member_id(index, kind)
}

fn member_id(index: usize, kind: MemberKind) -> Option<MemberId> {
    Some(MemberId {
        owner: HirId(0),
        kind,
        index: u32::try_from(index).ok()?,
    })
}

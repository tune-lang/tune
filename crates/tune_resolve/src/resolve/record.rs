use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::item::{Item, ItemKind, StructMember};

use crate::facts::{CompilerFact, CompilerFactPayload, FactOwner, TagFact, TagFactArg};
use crate::scope::BindingKind;

use super::ResolvedModule;

pub(super) fn record_defined_item_facts(resolved: &mut ResolvedModule, item: &Item) {
    let Some(name) = item.name.as_deref() else {
        return;
    };

    if resolved
        .scope
        .get(name)
        .is_some_and(|binding| binding.id == item.id)
    {
        record_item_facts(resolved, item, name);
    }
}

fn record_item_facts(resolved: &mut ResolvedModule, item: &Item, name: &str) {
    resolved.facts.push(CompilerFact {
        owner: FactOwner::Item(item.id),
        payload: CompilerFactPayload::Name(name.to_owned()),
        span: item.span,
    });
    resolved.facts.push(CompilerFact {
        owner: FactOwner::Item(item.id),
        payload: CompilerFactPayload::Visibility(item.visibility),
        span: item.span,
    });

    if let Some(doc) = &item.doc {
        resolved.facts.push(CompilerFact {
            owner: FactOwner::Item(item.id),
            payload: CompilerFactPayload::Doc(doc.clone()),
            span: item.span,
        });
    }

    if !item.params.is_empty() {
        resolved.facts.push(CompilerFact {
            owner: FactOwner::Item(item.id),
            payload: CompilerFactPayload::Params(
                item.params.iter().map(|param| param.id).collect(),
            ),
            span: item.span,
        });
    }

    if !item.type_params.is_empty() {
        resolved.facts.push(CompilerFact {
            owner: FactOwner::Item(item.id),
            payload: CompilerFactPayload::TypeParams(
                item.type_params.iter().map(|param| param.id).collect(),
            ),
            span: item.span,
        });
    }

    if item.kind == ItemKind::CallableDecl
        && let Some(shape) = &item.shape
    {
        resolved.facts.push(CompilerFact {
            owner: FactOwner::Item(item.id),
            payload: CompilerFactPayload::Return(shape.clone()),
            span: item.span,
        });
    }

    if !item.fields.is_empty() {
        resolved.facts.push(CompilerFact {
            owner: FactOwner::Item(item.id),
            payload: CompilerFactPayload::Fields(
                item.fields.iter().map(|field| field.id).collect(),
            ),
            span: item.span,
        });
    }

    if !item.variants.is_empty() {
        resolved.facts.push(CompilerFact {
            owner: FactOwner::Item(item.id),
            payload: CompilerFactPayload::Variants(
                item.variants.iter().map(|variant| variant.id).collect(),
            ),
            span: item.span,
        });
    }

    for param in &item.params {
        record_param_facts(resolved, param);
    }

    for param in &item.type_params {
        record_type_param_facts(resolved, param);
    }

    for field in &item.fields {
        record_field_facts(resolved, field);
    }

    for member in &item.struct_members {
        record_struct_member_facts(resolved, member);
    }

    for variant in &item.variants {
        record_variant_facts(resolved, variant);
    }

    for tag in &item.tags {
        record_tag_fact(resolved, item, tag);
    }
}

fn record_struct_member_facts(resolved: &mut ResolvedModule, member: &StructMember) {
    match member {
        StructMember::Callable(callable) => {
            if let Some(name) = &callable.name {
                resolved.facts.push(CompilerFact {
                    owner: FactOwner::Member(callable.id),
                    payload: CompilerFactPayload::Name(name.clone()),
                    span: callable.span,
                });
            }
            if let Some(doc) = &callable.doc {
                resolved.facts.push(CompilerFact {
                    owner: FactOwner::Member(callable.id),
                    payload: CompilerFactPayload::Doc(doc.clone()),
                    span: callable.span,
                });
            }
            if !callable.params.is_empty() {
                resolved.facts.push(CompilerFact {
                    owner: FactOwner::Member(callable.id),
                    payload: CompilerFactPayload::Params(
                        callable.params.iter().map(|param| param.id).collect(),
                    ),
                    span: callable.span,
                });
            }
            if let Some(shape) = &callable.shape {
                resolved.facts.push(CompilerFact {
                    owner: FactOwner::Member(callable.id),
                    payload: CompilerFactPayload::Return(shape.clone()),
                    span: callable.span,
                });
            }
            for param in &callable.params {
                record_param_facts(resolved, param);
            }
        }
        StructMember::Field(_)
        | StructMember::SequenceMaterializer(_)
        | StructMember::IndexAccess(_) => {}
    }
}

fn record_type_param_facts(resolved: &mut ResolvedModule, param: &tune_hir::item::TypeParam) {
    if let Some(name) = &param.name {
        resolved.facts.push(CompilerFact {
            owner: FactOwner::Member(param.id),
            payload: CompilerFactPayload::Name(name.clone()),
            span: param.span,
        });
    }
}

fn record_param_facts(resolved: &mut ResolvedModule, param: &tune_hir::item::Param) {
    if let Some(name) = &param.name {
        resolved.facts.push(CompilerFact {
            owner: FactOwner::Member(param.id),
            payload: CompilerFactPayload::Name(name.clone()),
            span: param.span,
        });
    }

    if let Some(shape) = &param.shape {
        resolved.facts.push(CompilerFact {
            owner: FactOwner::Member(param.id),
            payload: CompilerFactPayload::Shape(shape.clone()),
            span: param.span,
        });
    }
}

fn record_field_facts(resolved: &mut ResolvedModule, field: &tune_hir::item::Field) {
    if let Some(name) = &field.name {
        resolved.facts.push(CompilerFact {
            owner: FactOwner::Member(field.id),
            payload: CompilerFactPayload::Name(name.clone()),
            span: field.span,
        });
    }

    if let Some(doc) = &field.doc {
        resolved.facts.push(CompilerFact {
            owner: FactOwner::Member(field.id),
            payload: CompilerFactPayload::Doc(doc.clone()),
            span: field.span,
        });
    }

    if let Some(shape) = &field.shape {
        resolved.facts.push(CompilerFact {
            owner: FactOwner::Member(field.id),
            payload: CompilerFactPayload::Shape(shape.clone()),
            span: field.span,
        });
    }
}

fn record_variant_facts(resolved: &mut ResolvedModule, variant: &tune_hir::item::Variant) {
    if let Some(name) = &variant.name {
        resolved.facts.push(CompilerFact {
            owner: FactOwner::Member(variant.id),
            payload: CompilerFactPayload::Name(name.clone()),
            span: variant.span,
        });
    }

    if let Some(doc) = &variant.doc {
        resolved.facts.push(CompilerFact {
            owner: FactOwner::Member(variant.id),
            payload: CompilerFactPayload::Doc(doc.clone()),
            span: variant.span,
        });
    }

    if !variant.payload.is_empty() {
        resolved.facts.push(CompilerFact {
            owner: FactOwner::Member(variant.id),
            payload: CompilerFactPayload::Payload(variant.payload.clone()),
            span: variant.span,
        });
    }
}

fn record_tag_fact(
    resolved: &mut ResolvedModule,
    item: &Item,
    tag: &tune_hir::item::TagApplication,
) {
    match resolved.scope.get(&tag.name) {
        Some(binding) if binding.kind == BindingKind::Tag => {
            resolved.facts.push(CompilerFact {
                owner: FactOwner::Item(item.id),
                payload: CompilerFactPayload::Tag(TagFact {
                    name: tag.name.clone(),
                    args: tag
                        .args
                        .iter()
                        .map(|arg| TagFactArg {
                            name: arg.name.clone(),
                            value: arg.value.id,
                            span: arg.value.span,
                        })
                        .collect(),
                }),
                span: tag.span,
            });
        }
        Some(binding) => {
            let span = tag.span.or(item.span).unwrap_or_else(Span::synthetic);
            let mut builder = Diagnostic::error(
                codes::UNRESOLVED_NAME,
                format!("`{}` is not a tag", tag.name),
                span,
                "this application expects a tag declaration",
            );

            if let Some(binding_span) = binding.span {
                builder = builder.with_secondary(binding_span, "this name is declared here");
            }

            resolved.diagnostics.push(builder.build());
        }
        None => {
            let span = tag.span.or(item.span).unwrap_or_else(Span::synthetic);
            resolved.diagnostics.push(
                Diagnostic::error(
                    codes::UNRESOLVED_NAME,
                    format!("unresolved tag `{}`", tag.name),
                    span,
                    "this tag application has no matching tag declaration",
                )
                .build(),
            );
        }
    }
}

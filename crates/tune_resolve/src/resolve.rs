use std::collections::HashMap;

use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::item::{Item, ItemKind};
use tune_hir::module::Module;

use crate::facts::{CompilerFact, CompilerFactPayload, FactOwner};
use crate::scope::{Binding, BindingKind, Scope};

#[derive(Default)]
pub struct ResolvedModule {
    pub scope: Scope,
    pub facts: Vec<CompilerFact>,
    pub diagnostics: Vec<Diagnostic>,
}

#[must_use]
pub fn resolve_module(module: &Module) -> ResolvedModule {
    let mut resolved = ResolvedModule::default();

    for item in &module.items {
        define_item(&mut resolved, item);
    }

    for item in &module.items {
        validate_member_names(&mut resolved, item);
    }

    for item in &module.items {
        record_defined_item_facts(&mut resolved, item);
    }

    resolved
}

fn define_item(resolved: &mut ResolvedModule, item: &Item) {
    let Some(name) = item.name.as_deref() else {
        return;
    };

    let binding = Binding {
        id: item.id,
        kind: binding_kind(item.kind),
        span: item.span,
    };

    match resolved.scope.define(name, binding) {
        Ok(()) => {}
        Err(duplicate) => {
            let span = item.span.unwrap_or_else(Span::synthetic);
            let mut builder = Diagnostic::error(
                codes::DUPLICATE_NAME,
                format!("duplicate declaration `{}`", duplicate.name),
                span,
                "this declaration repeats an existing name",
            );

            if let Some(existing_span) = duplicate.existing.span {
                builder = builder.with_secondary(existing_span, "first declaration is here");
            }

            resolved.diagnostics.push(builder.build());
        }
    }
}

fn validate_member_names(resolved: &mut ResolvedModule, item: &Item) {
    validate_named_members(
        resolved,
        item,
        item.params
            .iter()
            .filter_map(|param| Some((param.name.as_deref()?, param.span))),
        "parameter",
    );
    validate_named_members(
        resolved,
        item,
        item.fields
            .iter()
            .filter_map(|field| Some((field.name.as_deref()?, field.span))),
        "field",
    );
    validate_named_members(
        resolved,
        item,
        item.variants
            .iter()
            .filter_map(|variant| Some((variant.name.as_deref()?, variant.span))),
        "variant",
    );
}

fn validate_named_members<'name>(
    resolved: &mut ResolvedModule,
    item: &Item,
    members: impl IntoIterator<Item = (&'name str, Option<Span>)>,
    kind: &str,
) {
    let mut seen = HashMap::new();
    for (name, span) in members {
        if let Some(existing_span) = seen.insert(name.to_owned(), span) {
            let span = span.or(item.span).unwrap_or_else(Span::synthetic);
            let mut builder = Diagnostic::error(
                codes::DUPLICATE_NAME,
                format!("duplicate {kind} `{name}`"),
                span,
                format!("this {kind} repeats an existing name"),
            );

            if let Some(existing_span) = existing_span {
                builder = builder.with_secondary(existing_span, "first declaration is here");
            }

            resolved.diagnostics.push(builder.build());
        }
    }
}

fn record_defined_item_facts(resolved: &mut ResolvedModule, item: &Item) {
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

    for field in &item.fields {
        record_field_facts(resolved, field);
    }

    for variant in &item.variants {
        record_variant_facts(resolved, variant);
    }

    for tag in &item.tags {
        record_tag_fact(resolved, item, tag);
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
                payload: CompilerFactPayload::Tag(tag.name.clone()),
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

const fn binding_kind(kind: ItemKind) -> BindingKind {
    match kind {
        ItemKind::Let => BindingKind::Value,
        ItemKind::CallableDecl => BindingKind::StableCallableDecl,
        ItemKind::Struct => BindingKind::Struct,
        ItemKind::Enum => BindingKind::Enum,
        ItemKind::Tag => BindingKind::Tag,
        ItemKind::Import => BindingKind::Module,
    }
}

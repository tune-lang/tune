use std::collections::HashMap;

use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::item::{Item, StructMember};

use super::ResolvedModule;

pub(super) fn validate_member_names(resolved: &mut ResolvedModule, item: &Item) {
    validate_named_members(
        resolved,
        item,
        item.type_params
            .iter()
            .filter_map(|param| Some((param.name.as_deref()?, param.span))),
        "type parameter",
    );
    validate_named_members(
        resolved,
        item,
        item.params
            .iter()
            .filter_map(|param| Some((param.name.as_deref()?, param.span))),
        "parameter",
    );
    validate_named_members(resolved, item, named_struct_value_members(item), "field");
    validate_named_members(
        resolved,
        item,
        item.variants
            .iter()
            .filter_map(|variant| Some((variant.name.as_deref()?, variant.span))),
        "variant",
    );
}

fn named_struct_value_members(item: &Item) -> Vec<(&str, Option<Span>)> {
    if item.struct_members.is_empty() {
        return item
            .fields
            .iter()
            .filter_map(|field| Some((field.name.as_deref()?, field.span)))
            .collect();
    }

    item.struct_members
        .iter()
        .filter_map(|member| match member {
            StructMember::Field(field) => Some((field.name.as_deref()?, field.span)),
            StructMember::Callable(callable) => Some((callable.name.as_deref()?, callable.span)),
            StructMember::SequenceMaterializer(_) | StructMember::IndexAccess(_) => None,
        })
        .collect()
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

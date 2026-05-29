use std::collections::HashMap;

use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::item::{ImportSelector, Item, StructMember};

use super::ResolvedModule;
use super::reserved;

pub(super) fn validate_member_names(resolved: &mut ResolvedModule, item: &Item) {
    if let Some(name) = &item.name {
        validate_user_name(resolved, name, item.span, "declaration");
    }

    if let Some(import) = &item.import {
        validate_import_selector(resolved, &import.selector, item.span);
    }

    for tag in &item.tags {
        for arg in &tag.args {
            if let Some(name) = &arg.name {
                validate_user_name(resolved, name, arg.value.span, "tag argument");
            }
        }
    }

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
    for member in &item.struct_members {
        match member {
            StructMember::Callable(callable) => validate_named_members(
                resolved,
                item,
                callable
                    .params
                    .iter()
                    .filter_map(|param| Some((param.name.as_deref()?, param.span))),
                "parameter",
            ),
            StructMember::SequenceMaterializer(materializer) => {
                if let Some(name) = &materializer.param_name {
                    validate_user_name(resolved, name, materializer.span, "materializer parameter");
                }
            }
            StructMember::IndexAccess(access) => {
                if let Some(name) = &access.index_param_name {
                    validate_user_name(resolved, name, access.span, "index parameter");
                }
            }
            StructMember::Field(_) => {}
        }
    }
    validate_named_members(
        resolved,
        item,
        item.variants
            .iter()
            .filter_map(|variant| Some((variant.name.as_deref()?, variant.span))),
        "variant",
    );
}

fn validate_import_selector(
    resolved: &mut ResolvedModule,
    selector: &ImportSelector,
    span: Option<Span>,
) {
    match selector {
        ImportSelector::Module => {}
        ImportSelector::Member(name) => validate_user_name(resolved, name, span, "import selector"),
        ImportSelector::Members(names) => {
            for name in names {
                validate_user_name(resolved, name, span, "import selector");
            }
        }
    }
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
        validate_user_name(resolved, name, span, kind);

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

fn validate_user_name(resolved: &mut ResolvedModule, name: &str, span: Option<Span>, kind: &str) {
    if name.starts_with("__") {
        resolved.diagnostics.push(
            Diagnostic::error(
                codes::COMPILER_RESERVED_NAME,
                format!("compiler-reserved {kind} name `{name}`"),
                span.unwrap_or_else(Span::synthetic),
                "`__` names are owned by compiler facts and generated helpers",
            )
            .with_help("rename this symbol without the leading `__` prefix")
            .build(),
        );
    }

    if reserved::is_stdcore_name(name) {
        resolved.diagnostics.push(
            Diagnostic::error(
                codes::COMPILER_RESERVED_NAME,
                format!("stdcore-reserved {kind} name `{name}`"),
                span.unwrap_or_else(Span::synthetic),
                "this name is owned by Tune's auto-included core world",
            )
            .with_help("choose a project-local name that does not shadow stdcore meaning")
            .build(),
        );
    }
}

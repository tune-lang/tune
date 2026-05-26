use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::item::{Item, ItemKind, Visibility};
use tune_hir::module::Module;

use crate::facts::{CompilerFact, CompilerFactKind};
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
        owner: item.id,
        kind: CompilerFactKind::Name,
        value: name.to_owned(),
        span: item.span,
    });
    resolved.facts.push(CompilerFact {
        owner: item.id,
        kind: CompilerFactKind::Visibility,
        value: visibility_name(item.visibility).to_owned(),
        span: item.span,
    });

    if let Some(doc) = &item.doc {
        resolved.facts.push(CompilerFact {
            owner: item.id,
            kind: CompilerFactKind::Doc,
            value: doc.clone(),
            span: item.span,
        });
    }

    for tag in &item.tags {
        record_tag_fact(resolved, item, tag);
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
                owner: item.id,
                kind: CompilerFactKind::Tag,
                value: tag.name.clone(),
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

const fn visibility_name(visibility: Visibility) -> &'static str {
    match visibility {
        Visibility::Private => "private",
        Visibility::Public => "public",
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

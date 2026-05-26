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
        Ok(()) => record_item_facts(resolved, item, name),
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

use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::item::{Item, ItemKind};
use tune_hir::module::Module;

use crate::scope::{Binding, BindingKind, Scope};

#[derive(Default)]
pub struct ResolvedModule {
    pub scope: Scope,
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

    if let Err(duplicate) = resolved.scope.define(name, binding) {
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

mod body;
mod record;
mod reserved;
mod validate;

use std::collections::HashMap;

use crate::facts::CompilerFact;
use crate::locals::{LocalBinding, NameRef, VariantPatternRef};
use crate::prelude::{Prelude, VariantId};
use crate::scope::{Binding, BindingKind, Scope};
use tune_diagnostics::{Diagnostic, Span};
use tune_hir::item::{Item, ItemKind};
use tune_hir::module::Module;

#[derive(Default)]
pub struct ResolvedModule {
    pub prelude: Prelude,
    pub scope: Scope,
    pub variants: VariantScope,
    pub facts: Vec<CompilerFact>,
    pub locals: Vec<LocalBinding>,
    pub name_refs: Vec<NameRef>,
    pub variant_pattern_refs: Vec<VariantPatternRef>,
    pub diagnostics: Vec<Diagnostic>,
}

#[derive(Default)]
pub struct VariantScope {
    bindings: HashMap<String, VariantId>,
    ambiguous: HashMap<String, Option<Span>>,
}

impl VariantScope {
    pub fn define(&mut self, name: impl Into<String>, variant: VariantId, span: Option<Span>) {
        let name = name.into();
        if self.ambiguous.contains_key(&name) {
            return;
        }

        if self.bindings.insert(name.clone(), variant).is_some() {
            self.bindings.remove(&name);
            self.ambiguous.insert(name, span);
        }
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<VariantId> {
        self.bindings.get(name).copied()
    }

    #[must_use]
    pub fn is_ambiguous(&self, name: &str) -> bool {
        self.ambiguous.contains_key(name)
    }
}

#[must_use]
pub fn resolve_module(module: &Module) -> ResolvedModule {
    let mut resolved = ResolvedModule::default();

    for item in &module.items {
        define_item(&mut resolved, item);
    }

    for item in &module.items {
        validate::validate_member_names(&mut resolved, item);
    }

    for item in &module.items {
        body::resolve_item_body(&mut resolved, item, &module.items);
    }

    for item in &module.items {
        record::record_defined_item_facts(&mut resolved, item);
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
        generic_arity: item.type_params.len(),
    };

    let _shadowed = resolved.scope.define(name, binding);
}

const fn binding_kind(kind: ItemKind) -> BindingKind {
    match kind {
        ItemKind::Let => BindingKind::Value,
        ItemKind::CallableDecl => BindingKind::StableCallableDecl,
        ItemKind::Struct => BindingKind::Struct,
        ItemKind::Enum => BindingKind::Enum,
        ItemKind::Tag => BindingKind::Tag,
        ItemKind::Import => BindingKind::Module,
        ItemKind::Expr => BindingKind::Value,
    }
}

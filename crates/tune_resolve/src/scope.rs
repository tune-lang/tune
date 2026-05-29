use std::collections::HashMap;

use tune_diagnostics::Span;
use tune_hir::HirId;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
    Value,
    StableCallableDecl,
    Struct,
    Enum,
    Tag,
    Module,
    CompilerFact,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Binding {
    pub id: HirId,
    pub kind: BindingKind,
    pub span: Option<Span>,
    pub generic_arity: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DuplicateBinding {
    pub name: String,
    pub existing: Binding,
}

#[derive(Default)]
pub struct Scope {
    bindings: HashMap<String, Binding>,
}

impl Scope {
    pub fn define(
        &mut self,
        name: impl Into<String>,
        binding: Binding,
    ) -> Result<(), DuplicateBinding> {
        let name = name.into();
        if let Some(existing) = self.bindings.get(&name).copied() {
            return Err(DuplicateBinding { name, existing });
        }

        self.bindings.insert(name, binding);
        Ok(())
    }

    #[must_use]
    pub fn get(&self, name: &str) -> Option<Binding> {
        self.bindings.get(name).copied()
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.bindings.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.bindings.is_empty()
    }
}

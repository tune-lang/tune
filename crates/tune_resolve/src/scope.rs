use std::collections::HashMap;

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

#[derive(Default)]
pub struct Scope {
    bindings: HashMap<String, BindingKind>,
}

impl Scope {
    pub fn define(&mut self, name: impl Into<String>, kind: BindingKind) {
        self.bindings.insert(name.into(), kind);
    }
}

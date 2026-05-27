#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreludeType {
    Result,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PreludeVariant {
    Ok,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariantId {
    Prelude(PreludeVariant),
    Member(tune_hir::MemberId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Prelude {
    pub result: PreludeType,
    pub ok: PreludeVariant,
    pub error: PreludeVariant,
}

impl Default for Prelude {
    fn default() -> Self {
        Self {
            result: PreludeType::Result,
            ok: PreludeVariant::Ok,
            error: PreludeVariant::Error,
        }
    }
}

impl Prelude {
    #[must_use]
    pub fn variant(self, name: &str) -> Option<PreludeVariant> {
        match name {
            "Ok" => Some(self.ok),
            "Error" => Some(self.error),
            _ => None,
        }
    }
}

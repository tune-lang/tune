use std::collections::HashMap;

use crate::SymbolId;

#[derive(Debug, Default)]
pub struct Interner {
    map: HashMap<String, SymbolId>,
    vec: Vec<String>,
}

impl Interner {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    pub fn intern(&mut self, text: &str) -> Option<SymbolId> {
        if let Some(&id) = self.map.get(text) {
            return Some(id);
        }

        let index = u32::try_from(self.vec.len()).ok()?;
        let id = SymbolId(index);
        let owned = text.to_string();
        self.vec.push(owned.clone());
        self.map.insert(owned, id);
        Some(id)
    }

    #[must_use]
    pub fn get(&self, text: &str) -> Option<SymbolId> {
        self.map.get(text).copied()
    }

    #[must_use]
    pub fn resolve(&self, id: SymbolId) -> Option<&str> {
        self.vec.get(id.0 as usize).map(String::as_str)
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.vec.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.vec.is_empty()
    }
}

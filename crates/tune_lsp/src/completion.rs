use std::collections::BTreeMap;

use tune_db::{FileId, TuneDb};
use tune_resolve::{CompilerFactPayload, FactOwner};

pub fn handle() {
    // LSP completion handler skeleton. This should query compiler facts, not infer.
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Function,
    Type,
    Value,
    Module,
    Keyword,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompletionItem {
    pub label: String,
    pub kind: CompletionKind,
    pub detail: Option<String>,
    pub documentation: Option<String>,
}

#[must_use]
pub fn items_for_file(db: &TuneDb, file: FileId) -> Vec<CompletionItem> {
    let Some(analysis) = db.analyze_file(file) else {
        return Vec::new();
    };
    let mut items = BTreeMap::new();

    for fact in &analysis.resolved.facts {
        let CompilerFactPayload::Name(name) = &fact.payload else {
            continue;
        };
        let FactOwner::Item(_) = fact.owner else {
            continue;
        };

        let owner_facts = analysis
            .resolved
            .facts
            .iter()
            .filter(|candidate| candidate.owner == fact.owner)
            .collect::<Vec<_>>();
        let item = CompletionItem {
            label: name.clone(),
            kind: completion_kind(&owner_facts),
            detail: crate::hover::hover_card(db, file, fact.owner)
                .and_then(|hover| hover.signature),
            documentation: owner_facts.iter().find_map(|fact| match &fact.payload {
                CompilerFactPayload::Doc(doc) => Some(doc.clone()),
                _ => None,
            }),
        };
        items.entry(item.label.clone()).or_insert(item);
    }

    items.into_values().collect()
}

fn completion_kind(facts: &[&tune_resolve::CompilerFact]) -> CompletionKind {
    if facts.iter().any(|fact| {
        matches!(
            fact.payload,
            CompilerFactPayload::Params(_) | CompilerFactPayload::Return(_)
        )
    }) {
        CompletionKind::Function
    } else if facts.iter().any(|fact| {
        matches!(
            fact.payload,
            CompilerFactPayload::Fields(_) | CompilerFactPayload::Variants(_)
        )
    }) {
        CompletionKind::Type
    } else if facts
        .iter()
        .any(|fact| matches!(fact.payload, CompilerFactPayload::Module(_)))
    {
        CompletionKind::Module
    } else {
        CompletionKind::Value
    }
}

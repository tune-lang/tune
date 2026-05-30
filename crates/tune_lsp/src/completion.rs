use std::collections::BTreeMap;

use tune_db::{FileId, TuneDb};
use tune_resolve::NameTarget;
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

#[must_use]
pub fn items_at(db: &TuneDb, file: FileId, position: crate::Position) -> Vec<CompletionItem> {
    let Some(offset) = crate::protocol::byte_offset(db, file, position) else {
        return Vec::new();
    };
    let Some(cursor) = db.semantic_at(file, offset) else {
        return Vec::new();
    };
    let mut items = BTreeMap::new();

    for binding in cursor.scope {
        let definition_owner = binding
            .definition
            .as_ref()
            .and_then(|definition| definition.owner);
        let item = CompletionItem {
            label: binding.name,
            kind: completion_kind_for_target(binding.target, definition_owner, db, file),
            detail: definition_owner
                .and_then(|owner| crate::hover::hover_card(db, file, owner))
                .and_then(|hover| hover.signature)
                .or_else(|| {
                    binding
                        .shape
                        .as_ref()
                        .map(crate::hover::semantic_shape_text)
                }),
            documentation: definition_owner
                .and_then(|owner| documentation_for_owner(db, file, owner)),
        };
        items.insert(item.label.clone(), item);
    }

    items.into_values().collect()
}

fn completion_kind_for_target(
    target: NameTarget,
    owner: Option<FactOwner>,
    db: &TuneDb,
    file: FileId,
) -> CompletionKind {
    match target {
        NameTarget::Param(_) | NameTarget::Local(_) | NameTarget::SelfValue => {
            CompletionKind::Value
        }
        NameTarget::Variant(_) => CompletionKind::Function,
        NameTarget::TopLevel(_) => owner
            .and_then(|owner| {
                let analysis = db.analyze_file(file)?;
                let facts = analysis
                    .resolved
                    .facts
                    .iter()
                    .filter(|fact| fact.owner == owner)
                    .collect::<Vec<_>>();
                Some(completion_kind(&facts))
            })
            .unwrap_or(CompletionKind::Value),
    }
}

fn documentation_for_owner(db: &TuneDb, file: FileId, owner: FactOwner) -> Option<String> {
    db.analyze_file(file).and_then(|analysis| {
        analysis
            .resolved
            .facts
            .iter()
            .filter(|fact| fact.owner == owner)
            .find_map(|fact| match &fact.payload {
                CompilerFactPayload::Doc(doc) => Some(doc.clone()),
                _ => None,
            })
    })
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

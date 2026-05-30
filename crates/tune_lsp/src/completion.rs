use std::collections::BTreeMap;

use tune_db::{FileId, TuneDb};
use tune_hir::item::{Item, StructMember};
use tune_resolve::NameTarget;
use tune_resolve::{CompilerFactPayload, FactOwner};
use tune_shape::{NominalShape, Shape};

pub fn handle() {
    // LSP completion handler skeleton. This should query compiler facts, not infer.
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompletionKind {
    Function,
    Method,
    Field,
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
    pub filter_text: Option<String>,
    pub sort_text: Option<String>,
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
            filter_text: None,
            sort_text: None,
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
    let Some(source) = db.source(file) else {
        return Vec::new();
    };
    if let Some(context) = member_completion_context(&source.text, offset) {
        return member_items_at(db, file, context);
    }
    let Some(cursor) = db.semantic_at(file, offset) else {
        return Vec::new();
    };
    let mut items = BTreeMap::new();
    let prefix = identifier_prefix(&source.text, offset);

    for binding in cursor.scope {
        if !matches_prefix(&binding.name, prefix) {
            continue;
        }
        let definition_owner = binding
            .definition
            .as_ref()
            .and_then(|definition| definition.owner);
        let kind = completion_kind_for_target(binding.target, definition_owner, db, file);
        let rank = rank_completion(binding.target, kind);
        let item = CompletionItem {
            label: binding.name,
            kind,
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
            filter_text: None,
            sort_text: Some(format!("{rank:02}")),
        };
        items.insert((rank, item.label.clone()), item);
    }

    items.into_values().collect()
}

fn member_items_at(
    db: &TuneDb,
    file: FileId,
    context: MemberCompletionContext<'_>,
) -> Vec<CompletionItem> {
    let Some(base_offset) = context
        .base_end
        .checked_sub(1)
        .and_then(|offset| u32::try_from(offset).ok())
        .map(tune_diagnostics::ByteOffset::new)
    else {
        return Vec::new();
    };
    let Some(base_shape) = db
        .semantic_at(file, base_offset)
        .and_then(|cursor| cursor.expr)
        .and_then(|expr| expr.shape)
    else {
        return Vec::new();
    };
    let Some(analysis) = db.analyze_file(file) else {
        return Vec::new();
    };
    let Some(item) = nominal_item(&analysis.module.items, &base_shape) else {
        return Vec::new();
    };
    let mut items = BTreeMap::new();
    for member in &item.struct_members {
        match member {
            StructMember::Field(field) => {
                let Some(name) = field.name.clone() else {
                    continue;
                };
                if !matches_prefix(&name, context.prefix) {
                    continue;
                }
                let item = CompletionItem {
                    label: name.clone(),
                    kind: CompletionKind::Field,
                    detail: field.shape.as_ref().map(crate::hover::surface_shape_text),
                    documentation: field.doc.clone(),
                    filter_text: None,
                    sort_text: Some(format!("00{name}")),
                };
                items.insert((0_u8, name), item);
            }
            StructMember::Callable(callable) => {
                let Some(name) = callable.name.clone() else {
                    continue;
                };
                if !matches_prefix(&name, context.prefix) {
                    continue;
                }
                let item = CompletionItem {
                    label: name.clone(),
                    kind: CompletionKind::Method,
                    detail: Some(callable_signature(callable)),
                    documentation: callable.doc.clone(),
                    filter_text: None,
                    sort_text: Some(format!("01{name}")),
                };
                items.insert((1_u8, name), item);
            }
            StructMember::SequenceMaterializer(_) | StructMember::IndexAccess(_) => {}
        }
    }
    items.into_values().collect()
}

fn nominal_item<'a>(items: &'a [Item], shape: &Shape) -> Option<&'a Item> {
    let nominal = shape.nominal()?;
    items
        .iter()
        .find(|item| nominal_matches_item(nominal, item))
}

fn nominal_matches_item(nominal: &NominalShape, item: &Item) -> bool {
    if let Some(id) = nominal.id {
        return item.id == id;
    }
    item.name.as_deref() == Some(nominal.name.as_str())
}

fn callable_signature(callable: &tune_hir::item::CallableMember) -> String {
    let name = callable.name.as_deref().unwrap_or("_");
    let params = callable
        .params
        .iter()
        .map(|param| {
            let name = param.name.as_deref().unwrap_or("_");
            param.shape.as_ref().map_or_else(
                || name.to_owned(),
                |shape| format!("{name}: {}", crate::hover::surface_shape_text(shape)),
            )
        })
        .collect::<Vec<_>>()
        .join(", ");
    callable.shape.as_ref().map_or_else(
        || format!("{name}({params})"),
        |shape| {
            format!(
                "{name}({params}): {}",
                crate::hover::surface_shape_text(shape)
            )
        },
    )
}

#[derive(Debug, Clone, Copy)]
struct MemberCompletionContext<'a> {
    base_end: usize,
    prefix: &'a str,
}

fn member_completion_context(
    source: &str,
    offset: tune_diagnostics::ByteOffset,
) -> Option<MemberCompletionContext<'_>> {
    let member_end = usize::try_from(offset.get()).ok()?.min(source.len());
    let member_start = scan_identifier_start(source, member_end).unwrap_or(member_end);
    let prefix = source.get(member_start..member_end)?;
    let (dot_start, dot) = previous_non_whitespace_char(source, member_start)?;
    if dot != '.' {
        return None;
    }
    let mut base_end = dot_start;
    base_end = previous_non_whitespace(source, base_end).unwrap_or(base_end);
    if scan_identifier_start(source, base_end)? == base_end {
        return None;
    }
    Some(MemberCompletionContext { base_end, prefix })
}

fn previous_non_whitespace(source: &str, mut index: usize) -> Option<usize> {
    while let Some((start, character)) = previous_char(source, index) {
        if !character.is_whitespace() {
            return Some(start + character.len_utf8());
        }
        index = start;
    }
    None
}

fn previous_non_whitespace_char(source: &str, mut index: usize) -> Option<(usize, char)> {
    while let Some((start, character)) = previous_char(source, index) {
        if !character.is_whitespace() {
            return Some((start, character));
        }
        index = start;
    }
    None
}

fn previous_char(source: &str, index: usize) -> Option<(usize, char)> {
    source.get(..index)?.char_indices().next_back()
}

fn scan_identifier_start(source: &str, end: usize) -> Option<usize> {
    let mut start = end;
    while let Some((previous, character)) = previous_char(source, start) {
        if !is_identifier_continue(character) {
            break;
        }
        start = previous;
    }
    if start == end || !source.is_char_boundary(start) {
        return None;
    }
    Some(start)
}

fn identifier_prefix(source: &str, offset: tune_diagnostics::ByteOffset) -> &str {
    let end = usize::try_from(offset.get()).unwrap_or(0).min(source.len());
    let Some(start) = scan_identifier_start(source, end) else {
        return "";
    };
    source.get(start..end).unwrap_or("")
}

fn is_identifier_continue(character: char) -> bool {
    character == '_' || character.is_ascii_alphanumeric()
}

fn matches_prefix(label: &str, prefix: &str) -> bool {
    prefix.is_empty() || label.starts_with(prefix)
}

fn rank_completion(target: NameTarget, kind: CompletionKind) -> u8 {
    match target {
        NameTarget::Local(_) => 0,
        NameTarget::Param(_) | NameTarget::SelfValue => 1,
        NameTarget::TopLevel(_) if kind == CompletionKind::Function => 2,
        NameTarget::TopLevel(_) => 3,
        NameTarget::Variant(_) => 4,
    }
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

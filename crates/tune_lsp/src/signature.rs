use tune_db::{FileId, TuneDb};
use tune_shape::CallTarget;

pub fn handle() {
    // LSP signature handler skeleton. This should query compiler facts, not infer.
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SignatureHelp {
    pub signature: String,
    pub active_parameter: Option<usize>,
}

#[must_use]
pub fn signature_help_at(
    db: &TuneDb,
    file: FileId,
    position: crate::Position,
) -> Option<SignatureHelp> {
    let offset = crate::protocol::byte_offset(db, file, position)?;
    let cursor = db.semantic_at(file, offset)?;
    let call = cursor.call.as_ref()?;
    let check = call.check.as_ref()?;
    let name = call_target_name(db, file, &cursor, check.target);
    let params = check
        .params
        .iter()
        .enumerate()
        .map(|(index, shape)| format!("arg{index}: {}", crate::hover::semantic_shape_text(shape)))
        .collect::<Vec<_>>()
        .join(", ");
    let ret = crate::hover::semantic_shape_text(&check.ret);

    Some(SignatureHelp {
        signature: format!("{name}({params}): {ret}"),
        active_parameter: call.active_arg,
    })
}

fn call_target_name(
    db: &TuneDb,
    file: FileId,
    cursor: &tune_db::SemanticCursor,
    target: CallTarget,
) -> String {
    match target {
        CallTarget::TopLevel(item) => db
            .analyze_file(file)
            .and_then(|analysis| {
                analysis
                    .module
                    .items
                    .iter()
                    .find(|candidate| candidate.id == item)
                    .and_then(|item| item.name.clone())
            })
            .unwrap_or_else(|| "call".to_owned()),
        CallTarget::Member(member) => fact_name(db, file, tune_resolve::FactOwner::Member(member))
            .unwrap_or_else(|| "call".to_owned()),
        CallTarget::Variant(tune_resolve::VariantId::Prelude(variant)) => match variant {
            tune_resolve::PreludeVariant::Ok => "Ok".to_owned(),
            tune_resolve::PreludeVariant::Error => "Error".to_owned(),
        },
        CallTarget::Variant(tune_resolve::VariantId::Member(member)) => {
            fact_name(db, file, tune_resolve::FactOwner::Member(member))
                .unwrap_or_else(|| "call".to_owned())
        }
        CallTarget::Bound | CallTarget::Unknown => cursor
            .reference
            .as_ref()
            .and_then(|reference| reference.definition.as_ref())
            .and_then(|definition| definition.name.clone())
            .unwrap_or_else(|| "call".to_owned()),
        CallTarget::StringLen => "len".to_owned(),
        CallTarget::TaskJoin => "join".to_owned(),
    }
}

fn fact_name(db: &TuneDb, file: FileId, owner: tune_resolve::FactOwner) -> Option<String> {
    db.analyze_file(file).and_then(|analysis| {
        analysis
            .resolved
            .facts
            .iter()
            .filter(|fact| fact.owner == owner)
            .find_map(|fact| match &fact.payload {
                tune_resolve::CompilerFactPayload::Name(name) => Some(name.clone()),
                _ => None,
            })
    })
}

use tune_db::{FileId, TuneDb};
use tune_diagnostics::Span;
use tune_resolve::{CompilerFactPayload, FactOwner, NameTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SemanticTokenKind {
    Function,
    Type,
    Variable,
    Parameter,
    Property,
    EnumMember,
    Module,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SemanticToken {
    pub range: crate::Range,
    pub kind: SemanticTokenKind,
}

#[must_use]
pub fn tokens_for_file(db: &TuneDb, file: FileId) -> Vec<SemanticToken> {
    let Some(analysis) = db.analyze_file(file) else {
        return Vec::new();
    };
    let mut tokens = Vec::new();

    for fact in &analysis.resolved.facts {
        let CompilerFactPayload::Name(_) = &fact.payload else {
            continue;
        };
        let Some(span) = fact.span else {
            continue;
        };
        push_token(
            db,
            &mut tokens,
            span,
            fact_owner_kind(&analysis, fact.owner),
        );
    }

    for reference in &analysis.resolved.name_refs {
        let Some(span) = reference.span else {
            continue;
        };
        push_token(db, &mut tokens, span, name_target_kind(reference.target));
    }

    tokens
}

fn push_token(db: &TuneDb, tokens: &mut Vec<SemanticToken>, span: Span, kind: SemanticTokenKind) {
    if let Some(range) = crate::protocol::range(db, span) {
        tokens.push(SemanticToken { range, kind });
    }
}

fn fact_owner_kind(analysis: &tune_db::ModuleAnalysis, owner: FactOwner) -> SemanticTokenKind {
    let facts = analysis
        .resolved
        .facts
        .iter()
        .filter(|fact| fact.owner == owner)
        .collect::<Vec<_>>();
    if matches!(owner, FactOwner::Member(_)) {
        return SemanticTokenKind::Parameter;
    }
    if facts.iter().any(|fact| {
        matches!(
            fact.payload,
            CompilerFactPayload::Fields(_) | CompilerFactPayload::Variants(_)
        )
    }) {
        SemanticTokenKind::Type
    } else if facts.iter().any(|fact| {
        matches!(
            fact.payload,
            CompilerFactPayload::Params(_) | CompilerFactPayload::Return(_)
        )
    }) {
        SemanticTokenKind::Function
    } else if facts
        .iter()
        .any(|fact| matches!(fact.payload, CompilerFactPayload::Module(_)))
    {
        SemanticTokenKind::Module
    } else {
        SemanticTokenKind::Variable
    }
}

fn name_target_kind(target: NameTarget) -> SemanticTokenKind {
    match target {
        NameTarget::TopLevel(_) => SemanticTokenKind::Variable,
        NameTarget::Variant(_) => SemanticTokenKind::EnumMember,
        NameTarget::Param(_) => SemanticTokenKind::Parameter,
        NameTarget::Local(_) | NameTarget::SelfValue => SemanticTokenKind::Variable,
    }
}

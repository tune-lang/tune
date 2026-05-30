use tune_db::{FileId, TuneDb};
use tune_diagnostics::Span;
use tune_hir::item::{ItemKind, StructMember};
use tune_hir::{HirId, MemberId};
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
        push_token(
            db,
            &mut tokens,
            span,
            name_target_kind(&analysis, reference.target),
        );
    }

    tokens.sort_by_key(|token| {
        (
            token.range.start.line,
            token.range.start.character,
            token.range.end.line,
            token.range.end.character,
        )
    });
    tokens.dedup_by_key(|token| {
        (
            token.range.start.line,
            token.range.start.character,
            token.range.end.line,
            token.range.end.character,
            token.kind,
        )
    });
    tokens
}

fn push_token(db: &TuneDb, tokens: &mut Vec<SemanticToken>, span: Span, kind: SemanticTokenKind) {
    if let Some(range) = crate::protocol::range(db, span) {
        tokens.push(SemanticToken { range, kind });
    }
}

fn fact_owner_kind(analysis: &tune_db::ModuleAnalysis, owner: FactOwner) -> SemanticTokenKind {
    if let FactOwner::Member(member) = owner {
        return member_token_kind(analysis, member);
    }
    let facts = analysis
        .resolved
        .facts
        .iter()
        .filter(|fact| fact.owner == owner)
        .collect::<Vec<_>>();
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

fn name_target_kind(analysis: &tune_db::ModuleAnalysis, target: NameTarget) -> SemanticTokenKind {
    match target {
        NameTarget::TopLevel(id) => top_level_token_kind(analysis, id),
        NameTarget::Variant(_) => SemanticTokenKind::EnumMember,
        NameTarget::Param(_) => SemanticTokenKind::Parameter,
        NameTarget::Local(_) | NameTarget::SelfValue => SemanticTokenKind::Variable,
    }
}

fn top_level_token_kind(analysis: &tune_db::ModuleAnalysis, id: HirId) -> SemanticTokenKind {
    analysis
        .module
        .items
        .iter()
        .find(|item| item.id == id)
        .map_or(SemanticTokenKind::Variable, |item| match item.kind {
            ItemKind::CallableDecl => SemanticTokenKind::Function,
            ItemKind::Struct | ItemKind::Enum | ItemKind::Tag => SemanticTokenKind::Type,
            ItemKind::Import if item.external.is_some() => SemanticTokenKind::Module,
            ItemKind::Import => SemanticTokenKind::Variable,
            ItemKind::Let => SemanticTokenKind::Variable,
            ItemKind::Expr => SemanticTokenKind::Variable,
        })
}

fn member_token_kind(analysis: &tune_db::ModuleAnalysis, id: MemberId) -> SemanticTokenKind {
    for item in &analysis.module.items {
        if item.params.iter().any(|param| param.id == id)
            || item.type_params.iter().any(|param| param.id == id)
        {
            return SemanticTokenKind::Parameter;
        }
        if item.fields.iter().any(|field| field.id == id) {
            return SemanticTokenKind::Property;
        }
        if item.variants.iter().any(|variant| variant.id == id) {
            return SemanticTokenKind::EnumMember;
        }
        for member in &item.struct_members {
            match member {
                StructMember::Field(field) if field.id == id => {
                    return SemanticTokenKind::Property;
                }
                StructMember::Callable(callable) if callable.id == id => {
                    return SemanticTokenKind::Function;
                }
                StructMember::Callable(callable)
                    if callable.params.iter().any(|param| param.id == id) =>
                {
                    return SemanticTokenKind::Parameter;
                }
                StructMember::SequenceMaterializer(member) if member.id == id => {
                    return SemanticTokenKind::Function;
                }
                StructMember::IndexAccess(member)
                    if member.id == id || member.index_param_id == id =>
                {
                    return SemanticTokenKind::Function;
                }
                _ => {}
            }
        }
    }
    SemanticTokenKind::Variable
}

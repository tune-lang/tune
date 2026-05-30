use tune_db::{FileId, TuneDb};
use tune_hir::MemberId;
use tune_hir::shape::{ShapeExpr, ShapeExprKind, StructuralShapeRequirementKind};
use tune_resolve::{CompilerFact, CompilerFactPayload, FactOwner};

pub fn handle() {
    // LSP hover handler skeleton. This should query compiler facts, not infer.
}

#[must_use]
pub fn facts_for_owner(db: &TuneDb, file: FileId, owner: FactOwner) -> Vec<CompilerFact> {
    db.analyze_file(file).map_or_else(Vec::new, |analysis| {
        analysis
            .resolved
            .facts
            .into_iter()
            .filter(|fact| fact.owner == owner)
            .collect()
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HoverCard {
    pub documentation: Option<String>,
    pub signature: Option<String>,
    pub facts: Vec<String>,
}

impl HoverCard {
    #[must_use]
    pub fn markdown(&self) -> String {
        let mut out = String::new();
        if let Some(documentation) = &self.documentation {
            out.push_str(documentation);
        }
        if let Some(signature) = &self.signature {
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str("```tn\n");
            out.push_str(signature);
            out.push_str("\n```");
        }
        if !self.facts.is_empty() {
            if !out.is_empty() {
                out.push_str("\n\n");
            }
            out.push_str("compiler facts:");
            for fact in &self.facts {
                out.push_str("\n- ");
                out.push_str(fact);
            }
        }
        out
    }
}

#[must_use]
pub fn hover_card(db: &TuneDb, file: FileId, owner: FactOwner) -> Option<HoverCard> {
    let analysis = db.analyze_file(file)?;
    let owner_facts = analysis
        .resolved
        .facts
        .iter()
        .filter(|fact| fact.owner == owner)
        .collect::<Vec<_>>();
    if owner_facts.is_empty() {
        return None;
    }

    let documentation = owner_facts.iter().find_map(|fact| match &fact.payload {
        CompilerFactPayload::Doc(doc) => Some(doc.clone()),
        _ => None,
    });
    let signature = signature_for_owner(&analysis.resolved.facts, owner, &owner_facts);
    let facts = owner_facts
        .iter()
        .filter_map(|fact| fact_summary(&fact.payload))
        .collect();

    Some(HoverCard {
        documentation,
        signature,
        facts,
    })
}

fn signature_for_owner(
    all_facts: &[CompilerFact],
    owner: FactOwner,
    owner_facts: &[&CompilerFact],
) -> Option<String> {
    let name = owner_facts.iter().find_map(|fact| match &fact.payload {
        CompilerFactPayload::Name(name) => Some(name.as_str()),
        _ => None,
    })?;
    let params = owner_facts.iter().find_map(|fact| match &fact.payload {
        CompilerFactPayload::Params(params) => Some(params.as_slice()),
        _ => None,
    });
    let ret = owner_facts.iter().find_map(|fact| match &fact.payload {
        CompilerFactPayload::Return(shape) => Some(shape_text(shape)),
        _ => None,
    });
    let shape = owner_facts.iter().find_map(|fact| match &fact.payload {
        CompilerFactPayload::Shape(shape) => Some(shape_text(shape)),
        _ => None,
    });

    if let Some(params) = params {
        let rendered_params = params
            .iter()
            .map(|param| member_signature(all_facts, *param))
            .collect::<Vec<_>>()
            .join(", ");
        let ret = ret.unwrap_or_else(|| "_".to_owned());
        Some(format!("let {name}({rendered_params}): {ret}"))
    } else if let Some(shape) = shape {
        Some(format!("{name}: {shape}"))
    } else {
        match owner {
            FactOwner::Item(_) => Some(name.to_owned()),
            FactOwner::Member(_) => Some(name.to_owned()),
        }
    }
}

fn member_signature(all_facts: &[CompilerFact], member: MemberId) -> String {
    let facts = all_facts
        .iter()
        .filter(|fact| fact.owner == FactOwner::Member(member))
        .collect::<Vec<_>>();
    let name = facts.iter().find_map(|fact| match &fact.payload {
        CompilerFactPayload::Name(name) => Some(name.as_str()),
        _ => None,
    });
    let shape = facts.iter().find_map(|fact| match &fact.payload {
        CompilerFactPayload::Shape(shape) => Some(shape_text(shape)),
        _ => None,
    });

    match (name, shape) {
        (Some(name), Some(shape)) => format!("{name}: {shape}"),
        (Some(name), None) => name.to_owned(),
        (None, Some(shape)) => format!("_: {shape}"),
        (None, None) => "_".to_owned(),
    }
}

fn fact_summary(payload: &CompilerFactPayload) -> Option<String> {
    match payload {
        CompilerFactPayload::Doc(_) | CompilerFactPayload::Name(_) => None,
        CompilerFactPayload::Return(shape) => Some(format!("returns {}", shape_text(shape))),
        CompilerFactPayload::Shape(shape) => Some(format!("shape {}", shape_text(shape))),
        CompilerFactPayload::Module(module) => Some(format!("module {module}")),
        CompilerFactPayload::Visibility(visibility) => Some(format!("visibility {visibility:?}")),
        CompilerFactPayload::Import(import) => Some(format!("import {import:?}")),
        CompilerFactPayload::Tag(tag) => Some(format!("tag {}", tag.name)),
        CompilerFactPayload::TypeParams(params) => Some(format!("type params {}", params.len())),
        CompilerFactPayload::Params(params) => Some(format!("params {}", params.len())),
        CompilerFactPayload::Fields(fields) => Some(format!("fields {}", fields.len())),
        CompilerFactPayload::Variants(variants) => Some(format!("variants {}", variants.len())),
        CompilerFactPayload::Payload(payloads) => Some(format!("payload {}", payloads.len())),
    }
}

fn shape_text(shape: &ShapeExpr) -> String {
    match &shape.kind {
        ShapeExprKind::Missing => "_".to_owned(),
        ShapeExprKind::Named(name) => name.clone(),
        ShapeExprKind::Generic { name, args } => {
            let args = args.iter().map(shape_text).collect::<Vec<_>>().join(", ");
            format!("{name}<{args}>")
        }
        ShapeExprKind::Sequence(inner) => format!("[{}]", shape_text(inner)),
        ShapeExprKind::Tuple(items) => {
            let items = items.iter().map(shape_text).collect::<Vec<_>>().join(", ");
            format!("({items})")
        }
        ShapeExprKind::Optional(inner) => format!("{}?", shape_text(inner)),
        ShapeExprKind::Union(items) => items.iter().map(shape_text).collect::<Vec<_>>().join(" | "),
        ShapeExprKind::Structural(requirements) => {
            let requirements = requirements
                .iter()
                .map(|requirement| match &requirement.kind {
                    StructuralShapeRequirementKind::Field { shape } => shape.as_ref().map_or_else(
                        || requirement.name.clone(),
                        |shape| format!("{}: {}", requirement.name, shape_text(shape)),
                    ),
                    StructuralShapeRequirementKind::Callable { params, ret } => {
                        let params = params.iter().map(shape_text).collect::<Vec<_>>().join(", ");
                        let ret = ret.as_ref().map_or_else(|| "_".to_owned(), shape_text);
                        format!("{}({params}): {ret}", requirement.name)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {requirements} }}")
        }
        ShapeExprKind::Callable { params, ret } => {
            let params = params.iter().map(shape_text).collect::<Vec<_>>().join(", ");
            format!("_({params}): {}", shape_text(ret))
        }
    }
}

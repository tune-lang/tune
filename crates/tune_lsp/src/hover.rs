use tune_db::{FileId, TuneDb};
use tune_diagnostics::{ByteOffset, Span};
use tune_hir::MemberId;
use tune_hir::item::{Item, StructMember};
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

#[must_use]
pub fn hover_card_at(db: &TuneDb, file: FileId, position: crate::Position) -> Option<HoverCard> {
    let offset = crate::protocol::byte_offset(db, file, position)?;
    let owner = owner_at_offset(db, file, offset)?;
    hover_card(db, file, owner)
}

#[must_use]
pub fn owner_at_offset(db: &TuneDb, file: FileId, offset: ByteOffset) -> Option<FactOwner> {
    let analysis = db.analyze_file(file)?;
    let mut best = None;
    for item in &analysis.module.items {
        consider_span(&mut best, item.span, FactOwner::Item(item.id), offset);
        for (span, owner) in member_spans(item) {
            consider_span(&mut best, span, owner, offset);
        }
    }
    best.map(|(_, owner)| owner)
}

fn consider_span(
    best: &mut Option<(u32, FactOwner)>,
    span: Option<Span>,
    owner: FactOwner,
    offset: ByteOffset,
) {
    let Some(span) = span else {
        return;
    };
    if !span.contains(offset) {
        return;
    }
    let len = span.len();
    if best.is_none_or(|(best_len, _)| len < best_len) {
        *best = Some((len, owner));
    }
}

fn member_spans(item: &Item) -> Vec<(Option<Span>, FactOwner)> {
    let mut spans = Vec::new();
    spans.extend(
        item.type_params
            .iter()
            .map(|param| (param.span, FactOwner::Member(param.id))),
    );
    spans.extend(
        item.params
            .iter()
            .map(|param| (param.span, FactOwner::Member(param.id))),
    );
    spans.extend(
        item.fields
            .iter()
            .map(|field| (field.span, FactOwner::Member(field.id))),
    );
    spans.extend(
        item.variants
            .iter()
            .map(|variant| (variant.span, FactOwner::Member(variant.id))),
    );
    for member in &item.struct_members {
        match member {
            StructMember::Field(field) => spans.push((field.span, FactOwner::Member(field.id))),
            StructMember::Callable(callable) => {
                spans.push((callable.span, FactOwner::Member(callable.id)));
                spans.extend(
                    callable
                        .params
                        .iter()
                        .map(|param| (param.span, FactOwner::Member(param.id))),
                );
            }
            StructMember::SequenceMaterializer(materializer) => {
                spans.push((materializer.span, FactOwner::Member(materializer.id)));
            }
            StructMember::IndexAccess(access) => {
                spans.push((access.span, FactOwner::Member(access.id)));
                spans.push((access.span, FactOwner::Member(access.index_param_id)));
            }
        }
    }
    spans
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

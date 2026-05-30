use tune_hir::HirId;
use tune_hir::item::Visibility;
use tune_shape::Shape;

#[derive(Debug, Clone)]
pub enum DeclFact {
    Name(String),
    Doc(String),
    Params(Vec<ParamFact>),
    Return(Shape),
    Module(String),
    Visibility(Visibility),
}

#[derive(Debug, Clone)]
pub struct ParamFact {
    pub name: String,
    pub doc: String,
    pub shape: Option<Shape>,
}

#[derive(Debug, Clone)]
pub struct DeclFacts {
    pub decl_id: HirId,
    pub facts: Vec<DeclFact>,
}

#[must_use]
pub fn from_compiler_facts(decl_id: HirId, facts: &[tune_resolve::CompilerFact]) -> DeclFacts {
    from_compiler_facts_and_analysis(decl_id, facts, None)
}

#[must_use]
pub fn from_compiler_facts_and_analysis(
    decl_id: HirId,
    facts: &[tune_resolve::CompilerFact],
    analysis: Option<&tune_shape::ShapeAnalysis>,
) -> DeclFacts {
    let mut decl_facts = syntax_decl_facts(decl_id, facts);
    if let Some(signature) = analysis.and_then(|analysis| analysis.inferred_signature.as_ref()) {
        decl_facts
            .facts
            .retain(|fact| !matches!(fact, DeclFact::Params(_) | DeclFact::Return(_)));
        decl_facts.facts.push(DeclFact::Params(param_facts(
            decl_id,
            facts,
            &signature.params,
        )));
        decl_facts
            .facts
            .push(DeclFact::Return(signature.ret.clone()));
    }
    decl_facts
}

fn syntax_decl_facts(decl_id: HirId, facts: &[tune_resolve::CompilerFact]) -> DeclFacts {
    DeclFacts {
        decl_id,
        facts: facts
            .iter()
            .filter(|fact| fact.owner == tune_resolve::FactOwner::Item(decl_id))
            .filter_map(|fact| match &fact.payload {
                tune_resolve::CompilerFactPayload::Name(name) => Some(DeclFact::Name(name.clone())),
                tune_resolve::CompilerFactPayload::Doc(doc) => Some(DeclFact::Doc(doc.clone())),
                tune_resolve::CompilerFactPayload::Return(shape) => {
                    Some(DeclFact::Return(tune_shape::lower_hir_shape(shape)))
                }
                tune_resolve::CompilerFactPayload::Visibility(visibility) => {
                    Some(DeclFact::Visibility(*visibility))
                }
                _ => None,
            })
            .collect(),
    }
}

fn param_facts(
    decl_id: HirId,
    facts: &[tune_resolve::CompilerFact],
    shapes: &[Shape],
) -> Vec<ParamFact> {
    param_ids(decl_id, facts)
        .iter()
        .enumerate()
        .map(|(index, param)| ParamFact {
            name: member_name(*param, facts).unwrap_or_else(|| format!("param{index}")),
            doc: member_doc(*param, facts).unwrap_or_default(),
            shape: shapes.get(index).cloned(),
        })
        .collect()
}

fn param_ids(decl_id: HirId, facts: &[tune_resolve::CompilerFact]) -> Vec<tune_hir::MemberId> {
    facts
        .iter()
        .find_map(|fact| {
            if fact.owner != tune_resolve::FactOwner::Item(decl_id) {
                return None;
            }
            match &fact.payload {
                tune_resolve::CompilerFactPayload::Params(params) => Some(params.clone()),
                _ => None,
            }
        })
        .unwrap_or_default()
}

fn member_name(member: tune_hir::MemberId, facts: &[tune_resolve::CompilerFact]) -> Option<String> {
    facts.iter().find_map(|fact| {
        if fact.owner != tune_resolve::FactOwner::Member(member) {
            return None;
        }
        match &fact.payload {
            tune_resolve::CompilerFactPayload::Name(name) => Some(name.clone()),
            _ => None,
        }
    })
}

fn member_doc(member: tune_hir::MemberId, facts: &[tune_resolve::CompilerFact]) -> Option<String> {
    facts.iter().find_map(|fact| {
        if fact.owner != tune_resolve::FactOwner::Member(member) {
            return None;
        }
        match &fact.payload {
            tune_resolve::CompilerFactPayload::Doc(doc) => Some(doc.clone()),
            _ => None,
        }
    })
}

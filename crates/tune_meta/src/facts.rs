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
    JsonInvoker,
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
                tune_resolve::CompilerFactPayload::JsonInvoker(_) => Some(DeclFact::JsonInvoker),
                _ => None,
            })
            .collect(),
    }
}

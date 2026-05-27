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

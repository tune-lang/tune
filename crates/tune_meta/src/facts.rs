#[derive(Debug, Clone)]
pub enum DeclFact {
    Name(String),
    Doc(String),
    Params(Vec<ParamFact>),
    Return(String),
    Module(String),
    Visibility(String),
    JsonInvoker,
}

#[derive(Debug, Clone)]
pub struct ParamFact {
    pub name: String,
    pub doc: String,
    pub shape: String,
}

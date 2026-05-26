#[derive(Debug, Clone)]
pub enum MetaPlan {
    StaticTaggedTable { tag: String },
    CompilerFact { fact: String },
    GeneratedJsonInvoker { function: String },
}

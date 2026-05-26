#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanFunction {
    pub name: String,
    pub ops: Vec<PlanOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanOp {
    DirectCall { function: String },
    BoundCall,
    WitnessCall,
    HostCall { symbol: String },
    FieldGet { field: String },
    FieldSet { field: String },
    SequenceGet { checked: bool },
    SequencePush,
    FiniteFor,
    StringBuild,
    ResultPropagate,
    Spawn,
    TaskJoin,
    Panic,
}

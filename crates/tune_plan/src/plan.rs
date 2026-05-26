#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanFunction {
    pub name: String,
    pub ops: Vec<PlanOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanOp {
    DirectCall { function: String },
    BoundCall,
    CallableValue,
    WitnessCall,
    HostCall { symbol: String },
    LocalLet { name: String },
    Assign,
    UnaryOp { op: String },
    BinaryOp { op: String },
    FieldGet { field: String },
    FieldSet { field: String },
    SequenceGet { checked: bool },
    SequencePush,
    FiniteFor,
    StringBuild,
    ResultPropagate,
    Return,
    Spawn,
    TaskJoin,
    Panic,
}

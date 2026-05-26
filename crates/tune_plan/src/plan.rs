use tune_hir::HirId;
use tune_hir::expr::{BinaryOp, UnaryOp};
use tune_resolve::LocalId;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanFunction {
    pub name: String,
    pub ops: Vec<PlanOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanOp {
    DirectCall { target: HirId },
    BoundCall,
    CallableValue,
    WitnessCall,
    HostCall { symbol: String },
    LocalLet { local: Option<LocalId> },
    Assign,
    UnaryOp { op: UnaryOp },
    BinaryOp { op: BinaryOp },
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

use tune_diagnostics::Span;
use tune_hir::ExprId;
use tune_hir::HirId;
use tune_hir::expr::{BinaryOp, UnaryOp};
use tune_hir::pattern::Pattern;
use tune_resolve::{LocalId, NameTarget, VariantId};
use tune_shape::MaterializationPlan;

use crate::meta::MetaPlan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanFunction {
    pub name: String,
    pub ops: Vec<PlanOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanIfBranch {
    pub condition: ExprId,
    pub body: ExprId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanMatchArm {
    pub pattern: Pattern,
    pub body: ExprId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanOp {
    DirectCall {
        target: HirId,
    },
    VariantConstruct {
        variant: VariantId,
    },
    BoundCall,
    CallableValue,
    WitnessCall,
    HostCall {
        symbol: String,
    },
    LocalLet {
        local: Option<LocalId>,
    },
    Assign,
    UnaryOp {
        op: UnaryOp,
    },
    BinaryOp {
        op: BinaryOp,
    },
    FieldGet {
        field: String,
    },
    FieldSet {
        field: String,
    },
    SequenceGet {
        checked: bool,
    },
    SequenceSet {
        checked: bool,
    },
    SequencePush,
    Materialize {
        plan: MaterializationPlan,
    },
    BindingSet {
        target: Option<NameTarget>,
    },
    FiniteFor {
        pattern: Pattern,
        iterable: ExprId,
        body: ExprId,
        span: Option<Span>,
    },
    StringBuild,
    If {
        branches: Vec<PlanIfBranch>,
        else_body: Option<ExprId>,
        span: Option<Span>,
    },
    Match {
        scrutinee: ExprId,
        arms: Vec<PlanMatchArm>,
        span: Option<Span>,
    },
    While {
        condition: ExprId,
        body: ExprId,
        span: Option<Span>,
    },
    Loop {
        body: ExprId,
        span: Option<Span>,
    },
    Break,
    Continue,
    ResultPropagate {
        expr: ExprId,
        span: Option<Span>,
    },
    Return,
    Spawn {
        body: ExprId,
        span: Option<Span>,
    },
    TaskJoin,
    Panic,
    Meta {
        plan: MetaPlan,
    },
}

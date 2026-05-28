use tune_diagnostics::Span;
use tune_hir::ExprId;
use tune_hir::HirId;
use tune_hir::MemberId;
use tune_hir::expr::{BinaryOp, UnaryOp};
use tune_hir::pattern::Pattern;
use tune_resolve::{LocalId, NameTarget, VariantId};
use tune_shape::MaterializationPlan;

use crate::meta::MetaPlan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanFunction {
    pub owner: Option<HirId>,
    pub member: Option<MemberId>,
    pub name: String,
    pub params: Vec<MemberId>,
    pub module_bindings: Vec<HirId>,
    pub ops: Vec<PlanOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanModule {
    pub entry: Option<PlanFunction>,
    pub functions: Vec<PlanFunction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanIfBranch {
    pub condition: ExprId,
    pub body: ExprId,
    pub condition_ops: Vec<PlanOp>,
    pub body_ops: Vec<PlanOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanMatchArm {
    pub pattern: Pattern,
    pub body: ExprId,
    pub variant: Option<VariantId>,
    pub bindings: Vec<PlanPatternBinding>,
    pub body_ops: Vec<PlanOp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanPatternBinding {
    pub local: Option<LocalId>,
    pub field_index: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiniteForContract {
    pub source: ExprId,
    pub len_member: Option<MemberId>,
    pub index_member: Option<MemberId>,
    pub source_evaluated_once: bool,
    pub length_evaluated_once: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructStatePlan {
    pub repr: StructStateRepr,
    pub ownership: StructOwnershipPlan,
}

impl StructStatePlan {
    pub const LOCAL: Self = Self {
        repr: StructStateRepr::LocalHandle,
        ownership: StructOwnershipPlan::NonAtomicRc,
    };

    pub const SHARED: Self = Self {
        repr: StructStateRepr::SharedHandle,
        ownership: StructOwnershipPlan::SharedAtomic,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructStateRepr {
    Inline,
    LocalHandle,
    SharedHandle,
    HostResource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructOwnershipPlan {
    Stack,
    DirectDrop,
    NonAtomicRc,
    Cow,
    SharedAtomic,
    HostRetained,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanOp {
    ConstInt {
        value: i64,
    },
    ConstBool {
        value: bool,
    },
    DirectCall {
        target: HirId,
        arg_count: usize,
    },
    VariantConstruct {
        variant: VariantId,
        arg_count: usize,
    },
    StructConstruct {
        item: HirId,
        state: StructStatePlan,
        fields: Vec<MemberId>,
    },
    BoundCall,
    MemberCall {
        member: Option<MemberId>,
        name: String,
        arg_count: usize,
    },
    CallableValue,
    WitnessCall,
    HostCall {
        symbol: String,
    },
    BindingGet {
        source: Option<NameTarget>,
    },
    LocalLet {
        local: Option<LocalId>,
        initialized: bool,
    },
    ModuleLet {
        item: HirId,
        initialized: bool,
        keep_value: bool,
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
        member: Option<MemberId>,
    },
    FieldSet {
        field: String,
        member: Option<MemberId>,
        base: Option<NameTarget>,
    },
    SequenceGet {
        checked: bool,
        index_member: Option<MemberId>,
    },
    SequenceSet {
        checked: bool,
        index_member: Option<MemberId>,
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
        contract: FiniteForContract,
        span: Option<Span>,
    },
    StringBuild,
    If {
        branches: Vec<PlanIfBranch>,
        else_body: Option<ExprId>,
        else_ops: Vec<PlanOp>,
        produces_value: bool,
        span: Option<Span>,
    },
    Match {
        scrutinee: ExprId,
        arms: Vec<PlanMatchArm>,
        produces_value: bool,
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

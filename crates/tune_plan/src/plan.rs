use tune_diagnostics::Span;
use tune_hir::ExprId;
use tune_hir::HirId;
use tune_hir::MemberId;
use tune_hir::expr::{BinaryOp, UnaryOp};
use tune_hir::pattern::Pattern;
use tune_resolve::{LocalId, NameTarget, VariantId};
use tune_shape::{MaterializationPlan, Shape};

use crate::meta::MetaPlan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanFunction {
    pub owner: Option<HirId>,
    pub member: Option<MemberId>,
    pub callable: Option<ExprId>,
    pub name: String,
    pub type_params: Vec<String>,
    pub span: Option<Span>,
    pub params: Vec<MemberId>,
    pub local_params: Vec<LocalId>,
    pub captures: Vec<Capture>,
    pub module_bindings: Vec<HirId>,
    pub struct_layouts: Vec<PlanStructLayout>,
    pub ops: Vec<PlanOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanStructLayout {
    pub owner: HirId,
    pub fields: Vec<MemberId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaptureSource {
    Local(LocalId),
    Param(MemberId),
    TopLevel(HirId),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CaptureMode {
    Reference,
    PrivateSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Capture {
    pub source: CaptureSource,
    pub mode: CaptureMode,
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
    pub tests: Vec<PlanPatternTest>,
    pub bindings: Vec<PlanPatternBinding>,
    pub body_ops: Vec<PlanOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanPatternTest {
    pub field_path: Vec<PlanPatternPathSegment>,
    pub kind: PlanPatternTestKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanPatternTestKind {
    Variant(VariantId),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanPatternBinding {
    pub local: Option<LocalId>,
    pub field_path: Vec<PlanPatternPathSegment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlanPatternPathSegment {
    VariantField(usize),
    TupleField(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FiniteForContract {
    pub source: ExprId,
    pub kind: FiniteForContractKind,
    pub len_member: Option<MemberId>,
    pub index_member: Option<MemberId>,
    pub source_evaluated_once: bool,
    pub length_evaluated_once: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FiniteForContractKind {
    Sequence,
    Range,
    MemberAccess,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructStatePlan {
    pub repr: StructStateRepr,
    pub ownership: StructOwnershipPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StructEscapeReason {
    Local,
    Returned,
    Captured,
    SpawnBoundary,
    HostRetained,
    OpaqueBoundary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StructStateDecision {
    pub reason: StructEscapeReason,
    pub plan: StructStatePlan,
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

    #[must_use]
    pub const fn for_escape(reason: StructEscapeReason) -> Self {
        match reason {
            StructEscapeReason::Local
            | StructEscapeReason::Returned
            | StructEscapeReason::Captured => Self::LOCAL,
            StructEscapeReason::SpawnBoundary
            | StructEscapeReason::HostRetained
            | StructEscapeReason::OpaqueBoundary => Self::SHARED,
        }
    }
}

impl StructStateDecision {
    #[must_use]
    pub const fn for_escape(reason: StructEscapeReason) -> Self {
        Self {
            reason,
            plan: StructStatePlan::for_escape(reason),
        }
    }
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
    ConstFloat {
        bits: u64,
    },
    ConstSize {
        value: u64,
    },
    ConstByte {
        value: u8,
    },
    ConstBool {
        value: bool,
    },
    ConstNone,
    ConstString {
        value: String,
    },
    DirectCall {
        target: HirId,
        arg_count: usize,
        type_args: Vec<Shape>,
        span: Option<Span>,
    },
    VariantConstruct {
        variant: VariantId,
        arg_count: usize,
        span: Option<Span>,
    },
    StructConstruct {
        item: HirId,
        escape: StructEscapeReason,
        state: StructStatePlan,
        fields: Vec<MemberId>,
        span: Option<Span>,
    },
    StructIs {
        item: HirId,
        span: Option<Span>,
    },
    BoundCall {
        arg_count: usize,
        span: Option<Span>,
    },
    MemberCall {
        member: Option<MemberId>,
        name: String,
        arg_count: usize,
        span: Option<Span>,
    },
    CallableValue {
        callable: ExprId,
        captures: Vec<Capture>,
        span: Option<Span>,
    },
    WitnessCall,
    HostCall {
        symbol: tune_host::HostSymbolId,
        arg_count: usize,
        span: Option<Span>,
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
        shape: Shape,
    },
    BinaryOp {
        op: BinaryOp,
        shape: Shape,
        span: Option<Span>,
    },
    BoolAnd {
        lhs_ops: Vec<PlanOp>,
        rhs_ops: Vec<PlanOp>,
        span: Option<Span>,
    },
    BoolOr {
        lhs_ops: Vec<PlanOp>,
        rhs_ops: Vec<PlanOp>,
        span: Option<Span>,
    },
    NoneCheck {
        is_not: bool,
        span: Option<Span>,
    },
    FieldGet {
        field: String,
        member: Option<MemberId>,
        span: Option<Span>,
    },
    FieldSet {
        field: String,
        member: Option<MemberId>,
        base: Option<NameTarget>,
        span: Option<Span>,
    },
    SequenceGet {
        checked: bool,
        index_member: Option<MemberId>,
    },
    SequenceSet {
        checked: bool,
        index_member: Option<MemberId>,
        base: Option<NameTarget>,
    },
    SequencePush,
    SequenceBuild {
        element_count: usize,
    },
    TupleBuild {
        element_count: usize,
    },
    Materialize {
        plan: MaterializationPlan,
        materializer: Option<MemberId>,
    },
    BindingSet {
        target: Option<NameTarget>,
    },
    FiniteFor {
        pattern: Pattern,
        iterable: ExprId,
        body: ExprId,
        binding: Option<LocalId>,
        iterable_ops: Vec<PlanOp>,
        body_ops: Vec<PlanOp>,
        contract: FiniteForContract,
        span: Option<Span>,
    },
    StringBuild {
        part_count: usize,
    },
    StringLen {
        span: Option<Span>,
    },
    StringGet {
        span: Option<Span>,
    },
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
        condition_ops: Vec<PlanOp>,
        body_ops: Vec<PlanOp>,
        span: Option<Span>,
    },
    Loop {
        body: ExprId,
        body_ops: Vec<PlanOp>,
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
        body_ops: Vec<PlanOp>,
        captures: Vec<Capture>,
        span: Option<Span>,
    },
    TaskJoin {
        span: Option<Span>,
    },
    Panic {
        arg_count: usize,
        span: Option<Span>,
    },
    Meta {
        plan: MetaPlan,
    },
}

use tune_diagnostics::Span;
use tune_hir::{ExprId, HirId, MemberId};
use tune_resolve::{LocalId, VariantId};
use tune_shape::Shape;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reg(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConstId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FieldId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HostSymbolId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IrCapture {
    pub reg: Reg,
    pub mode: IrCaptureMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrCaptureMode {
    Reference,
    PrivateSnapshot,
}

#[derive(Debug, Clone)]
pub struct IrFunction {
    pub owner: Option<HirId>,
    pub member: Option<MemberId>,
    pub callable: Option<ExprId>,
    pub name: String,
    pub span: Option<Span>,
    pub params: u32,
    pub regs: u32,
    pub locals: u32,
    pub constants: Vec<IrConst>,
    pub blocks: Vec<IrBlock>,
    pub task_functions: Vec<IrFunction>,
}

impl IrOp {
    #[must_use]
    pub const fn provenance_span(&self) -> Option<Span> {
        match self {
            Self::AddInt { span, .. }
            | Self::SubInt { span, .. }
            | Self::MulInt { span, .. }
            | Self::DivInt { span, .. }
            | Self::RemInt { span, .. }
            | Self::AddSizeChecked { span, .. }
            | Self::RangeInt { span, .. }
            | Self::NegInt { span, .. }
            | Self::NotBool { span, .. }
            | Self::GreaterInt { span, .. }
            | Self::CompareInt { span, .. }
            | Self::GetField { span, .. }
            | Self::SetField { span, .. }
            | Self::VariantConstruct { span, .. }
            | Self::StructConstruct { span, .. }
            | Self::StructIs { span, .. }
            | Self::CallDirect { span, .. }
            | Self::CallMember { span, .. }
            | Self::CallableValue { span, .. }
            | Self::CallBound { span, .. }
            | Self::Branch { span, .. }
            | Self::MatchVariant { span, .. }
            | Self::ResultPropagate { span, .. } => *span,
            Self::Spawn { span, .. } | Self::TaskJoin { span, .. } => *span,
            Self::Panic { span, .. } => *span,
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum IrConst {
    Int(i64),
    Float(f64),
    Size(u64),
    Byte(u8),
    Bool(bool),
    String(String),
}

#[derive(Debug, Clone)]
pub struct IrBlock {
    pub id: BlockId,
    pub ops: Vec<IrOp>,
}

#[derive(Debug, Clone)]
pub enum IrOp {
    LoadConst {
        dst: Reg,
        constant: ConstId,
        shape: Shape,
    },
    LoadLocal {
        dst: Reg,
        local: LocalId,
    },
    StoreLocal {
        local: LocalId,
        value: Reg,
    },
    Move {
        dst: Reg,
        src: Reg,
    },
    AddInt {
        dst: Reg,
        a: Reg,
        b: Reg,
        span: Option<Span>,
    },
    SubInt {
        dst: Reg,
        a: Reg,
        b: Reg,
        span: Option<Span>,
    },
    MulInt {
        dst: Reg,
        a: Reg,
        b: Reg,
        span: Option<Span>,
    },
    DivInt {
        dst: Reg,
        a: Reg,
        b: Reg,
        span: Option<Span>,
    },
    RemInt {
        dst: Reg,
        a: Reg,
        b: Reg,
        span: Option<Span>,
    },
    RangeInt {
        dst: Reg,
        start: Reg,
        end: Reg,
        inclusive: bool,
        span: Option<Span>,
    },
    NegInt {
        dst: Reg,
        value: Reg,
        span: Option<Span>,
    },
    NotBool {
        dst: Reg,
        value: Reg,
        span: Option<Span>,
    },
    GreaterInt {
        dst: Reg,
        a: Reg,
        b: Reg,
        span: Option<Span>,
    },
    CompareInt {
        dst: Reg,
        a: Reg,
        b: Reg,
        op: IrIntComparison,
        span: Option<Span>,
    },
    AddFloat {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    AddSizeChecked {
        dst: Reg,
        a: Reg,
        b: Reg,
        span: Option<Span>,
    },
    AddByteWrap {
        dst: Reg,
        a: Reg,
        b: Reg,
    },
    SeqBuild {
        dst: Reg,
        element_shape: Shape,
    },
    TupleBuild {
        dst: Reg,
        items: Vec<Reg>,
    },
    SeqPush {
        seq: Reg,
        value: Reg,
    },
    GetField {
        dst: Reg,
        base: Reg,
        field: FieldId,
        span: Option<Span>,
    },
    SetField {
        base: Reg,
        field: FieldId,
        value: Reg,
        span: Option<Span>,
    },
    SeqGet {
        dst: Reg,
        seq: Reg,
        index: Reg,
        checked: bool,
    },
    SeqSet {
        seq: Reg,
        index: Reg,
        value: Reg,
        checked: bool,
    },
    VariantConstruct {
        dst: Reg,
        variant: VariantId,
        args: Vec<Reg>,
        span: Option<Span>,
    },
    StructConstruct {
        dst: Reg,
        item: HirId,
        state: IrStructState,
        fields: Vec<StructField>,
        span: Option<Span>,
    },
    StructIs {
        dst: Reg,
        value: Reg,
        item: HirId,
        span: Option<Span>,
    },
    VariantField {
        dst: Reg,
        base: Reg,
        index: u32,
    },
    CallDirect {
        dst: Reg,
        function: HirId,
        args: Vec<Reg>,
        span: Option<Span>,
    },
    CallMember {
        dst: Reg,
        member: MemberId,
        args: Vec<Reg>,
        span: Option<Span>,
    },
    CallableValue {
        dst: Reg,
        callable: ExprId,
        captures: Vec<IrCapture>,
        span: Option<Span>,
    },
    CallBound {
        dst: Reg,
        callee: Reg,
        args: Vec<Reg>,
        span: Option<Span>,
    },
    CallWitness {
        dst: Reg,
        witness: Reg,
        args: Vec<Reg>,
    },
    CallHost {
        dst: Reg,
        symbol: HostSymbolId,
        args: Vec<Reg>,
    },
    Jump {
        target: BlockId,
    },
    Branch {
        condition: Reg,
        then_block: BlockId,
        else_block: BlockId,
        span: Option<Span>,
    },
    MatchVariant {
        scrutinee: Reg,
        arms: Vec<VariantArm>,
        else_block: Option<BlockId>,
        span: Option<Span>,
    },
    FiniteForInit {
        iterator: Reg,
        iterable: Reg,
        len: Reg,
    },
    FiniteForNext {
        iterator: Reg,
        iterable: Reg,
        len: Reg,
        index: Reg,
        item: Reg,
        body: BlockId,
        done: BlockId,
    },
    ResultPropagate {
        dst: Reg,
        result: Reg,
        expr: ExprId,
        span: Option<Span>,
    },
    Spawn {
        dst: Reg,
        function: u32,
        span: Option<Span>,
    },
    TaskJoin {
        dst: Reg,
        task: Reg,
        span: Option<Span>,
    },
    StringBuild {
        dst: Reg,
        parts: Vec<Reg>,
    },
    Panic {
        args: Vec<Reg>,
        span: Option<Span>,
    },
    Return {
        value: Option<Reg>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrIntComparison {
    Equal,
    NotEqual,
    Less,
    LessEqual,
    GreaterEqual,
}

#[derive(Debug, Clone)]
pub struct VariantArm {
    pub variant: VariantId,
    pub block: BlockId,
}

#[derive(Debug, Clone)]
pub struct StructField {
    pub field: FieldId,
    pub value: Reg,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IrStructState {
    pub repr: IrStateRepr,
    pub ownership: IrOwnershipPlan,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrStateRepr {
    Inline,
    LocalHandle,
    SharedHandle,
    HostResource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrOwnershipPlan {
    Stack,
    DirectDrop,
    NonAtomicRc,
    Cow,
    SharedAtomic,
    HostRetained,
}

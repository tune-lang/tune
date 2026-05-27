use tune_diagnostics::Span;
use tune_hir::{ExprId, HirId};
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

#[derive(Debug, Clone)]
pub struct IrFunction {
    pub owner: Option<HirId>,
    pub name: String,
    pub regs: u32,
    pub locals: u32,
    pub constants: Vec<IrConst>,
    pub blocks: Vec<IrBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrConst {
    Int(i64),
    Bool(bool),
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
    },
    AddFloat {
        dst: Reg,
        a: Reg,
        b: Reg,
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
    SeqPush {
        seq: Reg,
        value: Reg,
    },
    GetField {
        dst: Reg,
        base: Reg,
        field: FieldId,
    },
    SetField {
        base: Reg,
        field: FieldId,
        value: Reg,
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
    },
    CallDirect {
        dst: Reg,
        function: HirId,
        args: Vec<Reg>,
    },
    CallBound {
        dst: Reg,
        callee: Reg,
        args: Vec<Reg>,
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
    },
    MatchVariant {
        scrutinee: Reg,
        arms: Vec<VariantArm>,
        else_block: Option<BlockId>,
    },
    FiniteForInit {
        iterator: Reg,
        iterable: Reg,
        len: Reg,
    },
    FiniteForNext {
        iterator: Reg,
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
        callable: Reg,
    },
    TaskJoin {
        dst: Reg,
        task: Reg,
    },
    StringBuild {
        dst: Reg,
        parts: Vec<Reg>,
    },
    Panic {
        args: Vec<Reg>,
    },
    Return {
        value: Option<Reg>,
    },
}

#[derive(Debug, Clone)]
pub struct VariantArm {
    pub variant: VariantId,
    pub block: BlockId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Reg(pub u32);

#[derive(Debug, Clone)]
pub struct IrFunction {
    pub name: String,
    pub regs: u32,
    pub ops: Vec<IrOp>,
}

#[derive(Debug, Clone)]
pub enum IrOp {
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
    GetField {
        dst: Reg,
        base: Reg,
        field: u32,
    },
    SetField {
        base: Reg,
        field: u32,
        value: Reg,
    },
    SeqGet {
        dst: Reg,
        seq: Reg,
        index: Reg,
        checked: bool,
    },
    CallDirect {
        dst: Reg,
        function: u32,
        args: Vec<Reg>,
    },
    Return {
        value: Reg,
    },
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    Nop = 0,
    LoadConst = 1,
    AddInt = 2,
    AddFloat = 3,
    AddSizeChecked = 4,
    AddByteWrap = 5,
    SeqGetChecked = 6,
    SeqGetUnchecked = 7,
    FieldGet = 8,
    FieldSet = 9,
    CallDirect = 10,
    CallBound = 11,
    CallWitness = 12,
    CallHost = 13,
    ResultPropagate = 14,
    SpawnTask = 15,
    TaskJoin = 16,
    Return = 17,
}

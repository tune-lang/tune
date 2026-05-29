use std::collections::HashMap;

use tune_hir::{ExprId, HirId, MemberId};
use tune_ir::{BlockId, IrFunction};

use crate::Opcode;
use crate::artifact::BytecodeConst;
use crate::function::{
    BytecodeBoundCallSite, BytecodeCallSite, BytecodeCallableSite, BytecodeForSite,
    BytecodeMatchSite, BytecodePanicSite, BytecodeStructSite, BytecodeVariantSite, Instruction,
};

pub(crate) struct FunctionLowerer<'a> {
    pub(super) function: &'a IrFunction,
    pub(super) function_indices: &'a HashMap<HirId, u32>,
    pub(super) member_indices: &'a HashMap<MemberId, u32>,
    pub(super) callable_indices: &'a HashMap<ExprId, u32>,
    pub(super) block_offsets: HashMap<BlockId, u32>,
    pub(super) constants: &'a mut Vec<BytecodeConst>,
    pub(super) call_sites: Vec<BytecodeCallSite>,
    pub(super) bound_call_sites: Vec<BytecodeBoundCallSite>,
    pub(super) callable_sites: Vec<BytecodeCallableSite>,
    pub(super) struct_sites: Vec<BytecodeStructSite>,
    pub(super) variant_sites: Vec<BytecodeVariantSite>,
    pub(super) match_sites: Vec<BytecodeMatchSite>,
    pub(super) for_sites: Vec<BytecodeForSite>,
    pub(super) panic_sites: Vec<BytecodePanicSite>,
    pub(crate) instructions: Vec<Instruction>,
    pub(super) instruction_spans: Vec<Option<tune_diagnostics::Span>>,
}

impl FunctionLowerer<'_> {
    pub(super) fn push_instruction(&mut self, opcode: Opcode, a: u32, b: u32, c: u32) {
        self.instructions.push(Instruction { opcode, a, b, c });
    }
}

use tune_ir::{IrMutationMode, Reg};

use crate::Opcode;
use crate::lower::FunctionLowerer;

impl FunctionLowerer<'_> {
    pub(super) fn lower_range_int(&mut self, dst: Reg, start: Reg, end: Reg, inclusive: bool) {
        let opcode = if inclusive {
            Opcode::RangeInclusiveInt
        } else {
            Opcode::RangeExclusiveInt
        };
        self.push_instruction(opcode, dst.0, start.0, end.0);
    }

    pub(super) fn lower_seq_build(&mut self, dst: Reg) {
        self.push_instruction(Opcode::SeqBuild, dst.0, 0, 0);
    }

    pub(super) fn lower_seq_push(&mut self, seq: Reg, value: Reg, mode: &IrMutationMode) {
        let opcode = match mode {
            IrMutationMode::Exclusive => Opcode::SeqPushExclusive,
            IrMutationMode::SharedCow => Opcode::SeqPushShared,
        };
        self.push_instruction(opcode, seq.0, value.0, 0);
    }

    pub(super) fn lower_seq_get(&mut self, dst: Reg, seq: Reg, index: Reg, checked: bool) {
        let opcode = if checked {
            Opcode::SeqGetChecked
        } else {
            Opcode::SeqGetUnchecked
        };
        self.push_instruction(opcode, dst.0, seq.0, index.0);
    }

    pub(super) fn lower_seq_set(
        &mut self,
        seq: Reg,
        index: Reg,
        value: Reg,
        checked: bool,
        mode: &IrMutationMode,
    ) {
        let opcode = match (checked, mode) {
            (true, IrMutationMode::Exclusive) => Opcode::SeqSetCheckedExclusive,
            (false, IrMutationMode::Exclusive) => Opcode::SeqSetUncheckedExclusive,
            (true, IrMutationMode::SharedCow) => Opcode::SeqSetCheckedShared,
            (false, IrMutationMode::SharedCow) => Opcode::SeqSetUncheckedShared,
        };
        self.push_instruction(opcode, seq.0, index.0, value.0);
    }
}

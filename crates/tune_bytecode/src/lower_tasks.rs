use crate::Opcode;
use crate::function::Instruction;
use crate::lower::FunctionLowerer;
use tune_ir::Reg;

impl FunctionLowerer<'_> {
    pub(super) fn lower_spawn(&mut self, dst: Reg, value: Reg) {
        self.instructions.push(Instruction {
            opcode: Opcode::SpawnTask,
            a: dst.0,
            b: value.0,
            c: 0,
        });
    }

    pub(super) fn lower_task_join(&mut self, dst: Reg, task: Reg) {
        self.instructions.push(Instruction {
            opcode: Opcode::TaskJoin,
            a: dst.0,
            b: task.0,
            c: 0,
        });
    }
}

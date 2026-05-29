use crate::Opcode;
use crate::function::Instruction;
use crate::lower::FunctionLowerer;
use tune_ir::Reg;

impl FunctionLowerer<'_> {
    pub(super) fn lower_spawn(
        &mut self,
        dst: Reg,
        function: u32,
    ) -> Result<(), crate::lower::BytecodeLowerError> {
        let function = *self
            .task_indices
            .get(function as usize)
            .ok_or(crate::lower::BytecodeLowerError::UnknownFunction)?;
        self.instructions.push(Instruction {
            opcode: Opcode::SpawnTask,
            a: dst.0,
            b: function,
            c: 0,
        });
        Ok(())
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

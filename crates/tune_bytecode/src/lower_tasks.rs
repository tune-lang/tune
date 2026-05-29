use crate::Opcode;
use crate::function::{BytecodeCapture, BytecodeCaptureMode, BytecodeTaskSite, Instruction};
use crate::lower::FunctionLowerer;
use tune_ir::Reg;

impl FunctionLowerer<'_> {
    pub(super) fn lower_spawn(
        &mut self,
        dst: Reg,
        function: u32,
        captures: &[tune_ir::IrCapture],
    ) -> Result<(), crate::lower::BytecodeLowerError> {
        let function = *self
            .task_indices
            .get(function as usize)
            .ok_or(crate::lower::BytecodeLowerError::UnknownFunction)?;
        let site = u32::try_from(self.task_sites.len())
            .map_err(|_| crate::lower::BytecodeLowerError::ConstantLimit)?;
        self.task_sites.push(BytecodeTaskSite {
            function,
            captures: captures
                .iter()
                .map(|capture| BytecodeCapture {
                    register: capture.reg.0,
                    mode: match capture.mode {
                        tune_ir::IrCaptureMode::Reference => BytecodeCaptureMode::Reference,
                        tune_ir::IrCaptureMode::PrivateSnapshot => {
                            BytecodeCaptureMode::PrivateSnapshot
                        }
                    },
                })
                .collect(),
        });
        self.instructions.push(Instruction {
            opcode: Opcode::SpawnTask,
            a: dst.0,
            b: site,
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

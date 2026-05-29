use crate::Opcode;
use crate::function::BytecodeStringSite;
use crate::lower::BytecodeLowerError;
use crate::lower::context::FunctionLowerer;
use tune_ir::Reg;

impl FunctionLowerer<'_> {
    pub(super) fn lower_string_build(
        &mut self,
        dst: Reg,
        parts: &[Reg],
    ) -> Result<(), BytecodeLowerError> {
        let site = u32::try_from(self.string_sites.len())
            .map_err(|_| BytecodeLowerError::ConstantLimit)?;
        self.string_sites.push(BytecodeStringSite {
            parts: parts.iter().map(|part| part.0).collect(),
        });
        self.push_instruction(Opcode::StringBuild, dst.0, site, 0);
        Ok(())
    }
}

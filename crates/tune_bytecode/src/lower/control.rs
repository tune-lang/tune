use super::FunctionLowerer;
use super::error::BytecodeLowerError;
use crate::Opcode;
use crate::function::{BytecodeForSite, BytecodeMatchArm, BytecodeMatchSite, Instruction};
use crate::lower_tables::lower_variant;
use tune_ir::{BlockId, Reg, VariantArm};

#[derive(Debug, Clone, Copy)]
pub(super) struct FiniteForNextLowering {
    pub(super) iterator: Reg,
    pub(super) iterable: Reg,
    pub(super) len: Reg,
    pub(super) index: Reg,
    pub(super) item: Reg,
    pub(super) body: BlockId,
    pub(super) done: BlockId,
}

impl FunctionLowerer<'_> {
    pub(super) fn lower_jump(&mut self, target: BlockId) -> Result<(), BytecodeLowerError> {
        let target = self.block_target(target)?;
        self.push_instruction(Opcode::Jump, target, 0, 0);
        Ok(())
    }

    pub(super) fn lower_branch(
        &mut self,
        condition: Reg,
        then_block: BlockId,
        else_block: BlockId,
    ) -> Result<(), BytecodeLowerError> {
        let then_block = self.block_target(then_block)?;
        let else_block = self.block_target(else_block)?;
        self.instructions.push(Instruction {
            opcode: Opcode::JumpIfFalse,
            a: condition.0,
            b: else_block,
            c: 0,
        });
        self.push_instruction(Opcode::Jump, then_block, 0, 0);
        Ok(())
    }

    pub(super) fn lower_match_variant(
        &mut self,
        scrutinee: Reg,
        arms: &[VariantArm],
        else_block: Option<BlockId>,
    ) -> Result<(), BytecodeLowerError> {
        let match_site =
            u32::try_from(self.match_sites.len()).map_err(|_| BytecodeLowerError::ConstantLimit)?;
        let arms = arms
            .iter()
            .map(|arm| {
                Ok(BytecodeMatchArm {
                    variant: lower_variant(arm.variant),
                    target: self.block_target(arm.block)?,
                })
            })
            .collect::<Result<Vec<_>, BytecodeLowerError>>()?;
        let else_target = if let Some(else_block) = else_block {
            self.block_target(else_block)?
        } else {
            u32::MAX
        };
        self.match_sites.push(BytecodeMatchSite { arms });
        self.push_instruction(Opcode::MatchVariant, scrutinee.0, match_site, else_target);
        Ok(())
    }

    pub(super) fn lower_finite_for_init(&mut self, iterator: Reg, iterable: Reg, len: Reg) {
        self.push_instruction(Opcode::FiniteForInit, iterator.0, iterable.0, len.0);
    }

    pub(super) fn lower_finite_for_next(
        &mut self,
        for_next: FiniteForNextLowering,
    ) -> Result<(), BytecodeLowerError> {
        let body = self.block_target(for_next.body)?;
        let done = self.block_target(for_next.done)?;
        let site =
            u32::try_from(self.for_sites.len()).map_err(|_| BytecodeLowerError::ConstantLimit)?;
        self.for_sites.push(BytecodeForSite {
            iterable: for_next.iterable.0,
            len: for_next.len.0,
            index: for_next.index.0,
            item: for_next.item.0,
            body,
            done,
        });
        self.push_instruction(Opcode::FiniteForNext, for_next.iterator.0, site, 0);
        Ok(())
    }

    fn block_target(&self, block: BlockId) -> Result<u32, BytecodeLowerError> {
        self.block_offsets
            .get(&block)
            .copied()
            .ok_or(BytecodeLowerError::UnknownBlock)
    }
}

use tune_plan::PlanOp;

use tune_diagnostics::Span;

use crate::lower::Lowerer;
use crate::lower_slots::{local_offset, local_slot};
use crate::{BlockId, IrBlock, IrLowerError, IrOp};

impl Lowerer {
    pub(super) fn lower_if(
        &mut self,
        branches: &[tune_plan::PlanIfBranch],
        else_ops: &[PlanOp],
        span: Option<Span>,
        produces_value: bool,
    ) -> Result<(), IrLowerError> {
        let base_stack_len = self.stack.len();
        let result = produces_value.then(|| self.alloc_reg()).transpose()?;
        let join = self.alloc_block();
        let else_block = self.alloc_block();
        let body_blocks = branches
            .iter()
            .map(|_| self.alloc_block())
            .collect::<Vec<_>>();
        let condition_blocks = branches
            .iter()
            .skip(1)
            .map(|_| self.alloc_block())
            .collect::<Vec<_>>();

        for (index, branch) in branches.iter().enumerate() {
            for op in &branch.condition_ops {
                self.lower_op(op)?;
            }
            let condition = self.pop("if condition")?;
            let then_block = body_blocks[index];
            let false_block = condition_blocks.get(index).copied().unwrap_or(else_block);
            self.push_op(IrOp::Branch {
                condition,
                then_block,
                else_block: false_block,
                span,
            });
            self.switch_to_block(then_block);
            for op in &branch.body_ops {
                self.lower_op(op)?;
            }
            if !self.current_block_returns() {
                if let Some(result) = result {
                    let value = self.pop("if branch value")?;
                    self.push_op(IrOp::Move {
                        dst: result,
                        src: value,
                    });
                }
                self.push_op(IrOp::Jump { target: join });
            }
            self.stack.truncate(base_stack_len);
            self.switch_to_block(false_block);
        }

        for op in else_ops {
            self.lower_op(op)?;
        }
        if !self.current_block_returns() {
            if let Some(result) = result {
                let value = self.pop("if else value")?;
                self.push_op(IrOp::Move {
                    dst: result,
                    src: value,
                });
            }
            self.push_op(IrOp::Jump { target: join });
        }
        self.stack.truncate(base_stack_len);
        self.switch_to_block(join);
        if let Some(result) = result {
            self.stack.push(result);
        }
        Ok(())
    }

    pub(super) fn lower_match(
        &mut self,
        arms: &[tune_plan::PlanMatchArm],
        produces_value: bool,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let scrutinee = self.pop("match scrutinee")?;
        let base_stack_len = self.stack.len();
        let result = produces_value.then(|| self.alloc_reg()).transpose()?;
        let join = self.alloc_block();
        let fallback_block = arms
            .iter()
            .any(|arm| matches!(arm.pattern.kind, tune_hir::pattern::PatternKind::Else))
            .then(|| self.alloc_block());
        let mut variant_arms = Vec::new();
        let mut arm_blocks = Vec::new();

        for arm in arms {
            let block = if matches!(arm.pattern.kind, tune_hir::pattern::PatternKind::Else) {
                fallback_block.ok_or(IrLowerError::UnsupportedOp("match fallback"))?
            } else {
                self.alloc_block()
            };
            arm_blocks.push((block, arm));
            if let Some(variant) = arm.variant {
                variant_arms.push(crate::VariantArm { variant, block });
            }
        }

        self.push_op(IrOp::MatchVariant {
            scrutinee,
            arms: variant_arms,
            else_block: fallback_block,
            span,
        });

        for (block, arm) in arm_blocks {
            self.switch_to_block(block);
            for binding in &arm.bindings {
                let Some(local) = binding.local else {
                    continue;
                };
                let local = local_slot(local, local_offset(&self.module_bindings, &self.params))?;
                self.track_local(local)?;
                let dst = self.alloc_reg()?;
                self.push_op(IrOp::VariantField {
                    dst,
                    base: scrutinee,
                    index: u32::try_from(binding.field_index)
                        .map_err(|_| IrLowerError::RegisterLimit)?,
                });
                self.push_op(IrOp::StoreLocal { local, value: dst });
            }
            for op in &arm.body_ops {
                self.lower_op(op)?;
            }
            if !self.current_block_returns() {
                if let Some(result) = result {
                    let value = self.pop("match arm value")?;
                    self.push_op(IrOp::Move {
                        dst: result,
                        src: value,
                    });
                }
                self.push_op(IrOp::Jump { target: join });
            }
            self.stack.truncate(base_stack_len);
        }

        if let Some(fallback_block) = fallback_block {
            self.switch_to_block(fallback_block);
            if self.current_block_empty() {
                self.push_op(IrOp::Jump { target: join });
            }
        }
        self.switch_to_block(join);
        if let Some(result) = result {
            self.stack.push(result);
        }
        Ok(())
    }

    pub(super) fn lower_while(
        &mut self,
        condition_ops: &[PlanOp],
        body_ops: &[PlanOp],
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let base_stack_len = self.stack.len();
        let condition_block = self.alloc_block();
        let body_block = self.alloc_block();
        let done_block = self.alloc_block();

        self.push_op(IrOp::Jump {
            target: condition_block,
        });
        self.switch_to_block(condition_block);
        for op in condition_ops {
            self.lower_op(op)?;
        }
        let condition = self.pop("while condition")?;
        self.push_op(IrOp::Branch {
            condition,
            then_block: body_block,
            else_block: done_block,
            span,
        });
        self.stack.truncate(base_stack_len);

        self.switch_to_block(body_block);
        for op in body_ops {
            self.lower_op(op)?;
        }
        if !self.current_block_returns() {
            self.push_op(IrOp::Jump {
                target: condition_block,
            });
        }
        self.stack.truncate(base_stack_len);

        self.switch_to_block(done_block);
        Ok(())
    }

    pub(super) fn push_op(&mut self, op: IrOp) {
        if let Some(block) = self
            .blocks
            .iter_mut()
            .find(|block| block.id == self.current_block)
        {
            block.ops.push(op);
        }
    }

    fn alloc_block(&mut self) -> BlockId {
        let block = BlockId(self.next_block);
        self.next_block = self.next_block.saturating_add(1);
        self.blocks.push(IrBlock {
            id: block,
            ops: Vec::new(),
        });
        block
    }

    fn switch_to_block(&mut self, block: BlockId) {
        self.current_block = block;
    }

    fn current_block_returns(&self) -> bool {
        self.blocks
            .iter()
            .find(|block| block.id == self.current_block)
            .and_then(|block| block.ops.last())
            .is_some_and(|op| matches!(op, IrOp::Return { .. }))
    }

    fn current_block_empty(&self) -> bool {
        self.blocks
            .iter()
            .find(|block| block.id == self.current_block)
            .is_none_or(|block| block.ops.is_empty())
    }
}

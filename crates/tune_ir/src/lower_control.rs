use tune_plan::PlanOp;

use tune_diagnostics::Span;
use tune_hir::pattern::PatternKind;
use tune_resolve::LocalId;

use crate::lower::Lowerer;
use crate::lower_slots::{local_offset, local_slot};
use crate::{BlockId, IrBlock, IrLowerError, IrOp, Reg};

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
            if !self.current_block_terminates() {
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
        if !self.current_block_terminates() {
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
        let mut next_test = self.current_block;

        for arm in arms {
            self.switch_to_block(next_test);
            let arm_block = self.alloc_block();
            let failure_block = self.alloc_block();
            if matches!(arm.pattern.kind, PatternKind::Else) {
                self.push_op(IrOp::Jump { target: arm_block });
            } else {
                self.lower_plan_pattern_tests(
                    &arm.tests,
                    scrutinee,
                    arm_block,
                    failure_block,
                    span,
                )?;
            }

            self.switch_to_block(arm_block);
            for binding in &arm.bindings {
                let Some(local) = binding.local else {
                    continue;
                };
                let local = local_slot(
                    local,
                    local_offset(
                        &self.module_bindings,
                        &self.params,
                        &self.local_params,
                        &self.captures,
                    ),
                )?;
                self.track_local(local)?;
                let dst = self.lower_pattern_field_path(scrutinee, &binding.field_path)?;
                self.push_op(IrOp::StoreLocal { local, value: dst });
            }
            for op in &arm.body_ops {
                self.lower_op(op)?;
            }
            if !self.current_block_terminates() {
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
            next_test = failure_block;
        }

        self.switch_to_block(next_test);
        if self.current_block_empty() {
            self.push_op(IrOp::Jump { target: join });
        }
        self.switch_to_block(join);
        if let Some(result) = result {
            self.stack.push(result);
        }
        Ok(())
    }

    fn lower_plan_pattern_tests(
        &mut self,
        tests: &[tune_plan::PlanPatternTest],
        root: Reg,
        success: BlockId,
        failure: BlockId,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        if tests.is_empty() {
            self.push_op(IrOp::Jump { target: success });
            return Ok(());
        }
        for (index, test) in tests.iter().enumerate() {
            let passed = if index + 1 == tests.len() {
                success
            } else {
                self.alloc_block()
            };
            let value = self.lower_pattern_field_path(root, &test.field_path)?;
            self.push_op(IrOp::MatchVariant {
                scrutinee: value,
                arms: vec![crate::VariantArm {
                    variant: test.variant,
                    block: passed,
                }],
                else_block: Some(failure),
                span,
            });
            if index + 1 != tests.len() {
                self.switch_to_block(passed);
            }
        }
        Ok(())
    }

    fn lower_pattern_field_path(&mut self, root: Reg, path: &[usize]) -> Result<Reg, IrLowerError> {
        let mut base = root;
        for index in path {
            let dst = self.alloc_reg()?;
            self.push_op(IrOp::VariantField {
                dst,
                base,
                index: u32::try_from(*index).map_err(|_| IrLowerError::RegisterLimit)?,
            });
            base = dst;
        }
        Ok(base)
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
        self.loop_targets.push((condition_block, done_block));
        for op in body_ops {
            self.lower_op(op)?;
        }
        self.loop_targets.pop();
        if !self.current_block_terminates() {
            self.push_op(IrOp::Jump {
                target: condition_block,
            });
        }
        self.stack.truncate(base_stack_len);

        self.switch_to_block(done_block);
        Ok(())
    }

    pub(super) fn lower_loop(&mut self, body_ops: &[PlanOp]) -> Result<(), IrLowerError> {
        let base_stack_len = self.stack.len();
        let body_block = self.alloc_block();
        let done_block = self.alloc_block();

        self.push_op(IrOp::Jump { target: body_block });
        self.switch_to_block(body_block);
        self.loop_targets.push((body_block, done_block));
        for op in body_ops {
            self.lower_op(op)?;
        }
        self.loop_targets.pop();
        if !self.current_block_terminates() {
            self.push_op(IrOp::Jump { target: body_block });
        }
        self.stack.truncate(base_stack_len);
        self.switch_to_block(done_block);
        Ok(())
    }

    pub(super) fn lower_break(&mut self) -> Result<(), IrLowerError> {
        let Some((_, break_block)) = self.loop_targets.last().copied() else {
            return Err(IrLowerError::UnsupportedOp("break outside loop"));
        };
        self.push_op(IrOp::Jump {
            target: break_block,
        });
        Ok(())
    }

    pub(super) fn lower_continue(&mut self) -> Result<(), IrLowerError> {
        let Some((continue_block, _)) = self.loop_targets.last().copied() else {
            return Err(IrLowerError::UnsupportedOp("continue outside loop"));
        };
        self.push_op(IrOp::Jump {
            target: continue_block,
        });
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

    pub(super) fn alloc_block(&mut self) -> BlockId {
        let block = BlockId(self.next_block);
        self.next_block = self.next_block.saturating_add(1);
        self.blocks.push(IrBlock {
            id: block,
            ops: Vec::new(),
        });
        block
    }

    pub(super) fn switch_to_block(&mut self, block: BlockId) {
        self.current_block = block;
    }

    pub(super) fn store_for_binding(
        &mut self,
        binding: Option<LocalId>,
        item: Reg,
    ) -> Result<(), IrLowerError> {
        let Some(binding) = binding else {
            return Ok(());
        };
        let local = local_slot(
            binding,
            local_offset(
                &self.module_bindings,
                &self.params,
                &self.local_params,
                &self.captures,
            ),
        )?;
        self.track_local(local)?;
        self.push_op(IrOp::StoreLocal { local, value: item });
        Ok(())
    }

    pub(super) fn current_block_terminates(&self) -> bool {
        self.blocks
            .iter()
            .find(|block| block.id == self.current_block)
            .and_then(|block| block.ops.last())
            .is_some_and(|op| {
                matches!(
                    op,
                    IrOp::Return { .. }
                        | IrOp::Jump { .. }
                        | IrOp::Branch { .. }
                        | IrOp::MatchVariant { .. }
                )
            })
    }

    fn current_block_empty(&self) -> bool {
        self.blocks
            .iter()
            .find(|block| block.id == self.current_block)
            .is_none_or(|block| block.ops.is_empty())
    }
}

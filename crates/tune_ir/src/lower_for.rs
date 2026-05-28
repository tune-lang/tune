use tune_diagnostics::Span;
use tune_plan::PlanOp;
use tune_resolve::LocalId;

use crate::lower::{IrLowerError, Lowerer};
use crate::{ConstId, IrConst, IrIntComparison, IrOp, Reg};

impl Lowerer {
    pub(super) fn lower_finite_for(
        &mut self,
        binding: Option<LocalId>,
        iterable_ops: &[PlanOp],
        body_ops: &[PlanOp],
        contract: &tune_plan::FiniteForContract,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        if let (Some(len_member), Some(index_member)) = (contract.len_member, contract.index_member)
        {
            return self.lower_member_finite_for(
                binding,
                iterable_ops,
                body_ops,
                len_member,
                index_member,
                span,
            );
        }

        let base_stack_len = self.stack.len();
        for op in iterable_ops {
            self.lower_op(op)?;
        }
        let iterable = self.pop("for iterable")?;
        let iterator = self.alloc_reg()?;
        let len = self.alloc_reg()?;
        let index = self.alloc_reg()?;
        let item = self.alloc_reg()?;
        let next_block = self.alloc_block();
        let body_block = self.alloc_block();
        let done_block = self.alloc_block();

        self.push_op(IrOp::FiniteForInit {
            iterator,
            iterable,
            len,
        });
        self.push_op(IrOp::Jump { target: next_block });

        self.switch_to_block(next_block);
        self.push_op(IrOp::FiniteForNext {
            iterator,
            iterable,
            len,
            index,
            item,
            body: body_block,
            done: done_block,
        });

        self.switch_to_block(body_block);
        self.store_for_binding(binding, item)?;
        self.loop_targets.push((next_block, done_block));
        for op in body_ops {
            self.lower_op(op)?;
        }
        self.loop_targets.pop();
        if !self.current_block_terminates() {
            self.push_op(IrOp::Jump { target: next_block });
        }
        self.stack.truncate(base_stack_len);
        self.switch_to_block(done_block);
        self.stack.truncate(base_stack_len);
        let _ = span;
        Ok(())
    }

    fn lower_member_finite_for(
        &mut self,
        binding: Option<LocalId>,
        iterable_ops: &[PlanOp],
        body_ops: &[PlanOp],
        len_member: tune_hir::MemberId,
        index_member: tune_hir::MemberId,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let base_stack_len = self.stack.len();
        for op in iterable_ops {
            self.lower_op(op)?;
        }
        let iterable = self.pop("for iterable")?;
        let iterator = self.alloc_reg()?;
        let len = self.alloc_reg()?;
        let condition = self.alloc_reg()?;
        let item = self.alloc_reg()?;
        let one = self.alloc_reg()?;
        let next_block = self.alloc_block();
        let body_block = self.alloc_block();
        let done_block = self.alloc_block();

        self.load_int_into(iterator, 0)?;
        self.push_op(IrOp::CallMember {
            dst: len,
            member: len_member,
            args: vec![iterable],
            span,
        });
        self.load_int_into(one, 1)?;
        self.push_op(IrOp::Jump { target: next_block });

        self.switch_to_block(next_block);
        self.push_op(IrOp::CompareInt {
            dst: condition,
            a: iterator,
            b: len,
            op: IrIntComparison::Less,
            span,
        });
        self.push_op(IrOp::Branch {
            condition,
            then_block: body_block,
            else_block: done_block,
            span,
        });

        self.switch_to_block(body_block);
        self.push_op(IrOp::CallMember {
            dst: item,
            member: index_member,
            args: vec![iterable, iterator],
            span,
        });
        self.store_for_binding(binding, item)?;
        self.loop_targets.push((next_block, done_block));
        for op in body_ops {
            self.lower_op(op)?;
        }
        self.loop_targets.pop();
        if !self.current_block_terminates() {
            self.push_op(IrOp::AddInt {
                dst: iterator,
                a: iterator,
                b: one,
                span,
            });
            self.push_op(IrOp::Jump { target: next_block });
        }
        self.stack.truncate(base_stack_len);
        self.switch_to_block(done_block);
        self.stack.truncate(base_stack_len);
        Ok(())
    }

    fn load_int_into(&mut self, dst: Reg, value: i64) -> Result<(), IrLowerError> {
        let constant: ConstId = self.push_const(IrConst::Int(value))?;
        self.push_op(IrOp::LoadConst {
            dst,
            constant,
            shape: tune_shape::Shape::Int,
        });
        Ok(())
    }
}

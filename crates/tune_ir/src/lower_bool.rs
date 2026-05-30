use tune_diagnostics::Span;
use tune_plan::PlanOp;

use crate::lower::{IrLowerError, Lowerer};
use crate::{ConstId, IrConst, IrOp, IrTransfer, Reg};

impl Lowerer {
    pub(super) fn lower_bool_and(
        &mut self,
        lhs_ops: &[PlanOp],
        rhs_ops: &[PlanOp],
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        self.lower_short_circuit_bool(lhs_ops, rhs_ops, false, span)
    }

    pub(super) fn lower_bool_or(
        &mut self,
        lhs_ops: &[PlanOp],
        rhs_ops: &[PlanOp],
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        self.lower_short_circuit_bool(lhs_ops, rhs_ops, true, span)
    }

    fn lower_short_circuit_bool(
        &mut self,
        lhs_ops: &[PlanOp],
        rhs_ops: &[PlanOp],
        short_value: bool,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let base_stack_len = self.stack.len();
        let result = self.alloc_reg()?;
        let rhs_block = self.alloc_block();
        let short_block = self.alloc_block();
        let join = self.alloc_block();

        for op in lhs_ops {
            self.lower_op(op)?;
        }
        let lhs = self.pop("bool lhs")?;
        let (then_block, else_block) = if short_value {
            (short_block, rhs_block)
        } else {
            (rhs_block, short_block)
        };
        self.push_op(IrOp::Branch {
            condition: lhs,
            then_block,
            else_block,
            span,
        });
        self.stack.truncate(base_stack_len);

        self.switch_to_block(short_block);
        self.load_bool_into(result, short_value)?;
        self.push_op(IrOp::Jump { target: join });
        self.stack.truncate(base_stack_len);

        self.switch_to_block(rhs_block);
        for op in rhs_ops {
            self.lower_op(op)?;
        }
        let rhs = self.pop("bool rhs")?;
        self.push_op(IrOp::Move {
            dst: result,
            src: rhs,
            transfer: IrTransfer::Copy,
        });
        self.push_op(IrOp::Jump { target: join });
        self.stack.truncate(base_stack_len);

        self.switch_to_block(join);
        self.stack.push(result);
        Ok(())
    }

    fn load_bool_into(&mut self, dst: Reg, value: bool) -> Result<(), IrLowerError> {
        let constant: ConstId = self.push_const(IrConst::Bool(value))?;
        self.push_op(IrOp::LoadConst {
            dst,
            constant,
            shape: tune_shape::Shape::Bool,
        });
        Ok(())
    }
}

use tune_hir::{HirId, MemberId};

use crate::IrOp;
use crate::lower::{IrLowerError, Lowerer};

impl Lowerer {
    pub(super) fn lower_direct_call(
        &mut self,
        target: HirId,
        arg_count: usize,
    ) -> Result<(), IrLowerError> {
        let mut args = Vec::with_capacity(arg_count);
        for _ in 0..arg_count {
            args.push(self.pop("call argument")?);
        }
        args.reverse();
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::CallDirect {
            dst,
            function: target,
            args,
        });
        self.stack.push(dst);
        Ok(())
    }

    pub(super) fn lower_member_call(
        &mut self,
        member: MemberId,
        arg_count: usize,
    ) -> Result<(), IrLowerError> {
        let mut args = Vec::with_capacity(arg_count.saturating_add(1));
        for _ in 0..arg_count {
            args.push(self.pop("member call argument")?);
        }
        let receiver = self.pop("member call receiver")?;
        args.push(receiver);
        args.reverse();
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::CallMember { dst, member, args });
        self.stack.push(dst);
        Ok(())
    }
}

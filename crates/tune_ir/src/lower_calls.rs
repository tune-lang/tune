use tune_hir::{HirId, MemberId};

use tune_diagnostics::Span;
use tune_plan::CaptureSource;

use crate::IrOp;
use crate::lower::{IrLowerError, Lowerer};

impl Lowerer {
    pub(super) fn lower_direct_call(
        &mut self,
        target: HirId,
        arg_count: usize,
        span: Option<Span>,
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
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    pub(super) fn lower_member_call(
        &mut self,
        member: MemberId,
        arg_count: usize,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let mut args = Vec::with_capacity(arg_count.saturating_add(1));
        for _ in 0..arg_count {
            args.push(self.pop("member call argument")?);
        }
        let receiver = self.pop("member call receiver")?;
        args.push(receiver);
        args.reverse();
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::CallMember {
            dst,
            member,
            args,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    pub(super) fn lower_materialize(&mut self, member: MemberId) -> Result<(), IrLowerError> {
        let input = self.pop("materialization input")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::CallMember {
            dst,
            member,
            args: vec![input],
            span: None,
        });
        self.stack.push(dst);
        Ok(())
    }

    pub(super) fn lower_callable_value(
        &mut self,
        callable: tune_hir::ExprId,
        captures: &[CaptureSource],
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let mut capture_regs = Vec::with_capacity(captures.len());
        for capture in captures {
            let target = match capture {
                CaptureSource::Local(local) => tune_resolve::NameTarget::Local(*local),
                CaptureSource::TopLevel(item) => tune_resolve::NameTarget::TopLevel(*item),
            };
            self.lower_binding_get(target)?;
            capture_regs.push(self.pop("callable capture")?);
        }
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::CallableValue {
            dst,
            callable,
            captures: capture_regs,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    pub(super) fn lower_bound_call(
        &mut self,
        arg_count: usize,
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let mut args = Vec::with_capacity(arg_count);
        for _ in 0..arg_count {
            args.push(self.pop("bound call argument")?);
        }
        args.reverse();
        let callee = self.pop("bound call callee")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::CallBound {
            dst,
            callee,
            args,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }
}

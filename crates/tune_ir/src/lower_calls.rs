use tune_hir::{HirId, MemberId};

use tune_diagnostics::Span;
use tune_plan::{Capture, CaptureSource};
use tune_shape::Shape;

use crate::lower::{IrLowerError, Lowerer};
use crate::{HostSymbolId, IrCapture, IrCaptureMode, IrOp};

impl Lowerer {
    pub(super) fn lower_direct_call(
        &mut self,
        target: HirId,
        arg_count: usize,
        type_args: &[Shape],
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
            type_args: type_args.to_vec(),
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
        captures: &[Capture],
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let mut capture_regs = Vec::with_capacity(captures.len());
        for capture in captures {
            let target = match capture.source {
                CaptureSource::Local(local) => tune_resolve::NameTarget::Local(local),
                CaptureSource::Param(param) => tune_resolve::NameTarget::Param(param),
                CaptureSource::TopLevel(item) => tune_resolve::NameTarget::TopLevel(item),
            };
            self.lower_binding_get(target)?;
            capture_regs.push(IrCapture {
                reg: self.pop("callable capture")?,
                mode: match capture.mode {
                    tune_plan::CaptureMode::Reference => IrCaptureMode::Reference,
                    tune_plan::CaptureMode::PrivateSnapshot => IrCaptureMode::PrivateSnapshot,
                },
            });
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

    pub(super) fn lower_host_call(
        &mut self,
        symbol: u32,
        arg_count: usize,
        _span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let mut args = Vec::with_capacity(arg_count);
        for _ in 0..arg_count {
            args.push(self.pop("host call argument")?);
        }
        args.reverse();
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::CallHost {
            dst,
            symbol: HostSymbolId(symbol),
            args,
        });
        self.stack.push(dst);
        Ok(())
    }
}

use tune_diagnostics::Span;
use tune_plan::{Capture, CaptureMode, CaptureSource, PlanOp};

use crate::lower::{IrLowerError, Lowerer};
use crate::{IrCapture, IrCaptureMode, IrOp};

impl Lowerer {
    pub(super) fn lower_spawn(
        &mut self,
        body_ops: &[PlanOp],
        captures: &[Capture],
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let capture_regs = self.lower_spawn_captures(captures)?;
        let param_count = u32::try_from(captures.len()).map_err(|_| IrLowerError::RegisterLimit)?;
        let mut task = Lowerer {
            next_reg: 0,
            locals: param_count,
            params: Vec::new(),
            local_params: Vec::new(),
            captures: captures.to_vec(),
            module_bindings: Vec::new(),
            struct_layouts: self.struct_layouts.clone(),
            constants: Vec::new(),
            blocks: vec![crate::IrBlock {
                id: crate::BlockId(0),
                ops: Vec::new(),
            }],
            current_block: crate::BlockId(0),
            next_block: 1,
            stack: Vec::new(),
            loop_targets: Vec::new(),
            task_functions: Vec::new(),
        };
        for op in body_ops {
            task.lower_op(op)?;
        }
        let value = task.pop("spawn value")?;
        task.push_op(IrOp::Return { value: Some(value) });
        let function =
            u32::try_from(self.task_functions.len()).map_err(|_| IrLowerError::RegisterLimit)?;
        self.task_functions.push(crate::IrFunction {
            owner: None,
            member: None,
            callable: None,
            name: "<spawn>".to_owned(),
            span,
            params: param_count,
            regs: task.next_reg,
            locals: task.locals,
            constants: task.constants,
            struct_layouts: task.struct_layouts,
            blocks: task.blocks,
            task_functions: task.task_functions,
        });
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::Spawn {
            dst,
            function,
            captures: capture_regs,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    fn lower_spawn_captures(
        &mut self,
        captures: &[Capture],
    ) -> Result<Vec<IrCapture>, IrLowerError> {
        let mut capture_regs = Vec::with_capacity(captures.len());
        for capture in captures {
            let target = match capture.source {
                CaptureSource::Local(local) => tune_resolve::NameTarget::Local(local),
                CaptureSource::Param(param) => tune_resolve::NameTarget::Param(param),
                CaptureSource::TopLevel(item) => tune_resolve::NameTarget::TopLevel(item),
            };
            self.lower_binding_get(target)?;
            capture_regs.push(IrCapture {
                reg: self.pop("spawn capture")?,
                mode: match capture.mode {
                    CaptureMode::Reference => IrCaptureMode::Reference,
                    CaptureMode::PrivateSnapshot => IrCaptureMode::PrivateSnapshot,
                },
            });
        }
        Ok(capture_regs)
    }

    pub(super) fn lower_task_join(&mut self, span: Option<Span>) -> Result<(), IrLowerError> {
        let task = self.pop("task join")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::TaskJoin { dst, task, span });
        self.stack.push(dst);
        Ok(())
    }
}

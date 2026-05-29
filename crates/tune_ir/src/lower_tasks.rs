use tune_diagnostics::Span;
use tune_plan::PlanOp;

use crate::IrOp;
use crate::lower::{IrLowerError, Lowerer};

impl Lowerer {
    pub(super) fn lower_spawn(
        &mut self,
        body_ops: &[PlanOp],
        span: Option<Span>,
    ) -> Result<(), IrLowerError> {
        let mut task = Lowerer {
            next_reg: 0,
            locals: self.locals,
            params: self.params.clone(),
            local_params: self.local_params.clone(),
            captures: self.captures.clone(),
            module_bindings: self.module_bindings.clone(),
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
            params: 0,
            regs: task.next_reg,
            locals: task.locals,
            constants: task.constants,
            blocks: task.blocks,
            task_functions: task.task_functions,
        });
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::Spawn {
            dst,
            function,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }

    pub(super) fn lower_task_join(&mut self, span: Option<Span>) -> Result<(), IrLowerError> {
        let task = self.pop("task join")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::TaskJoin { dst, task, span });
        self.stack.push(dst);
        Ok(())
    }
}

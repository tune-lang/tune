use tune_hir::HirId;
use tune_hir::expr::BinaryOp;
use tune_plan::{PlanFunction, PlanOp};
use tune_resolve::{LocalId, NameTarget};
use tune_shape::Shape;

use crate::{BlockId, ConstId, IrBlock, IrConst, IrFunction, IrOp, Reg};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrLowerError {
    StackUnderflow(&'static str),
    UnsupportedOp(&'static str),
    RegisterLimit,
    ConstantLimit,
}

pub fn lower_plan_function(plan: &PlanFunction) -> Result<IrFunction, IrLowerError> {
    let mut lowerer = Lowerer {
        next_reg: 0,
        locals: 0,
        params: plan.params.clone(),
        module_bindings: plan.module_bindings.clone(),
        constants: Vec::new(),
        blocks: vec![IrBlock {
            id: BlockId(0),
            ops: Vec::new(),
        }],
        current_block: BlockId(0),
        next_block: 1,
        stack: Vec::new(),
    };

    for op in &plan.ops {
        lowerer.lower_op(op)?;
    }

    Ok(IrFunction {
        owner: plan.owner,
        name: plan.name.clone(),
        regs: lowerer.next_reg,
        locals: lowerer.locals,
        constants: lowerer.constants,
        blocks: lowerer.blocks,
    })
}

struct Lowerer {
    next_reg: u32,
    locals: u32,
    params: Vec<tune_hir::MemberId>,
    module_bindings: Vec<HirId>,
    constants: Vec<IrConst>,
    blocks: Vec<IrBlock>,
    current_block: BlockId,
    next_block: u32,
    stack: Vec<Reg>,
}

impl Lowerer {
    fn lower_op(&mut self, op: &PlanOp) -> Result<(), IrLowerError> {
        match op {
            PlanOp::ConstInt { value } => {
                let dst = self.alloc_reg()?;
                let constant = self.push_const(IrConst::Int(*value))?;
                self.push_op(IrOp::LoadConst {
                    dst,
                    constant,
                    shape: Shape::Int,
                });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::ConstBool { value } => {
                let dst = self.alloc_reg()?;
                let constant = self.push_const(IrConst::Bool(*value))?;
                self.push_op(IrOp::LoadConst {
                    dst,
                    constant,
                    shape: Shape::Bool,
                });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::BinaryOp { op: BinaryOp::Add } => {
                let rhs = self.pop("binary rhs")?;
                let lhs = self.pop("binary lhs")?;
                let dst = self.alloc_reg()?;
                self.push_op(IrOp::AddInt {
                    dst,
                    a: lhs,
                    b: rhs,
                });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::BindingGet {
                source: Some(NameTarget::Local(local)),
            } => {
                let local = local_slot(*local, local_offset(&self.module_bindings, &self.params))?;
                self.track_local(local)?;
                let dst = self.alloc_reg()?;
                self.push_op(IrOp::LoadLocal { dst, local });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::BindingGet {
                source: Some(NameTarget::Param(param)),
            } => {
                let local = param_slot(*param, &self.module_bindings, &self.params)?;
                self.track_local(local)?;
                let dst = self.alloc_reg()?;
                self.push_op(IrOp::LoadLocal { dst, local });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::BindingGet {
                source: Some(NameTarget::TopLevel(item)),
            } if self.module_bindings.contains(item) => {
                let local = module_slot(*item, &self.module_bindings)?;
                self.track_local(local)?;
                let dst = self.alloc_reg()?;
                self.push_op(IrOp::LoadLocal { dst, local });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::LocalLet {
                local: Some(local),
                initialized: true,
            } => {
                let local = local_slot(*local, local_offset(&self.module_bindings, &self.params))?;
                self.track_local(local)?;
                let value = self.pop("local initializer")?;
                self.push_op(IrOp::StoreLocal { local, value });
                Ok(())
            }
            PlanOp::LocalLet {
                local: None,
                initialized: true,
            } => Err(IrLowerError::UnsupportedOp("unresolved local initializer")),
            PlanOp::LocalLet {
                initialized: false, ..
            } => Ok(()),
            PlanOp::ModuleLet {
                item,
                initialized: true,
                keep_value,
            } => {
                let local = module_slot(*item, &self.module_bindings)?;
                self.track_local(local)?;
                let value = self.pop("module initializer")?;
                self.push_op(IrOp::StoreLocal { local, value });
                if *keep_value {
                    self.stack.push(value);
                }
                Ok(())
            }
            PlanOp::ModuleLet {
                initialized: false, ..
            } => Ok(()),
            PlanOp::Return => {
                let value = self.stack.pop();
                self.push_op(IrOp::Return { value });
                Ok(())
            }
            PlanOp::DirectCall { target, arg_count } => {
                let mut args = Vec::with_capacity(*arg_count);
                for _ in 0..*arg_count {
                    args.push(self.pop("call argument")?);
                }
                args.reverse();
                let dst = self.alloc_reg()?;
                self.push_op(IrOp::CallDirect {
                    dst,
                    function: *target,
                    args,
                });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::If {
                branches, else_ops, ..
            } => self.lower_if(branches, else_ops),
            PlanOp::BinaryOp { .. } => Err(IrLowerError::UnsupportedOp("binary op")),
            PlanOp::VariantConstruct { .. }
            | PlanOp::BindingGet { .. }
            | PlanOp::BoundCall
            | PlanOp::MemberCall { .. }
            | PlanOp::CallableValue
            | PlanOp::WitnessCall
            | PlanOp::HostCall { .. }
            | PlanOp::Assign
            | PlanOp::UnaryOp { .. }
            | PlanOp::FieldGet { .. }
            | PlanOp::FieldSet { .. }
            | PlanOp::SequenceGet { .. }
            | PlanOp::SequenceSet { .. }
            | PlanOp::SequencePush
            | PlanOp::Materialize { .. }
            | PlanOp::BindingSet { .. }
            | PlanOp::FiniteFor { .. }
            | PlanOp::StringBuild
            | PlanOp::Match { .. }
            | PlanOp::While { .. }
            | PlanOp::Loop { .. }
            | PlanOp::Break
            | PlanOp::Continue
            | PlanOp::ResultPropagate { .. }
            | PlanOp::Spawn { .. }
            | PlanOp::TaskJoin
            | PlanOp::Panic
            | PlanOp::Meta { .. } => Err(IrLowerError::UnsupportedOp("plan op")),
        }
    }

    fn alloc_reg(&mut self) -> Result<Reg, IrLowerError> {
        let reg = Reg(self.next_reg);
        self.next_reg = self
            .next_reg
            .checked_add(1)
            .ok_or(IrLowerError::RegisterLimit)?;
        Ok(reg)
    }

    fn push_const(&mut self, value: IrConst) -> Result<ConstId, IrLowerError> {
        let index = u32::try_from(self.constants.len()).map_err(|_| IrLowerError::ConstantLimit)?;
        self.constants.push(value);
        Ok(ConstId(index))
    }

    fn track_local(&mut self, local: tune_resolve::LocalId) -> Result<(), IrLowerError> {
        self.locals = self
            .locals
            .max(local.0.checked_add(1).ok_or(IrLowerError::RegisterLimit)?);
        Ok(())
    }

    fn pop(&mut self, context: &'static str) -> Result<Reg, IrLowerError> {
        self.stack
            .pop()
            .ok_or(IrLowerError::StackUnderflow(context))
    }

    fn lower_if(
        &mut self,
        branches: &[tune_plan::PlanIfBranch],
        else_ops: &[PlanOp],
    ) -> Result<(), IrLowerError> {
        let join = self.alloc_block();
        let else_block = self.alloc_block();
        let branch_blocks = branches
            .iter()
            .map(|_| self.alloc_block())
            .collect::<Vec<_>>();

        for (index, branch) in branches.iter().enumerate() {
            for op in &branch.condition_ops {
                self.lower_op(op)?;
            }
            let condition = self.pop("if condition")?;
            let then_block = branch_blocks[index];
            let false_block = branch_blocks.get(index + 1).copied().unwrap_or(else_block);
            self.push_op(IrOp::Branch {
                condition,
                then_block,
                else_block: false_block,
            });
            self.switch_to_block(then_block);
            for op in &branch.body_ops {
                self.lower_op(op)?;
            }
            if !self.current_block_returns() {
                self.push_op(IrOp::Jump { target: join });
            }
            self.switch_to_block(false_block);
        }

        for op in else_ops {
            self.lower_op(op)?;
        }
        if !self.current_block_returns() {
            self.push_op(IrOp::Jump { target: join });
        }
        self.switch_to_block(join);
        Ok(())
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

    fn push_op(&mut self, op: IrOp) {
        if let Some(block) = self
            .blocks
            .iter_mut()
            .find(|block| block.id == self.current_block)
        {
            block.ops.push(op);
        }
    }

    fn current_block_returns(&self) -> bool {
        self.blocks
            .iter()
            .find(|block| block.id == self.current_block)
            .and_then(|block| block.ops.last())
            .is_some_and(|op| matches!(op, IrOp::Return { .. }))
    }
}

fn local_offset(module_bindings: &[HirId], params: &[tune_hir::MemberId]) -> u32 {
    let offset = module_bindings.len().saturating_add(params.len());
    u32::try_from(offset).unwrap_or(u32::MAX)
}

fn local_slot(local: LocalId, offset: u32) -> Result<LocalId, IrLowerError> {
    Ok(LocalId(
        local
            .0
            .checked_add(offset)
            .ok_or(IrLowerError::RegisterLimit)?,
    ))
}

fn module_slot(item: HirId, module_bindings: &[HirId]) -> Result<LocalId, IrLowerError> {
    let index = module_bindings
        .iter()
        .position(|binding| *binding == item)
        .ok_or(IrLowerError::UnsupportedOp("module binding"))?;
    Ok(LocalId(
        u32::try_from(index).map_err(|_| IrLowerError::RegisterLimit)?,
    ))
}

fn param_slot(
    param: tune_hir::MemberId,
    module_bindings: &[HirId],
    params: &[tune_hir::MemberId],
) -> Result<LocalId, IrLowerError> {
    let index = params
        .iter()
        .position(|candidate| *candidate == param)
        .ok_or(IrLowerError::UnsupportedOp("param binding"))?;
    let slot = module_bindings
        .len()
        .checked_add(index)
        .ok_or(IrLowerError::RegisterLimit)?;
    Ok(LocalId(
        u32::try_from(slot).map_err(|_| IrLowerError::RegisterLimit)?,
    ))
}

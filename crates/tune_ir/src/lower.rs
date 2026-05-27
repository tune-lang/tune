use tune_hir::expr::BinaryOp;
use tune_plan::{PlanFunction, PlanOp};
use tune_resolve::NameTarget;
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
        constants: Vec::new(),
        ops: Vec::new(),
        stack: Vec::new(),
    };

    for op in &plan.ops {
        lowerer.lower_op(op)?;
    }

    Ok(IrFunction {
        name: plan.name.clone(),
        regs: lowerer.next_reg,
        locals: lowerer.locals,
        constants: lowerer.constants,
        blocks: vec![IrBlock {
            id: BlockId(0),
            ops: lowerer.ops,
        }],
    })
}

struct Lowerer {
    next_reg: u32,
    locals: u32,
    constants: Vec<IrConst>,
    ops: Vec<IrOp>,
    stack: Vec<Reg>,
}

impl Lowerer {
    fn lower_op(&mut self, op: &PlanOp) -> Result<(), IrLowerError> {
        match op {
            PlanOp::ConstInt { value } => {
                let dst = self.alloc_reg()?;
                let constant = self.push_const(IrConst::Int(*value))?;
                self.ops.push(IrOp::LoadConst {
                    dst,
                    constant,
                    shape: Shape::Int,
                });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::BinaryOp { op: BinaryOp::Add } => {
                let rhs = self.pop("binary rhs")?;
                let lhs = self.pop("binary lhs")?;
                let dst = self.alloc_reg()?;
                self.ops.push(IrOp::AddInt {
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
                self.track_local(*local)?;
                let dst = self.alloc_reg()?;
                self.ops.push(IrOp::LoadLocal { dst, local: *local });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::LocalLet {
                local: Some(local),
                initialized: true,
            } => {
                self.track_local(*local)?;
                let value = self.pop("local initializer")?;
                self.ops.push(IrOp::StoreLocal {
                    local: *local,
                    value,
                });
                Ok(())
            }
            PlanOp::LocalLet {
                local: None,
                initialized: true,
            } => Err(IrLowerError::UnsupportedOp("unresolved local initializer")),
            PlanOp::LocalLet {
                initialized: false, ..
            } => Ok(()),
            PlanOp::Return => {
                let value = self.stack.pop();
                self.ops.push(IrOp::Return { value });
                Ok(())
            }
            PlanOp::BinaryOp { .. } => Err(IrLowerError::UnsupportedOp("binary op")),
            PlanOp::DirectCall { .. }
            | PlanOp::VariantConstruct { .. }
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
            | PlanOp::If { .. }
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
}

use tune_hir::HirId;
use tune_hir::expr::BinaryOp;
use tune_plan::{PlanFunction, PlanOp};
use tune_resolve::NameTarget;
use tune_shape::Shape;

use crate::lower_slots::{local_offset, local_slot, module_slot, param_slot};
use crate::{BlockId, ConstId, FieldId, IrBlock, IrConst, IrFunction, IrOp, Reg, StructField};

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
        member: plan.member,
        name: plan.name.clone(),
        params: u32::try_from(plan.params.len()).map_err(|_| IrLowerError::RegisterLimit)?,
        regs: lowerer.next_reg,
        locals: lowerer.locals,
        constants: lowerer.constants,
        blocks: lowerer.blocks,
    })
}

pub(super) struct Lowerer {
    pub(super) next_reg: u32,
    pub(super) locals: u32,
    pub(super) params: Vec<tune_hir::MemberId>,
    pub(super) module_bindings: Vec<HirId>,
    pub(super) constants: Vec<IrConst>,
    pub(super) blocks: Vec<IrBlock>,
    pub(super) current_block: BlockId,
    pub(super) next_block: u32,
    pub(super) stack: Vec<Reg>,
}

impl Lowerer {
    pub(super) fn lower_op(&mut self, op: &PlanOp) -> Result<(), IrLowerError> {
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
            PlanOp::BinaryOp {
                op: BinaryOp::Greater,
            } => {
                let rhs = self.pop("binary rhs")?;
                let lhs = self.pop("binary lhs")?;
                let dst = self.alloc_reg()?;
                self.push_op(IrOp::GreaterInt {
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
                source: Some(NameTarget::SelfValue),
            } => {
                let local = tune_resolve::LocalId(0);
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
            PlanOp::BindingSet {
                target: Some(NameTarget::Local(local)),
            } => {
                let local = local_slot(*local, local_offset(&self.module_bindings, &self.params))?;
                self.track_local(local)?;
                let value = self.pop("local assignment")?;
                self.push_op(IrOp::StoreLocal { local, value });
                Ok(())
            }
            PlanOp::Return => {
                let value = self.stack.pop();
                self.push_op(IrOp::Return { value });
                Ok(())
            }
            PlanOp::DirectCall { target, arg_count } => self.lower_direct_call(*target, *arg_count),
            PlanOp::MemberCall {
                member: Some(member),
                arg_count,
                ..
            } => self.lower_member_call(*member, *arg_count),
            PlanOp::MemberCall { member: None, .. } => {
                Err(IrLowerError::UnsupportedOp("unresolved member call"))
            }
            PlanOp::VariantConstruct { variant, arg_count } => {
                let mut args = Vec::with_capacity(*arg_count);
                for _ in 0..*arg_count {
                    args.push(self.pop("variant argument")?);
                }
                args.reverse();
                let dst = self.alloc_reg()?;
                self.push_op(IrOp::VariantConstruct {
                    dst,
                    variant: *variant,
                    args,
                });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::StructConstruct {
                item,
                state,
                fields,
            } => {
                let mut values = Vec::with_capacity(fields.len());
                for field in fields.iter().rev() {
                    values.push(StructField {
                        field: FieldId(field.index),
                        value: self.pop("struct field initializer")?,
                    });
                }
                values.reverse();
                let dst = self.alloc_reg()?;
                self.push_op(IrOp::StructConstruct {
                    dst,
                    item: *item,
                    state: crate::lower_state::lower_struct_state(*state),
                    fields: values,
                });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::FieldGet {
                member: Some(member),
                ..
            } => {
                let base = self.pop("field base")?;
                let dst = self.alloc_reg()?;
                self.push_op(IrOp::GetField {
                    dst,
                    base,
                    field: FieldId(member.index),
                });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::FieldSet {
                member: Some(member),
                base: base_target,
                ..
            } => {
                let value = self.pop("field value")?;
                let base = self.pop("field base")?;
                self.push_op(IrOp::SetField {
                    base,
                    field: FieldId(member.index),
                    value,
                });
                if let Some(target) = base_target {
                    self.store_binding_target(*target, base)?;
                }
                Ok(())
            }
            PlanOp::FieldGet { member: None, .. } => {
                Err(IrLowerError::UnsupportedOp("unresolved field get"))
            }
            PlanOp::FieldSet { member: None, .. } => {
                Err(IrLowerError::UnsupportedOp("unresolved field set"))
            }
            PlanOp::ResultPropagate { expr, span } => {
                let result = self.pop("result propagation")?;
                let dst = self.alloc_reg()?;
                self.push_op(IrOp::ResultPropagate {
                    dst,
                    result,
                    expr: *expr,
                    span: *span,
                });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::Spawn { .. } => self.lower_spawn(),
            PlanOp::TaskJoin => self.lower_task_join(),
            PlanOp::If {
                branches, else_ops, ..
            } => self.lower_if(
                branches,
                else_ops,
                matches!(
                    op,
                    PlanOp::If {
                        produces_value: true,
                        ..
                    }
                ),
            ),
            PlanOp::Match {
                arms,
                produces_value,
                ..
            } => self.lower_match(arms, *produces_value),
            PlanOp::BinaryOp { .. } => Err(IrLowerError::UnsupportedOp("binary op")),
            PlanOp::BindingGet { .. }
            | PlanOp::BoundCall
            | PlanOp::CallableValue
            | PlanOp::WitnessCall
            | PlanOp::HostCall { .. }
            | PlanOp::Assign
            | PlanOp::UnaryOp { .. }
            | PlanOp::SequenceGet { .. }
            | PlanOp::SequenceSet { .. }
            | PlanOp::SequencePush
            | PlanOp::Materialize { .. }
            | PlanOp::BindingSet { .. }
            | PlanOp::FiniteFor { .. }
            | PlanOp::StringBuild
            | PlanOp::While { .. }
            | PlanOp::Loop { .. }
            | PlanOp::Break
            | PlanOp::Continue
            | PlanOp::Panic
            | PlanOp::Meta { .. } => Err(IrLowerError::UnsupportedOp("plan op")),
        }
    }

    pub(super) fn alloc_reg(&mut self) -> Result<Reg, IrLowerError> {
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

    pub(super) fn track_local(&mut self, local: tune_resolve::LocalId) -> Result<(), IrLowerError> {
        self.locals = self
            .locals
            .max(local.0.checked_add(1).ok_or(IrLowerError::RegisterLimit)?);
        Ok(())
    }

    pub(super) fn pop(&mut self, context: &'static str) -> Result<Reg, IrLowerError> {
        self.stack
            .pop()
            .ok_or(IrLowerError::StackUnderflow(context))
    }

    fn store_binding_target(&mut self, target: NameTarget, value: Reg) -> Result<(), IrLowerError> {
        match target {
            NameTarget::Local(local) => {
                let local = local_slot(local, local_offset(&self.module_bindings, &self.params))?;
                self.track_local(local)?;
                self.push_op(IrOp::StoreLocal { local, value });
                Ok(())
            }
            NameTarget::Param(param) => {
                let local = param_slot(param, &self.module_bindings, &self.params)?;
                self.track_local(local)?;
                self.push_op(IrOp::StoreLocal { local, value });
                Ok(())
            }
            NameTarget::TopLevel(item) if self.module_bindings.contains(&item) => {
                let local = module_slot(item, &self.module_bindings)?;
                self.track_local(local)?;
                self.push_op(IrOp::StoreLocal { local, value });
                Ok(())
            }
            NameTarget::SelfValue => {
                let local = tune_resolve::LocalId(0);
                self.track_local(local)?;
                self.push_op(IrOp::StoreLocal { local, value });
                Ok(())
            }
            NameTarget::TopLevel(_) | NameTarget::Variant(_) => Ok(()),
        }
    }
}

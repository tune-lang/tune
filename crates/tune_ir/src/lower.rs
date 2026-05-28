use tune_hir::HirId;
use tune_plan::{PlanFunction, PlanOp};
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
        loop_targets: Vec::new(),
    };

    for op in &plan.ops {
        lowerer.lower_op(op)?;
    }

    Ok(IrFunction {
        owner: plan.owner,
        member: plan.member,
        name: plan.name.clone(),
        span: plan.span,
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
    pub(super) loop_targets: Vec<(BlockId, BlockId)>,
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
            PlanOp::BinaryOp { op, span } => self.lower_binary(*op, *span),
            PlanOp::UnaryOp { op } => self.lower_unary(*op),
            PlanOp::BindingGet {
                source: Some(source),
            } => self.lower_binding_get(*source),
            PlanOp::LocalLet { local, initialized } => self.lower_local_let(*local, *initialized),
            PlanOp::ModuleLet {
                item,
                initialized,
                keep_value,
            } => self.lower_module_let(*item, *initialized, *keep_value),
            PlanOp::BindingSet { target } => self.lower_binding_set(*target),
            PlanOp::Return => {
                let value = self.stack.pop();
                self.push_op(IrOp::Return { value });
                Ok(())
            }
            PlanOp::DirectCall {
                target,
                arg_count,
                span,
            } => self.lower_direct_call(*target, *arg_count, *span),
            PlanOp::MemberCall {
                member: Some(member),
                arg_count,
                span,
                ..
            } => self.lower_member_call(*member, *arg_count, *span),
            PlanOp::MemberCall { member: None, .. } => {
                Err(IrLowerError::UnsupportedOp("unresolved member call"))
            }
            PlanOp::VariantConstruct {
                variant,
                arg_count,
                span,
            } => self.lower_variant_construct(*variant, *arg_count, *span),
            PlanOp::StructConstruct {
                item,
                state,
                fields,
                span,
            } => self.lower_struct_construct(*item, *state, fields, *span),
            PlanOp::FieldGet { member, span, .. } => self.lower_field_get(*member, *span),
            PlanOp::FieldSet {
                member,
                base: base_target,
                span,
                ..
            } => self.lower_field_set(*member, *base_target, *span),
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
            PlanOp::Spawn { span, .. } => self.lower_spawn(*span),
            PlanOp::TaskJoin { span } => self.lower_task_join(*span),
            PlanOp::If {
                branches,
                else_ops,
                span,
                ..
            } => self.lower_if(
                branches,
                else_ops,
                *span,
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
                span,
                ..
            } => self.lower_match(arms, *produces_value, *span),
            PlanOp::While {
                condition_ops,
                body_ops,
                span,
                ..
            } => self.lower_while(condition_ops, body_ops, *span),
            PlanOp::Loop { body_ops, .. } => self.lower_loop(body_ops),
            PlanOp::Break => self.lower_break(),
            PlanOp::Continue => self.lower_continue(),
            PlanOp::BindingGet { .. }
            | PlanOp::BoundCall
            | PlanOp::CallableValue
            | PlanOp::WitnessCall
            | PlanOp::HostCall { .. }
            | PlanOp::Assign
            | PlanOp::SequenceGet { .. }
            | PlanOp::SequenceSet { .. }
            | PlanOp::SequencePush
            | PlanOp::Materialize { .. }
            | PlanOp::FiniteFor { .. }
            | PlanOp::StringBuild
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
}

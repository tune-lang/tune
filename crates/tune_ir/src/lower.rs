use tune_diagnostics::Span;
use tune_hir::HirId;
use tune_plan::{Capture, PlanFunction, PlanOp};
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
    let param_count = u32::try_from(
        plan.captures
            .len()
            .saturating_add(plan.params.len())
            .saturating_add(plan.local_params.len()),
    )
    .map_err(|_| IrLowerError::RegisterLimit)?;
    let mut lowerer = Lowerer {
        next_reg: 0,
        locals: param_count,
        params: plan.params.clone(),
        local_params: plan.local_params.clone(),
        captures: plan.captures.clone(),
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
        task_functions: Vec::new(),
    };

    for op in &plan.ops {
        lowerer.lower_op(op)?;
    }

    Ok(IrFunction {
        owner: plan.owner,
        member: plan.member,
        callable: plan.callable,
        name: plan.name.clone(),
        span: plan.span,
        params: param_count,
        regs: lowerer.next_reg,
        locals: lowerer.locals,
        constants: lowerer.constants,
        blocks: lowerer.blocks,
        task_functions: lowerer.task_functions,
    })
}

pub(super) struct Lowerer {
    pub(super) next_reg: u32,
    pub(super) locals: u32,
    pub(super) params: Vec<tune_hir::MemberId>,
    pub(super) local_params: Vec<tune_resolve::LocalId>,
    pub(super) captures: Vec<Capture>,
    pub(super) module_bindings: Vec<HirId>,
    pub(super) constants: Vec<IrConst>,
    pub(super) blocks: Vec<IrBlock>,
    pub(super) current_block: BlockId,
    pub(super) next_block: u32,
    pub(super) stack: Vec<Reg>,
    pub(super) loop_targets: Vec<(BlockId, BlockId)>,
    pub(super) task_functions: Vec<IrFunction>,
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
            PlanOp::ConstFloat { bits } => {
                let dst = self.alloc_reg()?;
                let constant = self.push_const(IrConst::Float(f64::from_bits(*bits)))?;
                self.push_op(IrOp::LoadConst {
                    dst,
                    constant,
                    shape: Shape::Float,
                });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::ConstSize { value } => {
                let dst = self.alloc_reg()?;
                let constant = self.push_const(IrConst::Size(*value))?;
                self.push_op(IrOp::LoadConst {
                    dst,
                    constant,
                    shape: Shape::Size,
                });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::ConstByte { value } => {
                let dst = self.alloc_reg()?;
                let constant = self.push_const(IrConst::Byte(*value))?;
                self.push_op(IrOp::LoadConst {
                    dst,
                    constant,
                    shape: Shape::Byte,
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
            PlanOp::ConstNone => {
                let dst = self.alloc_reg()?;
                let constant = self.push_const(IrConst::None)?;
                self.push_op(IrOp::LoadConst {
                    dst,
                    constant,
                    shape: Shape::Optional(Box::new(Shape::Hole)),
                });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::ConstString { value } => {
                let dst = self.alloc_reg()?;
                let constant = self.push_const(IrConst::String(value.clone()))?;
                self.push_op(IrOp::LoadConst {
                    dst,
                    constant,
                    shape: Shape::String,
                });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::BinaryOp { op, shape, span } => self.lower_binary(*op, shape, *span),
            PlanOp::BoolAnd {
                lhs_ops,
                rhs_ops,
                span,
            } => self.lower_bool_and(lhs_ops, rhs_ops, *span),
            PlanOp::BoolOr {
                lhs_ops,
                rhs_ops,
                span,
            } => self.lower_bool_or(lhs_ops, rhs_ops, *span),
            PlanOp::NoneCheck { is_not, span } => {
                let value = self.pop("none check value")?;
                let dst = self.alloc_reg()?;
                self.push_op(IrOp::NoneCheck {
                    dst,
                    value,
                    is_not: *is_not,
                    span: *span,
                });
                self.stack.push(dst);
                Ok(())
            }
            PlanOp::UnaryOp { op, shape } => self.lower_unary(*op, shape),
            PlanOp::SequenceBuild { element_count } => self.lower_sequence_build(*element_count),
            PlanOp::SequencePush => self.lower_sequence_push(),
            PlanOp::TupleBuild { element_count } => self.lower_tuple_build(*element_count),
            PlanOp::SequenceGet {
                checked,
                index_member,
            } => self.lower_sequence_get(*checked, *index_member),
            PlanOp::SequenceSet {
                checked,
                index_member,
                base,
            } => self.lower_sequence_set(*checked, *index_member, *base),
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
            PlanOp::CallableValue {
                callable,
                captures,
                span,
            } => self.lower_callable_value(*callable, captures, *span),
            PlanOp::BoundCall { arg_count, span } => self.lower_bound_call(*arg_count, *span),
            PlanOp::Materialize {
                materializer: Some(member),
                ..
            } => self.lower_materialize(*member),
            PlanOp::Materialize {
                materializer: None, ..
            } => Ok(()),
            PlanOp::VariantConstruct {
                variant,
                arg_count,
                span,
            } => self.lower_variant_construct(*variant, *arg_count, *span),
            PlanOp::StructConstruct {
                item,
                escape: _,
                state,
                fields,
                span,
            } => self.lower_struct_construct(*item, *state, fields, *span),
            PlanOp::StructIs { item, span } => {
                let value = self.pop("struct test value")?;
                let dst = self.alloc_reg()?;
                self.push_op(IrOp::StructIs {
                    dst,
                    value,
                    item: *item,
                    span: *span,
                });
                self.stack.push(dst);
                Ok(())
            }
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
            PlanOp::Spawn { body_ops, span, .. } => self.lower_spawn(body_ops, *span),
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
            PlanOp::FiniteFor {
                binding,
                iterable_ops,
                body_ops,
                contract,
                span,
                ..
            } => self.lower_finite_for(*binding, iterable_ops, body_ops, contract, *span),
            PlanOp::Panic { arg_count, span } => self.lower_panic(*arg_count, *span),
            PlanOp::StringBuild { part_count } => self.lower_string_build(*part_count),
            PlanOp::StringLen { span } => self.lower_string_len(*span),
            PlanOp::StringGet { span } => self.lower_string_get(*span),
            PlanOp::BindingGet { .. }
            | PlanOp::WitnessCall
            | PlanOp::HostCall { .. }
            | PlanOp::Assign
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

    pub(super) fn push_const(&mut self, value: IrConst) -> Result<ConstId, IrLowerError> {
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

    fn lower_panic(
        &mut self,
        arg_count: usize,
        span: Option<tune_diagnostics::Span>,
    ) -> Result<(), IrLowerError> {
        let mut args = Vec::with_capacity(arg_count);
        for _ in 0..arg_count {
            args.push(self.pop("panic argument")?);
        }
        args.reverse();
        self.push_op(IrOp::Panic { args, span });
        let never = self.alloc_reg()?;
        self.stack.push(never);
        Ok(())
    }

    fn lower_string_build(&mut self, part_count: usize) -> Result<(), IrLowerError> {
        let mut parts = Vec::with_capacity(part_count);
        for _ in 0..part_count {
            parts.push(self.pop("string build part")?);
        }
        parts.reverse();
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::StringBuild { dst, parts });
        self.stack.push(dst);
        Ok(())
    }

    fn lower_string_len(&mut self, span: Option<Span>) -> Result<(), IrLowerError> {
        let value = self.pop("string len value")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::StringLen { dst, value, span });
        self.stack.push(dst);
        Ok(())
    }

    fn lower_string_get(&mut self, span: Option<Span>) -> Result<(), IrLowerError> {
        let index = self.pop("string index")?;
        let value = self.pop("string value")?;
        let dst = self.alloc_reg()?;
        self.push_op(IrOp::StringGet {
            dst,
            value,
            index,
            span,
        });
        self.stack.push(dst);
        Ok(())
    }
}

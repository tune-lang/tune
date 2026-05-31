use crate::Opcode;
use crate::function::{
    BytecodeFieldSite, BytecodePanicSite, BytecodeStructField, BytecodeStructSite,
    BytecodeTupleSite, BytecodeVariantSite, Instruction,
};
use crate::lower::control::FiniteForNextLowering;
use crate::lower::{BytecodeLowerError, FunctionLowerer};
use crate::lower_tables::{lower_variant, push_artifact_const};
use tune_ir::IrOp;

impl FunctionLowerer<'_> {
    pub(super) fn lower_op(&mut self, op: &IrOp) -> Result<(), BytecodeLowerError> {
        match op {
            IrOp::LoadConst { dst, constant, .. } => {
                let constant = self
                    .function
                    .constants
                    .get(constant.0 as usize)
                    .ok_or(BytecodeLowerError::ConstantLimit)?;
                let artifact_const = push_artifact_const(self.constants, constant)?;
                self.instructions.push(Instruction {
                    opcode: Opcode::LoadConst,
                    a: dst.0,
                    b: artifact_const,
                    c: 0,
                });
                Ok(())
            }
            IrOp::AddInt { .. }
            | IrOp::SubInt { .. }
            | IrOp::MulInt { .. }
            | IrOp::DivInt { .. }
            | IrOp::RemInt { .. }
            | IrOp::BitAndInt { .. }
            | IrOp::BitOrInt { .. }
            | IrOp::BitXorInt { .. }
            | IrOp::ShiftLeftInt { .. }
            | IrOp::ShiftRightInt { .. }
            | IrOp::AddFloat { .. }
            | IrOp::SubFloat { .. }
            | IrOp::MulFloat { .. }
            | IrOp::DivFloat { .. }
            | IrOp::AddSizeChecked { .. }
            | IrOp::SubSizeChecked { .. }
            | IrOp::MulSizeChecked { .. }
            | IrOp::DivSize { .. }
            | IrOp::RemSize { .. }
            | IrOp::BitAndSize { .. }
            | IrOp::BitOrSize { .. }
            | IrOp::BitXorSize { .. }
            | IrOp::ShiftLeftSize { .. }
            | IrOp::ShiftRightSize { .. }
            | IrOp::AddByteWrap { .. }
            | IrOp::NegInt { .. }
            | IrOp::NotBool { .. }
            | IrOp::BitNotInt { .. }
            | IrOp::BitNotSize { .. }
            | IrOp::NoneCheck { .. }
            | IrOp::GreaterInt { .. }
            | IrOp::GreaterFloat { .. }
            | IrOp::GreaterSize { .. }
            | IrOp::CompareInt { .. }
            | IrOp::CompareFloat { .. }
            | IrOp::CompareSize { .. }
            | IrOp::ByteBinary { .. } => self.lower_numeric_op(op),
            IrOp::RangeInt {
                dst,
                start,
                end,
                inclusive,
                ..
            } => {
                self.lower_range_int(*dst, *start, *end, *inclusive);
                Ok(())
            }
            IrOp::SeqBuild { dst, .. } => {
                self.lower_seq_build(*dst);
                Ok(())
            }
            IrOp::SeqPush { seq, value, mode } => {
                self.lower_seq_push(*seq, *value, mode);
                Ok(())
            }
            IrOp::SeqGet {
                dst,
                seq,
                index,
                checked,
            } => {
                self.lower_seq_get(*dst, *seq, *index, *checked);
                Ok(())
            }
            IrOp::SeqSet {
                seq,
                index,
                value,
                checked,
                mode,
            } => {
                self.lower_seq_set(*seq, *index, *value, *checked, mode);
                Ok(())
            }
            IrOp::Move { dst, src, .. } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::Move,
                    a: dst.0,
                    b: src.0,
                    c: 0,
                });
                Ok(())
            }
            IrOp::LoadLocal { dst, local, .. } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::LoadLocal,
                    a: dst.0,
                    b: local.0,
                    c: 0,
                });
                Ok(())
            }
            IrOp::StoreLocal { local, value, .. } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::StoreLocal,
                    a: local.0,
                    b: value.0,
                    c: 0,
                });
                Ok(())
            }
            IrOp::TupleBuild { dst, items } => {
                let site = u32::try_from(self.tuple_sites.len())
                    .map_err(|_| BytecodeLowerError::ConstantLimit)?;
                self.tuple_sites.push(BytecodeTupleSite {
                    items: items.iter().map(|item| item.0).collect(),
                });
                self.instructions.push(Instruction {
                    opcode: Opcode::TupleBuild,
                    a: dst.0,
                    b: site,
                    c: 0,
                });
                Ok(())
            }
            IrOp::GetField {
                dst,
                base,
                owner,
                field,
                ..
            } => {
                let site = u32::try_from(self.field_sites.len())
                    .map_err(|_| BytecodeLowerError::ConstantLimit)?;
                self.field_sites.push(BytecodeFieldSite {
                    owner: owner.0,
                    field: field.0,
                });
                self.instructions.push(Instruction {
                    opcode: Opcode::FieldGet,
                    a: dst.0,
                    b: base.0,
                    c: site,
                });
                Ok(())
            }
            IrOp::SetField {
                base,
                owner,
                field,
                value,
                ..
            } => {
                let site = u32::try_from(self.field_sites.len())
                    .map_err(|_| BytecodeLowerError::ConstantLimit)?;
                self.field_sites.push(BytecodeFieldSite {
                    owner: owner.0,
                    field: field.0,
                });
                self.instructions.push(Instruction {
                    opcode: Opcode::FieldSet,
                    a: base.0,
                    b: site,
                    c: value.0,
                });
                Ok(())
            }
            IrOp::CallDirect {
                dst,
                function,
                args,
                type_args,
                generic_strategy,
                ..
            } => self.lower_direct_call(*dst, *function, args, type_args, *generic_strategy),
            IrOp::CallMember {
                dst, member, args, ..
            } => self.lower_member_call(*dst, *member, args),
            IrOp::CallableValue {
                dst,
                callable,
                captures,
                ..
            } => self.lower_callable_value(*dst, *callable, captures),
            IrOp::CallBound {
                dst, callee, args, ..
            } => self.lower_bound_call(*dst, *callee, args),
            IrOp::CallHost {
                dst,
                symbol,
                task_safe,
                args,
            } => self.lower_host_call(*dst, *symbol, *task_safe, args),
            IrOp::VariantConstruct {
                dst, variant, args, ..
            } => {
                let variant_site = u32::try_from(self.variant_sites.len())
                    .map_err(|_| BytecodeLowerError::ConstantLimit)?;
                self.variant_sites.push(BytecodeVariantSite {
                    variant: lower_variant(*variant),
                    args: args.iter().map(|arg| arg.0).collect(),
                });
                self.instructions.push(Instruction {
                    opcode: Opcode::VariantConstruct,
                    a: dst.0,
                    b: variant_site,
                    c: 0,
                });
                Ok(())
            }
            IrOp::StructConstruct {
                dst,
                item,
                state,
                fields,
                ..
            } => {
                let site = u32::try_from(self.struct_sites.len())
                    .map_err(|_| BytecodeLowerError::ConstantLimit)?;
                self.struct_sites.push(BytecodeStructSite {
                    owner: item.0,
                    state: crate::lower_state::lower_struct_state(*state),
                    fields: fields
                        .iter()
                        .map(|field| BytecodeStructField {
                            field: field.field.0,
                            value: field.value.0,
                        })
                        .collect(),
                });
                self.instructions.push(Instruction {
                    opcode: Opcode::StructConstruct,
                    a: dst.0,
                    b: site,
                    c: 0,
                });
                Ok(())
            }
            IrOp::StructIs {
                dst, value, item, ..
            } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::StructIs,
                    a: dst.0,
                    b: value.0,
                    c: item.0,
                });
                Ok(())
            }
            IrOp::VariantField { dst, base, index } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::VariantField,
                    a: dst.0,
                    b: base.0,
                    c: *index,
                });
                Ok(())
            }
            IrOp::TupleField { dst, base, index } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::TupleField,
                    a: dst.0,
                    b: base.0,
                    c: *index,
                });
                Ok(())
            }
            IrOp::ResultPropagate { dst, result, .. } => {
                self.instructions.push(Instruction {
                    opcode: Opcode::ResultPropagate,
                    a: dst.0,
                    b: result.0,
                    c: 0,
                });
                Ok(())
            }
            IrOp::Spawn {
                dst,
                function,
                captures,
                ..
            } => self.lower_spawn(*dst, *function, captures),
            IrOp::TaskJoin { dst, task, .. } => {
                self.lower_task_join(*dst, *task);
                Ok(())
            }
            IrOp::Panic { args, .. } => {
                let site = u32::try_from(self.panic_sites.len())
                    .map_err(|_| BytecodeLowerError::ConstantLimit)?;
                self.panic_sites.push(BytecodePanicSite {
                    args: args.iter().map(|arg| arg.0).collect(),
                });
                self.push_instruction(Opcode::Panic, site, 0, 0);
                Ok(())
            }
            IrOp::StringBuild { dst, parts } => self.lower_string_build(*dst, parts),
            IrOp::StringLen { dst, value, .. } => {
                self.push_instruction(Opcode::StringLen, dst.0, value.0, 0);
                Ok(())
            }
            IrOp::SequenceLen { dst, value, .. } => {
                self.push_instruction(Opcode::SeqLen, dst.0, value.0, 0);
                Ok(())
            }
            IrOp::StringGet {
                dst, value, index, ..
            } => {
                self.push_instruction(Opcode::StringGet, dst.0, value.0, index.0);
                Ok(())
            }
            IrOp::Jump { target } => {
                self.lower_jump(*target)?;
                Ok(())
            }
            IrOp::Branch {
                condition,
                then_block,
                else_block,
                ..
            } => {
                self.lower_branch(*condition, *then_block, *else_block)?;
                Ok(())
            }
            IrOp::MatchVariant {
                scrutinee,
                arms,
                else_block,
                ..
            } => {
                self.lower_match_variant(*scrutinee, arms, *else_block)?;
                Ok(())
            }
            IrOp::FiniteForInit {
                iterator,
                iterable,
                len,
            } => {
                self.lower_finite_for_init(*iterator, *iterable, *len);
                Ok(())
            }
            IrOp::FiniteForNext {
                iterator,
                iterable,
                len,
                index,
                item,
                body,
                done,
            } => {
                self.lower_finite_for_next(FiniteForNextLowering {
                    iterator: *iterator,
                    iterable: *iterable,
                    len: *len,
                    index: *index,
                    item: *item,
                    body: *body,
                    done: *done,
                })?;
                Ok(())
            }
            IrOp::Return { value: Some(value) } => {
                self.push_instruction(Opcode::Return, value.0, 1, 0);
                Ok(())
            }
            IrOp::Return { value: None } => {
                self.push_instruction(Opcode::Return, 0, 0, 0);
                Ok(())
            }
            _ => Err(BytecodeLowerError::UnsupportedIr("ir op")),
        }
    }
}

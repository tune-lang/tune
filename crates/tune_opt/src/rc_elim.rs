use std::collections::HashMap;

use tune_ir::{IrFunction, IrMutationMode, IrOp, IrTransfer, LocalId, Reg};

use crate::{Pass, PassReport};

#[must_use]
pub fn run(function: &mut IrFunction) -> PassReport {
    let changed = eliminate_function(function);
    PassReport {
        pass: Pass::RcElim,
        changed,
    }
}

fn eliminate_function(function: &mut IrFunction) -> bool {
    let mut changed = false;
    for block in &mut function.blocks {
        changed |= eliminate_block(&mut block.ops);
    }
    for task in &mut function.task_functions {
        changed |= eliminate_function(task);
    }
    changed
}

fn eliminate_block(ops: &mut [IrOp]) -> bool {
    let mut regs = HashMap::<Reg, SequenceOrigin>::new();
    let mut locals = HashMap::<LocalId, bool>::new();
    let mut changed = false;

    for op in ops {
        match op {
            IrOp::SeqBuild { dst, .. } => {
                regs.insert(*dst, SequenceOrigin::Fresh);
            }
            IrOp::SeqPush { seq, value, mode } => {
                if regs.contains_key(seq) && *mode == IrMutationMode::SharedCow {
                    *mode = IrMutationMode::Exclusive;
                    changed = true;
                }
                invalidate_mutation_input(*seq, *value, &mut locals, &mut regs);
            }
            IrOp::SeqSet {
                seq,
                index,
                value,
                mode,
                ..
            } => {
                if regs.contains_key(seq) && *mode == IrMutationMode::SharedCow {
                    *mode = IrMutationMode::Exclusive;
                    changed = true;
                }
                invalidate_mutation_input(*seq, *index, &mut locals, &mut regs);
                invalidate_mutation_input(*seq, *value, &mut locals, &mut regs);
            }
            IrOp::LoadLocal { dst, local, .. } => {
                if locals.get(local).copied().unwrap_or(false)
                    && !regs
                        .values()
                        .any(|origin| *origin == SequenceOrigin::Local(*local))
                {
                    regs.insert(*dst, SequenceOrigin::Local(*local));
                } else {
                    locals.insert(*local, false);
                    regs.remove(dst);
                }
            }
            IrOp::StoreLocal { local, value, .. } => match regs.get(value).copied() {
                Some(SequenceOrigin::Fresh) => {
                    locals.insert(*local, true);
                }
                Some(SequenceOrigin::Local(origin))
                    if origin == *local && locals.get(local).copied().unwrap_or(false) =>
                {
                    locals.insert(*local, true);
                }
                _ => {
                    locals.insert(*local, false);
                }
            },
            IrOp::Move { dst, src, transfer } => {
                let origin = regs.get(src).copied();
                if *transfer == IrTransfer::Move {
                    if let Some(origin) = origin {
                        regs.insert(*dst, origin);
                    } else {
                        regs.remove(dst);
                    }
                } else {
                    regs.remove(dst);
                    match origin {
                        Some(SequenceOrigin::Fresh) => {
                            regs.remove(src);
                        }
                        Some(SequenceOrigin::Local(local)) => {
                            invalidate_local(local, &mut locals, &mut regs);
                        }
                        None => {}
                    }
                }
            }
            _ => {
                for input in input_regs(op) {
                    invalidate_reg(input, &mut locals, &mut regs);
                }
                for output in output_regs(op) {
                    regs.remove(&output);
                }
            }
        }
    }

    changed
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SequenceOrigin {
    Fresh,
    Local(LocalId),
}

fn invalidate_local(
    local: LocalId,
    locals: &mut HashMap<LocalId, bool>,
    regs: &mut HashMap<Reg, SequenceOrigin>,
) {
    locals.insert(local, false);
    regs.retain(|_, origin| *origin != SequenceOrigin::Local(local));
}

fn invalidate_mutation_input(
    mutated: Reg,
    input: Reg,
    locals: &mut HashMap<LocalId, bool>,
    regs: &mut HashMap<Reg, SequenceOrigin>,
) {
    if input != mutated {
        invalidate_reg(input, locals, regs);
    }
}

fn invalidate_reg(
    reg: Reg,
    locals: &mut HashMap<LocalId, bool>,
    regs: &mut HashMap<Reg, SequenceOrigin>,
) {
    match regs.get(&reg).copied() {
        Some(SequenceOrigin::Fresh) => {
            regs.remove(&reg);
        }
        Some(SequenceOrigin::Local(local)) => {
            invalidate_local(local, locals, regs);
        }
        None => {}
    }
}

fn input_regs(op: &IrOp) -> Vec<Reg> {
    match op {
        IrOp::TupleBuild { items, .. } => items.clone(),
        IrOp::VariantConstruct { args, .. }
        | IrOp::CallDirect { args, .. }
        | IrOp::CallMember { args, .. }
        | IrOp::CallBound { args, .. }
        | IrOp::CallWitness { args, .. }
        | IrOp::CallHost { args, .. }
        | IrOp::Panic { args, .. } => args.clone(),
        IrOp::StructConstruct { fields, .. } => fields.iter().map(|field| field.value).collect(),
        IrOp::GetField { base, .. }
        | IrOp::VariantField { base, .. }
        | IrOp::TupleField { base, .. }
        | IrOp::TaskJoin { task: base, .. }
        | IrOp::StringLen { value: base, .. } => vec![*base],
        IrOp::SetField { base, value, .. } => vec![*base, *value],
        IrOp::AddInt { a, b, .. }
        | IrOp::SubInt { a, b, .. }
        | IrOp::MulInt { a, b, .. }
        | IrOp::DivInt { a, b, .. }
        | IrOp::RemInt { a, b, .. }
        | IrOp::BitAndInt { a, b, .. }
        | IrOp::BitOrInt { a, b, .. }
        | IrOp::BitXorInt { a, b, .. }
        | IrOp::ShiftLeftInt { a, b, .. }
        | IrOp::ShiftRightInt { a, b, .. }
        | IrOp::GreaterInt { a, b, .. }
        | IrOp::CompareInt { a, b, .. }
        | IrOp::AddFloat { a, b, .. }
        | IrOp::SubFloat { a, b, .. }
        | IrOp::MulFloat { a, b, .. }
        | IrOp::DivFloat { a, b, .. }
        | IrOp::GreaterFloat { a, b, .. }
        | IrOp::CompareFloat { a, b, .. }
        | IrOp::AddSizeChecked { a, b, .. }
        | IrOp::SubSizeChecked { a, b, .. }
        | IrOp::MulSizeChecked { a, b, .. }
        | IrOp::DivSize { a, b, .. }
        | IrOp::RemSize { a, b, .. }
        | IrOp::BitAndSize { a, b, .. }
        | IrOp::BitOrSize { a, b, .. }
        | IrOp::BitXorSize { a, b, .. }
        | IrOp::ShiftLeftSize { a, b, .. }
        | IrOp::ShiftRightSize { a, b, .. }
        | IrOp::GreaterSize { a, b, .. }
        | IrOp::CompareSize { a, b, .. }
        | IrOp::AddByteWrap { a, b, .. }
        | IrOp::ByteBinary { a, b, .. }
        | IrOp::StringGet {
            value: a, index: b, ..
        }
        | IrOp::RangeInt {
            start: a, end: b, ..
        } => vec![*a, *b],
        IrOp::NegInt { value, .. }
        | IrOp::NotBool { value, .. }
        | IrOp::BitNotInt { value, .. }
        | IrOp::BitNotSize { value, .. }
        | IrOp::NoneCheck { value, .. }
        | IrOp::StructIs { value, .. }
        | IrOp::ResultPropagate { result: value, .. } => vec![*value],
        IrOp::Spawn { captures, .. } => captures.iter().map(|capture| capture.reg).collect(),
        IrOp::Branch { condition, .. } => vec![*condition],
        IrOp::MatchVariant { scrutinee, .. } => vec![*scrutinee],
        IrOp::CallableValue { captures, .. } => {
            captures.iter().map(|capture| capture.reg).collect()
        }
        IrOp::FiniteForInit { iterable, .. } => vec![*iterable],
        IrOp::FiniteForNext {
            iterator,
            iterable,
            len,
            ..
        } => vec![*iterator, *iterable, *len],
        IrOp::StringBuild { parts, .. } => parts.clone(),
        IrOp::Return { value } => value.iter().copied().collect(),
        IrOp::LoadConst { .. }
        | IrOp::LoadLocal { .. }
        | IrOp::StoreLocal { .. }
        | IrOp::Move { .. }
        | IrOp::SeqBuild { .. }
        | IrOp::SeqPush { .. }
        | IrOp::SeqGet { .. }
        | IrOp::SeqSet { .. }
        | IrOp::Jump { .. } => Vec::new(),
    }
}

fn output_regs(op: &IrOp) -> Vec<Reg> {
    match op {
        IrOp::LoadConst { dst, .. }
        | IrOp::LoadLocal { dst, .. }
        | IrOp::Move { dst, .. }
        | IrOp::AddInt { dst, .. }
        | IrOp::SubInt { dst, .. }
        | IrOp::MulInt { dst, .. }
        | IrOp::DivInt { dst, .. }
        | IrOp::RemInt { dst, .. }
        | IrOp::BitAndInt { dst, .. }
        | IrOp::BitOrInt { dst, .. }
        | IrOp::BitXorInt { dst, .. }
        | IrOp::ShiftLeftInt { dst, .. }
        | IrOp::ShiftRightInt { dst, .. }
        | IrOp::RangeInt { dst, .. }
        | IrOp::NegInt { dst, .. }
        | IrOp::NotBool { dst, .. }
        | IrOp::BitNotInt { dst, .. }
        | IrOp::BitNotSize { dst, .. }
        | IrOp::NoneCheck { dst, .. }
        | IrOp::GreaterInt { dst, .. }
        | IrOp::CompareInt { dst, .. }
        | IrOp::AddFloat { dst, .. }
        | IrOp::SubFloat { dst, .. }
        | IrOp::MulFloat { dst, .. }
        | IrOp::DivFloat { dst, .. }
        | IrOp::GreaterFloat { dst, .. }
        | IrOp::CompareFloat { dst, .. }
        | IrOp::AddSizeChecked { dst, .. }
        | IrOp::SubSizeChecked { dst, .. }
        | IrOp::MulSizeChecked { dst, .. }
        | IrOp::DivSize { dst, .. }
        | IrOp::RemSize { dst, .. }
        | IrOp::BitAndSize { dst, .. }
        | IrOp::BitOrSize { dst, .. }
        | IrOp::BitXorSize { dst, .. }
        | IrOp::ShiftLeftSize { dst, .. }
        | IrOp::ShiftRightSize { dst, .. }
        | IrOp::GreaterSize { dst, .. }
        | IrOp::CompareSize { dst, .. }
        | IrOp::AddByteWrap { dst, .. }
        | IrOp::ByteBinary { dst, .. }
        | IrOp::SeqBuild { dst, .. }
        | IrOp::TupleBuild { dst, .. }
        | IrOp::SeqGet { dst, .. }
        | IrOp::GetField { dst, .. }
        | IrOp::VariantConstruct { dst, .. }
        | IrOp::StructConstruct { dst, .. }
        | IrOp::StructIs { dst, .. }
        | IrOp::VariantField { dst, .. }
        | IrOp::TupleField { dst, .. }
        | IrOp::CallDirect { dst, .. }
        | IrOp::CallMember { dst, .. }
        | IrOp::CallableValue { dst, .. }
        | IrOp::CallBound { dst, .. }
        | IrOp::CallWitness { dst, .. }
        | IrOp::CallHost { dst, .. }
        | IrOp::ResultPropagate { dst, .. }
        | IrOp::Spawn { dst, .. }
        | IrOp::TaskJoin { dst, .. }
        | IrOp::StringBuild { dst, .. }
        | IrOp::StringLen { dst, .. }
        | IrOp::StringGet { dst, .. } => vec![*dst],
        IrOp::FiniteForInit { iterator, len, .. } => vec![*iterator, *len],
        IrOp::FiniteForNext { index, item, .. } => vec![*index, *item],
        IrOp::StoreLocal { .. }
        | IrOp::SeqPush { .. }
        | IrOp::SetField { .. }
        | IrOp::SeqSet { .. }
        | IrOp::Jump { .. }
        | IrOp::Branch { .. }
        | IrOp::MatchVariant { .. }
        | IrOp::Panic { .. }
        | IrOp::Return { .. } => Vec::new(),
    }
}

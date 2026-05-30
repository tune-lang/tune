use std::collections::HashMap;

use tune_ir::{ConstId, IrBlock, IrConst, IrFunction, IrOp, LocalId, Reg};

use crate::{Pass, PassReport};

#[must_use]
pub fn run(function: &mut IrFunction) -> PassReport {
    let changed = eliminate_function(function);
    PassReport {
        pass: Pass::BoundsCheckElim,
        changed,
    }
}

fn eliminate_function(function: &mut IrFunction) -> bool {
    let stable_local_seq_lengths = stable_local_sequence_lengths(&function.blocks);
    let mut changed = false;
    for block in &mut function.blocks {
        changed |= eliminate_block(
            &function.constants,
            &stable_local_seq_lengths,
            &mut block.ops,
        );
    }
    for task in &mut function.task_functions {
        changed |= eliminate_function(task);
    }
    changed
}

fn eliminate_block(
    constants: &[IrConst],
    stable_local_seq_lengths: &HashMap<LocalId, usize>,
    ops: &mut [IrOp],
) -> bool {
    let mut seq_lengths = HashMap::<Reg, usize>::new();
    let mut local_seq_lengths = HashMap::<LocalId, usize>::new();
    let mut const_regs = HashMap::<Reg, IrConst>::new();
    let mut local_consts = HashMap::<LocalId, IrConst>::new();
    let mut changed = false;

    for op in ops {
        match op {
            IrOp::LoadConst { dst, constant, .. } => {
                forget_reg(*dst, &mut seq_lengths, &mut const_regs);
                if let Some(value) = constant_value(constants, *constant) {
                    const_regs.insert(*dst, value);
                }
            }
            IrOp::SeqBuild { dst, .. } => {
                forget_reg(*dst, &mut seq_lengths, &mut const_regs);
                seq_lengths.insert(*dst, 0);
            }
            IrOp::SeqPush { seq, .. } => {
                if let Some(length) = seq_lengths.get_mut(seq) {
                    *length = length.saturating_add(1);
                }
            }
            IrOp::Move { dst, src } => {
                let length = seq_lengths.get(src).copied();
                let constant = const_regs.get(src).cloned();
                forget_reg(*dst, &mut seq_lengths, &mut const_regs);
                if let Some(length) = length {
                    seq_lengths.insert(*dst, length);
                }
                if let Some(constant) = constant {
                    const_regs.insert(*dst, constant);
                }
            }
            IrOp::StoreLocal { local, value } => {
                if let Some(length) = seq_lengths.get(value).copied() {
                    local_seq_lengths.insert(*local, length);
                } else {
                    local_seq_lengths.remove(local);
                }
                if let Some(value) = const_regs.get(value).cloned() {
                    local_consts.insert(*local, value);
                } else {
                    local_consts.remove(local);
                }
            }
            IrOp::LoadLocal { dst, local } => {
                let length = local_seq_lengths
                    .get(local)
                    .copied()
                    .or_else(|| stable_local_seq_lengths.get(local).copied());
                let constant = local_consts.get(local).cloned();
                forget_reg(*dst, &mut seq_lengths, &mut const_regs);
                if let Some(length) = length {
                    seq_lengths.insert(*dst, length);
                }
                if let Some(constant) = constant {
                    const_regs.insert(*dst, constant);
                }
            }
            IrOp::SeqGet {
                dst,
                seq,
                index,
                checked,
            } => {
                if *checked && index_in_bounds(&seq_lengths, &const_regs, *seq, *index) {
                    *checked = false;
                    changed = true;
                }
                forget_reg(*dst, &mut seq_lengths, &mut const_regs);
            }
            IrOp::SeqSet {
                seq,
                index,
                checked,
                ..
            } => {
                if *checked && index_in_bounds(&seq_lengths, &const_regs, *seq, *index) {
                    *checked = false;
                    changed = true;
                }
            }
            _ => {
                if let Some(dst) = op_dst(op) {
                    forget_reg(dst, &mut seq_lengths, &mut const_regs);
                }
            }
        }
    }

    changed
}

fn stable_local_sequence_lengths(blocks: &[IrBlock]) -> HashMap<LocalId, usize> {
    let mut candidates = HashMap::<LocalId, Option<usize>>::new();

    for block in blocks {
        let mut seq_lengths = HashMap::<Reg, usize>::new();
        let mut const_regs = HashMap::<Reg, IrConst>::new();

        for op in &block.ops {
            match op {
                IrOp::LoadConst { dst, .. } => {
                    forget_reg(*dst, &mut seq_lengths, &mut const_regs);
                }
                IrOp::SeqBuild { dst, .. } => {
                    forget_reg(*dst, &mut seq_lengths, &mut const_regs);
                    seq_lengths.insert(*dst, 0);
                }
                IrOp::SeqPush { seq, .. } => {
                    if let Some(length) = seq_lengths.get_mut(seq) {
                        *length = length.saturating_add(1);
                    }
                }
                IrOp::Move { dst, src } => {
                    let length = seq_lengths.get(src).copied();
                    forget_reg(*dst, &mut seq_lengths, &mut const_regs);
                    if let Some(length) = length {
                        seq_lengths.insert(*dst, length);
                    }
                }
                IrOp::StoreLocal { local, value } => {
                    record_local_length(&mut candidates, *local, seq_lengths.get(value).copied());
                }
                _ => {
                    if let Some(dst) = op_dst(op) {
                        forget_reg(dst, &mut seq_lengths, &mut const_regs);
                    }
                }
            }
        }
    }

    candidates
        .into_iter()
        .filter_map(|(local, length)| length.map(|length| (local, length)))
        .collect()
}

fn record_local_length(
    candidates: &mut HashMap<LocalId, Option<usize>>,
    local: LocalId,
    length: Option<usize>,
) {
    match candidates.get_mut(&local) {
        Some(candidate) if *candidate == length => {}
        Some(candidate) => *candidate = None,
        None => {
            candidates.insert(local, length);
        }
    }
}

fn constant_value(constants: &[IrConst], constant: ConstId) -> Option<IrConst> {
    constants.get(constant.0 as usize).cloned()
}

fn index_in_bounds(
    seq_lengths: &HashMap<Reg, usize>,
    const_regs: &HashMap<Reg, IrConst>,
    seq: Reg,
    index: Reg,
) -> bool {
    let Some(length) = seq_lengths.get(&seq).copied() else {
        return false;
    };
    let Some(index) = const_regs.get(&index).and_then(constant_index) else {
        return false;
    };
    index < length
}

fn constant_index(value: &IrConst) -> Option<usize> {
    match value {
        IrConst::Int(value) => usize::try_from(*value).ok(),
        IrConst::Size(value) => usize::try_from(*value).ok(),
        IrConst::Byte(value) => Some(usize::from(*value)),
        IrConst::Float(_) | IrConst::Bool(_) | IrConst::None | IrConst::String(_) => None,
    }
}

fn forget_reg(
    reg: Reg,
    seq_lengths: &mut HashMap<Reg, usize>,
    const_regs: &mut HashMap<Reg, IrConst>,
) {
    seq_lengths.remove(&reg);
    const_regs.remove(&reg);
}

fn op_dst(op: &IrOp) -> Option<Reg> {
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
        | IrOp::StringGet { dst, .. } => Some(*dst),
        IrOp::StoreLocal { .. }
        | IrOp::SeqPush { .. }
        | IrOp::SetField { .. }
        | IrOp::SeqSet { .. }
        | IrOp::Jump { .. }
        | IrOp::Branch { .. }
        | IrOp::MatchVariant { .. }
        | IrOp::FiniteForInit { .. }
        | IrOp::FiniteForNext { .. }
        | IrOp::Panic { .. }
        | IrOp::Return { .. } => None,
    }
}

use tune_bytecode::Opcode;
use tune_plan::{FiniteForContractKind, PlanOp};
use tune_shape::Shape;

use super::{BytecodeQuality, IrQuality, OpcodeCount, OptimizerQuality, PlanQuality};

pub(super) fn plan_quality(
    entry: Option<&tune_plan::PlanFunction>,
    functions: &[tune_plan::PlanFunction],
) -> PlanQuality {
    let mut quality = PlanQuality {
        functions: functions.len() + usize::from(entry.is_some()),
        ..PlanQuality::default()
    };
    if let Some(entry) = entry {
        collect_plan_ops(&entry.ops, &mut quality);
    }
    for function in functions {
        collect_plan_ops(&function.ops, &mut quality);
    }
    quality
}

fn collect_plan_ops(ops: &[PlanOp], quality: &mut PlanQuality) {
    for op in ops {
        quality.ops += 1;
        match op {
            PlanOp::DirectCall { .. } => quality.direct_calls += 1,
            PlanOp::BoundCall { .. } => quality.dynamic_bound_calls += 1,
            PlanOp::MemberCall { member: None, .. } => quality.unresolved_member_calls += 1,
            PlanOp::WitnessCall => quality.witness_calls += 1,
            PlanOp::HostCall { .. } => quality.host_calls += 1,
            PlanOp::SequenceGet {
                index_member: Some(_),
                ..
            } => quality.struct_index_gets += 1,
            PlanOp::SequenceSet {
                index_member: Some(_),
                ..
            } => quality.struct_index_sets += 1,
            PlanOp::FiniteFor {
                iterable_ops,
                body_ops,
                contract,
                ..
            } => {
                match contract.kind {
                    FiniteForContractKind::Sequence => quality.finite_for_sequence += 1,
                    FiniteForContractKind::Range => quality.finite_for_range += 1,
                    FiniteForContractKind::MemberAccess => quality.finite_for_member_access += 1,
                    FiniteForContractKind::Unknown => quality.finite_for_unknown += 1,
                }
                collect_plan_ops(iterable_ops, quality);
                collect_plan_ops(body_ops, quality);
            }
            PlanOp::BoolAnd {
                lhs_ops, rhs_ops, ..
            }
            | PlanOp::BoolOr {
                lhs_ops, rhs_ops, ..
            } => {
                collect_plan_ops(lhs_ops, quality);
                collect_plan_ops(rhs_ops, quality);
            }
            PlanOp::If {
                branches, else_ops, ..
            } => {
                for branch in branches {
                    collect_plan_ops(&branch.condition_ops, quality);
                    collect_plan_ops(&branch.body_ops, quality);
                }
                collect_plan_ops(else_ops, quality);
            }
            PlanOp::Match { arms, .. } => {
                for arm in arms {
                    collect_plan_ops(&arm.body_ops, quality);
                }
            }
            PlanOp::While {
                condition_ops,
                body_ops,
                ..
            } => {
                collect_plan_ops(condition_ops, quality);
                collect_plan_ops(body_ops, quality);
            }
            PlanOp::Loop { body_ops, .. } | PlanOp::Spawn { body_ops, .. } => {
                collect_plan_ops(body_ops, quality);
            }
            _ => {}
        }
    }
}

pub(super) fn ir_quality(functions: &[tune_ir::IrFunction]) -> IrQuality {
    let mut quality = IrQuality {
        functions: functions.len(),
        ..IrQuality::default()
    };
    for function in functions {
        collect_ir_function(function, &mut quality);
    }
    quality
}

fn collect_ir_function(function: &tune_ir::IrFunction, quality: &mut IrQuality) {
    for block in &function.blocks {
        for op in &block.ops {
            quality.ops += 1;
            match op {
                tune_ir::IrOp::LoadConst { shape, .. } => {
                    quality.shape_holes += shape_holes(shape);
                }
                tune_ir::IrOp::SeqBuild { element_shape, .. } => {
                    let holes = shape_holes(element_shape);
                    quality.shape_holes += holes;
                    if holes > 0 {
                        quality.sequence_build_holes += 1;
                    }
                }
                tune_ir::IrOp::SeqGet { checked, .. } | tune_ir::IrOp::SeqSet { checked, .. } => {
                    if *checked {
                        quality.checked_sequence_ops += 1;
                    } else {
                        quality.unchecked_sequence_ops += 1;
                    }
                }
                tune_ir::IrOp::FiniteForInit { .. } | tune_ir::IrOp::FiniteForNext { .. } => {
                    quality.generic_finite_for_ops += 1;
                }
                _ => {}
            }
        }
    }
    for task in &function.task_functions {
        collect_ir_function(task, quality);
    }
}

pub(super) fn optimizer_quality(functions: &mut [tune_ir::IrFunction]) -> OptimizerQuality {
    let mut quality = OptimizerQuality::default();
    let report = tune_opt::optimize_functions(functions);
    quality.changed_passes += report.passes.iter().filter(|pass| pass.changed).count();
    quality.stack += report.ownership.stack;
    quality.direct_drop += report.ownership.direct_drop;
    quality.non_atomic_rc += report.ownership.non_atomic_rc;
    quality.cow += report.ownership.cow;
    quality.shared_atomic += report.ownership.shared_atomic;
    quality.host_retained += report.ownership.host_retained;
    quality
}

pub(super) fn bytecode_quality(
    artifact: &tune_bytecode::artifact::BytecodeArtifact,
) -> BytecodeQuality {
    let mut quality = BytecodeQuality {
        functions: artifact.functions.len(),
        constants: artifact.constants.len(),
        opcodes: tune_bytecode::Opcode::ALL
            .iter()
            .copied()
            .map(|opcode| OpcodeCount { opcode, count: 0 })
            .collect(),
        ..BytecodeQuality::default()
    };
    for function in &artifact.functions {
        quality.registers += function.register_count as usize;
        quality.locals += function.local_count as usize;
        for instruction in &function.instructions {
            quality.instructions += 1;
            if let Some(count) = quality
                .opcodes
                .iter_mut()
                .find(|count| count.opcode == instruction.opcode)
            {
                count.count += 1;
            }
            collect_opcode(instruction.opcode, &mut quality);
        }
    }
    quality.opcodes.retain(|count| count.count > 0);
    quality
}

fn collect_opcode(opcode: Opcode, quality: &mut BytecodeQuality) {
    match opcode {
        Opcode::CallDirect => quality.direct_calls += 1,
        Opcode::CallBound => {
            quality.bound_calls += 1;
            quality.runtime_type_guard_pressure += 1;
        }
        Opcode::CallableValue => quality.callable_values += 1,
        Opcode::SeqGetChecked | Opcode::SeqSetChecked => {
            quality.checked_sequence_ops += 1;
            quality.runtime_type_guard_pressure += 1;
        }
        Opcode::SeqGetUnchecked | Opcode::SeqSetUnchecked => quality.unchecked_sequence_ops += 1,
        Opcode::FieldGet | Opcode::FieldSet => quality.field_accesses += 1,
        Opcode::VariantField => {
            quality.variant_field_accesses += 1;
            quality.runtime_type_guard_pressure += 1;
        }
        Opcode::FiniteForInit | Opcode::FiniteForNext => {
            quality.generic_finite_for_ops += 1;
            quality.runtime_type_guard_pressure += 1;
        }
        Opcode::CallWitness | Opcode::CallHost => {
            quality.unsupported_reserved_opcodes += 1;
            quality.runtime_type_guard_pressure += 1;
        }
        Opcode::JumpIfFalse
        | Opcode::StructIs
        | Opcode::StringLen
        | Opcode::StringGet
        | Opcode::ResultPropagate
        | Opcode::TaskJoin => quality.runtime_type_guard_pressure += 1,
        _ => {}
    }
}

fn shape_holes(shape: &Shape) -> usize {
    match shape {
        Shape::Hole => 1,
        Shape::Sequence(inner)
        | Shape::Range(inner)
        | Shape::Optional(inner)
        | Shape::Task(inner) => shape_holes(inner),
        Shape::Tuple(items) | Shape::Union(items) => items.iter().map(shape_holes).sum(),
        Shape::Callable { params, ret } => {
            params.iter().map(shape_holes).sum::<usize>() + shape_holes(ret)
        }
        Shape::Result { ok, err } => shape_holes(ok) + shape_holes(err),
        Shape::Apply { args, .. } => args.iter().map(shape_holes).sum(),
        Shape::Structural(requirements) => requirements
            .iter()
            .map(|requirement| match requirement {
                tune_shape::MemberRequirement::Field { shape, .. } => {
                    shape.as_ref().map_or(0, shape_holes)
                }
                tune_shape::MemberRequirement::Callable { params, ret, .. } => {
                    params.iter().map(shape_holes).sum::<usize>()
                        + ret.as_ref().map_or(0, shape_holes)
                }
            })
            .sum(),
        _ => 0,
    }
}

pub mod bce;
pub mod escape;
pub mod generics;
pub mod rc_elim;
pub mod strings;
pub mod thread_escape;

use tune_ir::{IrFunction, IrOp, IrOwnershipPlan};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pass {
    Escape,
    ThreadEscape,
    RcElim,
    BoundsCheckElim,
    Generics,
    Strings,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassReport {
    pub pass: Pass,
    pub changed: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptimizationReport {
    pub passes: Vec<PassReport>,
    pub ownership: OwnershipReport,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnershipReport {
    pub stack: usize,
    pub direct_drop: usize,
    pub non_atomic_rc: usize,
    pub cow: usize,
    pub shared_atomic: usize,
    pub host_retained: usize,
}

#[must_use]
pub fn optimize(function: &mut IrFunction) -> OptimizationReport {
    let passes = [
        escape::run(function),
        thread_escape::run(function),
        rc_elim::run(function),
        bce::run(function),
        generics::run(function),
        strings::run(function),
    ];

    OptimizationReport {
        passes: passes.into_iter().collect(),
        ownership: ownership_report(function),
    }
}

#[must_use]
pub fn ownership_report(function: &IrFunction) -> OwnershipReport {
    let mut report = OwnershipReport::default();
    collect_ownership(function, &mut report);
    report
}

fn collect_ownership(function: &IrFunction, report: &mut OwnershipReport) {
    for block in &function.blocks {
        for op in &block.ops {
            if let IrOp::StructConstruct { state, .. } = op {
                match state.ownership {
                    IrOwnershipPlan::Stack => report.stack += 1,
                    IrOwnershipPlan::DirectDrop => report.direct_drop += 1,
                    IrOwnershipPlan::NonAtomicRc => report.non_atomic_rc += 1,
                    IrOwnershipPlan::Cow => report.cow += 1,
                    IrOwnershipPlan::SharedAtomic => report.shared_atomic += 1,
                    IrOwnershipPlan::HostRetained => report.host_retained += 1,
                }
            }
        }
    }
    for task in &function.task_functions {
        collect_ownership(task, report);
    }
}

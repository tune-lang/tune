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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OptimizationProfile {
    Debug,
    Release,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OptimizeOptions {
    pub profile: OptimizationProfile,
    pub generic_max_instantiations: usize,
    pub generic_max_ops: usize,
}

impl OptimizeOptions {
    #[must_use]
    pub const fn debug() -> Self {
        Self {
            profile: OptimizationProfile::Debug,
            generic_max_instantiations: 0,
            generic_max_ops: 0,
        }
    }

    #[must_use]
    pub const fn release() -> Self {
        Self {
            profile: OptimizationProfile::Release,
            generic_max_instantiations: 4,
            generic_max_ops: 64,
        }
    }
}

#[must_use]
pub fn optimize(function: &mut IrFunction) -> OptimizationReport {
    optimize_with_options(function, OptimizeOptions::release())
}

#[must_use]
pub fn optimize_with_options(
    function: &mut IrFunction,
    options: OptimizeOptions,
) -> OptimizationReport {
    let passes = run_local_passes(function, options);

    OptimizationReport {
        passes,
        ownership: ownership_report(function),
    }
}

#[must_use]
pub fn optimize_functions(functions: &mut [IrFunction]) -> OptimizationReport {
    optimize_functions_with_options(functions, OptimizeOptions::release())
}

#[must_use]
pub fn optimize_functions_with_options(
    functions: &mut [IrFunction],
    options: OptimizeOptions,
) -> OptimizationReport {
    let mut passes = Vec::new();
    for function in functions.iter_mut() {
        passes.extend(run_local_passes(function, options));
    }
    passes.push(generics::run_module(functions, options));

    let mut ownership = OwnershipReport::default();
    for function in functions.iter() {
        let report = ownership_report(function);
        ownership.stack += report.stack;
        ownership.direct_drop += report.direct_drop;
        ownership.non_atomic_rc += report.non_atomic_rc;
        ownership.cow += report.cow;
        ownership.shared_atomic += report.shared_atomic;
        ownership.host_retained += report.host_retained;
    }

    OptimizationReport { passes, ownership }
}

fn run_local_passes(function: &mut IrFunction, options: OptimizeOptions) -> Vec<PassReport> {
    let passes = [
        escape::run(function),
        thread_escape::run(function),
        rc_elim::run(function),
        bce::run(function),
        generics::run(function, options),
        strings::run(function),
    ];

    passes.into_iter().collect()
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

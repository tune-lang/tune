pub mod bce;
pub mod escape;
pub mod generics;
pub mod rc_elim;
pub mod strings;
pub mod thread_escape;

use tune_ir::IrFunction;

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
    }
}

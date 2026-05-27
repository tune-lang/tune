use tune_ir::IrFunction;

use crate::{Pass, PassReport};

#[must_use]
pub fn run(_function: &mut IrFunction) -> PassReport {
    PassReport {
        pass: Pass::Strings,
        changed: false,
    }
}

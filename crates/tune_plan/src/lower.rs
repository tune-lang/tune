use crate::plan::PlanFunction;

pub fn lower_to_plan(name: &str) -> PlanFunction {
    PlanFunction {
        name: name.into(),
        ops: Vec::new(),
    }
}

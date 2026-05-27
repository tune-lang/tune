use tune_plan::{StructOwnershipPlan, StructStatePlan, StructStateRepr};

use crate::{IrOwnershipPlan, IrStateRepr, IrStructState};

pub(crate) fn lower_struct_state(state: StructStatePlan) -> IrStructState {
    IrStructState {
        repr: match state.repr {
            StructStateRepr::Inline => IrStateRepr::Inline,
            StructStateRepr::LocalHandle => IrStateRepr::LocalHandle,
            StructStateRepr::SharedHandle => IrStateRepr::SharedHandle,
            StructStateRepr::HostResource => IrStateRepr::HostResource,
        },
        ownership: match state.ownership {
            StructOwnershipPlan::Stack => IrOwnershipPlan::Stack,
            StructOwnershipPlan::DirectDrop => IrOwnershipPlan::DirectDrop,
            StructOwnershipPlan::NonAtomicRc => IrOwnershipPlan::NonAtomicRc,
            StructOwnershipPlan::Cow => IrOwnershipPlan::Cow,
            StructOwnershipPlan::SharedAtomic => IrOwnershipPlan::SharedAtomic,
            StructOwnershipPlan::HostRetained => IrOwnershipPlan::HostRetained,
        },
    }
}

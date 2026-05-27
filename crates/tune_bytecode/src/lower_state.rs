use crate::function::{BytecodeOwnershipPlan, BytecodeStateRepr, BytecodeStructState};
use tune_ir::{IrOwnershipPlan, IrStateRepr, IrStructState};

pub(crate) fn lower_struct_state(state: IrStructState) -> BytecodeStructState {
    BytecodeStructState {
        repr: match state.repr {
            IrStateRepr::Inline => BytecodeStateRepr::Inline,
            IrStateRepr::LocalHandle => BytecodeStateRepr::LocalHandle,
            IrStateRepr::SharedHandle => BytecodeStateRepr::SharedHandle,
            IrStateRepr::HostResource => BytecodeStateRepr::HostResource,
        },
        ownership: match state.ownership {
            IrOwnershipPlan::Stack => BytecodeOwnershipPlan::Stack,
            IrOwnershipPlan::DirectDrop => BytecodeOwnershipPlan::DirectDrop,
            IrOwnershipPlan::NonAtomicRc => BytecodeOwnershipPlan::NonAtomicRc,
            IrOwnershipPlan::Cow => BytecodeOwnershipPlan::Cow,
            IrOwnershipPlan::SharedAtomic => BytecodeOwnershipPlan::SharedAtomic,
            IrOwnershipPlan::HostRetained => BytecodeOwnershipPlan::HostRetained,
        },
    }
}

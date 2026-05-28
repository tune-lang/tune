use tune_bytecode::function::{BytecodeOwnershipPlan, BytecodeStateRepr};
use tune_runtime::ownership::OwnershipPlan;
use tune_runtime::state::StateRepr;

pub(crate) fn runtime_state_repr(repr: BytecodeStateRepr) -> StateRepr {
    match repr {
        BytecodeStateRepr::Inline => StateRepr::Inline,
        BytecodeStateRepr::LocalHandle => StateRepr::LocalHandle,
        BytecodeStateRepr::SharedHandle => StateRepr::SharedHandle,
        BytecodeStateRepr::HostResource => StateRepr::HostResource,
    }
}

pub(crate) fn runtime_ownership(ownership: BytecodeOwnershipPlan) -> OwnershipPlan {
    match ownership {
        BytecodeOwnershipPlan::Stack => OwnershipPlan::Stack,
        BytecodeOwnershipPlan::DirectDrop => OwnershipPlan::DirectDrop,
        BytecodeOwnershipPlan::NonAtomicRc => OwnershipPlan::NonAtomicRc,
        BytecodeOwnershipPlan::Cow => OwnershipPlan::Cow,
        BytecodeOwnershipPlan::SharedAtomic => OwnershipPlan::SharedAtomic,
        BytecodeOwnershipPlan::HostRetained => OwnershipPlan::HostRetained,
    }
}

use crate::ownership::OwnershipPlan;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateRepr {
    Inline,
    LocalHandle,
    SharedHandle,
    HostResource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StateId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StateHandle {
    pub id: StateId,
    pub repr: StateRepr,
    pub ownership: OwnershipPlan,
}

impl StateHandle {
    #[must_use]
    pub fn local(id: StateId) -> Self {
        Self {
            id,
            repr: StateRepr::LocalHandle,
            ownership: OwnershipPlan::NonAtomicRc,
        }
    }

    #[must_use]
    pub fn shared(id: StateId) -> Self {
        Self {
            id,
            repr: StateRepr::SharedHandle,
            ownership: OwnershipPlan::SharedAtomic,
        }
    }

    #[must_use]
    pub fn inline(id: StateId) -> Self {
        Self {
            id,
            repr: StateRepr::Inline,
            ownership: OwnershipPlan::Stack,
        }
    }
}

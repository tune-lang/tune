#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnershipPlan {
    Stack,
    DirectDrop,
    NonAtomicRc,
    Cow,
    SharedAtomic,
    HostRetained,
}

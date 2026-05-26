#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StateRepr {
    Inline,
    LocalHandle,
    SharedHandle,
    HostResource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrLocalAccess {
    Read,
    Borrow,
    Move,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrLocalStore {
    Init,
    Assign,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IrTransfer {
    Copy,
    Move,
    Alias,
    Borrow,
}

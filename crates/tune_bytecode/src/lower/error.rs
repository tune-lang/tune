#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BytecodeLowerError {
    UnsupportedIr(&'static str),
    UnknownFunction,
    UnknownBlock,
    ConstantLimit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CompilerFactKind {
    Name,
    Doc,
    Params,
    Return,
    Module,
    Visibility,
    JsonInvoker,
    Fields,
    Variants,
}

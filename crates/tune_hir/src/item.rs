#[derive(Debug, Clone)]
pub enum ItemKind {
    Let,
    CallableDecl,
    Struct,
    Enum,
    Tag,
    Import,
}

#[derive(Debug, Clone)]
pub struct Item {
    pub name: Option<String>,
    pub kind: ItemKind,
}

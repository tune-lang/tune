#[derive(Debug, Clone)]
pub enum Trivia {
    Whitespace(String),
    LineComment(String),
    BlockComment(String),
}

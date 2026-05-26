#[derive(Debug, Clone)]
pub enum Trivia {
    Whitespace(String),
    LineComment(String),
    DocComment(String),
}

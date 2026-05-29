/// V1 core string indexing unit.
///
/// Tune's core `String.len()` and `String[index]` are character-oriented.
/// For v1 bytecode execution, a character is a Unicode scalar value. Byte-level
/// access belongs to std/platform helpers, and a later edition can introduce
/// grapheme-cluster helpers without changing the core byte/string boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextUnit {
    UnicodeScalar,
}

pub const CORE_TEXT_UNIT: TextUnit = TextUnit::UnicodeScalar;

#[must_use]
pub fn character_len(value: &str) -> usize {
    value.chars().count()
}

#[must_use]
pub fn character_at(value: &str, index: usize) -> Option<String> {
    value.chars().nth(index).map(|ch| ch.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FileId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ByteOffset(pub u32);

impl ByteOffset {
    #[must_use]
    pub const fn new(offset: u32) -> Self {
        Self(offset)
    }

    #[must_use]
    pub const fn get(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub file: FileId,
    pub start: ByteOffset,
    pub end: ByteOffset,
}

impl Span {
    #[must_use]
    pub fn new(file: FileId, start: ByteOffset, end: ByteOffset) -> Self {
        debug_assert!(
            start.0 <= end.0,
            "span start must not be greater than span end"
        );
        Self { file, start, end }
    }

    #[must_use]
    pub const fn checked(file: FileId, start: ByteOffset, end: ByteOffset) -> Option<Self> {
        if start.0 <= end.0 {
            Some(Self { file, start, end })
        } else {
            None
        }
    }

    #[must_use]
    pub const fn new_unchecked(file: FileId, start: ByteOffset, end: ByteOffset) -> Self {
        Self { file, start, end }
    }

    #[must_use]
    pub const fn empty(file: FileId, offset: ByteOffset) -> Self {
        Self {
            file,
            start: offset,
            end: offset,
        }
    }

    #[must_use]
    pub const fn synthetic() -> Self {
        Self::empty(FileId(u32::MAX), ByteOffset::new(0))
    }

    #[must_use]
    pub const fn len(self) -> u32 {
        self.end.0.saturating_sub(self.start.0)
    }

    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.start.0 >= self.end.0
    }

    #[must_use]
    pub const fn contains(self, offset: ByteOffset) -> bool {
        self.start.0 <= offset.0 && offset.0 < self.end.0
    }

    #[must_use]
    pub const fn is_valid(self) -> bool {
        self.start.0 <= self.end.0
    }
}

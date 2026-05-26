pub mod codes;
pub mod diagnostic;
pub mod render;
pub mod span;

pub use codes::DiagnosticCode;
pub use diagnostic::{Diagnostic, DiagnosticBuilder, Label, LabelStyle, Related, Severity};
pub use span::{ByteOffset, FileId, Span};

pub type DiagResult<T> = Result<T, Vec<Diagnostic>>;

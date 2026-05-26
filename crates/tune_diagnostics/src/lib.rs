pub mod codes;
pub mod diagnostic;
pub mod render;
pub mod span;

pub use codes::DiagnosticCode;
pub use diagnostic::{
    Diagnostic, DiagnosticBuilder, Fact, FactEntry, Fix, FixApplicability, Help, Label, LabelKind,
    Note, Severity,
};
pub use render::DiagnosticRenderMode;
pub use span::{ByteOffset, FileId, Span};

pub type DiagResult<T> = Result<T, Vec<Diagnostic>>;

use core::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiagnosticCode(&'static str);

impl DiagnosticCode {
    #[must_use]
    pub const fn new(code: &'static str) -> Self {
        Self(code)
    }

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        self.0
    }
}

impl fmt::Display for DiagnosticCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

pub const PARSE_ERROR: DiagnosticCode = DiagnosticCode::new("T0001");
pub const UNRESOLVED_NAME: DiagnosticCode = DiagnosticCode::new("T0101");
pub const SHAPE_MISMATCH: DiagnosticCode = DiagnosticCode::new("T0201");
pub const UNSOLVED_HOLE: DiagnosticCode = DiagnosticCode::new("T0202");
pub const MATERIALIZATION_FAILED: DiagnosticCode = DiagnosticCode::new("T0301");
pub const ITERATION_CONTRACT: DiagnosticCode = DiagnosticCode::new("T0401");
pub const TAG_FACT_MISSING: DiagnosticCode = DiagnosticCode::new("T0501");

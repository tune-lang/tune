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

pub const PARSE_ERROR: DiagnosticCode = DiagnosticCode::new("T0101");
pub const UNRESOLVED_NAME: DiagnosticCode = DiagnosticCode::new("T0201");
pub const DUPLICATE_NAME: DiagnosticCode = DiagnosticCode::new("T0202");
pub const INVALID_ASSIGNMENT_TARGET: DiagnosticCode = DiagnosticCode::new("T0203");
pub const ASSIGNMENT_SHAPE_MISMATCH: DiagnosticCode = DiagnosticCode::new("T0204");
pub const IMPORT_NOT_VISIBLE: DiagnosticCode = DiagnosticCode::new("T0205");
pub const SHAPE_MISMATCH: DiagnosticCode = DiagnosticCode::new("T0301");
pub const MATERIALIZATION_FAILED: DiagnosticCode = DiagnosticCode::new("T0302");
pub const NUMERIC_OVERFLOW: DiagnosticCode = DiagnosticCode::new("T0401");
pub const CALLABLE_MISMATCH: DiagnosticCode = DiagnosticCode::new("T0501");
pub const SELF_STATE_ERROR: DiagnosticCode = DiagnosticCode::new("T0601");
pub const ITERATION_LEN_MISSING: DiagnosticCode = DiagnosticCode::new("T0701");
pub const ITERATION_INDEX_MISSING: DiagnosticCode = DiagnosticCode::new("T0702");
pub const ITERATION_SOURCE_MUTATED: DiagnosticCode = DiagnosticCode::new("T0705");
pub const MATCH_NOT_EXHAUSTIVE: DiagnosticCode = DiagnosticCode::new("T0803");
pub const MATCH_HOLE_FALLBACK: DiagnosticCode = DiagnosticCode::new("T0804");
pub const RESULT_PROPAGATION_ERROR: DiagnosticCode = DiagnosticCode::new("T0901");
pub const RUNTIME_ERROR: DiagnosticCode = DiagnosticCode::new("T0903");
pub const SPAWN_TASK_ERROR: DiagnosticCode = DiagnosticCode::new("T1001");
pub const PUBLIC_API_INFERENCE: DiagnosticCode = DiagnosticCode::new("T1101");
pub const COMPILER_RESERVED_NAME: DiagnosticCode = DiagnosticCode::new("T1201");
pub const TAG_FACT_MISSING: DiagnosticCode = DiagnosticCode::new("T1203");
pub const HOST_AUTHORITY_DENIED: DiagnosticCode = DiagnosticCode::new("T1301");
pub const EXECUTABLE_LOWERING_ERROR: DiagnosticCode = DiagnosticCode::new("T1401");

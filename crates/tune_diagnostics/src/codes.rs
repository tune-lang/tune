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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagnosticCodeInfo {
    pub code: DiagnosticCode,
    pub title: &'static str,
    pub explanation: &'static str,
}

const ALL_CODES: &[DiagnosticCodeInfo] = &[
    DiagnosticCodeInfo {
        code: PARSE_ERROR,
        title: "parse error",
        explanation: "Tune could not read the source as valid syntax.",
    },
    DiagnosticCodeInfo {
        code: UNRESOLVED_NAME,
        title: "unresolved name",
        explanation: "A referenced name is not visible in the current scope or import set.",
    },
    DiagnosticCodeInfo {
        code: DUPLICATE_NAME,
        title: "duplicate name",
        explanation: "Two declarations define the same name where Tune requires one meaning.",
    },
    DiagnosticCodeInfo {
        code: INVALID_ASSIGNMENT_TARGET,
        title: "invalid assignment target",
        explanation: "The left side of an assignment is not a mutable storage location.",
    },
    DiagnosticCodeInfo {
        code: ASSIGNMENT_SHAPE_MISMATCH,
        title: "assignment shape mismatch",
        explanation: "The assigned value cannot materialize into the target storage shape.",
    },
    DiagnosticCodeInfo {
        code: IMPORT_NOT_VISIBLE,
        title: "import is not visible",
        explanation: "An import reaches a declaration that is private to its source module.",
    },
    DiagnosticCodeInfo {
        code: SHAPE_MISMATCH,
        title: "shape mismatch",
        explanation: "A value's compile-time meaning does not satisfy the expected shape.",
    },
    DiagnosticCodeInfo {
        code: MATERIALIZATION_FAILED,
        title: "materialization failed",
        explanation: "An unresolved literal or value could not commit to the required shape.",
    },
    DiagnosticCodeInfo {
        code: NUMERIC_OVERFLOW,
        title: "numeric overflow",
        explanation: "A numeric operation cannot be represented by its planned result shape.",
    },
    DiagnosticCodeInfo {
        code: CALLABLE_MISMATCH,
        title: "callable mismatch",
        explanation: "A call target or argument list does not match the callable shape.",
    },
    DiagnosticCodeInfo {
        code: SELF_STATE_ERROR,
        title: "self state error",
        explanation: "A receiver-state operation violates Tune's struct state rules.",
    },
    DiagnosticCodeInfo {
        code: ITERATION_LEN_MISSING,
        title: "iteration len missing",
        explanation: "A finite for source does not expose the required len(): Size contract.",
    },
    DiagnosticCodeInfo {
        code: ITERATION_INDEX_MISSING,
        title: "iteration index missing",
        explanation: "A finite for source does not expose indexed access by Size.",
    },
    DiagnosticCodeInfo {
        code: ITERATION_SOURCE_MUTATED,
        title: "iteration source mutated",
        explanation: "A finite for loop mutates the value it is iterating over.",
    },
    DiagnosticCodeInfo {
        code: MATCH_NOT_EXHAUSTIVE,
        title: "match is not exhaustive",
        explanation: "A match expression leaves a possible shape or variant uncovered.",
    },
    DiagnosticCodeInfo {
        code: MATCH_HOLE_FALLBACK,
        title: "match hole fallback",
        explanation: "`_` is a hole pattern, not a catch-all fallback; use `else`.",
    },
    DiagnosticCodeInfo {
        code: RESULT_PROPAGATION_ERROR,
        title: "result propagation error",
        explanation: "A Result error reached a boundary where it had to become diagnostic.",
    },
    DiagnosticCodeInfo {
        code: RUNTIME_ERROR,
        title: "runtime error",
        explanation: "Typed bytecode execution failed after the frontend accepted the program.",
    },
    DiagnosticCodeInfo {
        code: SPAWN_TASK_ERROR,
        title: "spawn task error",
        explanation: "A spawned task or task join violated task execution rules.",
    },
    DiagnosticCodeInfo {
        code: PUBLIC_API_INFERENCE,
        title: "public API inference",
        explanation: "A public declaration exposes inferred facts that should be made explicit.",
    },
    DiagnosticCodeInfo {
        code: COMPILER_RESERVED_NAME,
        title: "compiler reserved name",
        explanation: "A user declaration uses a name reserved for compiler-owned facts.",
    },
    DiagnosticCodeInfo {
        code: TAG_FACT_MISSING,
        title: "tag fact missing",
        explanation: "A tag or compiler fact required by metadata analysis is missing.",
    },
    DiagnosticCodeInfo {
        code: HOST_AUTHORITY_DENIED,
        title: "host authority denied",
        explanation: "A host call requires an authority that the active profile did not grant.",
    },
    DiagnosticCodeInfo {
        code: EXECUTABLE_LOWERING_ERROR,
        title: "executable lowering error",
        explanation: "A planned program could not lower into executable IR or bytecode.",
    },
];

#[must_use]
pub const fn all() -> &'static [DiagnosticCodeInfo] {
    ALL_CODES
}

#[must_use]
pub fn explain(code: &str) -> Option<DiagnosticCodeInfo> {
    ALL_CODES
        .iter()
        .copied()
        .find(|info| info.code.as_str() == code)
}

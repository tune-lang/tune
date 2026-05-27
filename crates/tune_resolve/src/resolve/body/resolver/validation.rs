use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::pattern::{Pattern, PatternKind};

use super::BodyResolver;

impl BodyResolver<'_> {
    pub(super) fn validate_match_pattern(&mut self, pattern: &Pattern) {
        if matches!(pattern.kind, PatternKind::Hole) {
            self.resolved.diagnostics.push(
                Diagnostic::error(
                    codes::MATCH_HOLE_FALLBACK,
                    "`_` is a pattern hole, not a match fallback",
                    pattern.span.unwrap_or_else(Span::synthetic),
                    "use `else` for the fallback arm",
                )
                .with_help("write `else => ...` when every remaining value should take this arm")
                .build(),
            );
        }
    }

    pub(super) fn validate_user_name(&mut self, name: &str, span: Option<Span>, kind: &str) {
        if !name.starts_with("__") {
            return;
        }

        self.resolved.diagnostics.push(
            Diagnostic::error(
                codes::COMPILER_RESERVED_NAME,
                format!("compiler-reserved {kind} name `{name}`"),
                span.unwrap_or_else(Span::synthetic),
                "`__` names are owned by compiler facts and generated helpers",
            )
            .with_help("rename this symbol without the leading `__` prefix")
            .build(),
        );
    }
}

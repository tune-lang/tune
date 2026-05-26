use crate::{DiagnosticCode, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Info => "info",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelKind {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub kind: LabelKind,
    pub span: Span,
    pub message: String,
}

impl Label {
    #[must_use]
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self {
            kind: LabelKind::Primary,
            span,
            message: message.into(),
        }
    }

    #[must_use]
    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self {
            kind: LabelKind::Secondary,
            span,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Note {
    pub message: String,
}

impl Note {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Help {
    pub message: String,
    pub fix: Option<Fix>,
}

impl Help {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            fix: None,
        }
    }

    #[must_use]
    pub fn with_fix(message: impl Into<String>, fix: Fix) -> Self {
        Self {
            message: message.into(),
            fix: Some(fix),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fact {
    pub title: String,
    pub entries: Vec<String>,
}

impl Fact {
    #[must_use]
    pub fn new(
        title: impl Into<String>,
        entries: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            title: title.into(),
            entries: entries.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fix {
    pub span: Span,
    pub replacement: String,
    pub applicability: FixApplicability,
}

impl Fix {
    #[must_use]
    pub fn new(
        span: Span,
        replacement: impl Into<String>,
        applicability: FixApplicability,
    ) -> Self {
        Self {
            span,
            replacement: replacement.into(),
            applicability,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixApplicability {
    MachineApplicable,
    MaybeIncorrect,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: DiagnosticCode,
    pub title: String,
    pub primary: Label,
    pub labels: Vec<Label>,
    pub notes: Vec<Note>,
    pub helps: Vec<Help>,
    pub facts: Vec<Fact>,
    pub fixes: Vec<Fix>,
}

impl Diagnostic {
    #[must_use]
    pub fn error(code: DiagnosticCode, title: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(Severity::Error, code, title)
    }

    #[must_use]
    pub fn warning(code: DiagnosticCode, title: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(Severity::Warning, code, title)
    }

    #[must_use]
    pub fn info(code: DiagnosticCode, title: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(Severity::Info, code, title)
    }

    #[must_use]
    pub const fn primary_span(&self) -> Span {
        self.primary.span
    }
}

#[derive(Debug, Clone)]
pub struct DiagnosticBuilder {
    severity: Severity,
    code: DiagnosticCode,
    title: String,
    primary: Option<Label>,
    labels: Vec<Label>,
    notes: Vec<Note>,
    helps: Vec<Help>,
    facts: Vec<Fact>,
    fixes: Vec<Fix>,
}

impl DiagnosticBuilder {
    #[must_use]
    pub fn new(severity: Severity, code: DiagnosticCode, title: impl Into<String>) -> Self {
        Self {
            severity,
            code,
            title: title.into(),
            primary: None,
            labels: Vec::new(),
            notes: Vec::new(),
            helps: Vec::new(),
            facts: Vec::new(),
            fixes: Vec::new(),
        }
    }

    #[must_use]
    pub fn with_label(mut self, label: Label) -> Self {
        if label.kind == LabelKind::Primary && self.primary.is_none() {
            self.primary = Some(label);
        } else {
            self.labels.push(label);
        }
        self
    }

    #[must_use]
    pub fn with_primary(mut self, span: Span, message: impl Into<String>) -> Self {
        let label = Label::primary(span, message);
        if let Some(previous) = self.primary.replace(label) {
            self.labels.push(previous);
        }
        self
    }

    #[must_use]
    pub fn with_secondary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.labels.push(Label::secondary(span, message));
        self
    }

    #[must_use]
    pub fn with_note(mut self, message: impl Into<String>) -> Self {
        self.notes.push(Note::new(message));
        self
    }

    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.helps.push(Help::new(help));
        self
    }

    #[must_use]
    pub fn with_help_fix(mut self, help: impl Into<String>, fix: Fix) -> Self {
        self.helps.push(Help::with_fix(help, fix));
        self
    }

    #[must_use]
    pub fn with_fact(
        mut self,
        title: impl Into<String>,
        entries: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        self.facts.push(Fact::new(title, entries));
        self
    }

    #[must_use]
    pub fn with_fix(mut self, fix: Fix) -> Self {
        self.fixes.push(fix);
        self
    }

    #[must_use]
    pub fn build(self) -> Diagnostic {
        let primary = self.primary.unwrap_or_else(|| {
            Label::primary(Span::synthetic(), "diagnostic location unavailable")
        });

        Diagnostic {
            severity: self.severity,
            code: self.code,
            title: self.title,
            primary,
            labels: self.labels,
            notes: self.notes,
            helps: self.helps,
            facts: self.facts,
            fixes: self.fixes,
        }
    }
}

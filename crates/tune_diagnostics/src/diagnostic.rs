use crate::{DiagnosticCode, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl Severity {
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Error => "error",
            Self::Warning => "warning",
            Self::Note => "note",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LabelStyle {
    Primary,
    Secondary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    pub style: LabelStyle,
    pub span: Span,
    pub message: Option<String>,
}

impl Label {
    #[must_use]
    pub fn primary(span: Span, message: impl Into<String>) -> Self {
        Self {
            style: LabelStyle::Primary,
            span,
            message: Some(message.into()),
        }
    }

    #[must_use]
    pub fn secondary(span: Span, message: impl Into<String>) -> Self {
        Self {
            style: LabelStyle::Secondary,
            span,
            message: Some(message.into()),
        }
    }

    #[must_use]
    pub const fn anonymous(style: LabelStyle, span: Span) -> Self {
        Self {
            style,
            span,
            message: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Related {
    pub span: Span,
    pub message: String,
}

impl Related {
    #[must_use]
    pub fn new(span: Span, message: impl Into<String>) -> Self {
        Self {
            span,
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub code: DiagnosticCode,
    pub severity: Severity,
    pub message: String,
    pub labels: Vec<Label>,
    pub help: Vec<String>,
    pub related: Vec<Related>,
}

impl Diagnostic {
    #[must_use]
    pub fn error(code: DiagnosticCode, message: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(Severity::Error, code, message)
    }

    #[must_use]
    pub fn warning(code: DiagnosticCode, message: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(Severity::Warning, code, message)
    }

    #[must_use]
    pub fn note(code: DiagnosticCode, message: impl Into<String>) -> DiagnosticBuilder {
        DiagnosticBuilder::new(Severity::Note, code, message)
    }

    #[must_use]
    pub fn primary_span(&self) -> Option<Span> {
        self.labels
            .iter()
            .find(|label| label.style == LabelStyle::Primary)
            .map(|label| label.span)
    }
}

#[derive(Debug, Clone)]
pub struct DiagnosticBuilder {
    diagnostic: Diagnostic,
}

impl DiagnosticBuilder {
    #[must_use]
    pub fn new(severity: Severity, code: DiagnosticCode, message: impl Into<String>) -> Self {
        Self {
            diagnostic: Diagnostic {
                code,
                severity,
                message: message.into(),
                labels: Vec::new(),
                help: Vec::new(),
                related: Vec::new(),
            },
        }
    }

    #[must_use]
    pub fn with_label(mut self, label: Label) -> Self {
        self.diagnostic.labels.push(label);
        self
    }

    #[must_use]
    pub fn with_primary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.diagnostic.labels.push(Label::primary(span, message));
        self
    }

    #[must_use]
    pub fn with_secondary(mut self, span: Span, message: impl Into<String>) -> Self {
        self.diagnostic.labels.push(Label::secondary(span, message));
        self
    }

    #[must_use]
    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.diagnostic.help.push(help.into());
        self
    }

    #[must_use]
    pub fn with_related(mut self, span: Span, message: impl Into<String>) -> Self {
        self.diagnostic.related.push(Related::new(span, message));
        self
    }

    #[must_use]
    pub fn build(self) -> Diagnostic {
        self.diagnostic
    }
}

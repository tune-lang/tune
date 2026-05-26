use crate::{Diagnostic, LabelKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticRenderMode {
    CliFull,
    LspHoverCompact,
    JsonMachine,
}

#[must_use]
pub fn render_plain(diag: &Diagnostic) -> String {
    render(diag, DiagnosticRenderMode::CliFull)
}

#[must_use]
pub fn render(diag: &Diagnostic, mode: DiagnosticRenderMode) -> String {
    match mode {
        DiagnosticRenderMode::CliFull => render_cli_full(diag),
        DiagnosticRenderMode::LspHoverCompact => render_lsp_hover_compact(diag),
        DiagnosticRenderMode::JsonMachine => render_json_machine(diag),
    }
}

fn render_cli_full(diag: &Diagnostic) -> String {
    let mut out = format!("{}[{}]: {}", diag.severity.as_str(), diag.code, diag.title);

    out.push_str("\nprimary: ");
    push_span(&mut out, diag.primary.span);
    out.push_str(": ");
    out.push_str(&diag.primary.message);

    for label in &diag.labels {
        let style = match label.kind {
            LabelKind::Primary => "primary",
            LabelKind::Secondary => "secondary",
        };

        out.push('\n');
        out.push_str(style);
        out.push_str(": ");
        push_span(&mut out, label.span);

        out.push_str(": ");
        out.push_str(&label.message);
    }

    for fact in &diag.facts {
        out.push_str("\nfacts:\n  ");
        out.push_str(&fact.title);
        for entry in &fact.entries {
            out.push_str("\n  - ");
            out.push_str(entry);
        }
    }

    for note in &diag.notes {
        out.push_str("\nnote: ");
        out.push_str(&note.message);
    }

    for help in &diag.helps {
        out.push_str("\nhelp: ");
        out.push_str(&help.message);
        if let Some(fix) = &help.fix {
            out.push_str("\nfix: replace ");
            push_span(&mut out, fix.span);
            out.push_str(" with ");
            out.push_str(&fix.replacement);
        }
    }

    out
}

fn render_lsp_hover_compact(diag: &Diagnostic) -> String {
    let mut out = format!("{}[{}]: {}", diag.severity.as_str(), diag.code, diag.title);

    if !diag.primary.message.is_empty() {
        out.push_str("\n\n");
        out.push_str(&diag.primary.message);
    }

    if let Some(fact) = diag.facts.first() {
        out.push_str("\n\n");
        out.push_str(&fact.title);
        for entry in &fact.entries {
            out.push_str("\n  ");
            out.push_str(entry);
        }
    }

    if !diag.helps.is_empty() {
        out.push_str("\n\nhelp:");
        for help in &diag.helps {
            out.push_str("\n  ");
            out.push_str(&help.message);
        }
    }

    out
}

fn render_json_machine(diag: &Diagnostic) -> String {
    format!(
        "{{\"severity\":\"{}\",\"code\":\"{}\",\"title\":\"{}\"}}",
        diag.severity.as_str(),
        diag.code,
        escape_json(&diag.title)
    )
}

fn push_span(out: &mut String, span: crate::Span) {
    out.push_str("file ");
    out.push_str(&span.file.0.to_string());
    out.push(':');
    out.push_str(&span.start.get().to_string());
    out.push_str("..");
    out.push_str(&span.end.get().to_string());
}

fn escape_json(input: &str) -> String {
    let mut escaped = String::new();
    for ch in input.chars() {
        match ch {
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

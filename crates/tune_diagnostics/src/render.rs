use crate::{Diagnostic, LabelKind};
use serde_json::json;

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
            out.push_str(&entry.message);
            if let Some(span) = entry.span {
                out.push_str(" (");
                push_span(&mut out, span);
                out.push(')');
            }
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

    for fix in &diag.fixes {
        out.push_str("\nfix: replace ");
        push_span(&mut out, fix.span);
        out.push_str(" with ");
        out.push_str(&fix.replacement);
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
            out.push_str(&entry.message);
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
    json!({
        "severity": diag.severity.as_str(),
        "code": diag.code.to_string(),
        "title": diag.title,
        "primary": label_json(&diag.primary),
        "labels": diag.labels.iter().map(label_json).collect::<Vec<_>>(),
        "facts": diag.facts.iter().map(|fact| {
            json!({
                "title": fact.title,
                "entries": fact.entries.iter().map(|entry| {
                    json!({
                        "message": entry.message,
                        "span": entry.span.map(span_json),
                    })
                }).collect::<Vec<_>>(),
            })
        }).collect::<Vec<_>>(),
        "notes": diag.notes.iter().map(|note| note.message.as_str()).collect::<Vec<_>>(),
        "helps": diag.helps.iter().map(|help| {
            json!({
                "message": help.message,
                "fix": help.fix.as_ref().map(fix_json),
            })
        }).collect::<Vec<_>>(),
        "fixes": diag.fixes.iter().map(fix_json).collect::<Vec<_>>(),
    })
    .to_string()
}

fn push_span(out: &mut String, span: crate::Span) {
    out.push_str("file ");
    out.push_str(&span.file.0.to_string());
    out.push(':');
    out.push_str(&span.start.get().to_string());
    out.push_str("..");
    out.push_str(&span.end.get().to_string());
}

fn label_json(label: &crate::Label) -> serde_json::Value {
    json!({
        "kind": match label.kind {
            LabelKind::Primary => "primary",
            LabelKind::Secondary => "secondary",
        },
        "span": span_json(label.span),
        "message": label.message,
    })
}

fn fix_json(fix: &crate::Fix) -> serde_json::Value {
    json!({
        "span": span_json(fix.span),
        "replacement": fix.replacement,
        "applicability": match fix.applicability {
            crate::FixApplicability::MachineApplicable => "machine-applicable",
            crate::FixApplicability::MaybeIncorrect => "maybe-incorrect",
            crate::FixApplicability::Manual => "manual",
        },
    })
}

fn span_json(span: crate::Span) -> serde_json::Value {
    json!({
        "file": span.file.0,
        "start": span.start.get(),
        "end": span.end.get(),
    })
}

use crate::{Diagnostic, FileId, Label, LabelKind};
use serde_json::json;

#[derive(Debug, Clone, Copy)]
pub struct SourceView<'source> {
    pub path: &'source str,
    pub text: &'source str,
}

pub trait SourceProvider {
    fn source(&self, file: FileId) -> Option<SourceView<'_>>;
}

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
pub fn render_plain_with_sources(diag: &Diagnostic, sources: &impl SourceProvider) -> String {
    render_cli_full(diag, Some(sources))
}

#[must_use]
pub fn render(diag: &Diagnostic, mode: DiagnosticRenderMode) -> String {
    match mode {
        DiagnosticRenderMode::CliFull => render_cli_full(diag, None::<&NoSources>),
        DiagnosticRenderMode::LspHoverCompact => render_lsp_hover_compact(diag),
        DiagnosticRenderMode::JsonMachine => render_json_machine(diag),
    }
}

struct NoSources;

impl SourceProvider for NoSources {
    fn source(&self, _file: FileId) -> Option<SourceView<'_>> {
        None
    }
}

fn render_cli_full(diag: &Diagnostic, sources: Option<&impl SourceProvider>) -> String {
    let mut out = format!("{}[{}]: {}", diag.severity.as_str(), diag.code, diag.title);

    if let Some(sources) = sources {
        push_source_label(&mut out, &diag.primary, sources);
    }

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

        if let Some(sources) = sources {
            push_source_label(&mut out, label, sources);
        }
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

fn push_source_label(out: &mut String, label: &Label, sources: &impl SourceProvider) {
    let Some(source) = sources.source(label.span.file) else {
        return;
    };
    let Some(line) = locate_line(source.text, label.span) else {
        return;
    };

    out.push_str("\n --> ");
    out.push_str(source.path);
    out.push(':');
    out.push_str(&line.number.to_string());
    out.push(':');
    out.push_str(&line.column.to_string());
    out.push_str("\n  |");
    out.push('\n');
    out.push_str(&line.number.to_string());
    out.push_str(" | ");
    out.push_str(line.text);
    out.push('\n');
    out.push_str("  | ");
    out.push_str(&" ".repeat(line.column.saturating_sub(1)));
    out.push_str(&"^".repeat(line.width.max(1)));
    if !label.message.is_empty() {
        out.push(' ');
        out.push_str(&label.message);
    }
}

struct LocatedLine<'source> {
    number: usize,
    column: usize,
    width: usize,
    text: &'source str,
}

fn locate_line(text: &str, span: crate::Span) -> Option<LocatedLine<'_>> {
    let start = usize::try_from(span.start.get()).ok()?;
    let raw_end = usize::try_from(span.end.get()).ok()?;
    if start > text.len() {
        return None;
    }
    let end = raw_end.min(text.len()).max(start);
    if !text.is_char_boundary(start) || !text.is_char_boundary(end) {
        return None;
    }

    let line_start = text[..start].rfind('\n').map_or(0, |index| index + 1);
    let line_end = text[start..]
        .find('\n')
        .map_or(text.len(), |index| start + index);
    let line_text = &text[line_start..line_end];
    let line_number = text[..line_start]
        .bytes()
        .filter(|byte| *byte == b'\n')
        .count()
        + 1;
    let column = text[line_start..start].chars().count() + 1;
    let highlight_end = end.min(line_end);
    let width = text[start..highlight_end].chars().count().max(1);

    Some(LocatedLine {
        number: line_number,
        column,
        width,
        text: line_text,
    })
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

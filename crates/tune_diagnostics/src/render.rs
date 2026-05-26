use crate::{Diagnostic, LabelStyle};

#[must_use]
pub fn render_plain(diag: &Diagnostic) -> String {
    let mut out = format!(
        "{}[{}]: {}",
        diag.severity.as_str(),
        diag.code,
        diag.message
    );

    for label in &diag.labels {
        let style = match label.style {
            LabelStyle::Primary => "primary",
            LabelStyle::Secondary => "secondary",
        };

        out.push('\n');
        out.push_str(style);
        out.push_str(": ");
        push_span(&mut out, label.span);

        if let Some(message) = &label.message {
            out.push_str(": ");
            out.push_str(message);
        }
    }

    for related in &diag.related {
        out.push_str("\nrelated: ");
        push_span(&mut out, related.span);
        out.push_str(": ");
        out.push_str(&related.message);
    }

    for help in &diag.help {
        out.push_str("\nhelp: ");
        out.push_str(help);
    }

    out
}

fn push_span(out: &mut String, span: crate::Span) {
    out.push_str("file ");
    out.push_str(&span.file.0.to_string());
    out.push(':');
    out.push_str(&span.start.get().to_string());
    out.push_str("..");
    out.push_str(&span.end.get().to_string());
}

use crate::Diagnostic;

pub fn render_plain(diag: &Diagnostic) -> String {
    let mut out = format!("{}: {}", diag.code, diag.message);
    for help in &diag.help {
        out.push_str("\nhelp: ");
        out.push_str(help);
    }
    out
}

use tune_diagnostics::{Diagnostic, Span, codes};

pub(crate) fn unresolved_import(path: &str, span: Option<Span>) -> Diagnostic {
    Diagnostic::error(
        codes::UNRESOLVED_NAME,
        format!("unresolved import `{path}`"),
        span.unwrap_or_else(Span::synthetic),
        "this import path does not match a loaded project source",
    )
    .build()
}

pub(crate) fn import_cycle(path: &str, span: Option<Span>) -> Diagnostic {
    Diagnostic::error(
        codes::UNRESOLVED_NAME,
        format!("source import cycle through `{path}`"),
        span.unwrap_or_else(Span::synthetic),
        "source imports cannot form a cycle",
    )
    .build()
}

pub(crate) fn unresolved_import_member(name: &str, span: Option<Span>) -> Diagnostic {
    Diagnostic::error(
        codes::UNRESOLVED_NAME,
        format!("unresolved import member `{name}`"),
        span.unwrap_or_else(Span::synthetic),
        "this selector does not name a declaration in the imported source",
    )
    .build()
}

pub(crate) fn private_import_member(
    name: &str,
    span: Option<Span>,
    declaration_span: Option<Span>,
) -> Diagnostic {
    let mut diagnostic = Diagnostic::error(
        codes::IMPORT_NOT_VISIBLE,
        format!("import member `{name}` is private"),
        span.unwrap_or_else(Span::synthetic),
        "this selector names a private declaration",
    );
    if let Some(declaration_span) = declaration_span {
        diagnostic = diagnostic.with_secondary(declaration_span, "declaration is private here");
    }
    diagnostic
        .with_help("mark the declaration `pub` to import it from another source")
        .build()
}

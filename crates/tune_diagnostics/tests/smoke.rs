#[test]
fn diagnostic_builder_tracks_primary_span_and_help() {
    let span = tune_diagnostics::Span::new(
        tune_diagnostics::FileId(7),
        tune_diagnostics::ByteOffset::new(4),
        tune_diagnostics::ByteOffset::new(9),
    );

    let diag = tune_diagnostics::Diagnostic::error(
        tune_diagnostics::codes::PARSE_ERROR,
        "expected expression",
    )
    .with_primary(span, "expression starts here")
    .with_help("insert an expression")
    .build();

    assert_eq!(diag.primary_span(), span);
    assert_eq!(diag.helps[0].message, "insert an expression");
}

#[test]
fn plain_renderer_includes_labels_facts_notes_and_help() {
    let primary = tune_diagnostics::Span::new(
        tune_diagnostics::FileId(1),
        tune_diagnostics::ByteOffset::new(10),
        tune_diagnostics::ByteOffset::new(12),
    );
    let related = tune_diagnostics::Span::new(
        tune_diagnostics::FileId(1),
        tune_diagnostics::ByteOffset::new(2),
        tune_diagnostics::ByteOffset::new(5),
    );

    let rendered = tune_diagnostics::render::render_plain(
        &tune_diagnostics::Diagnostic::warning(
            tune_diagnostics::codes::UNRESOLVED_NAME,
            "unknown name",
        )
        .with_primary(primary, "not found in this scope")
        .with_secondary(related, "scope starts here")
        .with_fact("known scopes", ["module", "function"])
        .with_note("names are resolved before shape checking")
        .with_help("define the name before using it")
        .build(),
    );

    assert!(rendered.contains("warning[T0201]: unknown name"));
    assert!(rendered.contains("primary: file 1:10..12: not found in this scope"));
    assert!(rendered.contains("secondary: file 1:2..5: scope starts here"));
    assert!(rendered.contains("facts:"));
    assert!(rendered.contains("note: names are resolved before shape checking"));
    assert!(rendered.contains("help: define the name before using it"));
}

#[test]
fn compact_hover_renderer_uses_same_structured_facts() {
    let span = tune_diagnostics::Span::new(
        tune_diagnostics::FileId(1),
        tune_diagnostics::ByteOffset::new(0),
        tune_diagnostics::ByteOffset::new(1),
    );
    let diag = tune_diagnostics::Diagnostic::error(
        tune_diagnostics::codes::ASSIGNMENT_SHAPE_MISMATCH,
        "assignment does not fit binding shape",
    )
    .with_primary(span, "expected Int, found String")
    .with_fact("x was solved as Int from", ["let x = 0"])
    .with_help("use `let x = \"hello\"` to shadow")
    .build();

    let rendered = tune_diagnostics::render::render(
        &diag,
        tune_diagnostics::DiagnosticRenderMode::LspHoverCompact,
    );

    assert!(rendered.contains("error[T0204]: assignment does not fit binding shape"));
    assert!(rendered.contains("expected Int, found String"));
    assert!(rendered.contains("x was solved as Int from"));
}

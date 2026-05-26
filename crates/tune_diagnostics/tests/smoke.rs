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

    assert_eq!(diag.primary_span(), Some(span));
    assert_eq!(diag.help, ["insert an expression"]);
}

#[test]
fn plain_renderer_includes_labels_and_related_notes() {
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
        .with_related(related, "scope starts here")
        .build(),
    );

    assert!(rendered.contains("warning[T0101]: unknown name"));
    assert!(rendered.contains("primary: file 1:10..12: not found in this scope"));
    assert!(rendered.contains("related: file 1:2..5: scope starts here"));
}

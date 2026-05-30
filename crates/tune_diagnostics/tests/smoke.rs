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
        span,
        "expression starts here",
    )
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
            primary,
            "not found in this scope",
        )
        .with_secondary(related, "scope starts here")
        .with_spanned_fact("known scopes", [(related, "module"), (primary, "function")])
        .with_note("names are resolved before shape checking")
        .with_help("define the name before using it")
        .build(),
    );

    assert!(rendered.contains("warning[T0201]: unknown name"));
    assert!(rendered.contains("primary: file 1:10..12: not found in this scope"));
    assert!(rendered.contains("secondary: file 1:2..5: scope starts here"));
    assert!(rendered.contains("facts:"));
    assert!(rendered.contains("- module (file 1:2..5)"));
    assert!(rendered.contains("note: names are resolved before shape checking"));
    assert!(rendered.contains("help: define the name before using it"));
}

#[test]
fn plain_renderer_can_include_source_snippets() {
    struct Sources;

    impl tune_diagnostics::render::SourceProvider for Sources {
        fn source(
            &self,
            file: tune_diagnostics::FileId,
        ) -> Option<tune_diagnostics::render::SourceView<'_>> {
            if file == tune_diagnostics::FileId(1) {
                Some(tune_diagnostics::render::SourceView {
                    path: "app.tn",
                    text: "let value: Int = \"bad\"\n",
                })
            } else {
                None
            }
        }
    }

    let span = tune_diagnostics::Span::new(
        tune_diagnostics::FileId(1),
        tune_diagnostics::ByteOffset::new(17),
        tune_diagnostics::ByteOffset::new(22),
    );
    let diag = tune_diagnostics::Diagnostic::error(
        tune_diagnostics::codes::SHAPE_MISMATCH,
        "shape mismatch",
        span,
        "String cannot materialize as Int",
    )
    .build();

    let rendered = tune_diagnostics::render::render_plain_with_sources(&diag, &Sources);

    assert!(rendered.contains("--> app.tn:1:18"));
    assert!(rendered.contains("1 | let value: Int = \"bad\""));
    assert!(rendered.contains("^^^^^ String cannot materialize as Int"));
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
        span,
        "expected Int, found String",
    )
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

#[test]
fn machine_renderer_includes_structured_parts() -> Result<(), serde_json::Error> {
    let span = tune_diagnostics::Span::new(
        tune_diagnostics::FileId(1),
        tune_diagnostics::ByteOffset::new(0),
        tune_diagnostics::ByteOffset::new(1),
    );
    let fix = tune_diagnostics::Fix::new(
        span,
        "x",
        tune_diagnostics::FixApplicability::MachineApplicable,
    );

    let diag = tune_diagnostics::Diagnostic::error(
        tune_diagnostics::codes::PARSE_ERROR,
        "expected name",
        span,
        "this token is not a name",
    )
    .with_fact_entries(
        "parser context",
        [tune_diagnostics::FactEntry::spanned(
            span,
            "inside let declaration",
        )],
    )
    .with_fix(fix)
    .build();

    let rendered = tune_diagnostics::render::render(
        &diag,
        tune_diagnostics::DiagnosticRenderMode::JsonMachine,
    );
    let json: serde_json::Value = serde_json::from_str(&rendered)?;

    assert_eq!(json["severity"], "error");
    assert_eq!(json["code"], "T0101");
    assert_eq!(json["primary"]["span"]["start"], 0);
    assert_eq!(json["facts"][0]["entries"][0]["span"]["end"], 1);
    assert_eq!(json["fixes"][0]["replacement"], "x");

    Ok(())
}

#[test]
fn span_checked_rejects_reversed_ranges() {
    let span = tune_diagnostics::Span::checked(
        tune_diagnostics::FileId(1),
        tune_diagnostics::ByteOffset::new(5),
        tune_diagnostics::ByteOffset::new(4),
    );

    assert!(span.is_none());
}

#[test]
fn diagnostic_docs_exist_for_all_registered_codes() {
    let workspace = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let docs = workspace.join("docs/diagnostics");

    for info in tune_diagnostics::codes::all() {
        let path = docs.join(format!("{}.md", info.code.as_str()));
        assert!(
            path.is_file(),
            "missing diagnostic doc for {}",
            info.code.as_str()
        );
    }
}

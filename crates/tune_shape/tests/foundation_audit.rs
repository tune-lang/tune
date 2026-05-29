fn analyze_source(source: &str) -> Vec<tune_shape::ShapeAnalysis> {
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    tune_shape::analyze_module(&module, &resolved)
}

#[test]
fn analyzer_rejects_finite_for_len_that_is_not_size() {
    let analyses = analyze_source(
        r#"
struct Window {
  values: [Int]
  len(): Int = 3
  Window[index: Size]: Int = self.values[index]
}
let result: Int = {
  let window: Window = Window { values = [1, 2, 3] }
  let total: Int = 0
  for item in window {
    total = total + item
  }
  total
}
"#,
    );

    assert!(
        analyses
            .iter()
            .flat_map(|analysis| &analysis.diagnostics)
            .any(
                |diagnostic| diagnostic.code == tune_diagnostics::codes::ITERATION_LEN_MISSING
                    && diagnostic.title == "finite `for` source has no `len(): Size` contract"
            )
    );
}

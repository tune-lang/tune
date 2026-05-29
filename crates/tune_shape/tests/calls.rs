#[test]
fn analyzer_checks_top_level_call_signatures() -> Result<(), &'static str> {
    let source = r#"
let add(value: Int): Int = value
let run(): Int = add("bad")
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(
        analysis.calls.iter().any(|call| {
            matches!(
                call.target,
                tune_shape::CallTarget::TopLevel(tune_hir::HirId(0))
            ) && call.params == vec![tune_shape::Shape::Int]
                && call.ret == tune_shape::Shape::Int
        }),
        "{:?}",
        analysis.calls
    );
    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::CALLABLE_MISMATCH
            && diagnostic.title == "call argument does not match callable parameter shape"
    }));

    Ok(())
}

#[test]
fn analyzer_solves_generic_callable_params_from_arguments() -> Result<(), &'static str> {
    let source = r#"
let id<T>(value: T): T = value
let result = id(1)
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(analysis.diagnostics.is_empty());
    assert!(analysis.calls.iter().any(|call| {
        matches!(
            call.target,
            tune_shape::CallTarget::TopLevel(tune_hir::HirId(0))
        ) && matches!(
            call.params.as_slice(),
            [tune_shape::Shape::Literal(tune_shape::LiteralFact::Numeric { text })] if text == "1"
        ) && matches!(
            call.ret,
            tune_shape::Shape::Literal(tune_shape::LiteralFact::Numeric { ref text }) if text == "1"
        )
    }));

    Ok(())
}

#[test]
fn analyzer_solves_generic_callable_params_from_expected_return() -> Result<(), &'static str> {
    let source = r#"
let id<T>(value: T): T = value
let result: Int = id(1)
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(
        analysis.calls.iter().any(|call| {
            matches!(
                call.target,
                tune_shape::CallTarget::TopLevel(tune_hir::HirId(0))
            ) && call.params == vec![tune_shape::Shape::Int]
                && call.ret == tune_shape::Shape::Int
        }),
        "{:?}",
        analysis.calls
    );

    Ok(())
}

#[test]
fn analyzer_checks_member_call_signatures_with_receiver() -> Result<(), &'static str> {
    let source = r#"
struct Counter {
  inc(value: Int): Int = value
}
let run(counter: Counter): Int = counter.inc("bad")
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(analysis.calls.iter().any(|call| {
        matches!(call.target, tune_shape::CallTarget::Member(_))
            && call.params == vec![tune_shape::Shape::Int]
            && call.ret == tune_shape::Shape::Int
            && call
                .receiver
                .as_ref()
                .and_then(tune_shape::Shape::nominal_name)
                == Some("Counter")
    }));
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.code == tune_diagnostics::codes::CALLABLE_MISMATCH })
    );

    Ok(())
}

#[test]
fn analyzer_rejects_calling_non_callable_values() {
    let source = r#"
let result = {
  let value: Int = 1
  value()
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);

    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::CALLABLE_MISMATCH
            && diagnostic.title == "called value is not callable"
    }));
}

#[test]
fn analyzer_checks_enum_constructor_payloads() -> Result<(), &'static str> {
    let source = r#"
enum Color {
  Rgb(Int, Int, Int)
}
let make: Color = Rgb(1, "bad", 3)
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(analysis.calls.iter().any(|call| {
        matches!(call.target, tune_shape::CallTarget::Variant(_))
            && call.params
                == vec![
                    tune_shape::Shape::Int,
                    tune_shape::Shape::Int,
                    tune_shape::Shape::Int,
                ]
    }));
    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::CALLABLE_MISMATCH
            && diagnostic.title == "call argument does not match callable parameter shape"
    }));

    Ok(())
}

#[test]
fn analyzer_checks_enum_constructor_arity() -> Result<(), &'static str> {
    let source = r#"
enum Color {
  Rgb(Int, Int, Int)
}
let make: Color = Rgb(1)
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::CALLABLE_MISMATCH
            && diagnostic.title == "call argument count does not match callable signature"
    }));

    Ok(())
}

#[test]
fn analyzer_checks_bound_callable_value_signatures() -> Result<(), &'static str> {
    let source = r#"
let run(): Int = {
  let f = _(value: Int) = value
  f("bad")
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);

    assert!(analysis.expr_shapes.iter().any(|shape| {
        matches!(
            &shape.shape,
            tune_shape::Shape::Callable { params, ret }
                if params == &vec![tune_shape::Shape::Int] && **ret == tune_shape::Shape::Int
        )
    }));
    assert!(analysis.calls.iter().any(|call| {
        call.target == tune_shape::CallTarget::Bound
            && call.params == vec![tune_shape::Shape::Int]
            && call.ret == tune_shape::Shape::Int
    }));
    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::CALLABLE_MISMATCH
            && diagnostic.title == "call argument does not match callable parameter shape"
    }));

    Ok(())
}

#[test]
fn analyzer_checks_explicit_return_shapes() -> Result<(), &'static str> {
    let source = r#"
let run(flag): Int = {
  if flag { return "bad" }
  1
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);

    assert_eq!(analysis.returns.len(), 1);
    assert!(analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == tune_diagnostics::codes::ASSIGNMENT_SHAPE_MISMATCH
            && diagnostic.title == "returned value does not match callable return shape"
    }));

    Ok(())
}

#[test]
fn callable_value_returns_do_not_leak_to_outer_function() -> Result<(), &'static str> {
    let source = r#"
let run(): Int = {
  let f = _(value: String) = { return value }
  1
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[0]);

    assert!(analysis.returns.is_empty());
    assert!(!analysis.diagnostics.iter().any(|diagnostic| {
        diagnostic.title == "returned value does not match callable return shape"
    }));

    Ok(())
}

#[test]
fn callable_signature_infers_param_from_struct_field_context() -> Result<(), &'static str> {
    let source = r#"
struct Counter {
  value: Int
}
let make(seed) = Counter {
  value = seed
}
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    let signature = analysis
        .inferred_signature
        .as_ref()
        .ok_or("callable should have inferred signature")?;
    assert_eq!(signature.params, vec![tune_shape::Shape::Int]);
    assert_eq!(signature.ret.nominal_name(), Some("Counter"));

    Ok(())
}

#[test]
fn generic_struct_field_access_substitutes_owner_args() -> Result<(), &'static str> {
    let source = r#"
struct Box<T> {
  value: T
}
let read(boxed: Box<Int>): Int = boxed.value
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    assert!(analysis.expr_shapes.iter().any(|expr| {
        expr.shape == tune_shape::Shape::Int
            && matches!(
                module.items[1].body.as_ref().map(|body| body.id),
                Some(id) if id == expr.expr
            )
    }));

    Ok(())
}

#[test]
fn generic_struct_member_calls_substitute_owner_args() -> Result<(), &'static str> {
    let source = r#"
struct Box<T> {
  value: T
  get(): T = self.value
}
let read(boxed: Box<Int>): Int = boxed.get()
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    assert!(analysis.calls.iter().any(|call| {
        matches!(call.target, tune_shape::CallTarget::Member(_))
            && call.params.is_empty()
            && call.ret == tune_shape::Shape::Int
    }));

    Ok(())
}

#[test]
fn generic_struct_literal_uses_expected_owner_args_for_fields() -> Result<(), &'static str> {
    let source = r#"
struct Box<T> {
  value: T
}
let boxed: Box<Int> = Box { value = 1 }
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analysis = tune_shape::analyze_item(&module, &resolved, &module.items[1]);

    assert!(
        analysis.diagnostics.is_empty(),
        "{:?}",
        analysis.diagnostics
    );
    let body = module.items[1].body.as_ref().ok_or("expected body")?;
    assert!(analysis.expr_shapes.iter().any(|expr| {
        expr.expr == body.id
            && expr.shape
                == tune_shape::Shape::Apply {
                    nominal: tune_shape::NominalShape::new(module.items[0].id, "Box"),
                    args: vec![tune_shape::Shape::Int],
                }
    }));

    Ok(())
}

#[test]
fn explicit_structural_type_param_allows_member_call() -> Result<(), &'static str> {
    let source = r#"
struct Duck {
  quack(): String = "quack"
}
let speak<T: { quack(): String }>(duck: T): String = duck.quack()
let result: String = speak(Duck {})
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let resolved = tune_resolve::resolve_module(&module);
    let analyses = tune_shape::analyze_module(&module, &resolved);

    assert!(
        analyses
            .iter()
            .all(|analysis| analysis.diagnostics.is_empty())
    );
    let speak = &analyses[1];
    assert!(matches!(
        speak.calls.first().map(|call| &call.ret),
        Some(tune_shape::Shape::String)
    ));
    Ok(())
}

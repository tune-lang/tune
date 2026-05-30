#[test]
fn lowers_declaration_body_expressions() -> Result<(), &'static str> {
    let source = r#"
let value = items[0].name!
let task = spawn fetch()
let looped = for item in items { handle(item) }
let numbers = [1, 2, 3]
let callable = _(x: Int): Int = x
let block = { let x = 1; x = x; return x }
let grouped = (1 + 2)
let range = 0..=10
let ops = (not value and other) or (other is not none)
let inline_branch = if count == 1 => "item" else "items"
let branched = if ready { Ok(value) } elif waiting { Error("wait") } else { panic("bad") }
let matched = match result { Ok(value) => value; Error(err) => panic(err); else none }
let matched_shape = match duck { { quack(): String } => duck.quack(); else none }
let repeated = while ready { continue }
let forever = loop { break }
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);

    let value = module.items[0].body.as_ref().ok_or("expected value body")?;
    assert!(matches!(value.kind, tune_hir::expr::ExprKind::Propagate(_)));

    let task = module.items[1].body.as_ref().ok_or("expected task body")?;
    assert!(matches!(task.kind, tune_hir::expr::ExprKind::Spawn(_)));

    let looped = module.items[2].body.as_ref().ok_or("expected loop body")?;
    let tune_hir::expr::ExprKind::For { pattern, .. } = &looped.kind else {
        return Err("expected for expression");
    };
    assert!(matches!(
        pattern.kind,
        tune_hir::pattern::PatternKind::Binding(ref name) if name == "item"
    ));

    let numbers = module.items[3]
        .body
        .as_ref()
        .ok_or("expected numbers body")?;
    let tune_hir::expr::ExprKind::Sequence(elements) = &numbers.kind else {
        return Err("expected sequence literal");
    };
    assert_eq!(elements.len(), 3);

    let callable = module.items[4]
        .body
        .as_ref()
        .ok_or("expected callable body")?;
    let tune_hir::expr::ExprKind::CallableValue { params, body } = &callable.kind else {
        return Err("expected callable value");
    };
    assert_eq!(params.len(), 1);
    assert_eq!(params[0].name.as_deref(), Some("x"));
    assert!(params[0].shape.is_some());
    assert!(matches!(body.kind, tune_hir::expr::ExprKind::Name(_)));

    let block = module.items[5].body.as_ref().ok_or("expected block body")?;
    let tune_hir::expr::ExprKind::Block(exprs) = &block.kind else {
        return Err("expected block expression");
    };
    assert!(matches!(
        exprs[0].kind,
        tune_hir::expr::ExprKind::Let { .. }
    ));
    assert!(matches!(
        exprs[1].kind,
        tune_hir::expr::ExprKind::Assign { .. }
    ));
    assert!(matches!(exprs[2].kind, tune_hir::expr::ExprKind::Return(_)));

    let grouped = module.items[6]
        .body
        .as_ref()
        .ok_or("expected grouped body")?;
    assert!(matches!(
        grouped.kind,
        tune_hir::expr::ExprKind::Binary {
            op: tune_hir::expr::BinaryOp::Add,
            ..
        }
    ));

    let range = module.items[7].body.as_ref().ok_or("expected range body")?;
    assert!(matches!(
        range.kind,
        tune_hir::expr::ExprKind::Binary {
            op: tune_hir::expr::BinaryOp::RangeInclusive,
            ..
        }
    ));

    let ops = module.items[8].body.as_ref().ok_or("expected ops body")?;
    let tune_hir::expr::ExprKind::Binary { op, .. } = &ops.kind else {
        return Err("expected binary expression");
    };
    assert_eq!(*op, tune_hir::expr::BinaryOp::Or);

    let inline_branch = module.items[9]
        .body
        .as_ref()
        .ok_or("expected inline branch body")?;
    assert!(matches!(
        inline_branch.kind,
        tune_hir::expr::ExprKind::If { .. }
    ));

    let branched = module.items[10]
        .body
        .as_ref()
        .ok_or("expected branched body")?;
    let tune_hir::expr::ExprKind::If {
        branches,
        else_branch,
    } = &branched.kind
    else {
        return Err("expected if expression");
    };
    assert_eq!(branches.len(), 2);
    assert!(else_branch.is_some());

    let matched = module.items[11]
        .body
        .as_ref()
        .ok_or("expected matched body")?;
    let tune_hir::expr::ExprKind::Match { arms, .. } = &matched.kind else {
        return Err("expected match expression");
    };
    assert_eq!(arms.len(), 3);
    assert!(matches!(
        &arms[0].pattern.kind,
        tune_hir::pattern::PatternKind::Variant { name, args }
            if name == "Ok" && args.len() == 1
    ));
    assert!(matches!(
        &arms[1].pattern.kind,
        tune_hir::pattern::PatternKind::Variant { name, args }
            if name == "Error" && args.len() == 1
    ));
    assert!(matches!(
        arms[2].pattern.kind,
        tune_hir::pattern::PatternKind::Else
    ));

    let matched_shape = module.items[12]
        .body
        .as_ref()
        .ok_or("expected structural match body")?;
    let tune_hir::expr::ExprKind::Match { arms, .. } = &matched_shape.kind else {
        return Err("expected structural match expression");
    };
    assert!(matches!(
        &arms[0].pattern.kind,
        tune_hir::pattern::PatternKind::StructuralShape(requirements)
            if requirements.len() == 1
    ));

    let repeated = module.items[13]
        .body
        .as_ref()
        .ok_or("expected repeated body")?;
    assert!(matches!(
        repeated.kind,
        tune_hir::expr::ExprKind::While { .. }
    ));

    let forever = module.items[14]
        .body
        .as_ref()
        .ok_or("expected forever body")?;
    assert!(matches!(forever.kind, tune_hir::expr::ExprKind::Loop(_)));

    Ok(())
}

#[test]
fn lowers_top_level_declarations() {
    let source = r#"
import "std/json"
-- Tool docs.
tag tool {}
-/
Run docs.
Can be multiline.
/-
pub let run(input) = input
struct Counter {}
enum Result {}
let value = 1
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);

    assert_eq!(module.items.len(), 6);
    assert!(matches!(
        module.items[0].kind,
        tune_hir::item::ItemKind::Import
    ));
    assert_eq!(module.items[0].name.as_deref(), Some("std/json"));
    assert_eq!(module.items[1].name.as_deref(), Some("tool"));
    assert_eq!(module.items[1].doc.as_deref(), Some("Tool docs."));
    assert!(matches!(
        module.items[2].kind,
        tune_hir::item::ItemKind::CallableDecl
    ));
    assert_eq!(
        module.items[2].visibility,
        tune_hir::item::Visibility::Public
    );
    assert_eq!(
        module.items[2].doc.as_deref(),
        Some("Run docs.\nCan be multiline.")
    );
    assert_eq!(module.items[5].name.as_deref(), Some("value"));
    assert_eq!(module.items[5].id, tune_hir::HirId(5));
}

#[test]
fn lowers_shape_annotations_to_hir_shape_exprs() -> Result<(), &'static str> {
    let source = "let value: [Int | String]? = none";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let shape = module.items[0]
        .shape
        .as_ref()
        .ok_or("expected shape annotation")?;

    assert!(matches!(
        shape.kind,
        tune_hir::shape::ShapeExprKind::Optional(_)
    ));
    let tune_hir::shape::ShapeExprKind::Optional(inner) = &shape.kind else {
        return Err("expected optional shape");
    };
    let tune_hir::shape::ShapeExprKind::Sequence(element) = &inner.kind else {
        return Err("optional should wrap sequence");
    };
    assert!(matches!(
        element.kind,
        tune_hir::shape::ShapeExprKind::Union(_)
    ));

    Ok(())
}

#[test]
fn lowers_callable_signature_params_and_return_shape() {
    let source = "let parse(text: String, strict: Bool): Result = text";
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);
    let item = &module.items[0];

    assert_eq!(item.name.as_deref(), Some("parse"));
    assert_eq!(item.params.len(), 2);
    assert_eq!(item.params[0].name.as_deref(), Some("text"));
    assert!(item.params[0].shape.is_some());
    assert_eq!(item.params[1].name.as_deref(), Some("strict"));
    assert!(item.params[1].shape.is_some());
    assert!(item.shape.is_some());
}

#[test]
fn lowers_tag_applications_to_hir_items() {
    let source = r#"
tag tool {}
@tool
pub let search(query) = query
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);

    assert_eq!(module.items[1].name.as_deref(), Some("search"));
    assert_eq!(module.items[1].tags.len(), 1);
    assert_eq!(module.items[1].tags[0].name, "tool");
    assert!(module.items[1].tags[0].span.is_some());
}

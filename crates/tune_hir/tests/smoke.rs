#[test]
fn lowers_top_level_declarations() {
    let source = r#"
tag tool {}
pub let run(input) = input
struct Counter {}
enum Result {}
let value = 1
"#;
    let parsed = tune_syntax::parse(source);
    let module = tune_hir::lower::lower_module(source, &parsed.cst);

    assert_eq!(module.items.len(), 5);
    assert_eq!(module.items[0].name.as_deref(), Some("tool"));
    assert!(matches!(
        module.items[1].kind,
        tune_hir::item::ItemKind::CallableDecl
    ));
    assert_eq!(
        module.items[1].visibility,
        tune_hir::item::Visibility::Public
    );
    assert_eq!(module.items[4].name.as_deref(), Some("value"));
    assert_eq!(module.items[4].id, tune_hir::HirId(4));
}

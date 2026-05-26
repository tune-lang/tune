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

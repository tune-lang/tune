use tune_ast::AstNode;
use tune_ast::nodes::{
    DocumentedItem, EnumDecl, ImportDecl, Item as AstItem, LetDecl, Root, StructDecl, TagDecl,
};
use tune_syntax::CstNode;

use crate::item::{Item, ItemKind, Visibility};
use crate::module::Module;
use crate::{HirId, ModuleId};

#[must_use]
pub fn lower_module(source: &str, cst: &CstNode) -> Module {
    let items = Root::cast(cst)
        .map(|root| lower_items(source, root))
        .unwrap_or_default();

    Module {
        id: ModuleId(0),
        items,
    }
}

fn lower_items(source: &str, root: Root<'_>) -> Vec<Item> {
    let mut items = Vec::new();
    for item in root.documented_items() {
        lower_item(source, item, Visibility::Private, &mut items);
    }
    items
}

fn lower_item(
    source: &str,
    documented: DocumentedItem<'_>,
    visibility: Visibility,
    items: &mut Vec<Item>,
) {
    let doc = documented.doc_text(source);
    match documented.item {
        AstItem::Import(node) => push_item(items, lower_import(source, node, visibility, doc)),
        AstItem::Let(node) => push_item(items, lower_let(source, node, visibility, doc)),
        AstItem::Struct(node) => push_item(
            items,
            lower_named(source, node, ItemKind::Struct, visibility, doc),
        ),
        AstItem::Enum(node) => push_item(
            items,
            lower_named(source, node, ItemKind::Enum, visibility, doc),
        ),
        AstItem::Tag(node) => push_item(
            items,
            lower_named(source, node, ItemKind::Tag, visibility, doc),
        ),
        AstItem::Pub(node) => {
            if let Some(item) = node.item() {
                lower_item(
                    source,
                    DocumentedItem {
                        item,
                        docs: documented.docs,
                    },
                    Visibility::Public,
                    items,
                );
            }
        }
    }
}

fn push_item(items: &mut Vec<Item>, mut item: Item) {
    if let Ok(index) = u32::try_from(items.len()) {
        item.id = HirId(index);
        items.push(item);
    }
}

fn lower_import(
    source: &str,
    node: ImportDecl<'_>,
    visibility: Visibility,
    doc: Option<String>,
) -> Item {
    Item {
        id: HirId(0),
        name: node.path(source).map(str::to_owned),
        kind: ItemKind::Import,
        visibility,
        span: node.syntax().span,
        doc,
    }
}

fn lower_let(source: &str, node: LetDecl<'_>, visibility: Visibility, doc: Option<String>) -> Item {
    Item {
        id: HirId(0),
        name: node.name(source).map(str::to_owned),
        kind: if node.is_callable_decl() {
            ItemKind::CallableDecl
        } else {
            ItemKind::Let
        },
        visibility,
        span: node.syntax().span,
        doc,
    }
}

trait NamedDecl {
    fn name(self, source: &str) -> Option<&str>;
    fn span(self) -> Option<tune_diagnostics::Span>;
}

impl NamedDecl for StructDecl<'_> {
    fn name(self, source: &str) -> Option<&str> {
        self.name(source)
    }

    fn span(self) -> Option<tune_diagnostics::Span> {
        self.syntax().span
    }
}

impl NamedDecl for EnumDecl<'_> {
    fn name(self, source: &str) -> Option<&str> {
        self.name(source)
    }

    fn span(self) -> Option<tune_diagnostics::Span> {
        self.syntax().span
    }
}

impl NamedDecl for TagDecl<'_> {
    fn name(self, source: &str) -> Option<&str> {
        self.name(source)
    }

    fn span(self) -> Option<tune_diagnostics::Span> {
        self.syntax().span
    }
}

fn lower_named(
    source: &str,
    node: impl NamedDecl + Copy,
    kind: ItemKind,
    visibility: Visibility,
    doc: Option<String>,
) -> Item {
    Item {
        id: HirId(0),
        name: node.name(source).map(str::to_owned),
        kind,
        visibility,
        span: node.span(),
        doc,
    }
}

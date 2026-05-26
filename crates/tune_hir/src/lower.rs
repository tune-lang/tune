use tune_ast::AstNode;
use tune_ast::nodes::{
    DocumentedItem, EnumDecl, ImportDecl, Item as AstItem, LetDecl, Root, Shape as AstShape,
    StructDecl, TagDecl,
};
use tune_syntax::{CstElement, CstNode, SyntaxKind, TokenKind};

use crate::item::{Item, ItemKind, TagApplication, Visibility};
use crate::module::Module;
use crate::shape::{ShapeExpr, ShapeExprKind};
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
    let tags = lower_tags(source, &documented.tags);
    match documented.item {
        AstItem::Import(node) => {
            push_item(items, lower_import(source, node, visibility, doc, tags))
        }
        AstItem::Let(node) => push_item(items, lower_let(source, node, visibility, doc, tags)),
        AstItem::Struct(node) => push_item(
            items,
            lower_named(source, node, ItemKind::Struct, visibility, doc, tags),
        ),
        AstItem::Enum(node) => push_item(
            items,
            lower_named(source, node, ItemKind::Enum, visibility, doc, tags),
        ),
        AstItem::Tag(node) => push_item(
            items,
            lower_named(source, node, ItemKind::Tag, visibility, doc, tags),
        ),
        AstItem::Pub(node) => {
            if let Some(item) = node.item() {
                lower_item(
                    source,
                    DocumentedItem {
                        item,
                        docs: documented.docs,
                        tags: documented.tags,
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
    tags: Vec<TagApplication>,
) -> Item {
    Item {
        id: HirId(0),
        name: node.path(source).map(str::to_owned),
        kind: ItemKind::Import,
        visibility,
        span: node.syntax().span,
        doc,
        tags,
        shape: None,
    }
}

fn lower_let(
    source: &str,
    node: LetDecl<'_>,
    visibility: Visibility,
    doc: Option<String>,
    tags: Vec<TagApplication>,
) -> Item {
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
        tags,
        shape: node
            .shape_annotation()
            .map(|shape| lower_shape(source, shape)),
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
    tags: Vec<TagApplication>,
) -> Item {
    Item {
        id: HirId(0),
        name: node.name(source).map(str::to_owned),
        kind,
        visibility,
        span: node.span(),
        doc,
        tags,
        shape: None,
    }
}

fn lower_tags(source: &str, tags: &[tune_ast::nodes::TagApplication<'_>]) -> Vec<TagApplication> {
    tags.iter()
        .filter_map(|tag| {
            Some(TagApplication {
                name: tag.name(source)?.to_owned(),
                span: tag.syntax().span,
            })
        })
        .collect()
}

fn lower_shape(source: &str, shape: AstShape<'_>) -> ShapeExpr {
    let span = shape.syntax().span;
    let kind = match shape {
        AstShape::Named(node) => first_shape_name(node.syntax(), source)
            .map(str::to_owned)
            .map(ShapeExprKind::Named)
            .unwrap_or(ShapeExprKind::Missing),
        AstShape::Sequence(node) => child_shapes(node.syntax())
            .into_iter()
            .next()
            .map(|shape| ShapeExprKind::Sequence(Box::new(lower_shape(source, shape))))
            .unwrap_or(ShapeExprKind::Missing),
        AstShape::Tuple(node) => ShapeExprKind::Tuple(shape_list_items(source, node.syntax())),
        AstShape::Optional(node) => child_shapes(node.syntax())
            .into_iter()
            .next()
            .map(|shape| ShapeExprKind::Optional(Box::new(lower_shape(source, shape))))
            .unwrap_or(ShapeExprKind::Missing),
        AstShape::Union(node) => ShapeExprKind::Union(
            child_shapes(node.syntax())
                .into_iter()
                .map(|shape| lower_shape(source, shape))
                .collect(),
        ),
        AstShape::Callable(node) => lower_callable_shape(source, node.syntax()),
    };

    ShapeExpr { kind, span }
}

fn lower_callable_shape(source: &str, node: &CstNode) -> ShapeExprKind {
    let mut children = child_shapes(node);
    let params = children
        .first()
        .map(|shape| shape_list_items(source, shape.syntax()))
        .unwrap_or_default();
    let ret = children
        .pop()
        .map(|shape| lower_shape(source, shape))
        .unwrap_or(ShapeExpr {
            kind: ShapeExprKind::Missing,
            span: node.span,
        });

    ShapeExprKind::Callable {
        params,
        ret: Box::new(ret),
    }
}

fn shape_list_items(source: &str, node: &CstNode) -> Vec<ShapeExpr> {
    node.children
        .iter()
        .flat_map(|child| match child {
            CstElement::Node(node) if node.kind == SyntaxKind::ShapeList => child_shapes(node),
            CstElement::Node(node) => AstShape::cast(node).into_iter().collect(),
            CstElement::Token(_) => Vec::new(),
        })
        .map(|shape| lower_shape(source, shape))
        .collect()
}

fn child_shapes<'tree>(node: &'tree CstNode) -> Vec<AstShape<'tree>> {
    node.children
        .iter()
        .filter_map(|child| match child {
            CstElement::Node(node) => AstShape::cast(node),
            CstElement::Token(_) => None,
        })
        .collect()
}

fn first_shape_name<'src>(node: &CstNode, source: &'src str) -> Option<&'src str> {
    node.children.iter().find_map(|child| match child {
        CstElement::Token(token)
            if matches!(token.kind, TokenKind::Ident | TokenKind::KeywordNever) =>
        {
            let start = token.span.start.get() as usize;
            let end = token.span.end.get() as usize;
            source.get(start..end)
        }
        CstElement::Node(node) => first_shape_name(node, source),
        CstElement::Token(_) => None,
    })
}

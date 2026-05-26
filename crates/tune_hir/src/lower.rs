mod shapes;

use tune_ast::AstNode;
use tune_ast::nodes::{
    DocumentedItem, EnumDecl, ImportDecl, Item as AstItem, LetDecl, Root, StructDecl, TagDecl,
};
use tune_syntax::CstNode;

use crate::item::{Field, Item, ItemKind, Param, TagApplication, Variant, Visibility};
use crate::module::Module;
use crate::{HirId, MemberId, ModuleId};

use shapes::lower_shape;

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
        AstItem::Struct(node) => {
            push_item(items, lower_struct(source, node, visibility, doc, tags))
        }
        AstItem::Enum(node) => push_item(items, lower_enum(source, node, visibility, doc, tags)),
        AstItem::Tag(node) => push_item(items, lower_tag(source, node, visibility, doc, tags)),
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
        assign_member_owners(&mut item);
        items.push(item);
    }
}

fn assign_member_owners(item: &mut Item) {
    for param in &mut item.params {
        param.id.owner = item.id;
    }
    for field in &mut item.fields {
        field.id.owner = item.id;
    }
    for variant in &mut item.variants {
        variant.id.owner = item.id;
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
        params: Vec::new(),
        fields: Vec::new(),
        variants: Vec::new(),
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
        params: lower_params(source, node),
        fields: Vec::new(),
        variants: Vec::new(),
        shape: node
            .shape_annotation()
            .map(|shape| lower_shape(source, shape)),
    }
}

fn lower_struct(
    source: &str,
    node: StructDecl<'_>,
    visibility: Visibility,
    doc: Option<String>,
    tags: Vec<TagApplication>,
) -> Item {
    Item {
        id: HirId(0),
        name: node.name(source).map(str::to_owned),
        kind: ItemKind::Struct,
        visibility,
        span: node.syntax().span,
        doc,
        tags,
        params: Vec::new(),
        fields: lower_fields(source, node.fields()),
        variants: Vec::new(),
        shape: None,
    }
}

fn lower_enum(
    source: &str,
    node: EnumDecl<'_>,
    visibility: Visibility,
    doc: Option<String>,
    tags: Vec<TagApplication>,
) -> Item {
    Item {
        id: HirId(0),
        name: node.name(source).map(str::to_owned),
        kind: ItemKind::Enum,
        visibility,
        span: node.syntax().span,
        doc,
        tags,
        params: Vec::new(),
        fields: Vec::new(),
        variants: lower_variants(source, node.variants()),
        shape: None,
    }
}

fn lower_tag(
    source: &str,
    node: TagDecl<'_>,
    visibility: Visibility,
    doc: Option<String>,
    tags: Vec<TagApplication>,
) -> Item {
    Item {
        id: HirId(0),
        name: node.name(source).map(str::to_owned),
        kind: ItemKind::Tag,
        visibility,
        span: node.syntax().span,
        doc,
        tags,
        params: Vec::new(),
        fields: lower_fields(source, node.fields()),
        variants: Vec::new(),
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

fn lower_params(source: &str, node: LetDecl<'_>) -> Vec<Param> {
    node.params()
        .into_iter()
        .flat_map(|params| params.params())
        .enumerate()
        .filter_map(|(index, param)| {
            Some(Param {
                id: member_id(index)?,
                name: param.name(source).map(str::to_owned),
                span: param.syntax().span,
                shape: param
                    .shape_annotation()
                    .map(|shape| lower_shape(source, shape)),
            })
        })
        .collect()
}

fn lower_fields(source: &str, fields: Vec<tune_ast::nodes::DocumentedField<'_>>) -> Vec<Field> {
    fields
        .into_iter()
        .enumerate()
        .filter_map(|(index, documented)| {
            Some(Field {
                id: member_id(index)?,
                name: documented.field.name(source).map(str::to_owned),
                span: documented.field.syntax().span,
                doc: documented.doc_text(source),
                shape: documented
                    .field
                    .shape_annotation()
                    .map(|shape| lower_shape(source, shape)),
            })
        })
        .collect()
}

fn lower_variants(
    source: &str,
    variants: Vec<tune_ast::nodes::DocumentedVariant<'_>>,
) -> Vec<Variant> {
    variants
        .into_iter()
        .enumerate()
        .filter_map(|(index, documented)| {
            Some(Variant {
                id: member_id(index)?,
                name: documented.variant.name(source).map(str::to_owned),
                span: documented.variant.syntax().span,
                doc: documented.doc_text(source),
                payload: documented
                    .variant
                    .payload_shapes()
                    .into_iter()
                    .map(|shape| lower_shape(source, shape))
                    .collect(),
            })
        })
        .collect()
}

fn member_id(index: usize) -> Option<MemberId> {
    Some(MemberId {
        owner: HirId(0),
        index: u32::try_from(index).ok()?,
    })
}

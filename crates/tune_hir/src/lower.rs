mod exprs;
mod members;
mod shapes;

use tune_ast::AstNode;
use tune_ast::nodes::{
    DocumentedItem, EnumDecl, ImportDecl, Item as AstItem, LetDecl, Root, StructDecl, TagDecl,
};
use tune_syntax::CstNode;

use crate::item::{Item, ItemKind, StructMember, TagApplication, TagArg, TypeParam, Visibility};
use crate::module::Module;
use crate::{HirId, MemberId, MemberKind, ModuleId};

use exprs::ExprLowerer;
use members::{lower_fields, lower_params, lower_struct_members, lower_variants};
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
    let mut exprs = ExprLowerer::default();
    for item in root.documented_items() {
        lower_item(source, item, Visibility::Private, &mut items, &mut exprs);
    }
    items
}

fn lower_item(
    source: &str,
    documented: DocumentedItem<'_>,
    visibility: Visibility,
    items: &mut Vec<Item>,
    exprs: &mut ExprLowerer,
) {
    let doc = documented.doc_text(source);
    let tags = lower_tags(source, &documented.tags, exprs);
    match documented.item {
        AstItem::Import(node) => {
            push_item(items, lower_import(source, node, visibility, doc, tags))
        }
        AstItem::Let(node) => {
            push_item(items, lower_let(source, node, visibility, doc, tags, exprs))
        }
        AstItem::Struct(node) => push_item(
            items,
            lower_struct(source, node, visibility, doc, tags, exprs),
        ),
        AstItem::Enum(node) => push_item(items, lower_enum(source, node, visibility, doc, tags)),
        AstItem::Tag(node) => {
            push_item(items, lower_tag(source, node, visibility, doc, tags, exprs))
        }
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
                    exprs,
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
    for param in &mut item.type_params {
        param.id.owner = item.id;
    }
    let mut next_param_index = 0;
    for param in &mut item.params {
        param.id.owner = item.id;
        param.id.index = next_param_index;
        next_param_index = next_param_index.saturating_add(1);
    }
    for field in &mut item.fields {
        field.id.owner = item.id;
    }
    for member in &mut item.struct_members {
        assign_struct_member_owner(member, item.id, &mut next_param_index);
    }
    for variant in &mut item.variants {
        variant.id.owner = item.id;
    }
}

fn assign_struct_member_owner(member: &mut StructMember, owner: HirId, next_param_index: &mut u32) {
    match member {
        StructMember::Field(field) => field.id.owner = owner,
        StructMember::Callable(callable) => {
            callable.id.owner = owner;
            for param in &mut callable.params {
                param.id.owner = owner;
                param.id.index = *next_param_index;
                *next_param_index = next_param_index.saturating_add(1);
            }
        }
        StructMember::SequenceMaterializer(materializer) => materializer.id.owner = owner,
        StructMember::IndexAccess(access) => {
            access.id.owner = owner;
            access.index_param_id.owner = owner;
            access.index_param_id.index = *next_param_index;
            *next_param_index = next_param_index.saturating_add(1);
        }
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
        type_params: Vec::new(),
        params: Vec::new(),
        struct_members: Vec::new(),
        fields: Vec::new(),
        variants: Vec::new(),
        shape: None,
        body: None,
    }
}

fn lower_let(
    source: &str,
    node: LetDecl<'_>,
    visibility: Visibility,
    doc: Option<String>,
    tags: Vec<TagApplication>,
    exprs: &mut ExprLowerer,
) -> Item {
    let body = node.body_expr().map(|expr| exprs.lower(source, expr));
    Item {
        id: HirId(0),
        name: binding_name(node.name(source)),
        kind: if node.is_callable_decl() {
            ItemKind::CallableDecl
        } else {
            ItemKind::Let
        },
        visibility,
        span: node.syntax().span,
        doc,
        tags,
        type_params: Vec::new(),
        params: lower_params(source, node),
        struct_members: Vec::new(),
        fields: Vec::new(),
        variants: Vec::new(),
        shape: node
            .shape_annotation()
            .map(|shape| lower_shape(source, shape)),
        body,
    }
}

fn binding_name(name: Option<&str>) -> Option<String> {
    name.filter(|name| *name != "_").map(str::to_owned)
}

fn lower_struct(
    source: &str,
    node: StructDecl<'_>,
    visibility: Visibility,
    doc: Option<String>,
    tags: Vec<TagApplication>,
    exprs: &mut ExprLowerer,
) -> Item {
    let struct_members = lower_struct_members(source, node.members(), exprs);
    Item {
        id: HirId(0),
        name: node.name(source).map(str::to_owned),
        kind: ItemKind::Struct,
        visibility,
        span: node.syntax().span,
        doc,
        tags,
        type_params: lower_type_params(source, node.type_params()),
        params: Vec::new(),
        fields: struct_members
            .iter()
            .filter_map(|member| match member {
                StructMember::Field(field) => Some(field.clone()),
                StructMember::Callable(_)
                | StructMember::SequenceMaterializer(_)
                | StructMember::IndexAccess(_) => None,
            })
            .collect(),
        struct_members,
        variants: Vec::new(),
        shape: None,
        body: None,
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
        type_params: lower_type_params(source, node.type_params()),
        params: Vec::new(),
        struct_members: Vec::new(),
        fields: Vec::new(),
        variants: lower_variants(source, node.variants()),
        shape: None,
        body: None,
    }
}

fn lower_tag(
    source: &str,
    node: TagDecl<'_>,
    visibility: Visibility,
    doc: Option<String>,
    tags: Vec<TagApplication>,
    exprs: &mut ExprLowerer,
) -> Item {
    Item {
        id: HirId(0),
        name: node.name(source).map(str::to_owned),
        kind: ItemKind::Tag,
        visibility,
        span: node.syntax().span,
        doc,
        tags,
        type_params: Vec::new(),
        params: Vec::new(),
        struct_members: Vec::new(),
        fields: lower_fields(source, node.fields(), exprs),
        variants: Vec::new(),
        shape: None,
        body: None,
    }
}

fn lower_type_params(
    source: &str,
    params: Vec<tune_ast::nodes::TypeParamDecl<'_>>,
) -> Vec<TypeParam> {
    params
        .into_iter()
        .enumerate()
        .filter_map(|(index, param)| {
            Some(TypeParam {
                id: MemberId {
                    owner: HirId(0),
                    kind: MemberKind::TypeParam,
                    index: u32::try_from(index).ok()?,
                },
                name: param.name(source).map(str::to_owned),
                span: param.syntax().span,
            })
        })
        .collect()
}

fn lower_tags(
    source: &str,
    tags: &[tune_ast::nodes::TagApplication<'_>],
    exprs: &mut ExprLowerer,
) -> Vec<TagApplication> {
    tags.iter()
        .filter_map(|tag| {
            Some(TagApplication {
                name: tag.name(source)?.to_owned(),
                span: tag.syntax().span,
                args: tag
                    .args()
                    .into_iter()
                    .filter_map(|arg| {
                        Some(TagArg {
                            name: arg.name(source).map(str::to_owned),
                            value: exprs.lower(source, arg.value_expr()?),
                        })
                    })
                    .collect(),
            })
        })
        .collect()
}

mod resolver;

use tune_hir::item::{Item, StructMember};

use crate::locals::LocalKind;

use self::resolver::BodyResolver;
use super::ResolvedModule;

pub(super) fn resolve_item_body(resolved: &mut ResolvedModule, item: &Item, items: &[Item]) {
    if item.tags.iter().any(|tag| !tag.args.is_empty()) {
        let mut resolver = BodyResolver::new(resolved, items, item.id);
        for tag in &item.tags {
            for arg in &tag.args {
                resolver.resolve_expr_names(&arg.value);
            }
        }
    }

    if let Some(body) = &item.body {
        let mut resolver = BodyResolver::new(resolved, items, item.id);

        for param in &item.params {
            if let Some(name) = &param.name {
                resolver.bind_param(name, param.id);
            }
        }

        resolver.resolve_expr_names_with_expected(body, item.shape.as_ref());
    }

    for member in &item.struct_members {
        resolve_struct_member_body(resolved, item, member, items);
    }
}

fn resolve_struct_member_body(
    resolved: &mut ResolvedModule,
    item: &Item,
    member: &StructMember,
    items: &[Item],
) {
    match member {
        StructMember::Callable(callable) => {
            let Some(body) = &callable.body else {
                return;
            };
            let mut resolver = BodyResolver::new(resolved, items, item.id);
            for param in &callable.params {
                if let Some(name) = &param.name {
                    resolver.bind_param(name, param.id);
                }
            }
            resolver.resolve_expr_names_with_expected(body, callable.shape.as_ref());
        }
        StructMember::SequenceMaterializer(materializer) => {
            let Some(body) = &materializer.body else {
                return;
            };
            let mut resolver = BodyResolver::new(resolved, items, item.id);
            if let Some(name) = &materializer.param_name {
                resolver.bind_local(name, LocalKind::Pattern, None, materializer.span);
            }
            resolver.resolve_expr_names(body);
        }
        StructMember::IndexAccess(access) => {
            let Some(body) = &access.body else {
                return;
            };
            let mut resolver = BodyResolver::new(resolved, items, item.id);
            if let Some(name) = &access.index_param_name {
                resolver.bind_param(name, access.index_param_id);
            }
            resolver.resolve_expr_names(body);
        }
        StructMember::Field(_) => {}
    }
}

use std::collections::{HashSet, VecDeque};

use tune_hir::HirId;
use tune_hir::item::{CallableMember, Field, Item, ItemKind, StructMember, TypeParam, Variant};
use tune_hir::module::Module;
use tune_hir::shape::{ShapeExpr, ShapeExprKind, StructuralShapeRequirementKind};
use tune_resolve::{NameTarget, ResolvedModule};

use crate::imports_remap::item_expr_ids;

pub(crate) fn selected_import_closure(
    imported: &Module,
    imported_resolved: &ResolvedModule,
    roots: &[HirId],
) -> Vec<HirId> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();
    let mut queue = roots.iter().copied().collect::<VecDeque<_>>();

    while let Some(item_id) = queue.pop_front() {
        if !seen.insert(item_id) {
            continue;
        }
        result.push(item_id);

        let Some(item) = imported.items.iter().find(|item| item.id == item_id) else {
            continue;
        };
        for dependency in item_dependencies(imported, imported_resolved, item) {
            if !seen.contains(&dependency) {
                queue.push_back(dependency);
            }
        }
    }

    result
}

pub(crate) fn item_by_name<'a>(module: &'a Module, name: &str) -> Option<&'a Item> {
    module
        .items
        .iter()
        .find(|item| item.name.as_deref() == Some(name))
}

fn item_dependencies(
    imported: &Module,
    imported_resolved: &ResolvedModule,
    item: &Item,
) -> Vec<HirId> {
    let expr_ids = item_expr_ids(item).into_iter().collect::<HashSet<_>>();
    let mut dependencies = Vec::new();

    for name_ref in &imported_resolved.name_refs {
        if !expr_ids.contains(&name_ref.expr.0) {
            continue;
        }
        let NameTarget::TopLevel(target) = name_ref.target else {
            continue;
        };
        push_dependency(imported, item.id, target, &mut dependencies);
    }

    for name in item_shape_refs(item) {
        if let Some(target) = item_by_name(imported, &name).map(|item| item.id) {
            push_dependency(imported, item.id, target, &mut dependencies);
        }
    }

    dependencies
}

fn push_dependency(imported: &Module, owner: HirId, target: HirId, dependencies: &mut Vec<HirId>) {
    if target == owner || dependencies.contains(&target) {
        return;
    }
    let Some(item) = imported.items.iter().find(|item| item.id == target) else {
        return;
    };
    if item.kind == ItemKind::Import {
        return;
    }
    dependencies.push(target);
}

fn item_shape_refs(item: &Item) -> Vec<String> {
    let mut refs = Vec::new();
    for type_param in &item.type_params {
        collect_type_param_shape_refs(type_param, &mut refs);
    }
    for param in &item.params {
        if let Some(shape) = &param.shape {
            collect_shape_refs(shape, &mut refs);
        }
    }
    if let Some(shape) = &item.shape {
        collect_shape_refs(shape, &mut refs);
    }
    for field in &item.fields {
        collect_field_shape_refs(field, &mut refs);
    }
    for member in &item.struct_members {
        collect_struct_member_shape_refs(member, &mut refs);
    }
    for variant in &item.variants {
        collect_variant_shape_refs(variant, &mut refs);
    }
    refs
}

fn collect_type_param_shape_refs(type_param: &TypeParam, refs: &mut Vec<String>) {
    if let Some(shape) = &type_param.constraint {
        collect_shape_refs(shape, refs);
    }
}

fn collect_field_shape_refs(field: &Field, refs: &mut Vec<String>) {
    if let Some(shape) = &field.shape {
        collect_shape_refs(shape, refs);
    }
}

fn collect_struct_member_shape_refs(member: &StructMember, refs: &mut Vec<String>) {
    match member {
        StructMember::Field(field) => collect_field_shape_refs(field, refs),
        StructMember::Callable(CallableMember { params, shape, .. }) => {
            for param in params {
                if let Some(shape) = &param.shape {
                    collect_shape_refs(shape, refs);
                }
            }
            if let Some(shape) = shape {
                collect_shape_refs(shape, refs);
            }
        }
        StructMember::SequenceMaterializer(_) => {}
        StructMember::IndexAccess(access) => {
            if let Some(shape) = &access.index_shape {
                collect_shape_refs(shape, refs);
            }
            if let Some(shape) = &access.result_shape {
                collect_shape_refs(shape, refs);
            }
        }
    }
}

fn collect_variant_shape_refs(variant: &Variant, refs: &mut Vec<String>) {
    for payload in &variant.payload {
        collect_shape_refs(payload, refs);
    }
}

fn collect_shape_refs(shape: &ShapeExpr, refs: &mut Vec<String>) {
    match &shape.kind {
        ShapeExprKind::Named(name) => refs.push(name.clone()),
        ShapeExprKind::Generic { name, args } => {
            refs.push(name.clone());
            for arg in args {
                collect_shape_refs(arg, refs);
            }
        }
        ShapeExprKind::Sequence(inner) | ShapeExprKind::Optional(inner) => {
            collect_shape_refs(inner, refs);
        }
        ShapeExprKind::Union(items) | ShapeExprKind::Tuple(items) => {
            for item in items {
                collect_shape_refs(item, refs);
            }
        }
        ShapeExprKind::Structural(requirements) => {
            for requirement in requirements {
                collect_structural_requirement_refs(requirement, refs);
            }
        }
        ShapeExprKind::Callable { params, ret } => {
            for param in params {
                collect_shape_refs(param, refs);
            }
            collect_shape_refs(ret, refs);
        }
        ShapeExprKind::Missing => {}
    }
}

fn collect_structural_requirement_refs(
    requirement: &tune_hir::shape::StructuralShapeRequirement,
    refs: &mut Vec<String>,
) {
    match &requirement.kind {
        StructuralShapeRequirementKind::Field { shape } => {
            if let Some(shape) = shape {
                collect_shape_refs(shape, refs);
            }
        }
        StructuralShapeRequirementKind::Callable { params, ret } => {
            for param in params {
                collect_shape_refs(param, refs);
            }
            if let Some(ret) = ret {
                collect_shape_refs(ret, refs);
            }
        }
    }
}

use std::collections::{HashMap, HashSet};

use tune_db::{FileId, TuneDb};
use tune_diagnostics::{Diagnostic, Span};
use tune_hir::item::{
    ExternalItem, ExternalSymbolId, ImportSelector, Item, ItemKind, ModuleNamespaceMember, Param,
    Visibility,
};
use tune_hir::module::Module;
use tune_hir::{HirId, MemberId, MemberKind};
use tune_host::HostFunction;
use tune_resolve::ResolvedModule;

use crate::host::HostRegistry;
use crate::imports_closure::{item_by_name, selected_import_closure};
use crate::imports_diagnostics::{
    import_cycle, private_import_member, unresolved_import, unresolved_import_member,
};
use crate::imports_internalize::{ImportInternalNames, internalized_import_item};
use crate::imports_remap::{next_expr_id, remap_item};
use crate::imports_shapes::shape_expr;

pub(crate) struct LinkedModule {
    pub(crate) parsed: Vec<tune_syntax::Parsed>,
    pub(crate) module: Module,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

pub(crate) fn link_entry_imports(
    db: &TuneDb,
    entry: FileId,
    hosts: &HostRegistry,
) -> Option<LinkedModule> {
    link_entry_imports_with_sources(db, entry, hosts, None, &[])
}

pub(crate) fn link_entry_imports_for_files(
    db: &TuneDb,
    entry: FileId,
    hosts: &HostRegistry,
    files: &[FileId],
    import_aliases: &[(String, FileId)],
) -> Option<LinkedModule> {
    let allowed = files.iter().copied().collect::<HashSet<_>>();
    if !allowed.contains(&entry) {
        return None;
    }
    link_entry_imports_with_sources(db, entry, hosts, Some(&allowed), import_aliases)
}

fn link_entry_imports_with_sources(
    db: &TuneDb,
    entry: FileId,
    hosts: &HostRegistry,
    allowed: Option<&HashSet<FileId>>,
    import_aliases: &[(String, FileId)],
) -> Option<LinkedModule> {
    let mut parsed = Vec::new();
    let mut diagnostics = Vec::new();
    let mut sources_by_path = db
        .sources()
        .iter()
        .filter(|source| allowed.is_none_or(|allowed| allowed.contains(&source.id)))
        .map(|source| (source.path.clone(), source.id))
        .collect::<HashMap<_, _>>();
    sources_by_path.extend(import_aliases.iter().cloned());
    let mut stack = Vec::new();

    let module = link_source_imports(
        db,
        entry,
        hosts,
        &sources_by_path,
        &mut stack,
        &mut parsed,
        &mut diagnostics,
    )?;

    Some(LinkedModule {
        parsed,
        module,
        diagnostics,
    })
}

fn link_source_imports(
    db: &TuneDb,
    file: FileId,
    hosts: &HostRegistry,
    sources_by_path: &HashMap<String, FileId>,
    stack: &mut Vec<FileId>,
    parsed: &mut Vec<tune_syntax::Parsed>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Module> {
    let source = db.source(file)?;
    let parsed_source = tune_syntax::parse_with_file(file, &source.text);
    let mut module = tune_hir::lower::lower_module(&source.text, &parsed_source.cst);
    parsed.push(parsed_source);
    stack.push(file);

    let imports = module
        .items
        .iter()
        .filter_map(|item| Some((item.span, item.import.clone()?)))
        .collect::<Vec<_>>();

    for (span, import) in imports {
        let Some(imported_file) = sources_by_path.get(import.path.as_str()).copied() else {
            if append_host_imports(
                &mut module,
                hosts,
                &import.path,
                &import.selector,
                span,
                diagnostics,
            ) {
                continue;
            }
            diagnostics.push(unresolved_import(&import.path, span));
            continue;
        };
        if stack.contains(&imported_file) {
            diagnostics.push(import_cycle(&import.path, span));
            continue;
        }
        let Some(imported_module) = link_source_imports(
            db,
            imported_file,
            hosts,
            sources_by_path,
            stack,
            parsed,
            diagnostics,
        ) else {
            diagnostics.push(unresolved_import(&import.path, span));
            continue;
        };
        let imported_resolved = tune_resolve::resolve_module(&imported_module);
        append_selected_imports(
            &mut module,
            &imported_module,
            &imported_resolved,
            &import.path,
            &import.selector,
            span,
            diagnostics,
        );
    }
    append_stdcore_prelude(&mut module, hosts);
    stack.pop();

    Some(module)
}

fn append_stdcore_prelude(module: &mut Module, hosts: &HostRegistry) {
    for function in tune_std::prelude::stdcore().functions {
        let Some(host_function) = function.host_function() else {
            continue;
        };
        if module
            .items
            .iter()
            .any(|item| item.name.as_deref() == Some(host_function.function))
        {
            continue;
        }
        let Some((symbol, function)) = hosts.function(host_function.module, host_function.function)
        else {
            continue;
        };
        append_host_item(module, symbol, function, None);
    }
}

fn append_host_imports(
    module: &mut Module,
    hosts: &HostRegistry,
    path: &str,
    selector: &ImportSelector,
    span: Option<Span>,
    diagnostics: &mut Vec<Diagnostic>,
) -> bool {
    let names = match selector {
        ImportSelector::Module => {
            return append_host_module_import(module, hosts, path, span);
        }
        ImportSelector::Member(name) => vec![name.as_str()],
        ImportSelector::Members(names) => names.iter().map(String::as_str).collect(),
    };
    let mut matched_module = false;
    for name in names {
        let Some((symbol, function)) = hosts.function(path, name) else {
            if hosts.modules().iter().any(|module| module.name == path) {
                matched_module = true;
                diagnostics.push(unresolved_import_member(name, span));
            }
            continue;
        };
        matched_module = true;
        append_host_item(module, symbol, function, span);
    }
    matched_module
}

fn append_host_module_import(
    module: &mut Module,
    hosts: &HostRegistry,
    path: &str,
    span: Option<Span>,
) -> bool {
    let Some(host_module) = hosts.modules().iter().find(|module| module.name == path) else {
        return false;
    };

    let mut members = Vec::new();
    for function in &host_module.functions {
        let Some((symbol, function)) = hosts.function(path, &function.name) else {
            continue;
        };
        let item_name = internal_host_name(path, &function.name, symbol);
        let Some(item) = append_host_item_named(module, symbol, function, item_name, None) else {
            continue;
        };
        members.push(ModuleNamespaceMember {
            name: function.name.clone(),
            item,
        });
    }
    append_module_namespace_item(module, module_alias(path), members, span);
    true
}

fn append_host_item(
    module: &mut Module,
    symbol: tune_host::HostSymbolId,
    function: &HostFunction,
    span: Option<Span>,
) {
    let _ = append_host_item_named(module, symbol, function, function.name.clone(), span);
}

fn append_host_item_named(
    module: &mut Module,
    symbol: tune_host::HostSymbolId,
    function: &HostFunction,
    item_name: String,
    span: Option<Span>,
) -> Option<HirId> {
    let index = u32::try_from(module.items.len()).ok()?;
    let owner = HirId(index);
    let params = function
        .params
        .iter()
        .enumerate()
        .filter_map(|(index, param)| {
            Some(Param {
                id: MemberId {
                    owner,
                    kind: MemberKind::Param,
                    index: u32::try_from(index).ok()?,
                },
                name: Some(param.name.clone()),
                span,
                shape: Some(shape_expr(&param.shape)),
            })
        })
        .collect();
    module.items.push(Item {
        id: owner,
        name: Some(item_name),
        kind: ItemKind::CallableDecl,
        visibility: Visibility::Public,
        span,
        doc: None,
        tags: Vec::new(),
        import: None,
        type_params: Vec::new(),
        params,
        struct_members: Vec::new(),
        fields: Vec::new(),
        variants: Vec::new(),
        shape: Some(shape_expr(&function.ret)),
        body: None,
        external: Some(ExternalItem::HostFunction {
            symbol: ExternalSymbolId(symbol.0),
            task_safe: function.task_safe,
        }),
    });
    Some(owner)
}

fn append_selected_imports(
    module: &mut Module,
    imported: &Module,
    imported_resolved: &ResolvedModule,
    path: &str,
    selector: &ImportSelector,
    span: Option<Span>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let names = match selector {
        ImportSelector::Module => {
            append_module_import(module, imported, imported_resolved, path, span);
            return;
        }
        ImportSelector::Member(name) => vec![name.as_str()],
        ImportSelector::Members(names) => names.iter().map(String::as_str).collect(),
    };
    let mut selected = Vec::new();
    for name in names {
        let Some(item) = item_by_name(imported, name) else {
            diagnostics.push(unresolved_import_member(name, span));
            continue;
        };
        if item.visibility != Visibility::Public {
            diagnostics.push(private_import_member(name, span, item.span));
            continue;
        }
        selected.push(item.id);
    }

    let closure = selected_import_closure(imported, imported_resolved, &selected);
    let internal_names = ImportInternalNames::for_closure(imported, path, &selected, &closure);
    for item_id in closure {
        if let Some(item) = imported.items.iter().find(|item| item.id == item_id) {
            let item = internalized_import_item(item, imported_resolved, &internal_names);
            append_imported_item(module, item);
        }
    }
}

fn append_module_import(
    module: &mut Module,
    imported: &Module,
    imported_resolved: &ResolvedModule,
    path: &str,
    span: Option<Span>,
) {
    let selected = imported
        .items
        .iter()
        .filter(|item| item.visibility == Visibility::Public && item.kind != ItemKind::Import)
        .map(|item| item.id)
        .collect::<Vec<_>>();
    let closure = selected_import_closure(imported, imported_resolved, &selected);
    let internal_names = ImportInternalNames::for_closure(imported, path, &[], &closure);
    let mut members = Vec::new();
    for item_id in closure {
        let Some(original) = imported.items.iter().find(|item| item.id == item_id) else {
            continue;
        };
        let item = internalized_import_item(original, imported_resolved, &internal_names);
        let Some(new_id) = append_imported_item(module, item) else {
            continue;
        };
        if selected.contains(&item_id)
            && let Some(name) = original.name.clone()
        {
            members.push(ModuleNamespaceMember { name, item: new_id });
        }
    }
    append_module_namespace_item(module, module_alias(path), members, span);
}

fn append_imported_item(module: &mut Module, mut item: Item) -> Option<HirId> {
    if let Ok(index) = u32::try_from(module.items.len()) {
        let old = item.id;
        let new = HirId(index);
        let expr_offset = next_expr_id(module);
        remap_item(&mut item, old, new, expr_offset);
        module.items.push(item);
        return Some(new);
    }
    None
}

fn append_module_namespace_item(
    module: &mut Module,
    name: String,
    members: Vec<ModuleNamespaceMember>,
    span: Option<Span>,
) {
    let Ok(index) = u32::try_from(module.items.len()) else {
        return;
    };
    module.items.push(Item {
        id: HirId(index),
        name: Some(name),
        kind: ItemKind::Import,
        visibility: Visibility::Private,
        span,
        doc: None,
        tags: Vec::new(),
        import: None,
        type_params: Vec::new(),
        params: Vec::new(),
        struct_members: Vec::new(),
        fields: Vec::new(),
        variants: Vec::new(),
        shape: None,
        body: None,
        external: Some(ExternalItem::ModuleNamespace { members }),
    });
}

fn module_alias(path: &str) -> String {
    std::path::Path::new(path)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .unwrap_or(path)
        .to_owned()
}

fn internal_host_name(path: &str, name: &str, symbol: tune_host::HostSymbolId) -> String {
    let mut out = String::from("__host_");
    for ch in path.chars().chain(std::iter::once('_')).chain(name.chars()) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    out.push('_');
    out.push_str(&symbol.0.to_string());
    out
}

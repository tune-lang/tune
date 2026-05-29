use std::collections::HashMap;

use tune_db::{FileId, TuneDb};
use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::HirId;
use tune_hir::item::{ImportSelector, Item};
use tune_hir::module::Module;

use crate::imports_remap::{next_expr_id, remap_item};

pub(crate) struct LinkedModule {
    pub(crate) parsed: Vec<tune_syntax::Parsed>,
    pub(crate) module: Module,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

pub(crate) fn link_entry_imports(db: &TuneDb, entry: FileId) -> Option<LinkedModule> {
    let entry_source = db.source(entry)?;
    let entry_parsed = tune_syntax::parse_with_file(entry, &entry_source.text);
    let mut module = tune_hir::lower::lower_module(&entry_source.text, &entry_parsed.cst);
    let mut parsed = vec![entry_parsed];
    let mut diagnostics = Vec::new();
    let sources_by_path = db
        .sources()
        .iter()
        .map(|source| (source.path.as_str(), source.id))
        .collect::<HashMap<_, _>>();

    let imports = module
        .items
        .iter()
        .filter_map(|item| Some((item.span, item.import.clone()?)))
        .collect::<Vec<_>>();

    for (span, import) in imports {
        let Some(imported_file) = sources_by_path.get(import.path.as_str()).copied() else {
            diagnostics.push(unresolved_import(&import.path, span));
            continue;
        };
        if imported_file == entry {
            diagnostics.push(self_import(&import.path, span));
            continue;
        }
        let Some(source) = db.source(imported_file) else {
            diagnostics.push(unresolved_import(&import.path, span));
            continue;
        };
        let imported_parsed = tune_syntax::parse_with_file(imported_file, &source.text);
        let imported_module = tune_hir::lower::lower_module(&source.text, &imported_parsed.cst);
        parsed.push(imported_parsed);
        append_selected_imports(
            &mut module,
            &imported_module,
            &import.selector,
            span,
            &mut diagnostics,
        );
    }

    Some(LinkedModule {
        parsed,
        module,
        diagnostics,
    })
}

fn append_selected_imports(
    module: &mut Module,
    imported: &Module,
    selector: &ImportSelector,
    span: Option<Span>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    let names = match selector {
        ImportSelector::Module => return,
        ImportSelector::Member(name) => vec![name.as_str()],
        ImportSelector::Members(names) => names.iter().map(String::as_str).collect(),
    };
    for name in names {
        let Some(item) = imported
            .items
            .iter()
            .find(|item| item.name.as_deref() == Some(name))
        else {
            diagnostics.push(unresolved_import_member(name, span));
            continue;
        };
        append_imported_item(module, item);
    }
}

fn append_imported_item(module: &mut Module, item: &Item) {
    if let Ok(index) = u32::try_from(module.items.len()) {
        let mut item = item.clone();
        let old = item.id;
        let new = HirId(index);
        let expr_offset = next_expr_id(module);
        remap_item(&mut item, old, new, expr_offset);
        module.items.push(item);
    }
}

fn unresolved_import(path: &str, span: Option<Span>) -> Diagnostic {
    Diagnostic::error(
        codes::UNRESOLVED_NAME,
        format!("unresolved import `{path}`"),
        span.unwrap_or_else(Span::synthetic),
        "this import path does not match a loaded project source",
    )
    .build()
}

fn self_import(path: &str, span: Option<Span>) -> Diagnostic {
    Diagnostic::error(
        codes::UNRESOLVED_NAME,
        format!("source imports itself as `{path}`"),
        span.unwrap_or_else(Span::synthetic),
        "a source file cannot import itself",
    )
    .build()
}

fn unresolved_import_member(name: &str, span: Option<Span>) -> Diagnostic {
    Diagnostic::error(
        codes::UNRESOLVED_NAME,
        format!("unresolved import member `{name}`"),
        span.unwrap_or_else(Span::synthetic),
        "this selector does not name a declaration in the imported source",
    )
    .build()
}

use std::collections::HashMap;

use tune_db::{FileId, TuneDb};
use tune_diagnostics::{Diagnostic, Span, codes};
use tune_hir::item::{
    ExternalItem, ExternalSymbolId, ImportSelector, Item, ItemKind, Param, Visibility,
};
use tune_hir::module::Module;
use tune_hir::shape::{ShapeExpr, ShapeExprKind};
use tune_hir::{HirId, MemberId, MemberKind};
use tune_host::HostFunction;
use tune_shape::Shape;

use crate::host::HostRegistry;
use crate::imports_remap::{next_expr_id, remap_item};

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
            if append_host_imports(
                &mut module,
                hosts,
                &import.path,
                &import.selector,
                span,
                &mut diagnostics,
            ) {
                continue;
            }
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
    append_stdcore_prelude(&mut module, hosts);

    Some(LinkedModule {
        parsed,
        module,
        diagnostics,
    })
}

fn append_stdcore_prelude(module: &mut Module, hosts: &HostRegistry) {
    if module
        .items
        .iter()
        .any(|item| item.name.as_deref() == Some("print"))
    {
        return;
    }
    let Some((symbol, function)) = hosts.function("io", "print") else {
        return;
    };
    append_host_item(module, symbol, function, None);
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
            return hosts.modules().iter().any(|module| module.name == path);
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

fn append_host_item(
    module: &mut Module,
    symbol: tune_host::HostSymbolId,
    function: &HostFunction,
    span: Option<Span>,
) {
    let Ok(index) = u32::try_from(module.items.len()) else {
        return;
    };
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
        name: Some(function.name.clone()),
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
        }),
    });
}

fn shape_expr(shape: &Shape) -> ShapeExpr {
    ShapeExpr {
        kind: shape_expr_kind(shape),
        span: None,
    }
}

fn shape_expr_kind(shape: &Shape) -> ShapeExprKind {
    match shape {
        Shape::Hole => ShapeExprKind::Missing,
        Shape::Never => ShapeExprKind::Named("Never".into()),
        Shape::Unit => ShapeExprKind::Named("Unit".into()),
        Shape::Int => ShapeExprKind::Named("Int".into()),
        Shape::Float => ShapeExprKind::Named("Float".into()),
        Shape::Size => ShapeExprKind::Named("Size".into()),
        Shape::Byte => ShapeExprKind::Named("Byte".into()),
        Shape::Bool => ShapeExprKind::Named("Bool".into()),
        Shape::String => ShapeExprKind::Named("String".into()),
        Shape::Sequence(inner) => ShapeExprKind::Sequence(Box::new(shape_expr(inner))),
        Shape::Tuple(items) => ShapeExprKind::Tuple(items.iter().map(shape_expr).collect()),
        Shape::Optional(inner) => ShapeExprKind::Optional(Box::new(shape_expr(inner))),
        Shape::Union(items) => ShapeExprKind::Union(items.iter().map(shape_expr).collect()),
        Shape::Callable { params, ret } => ShapeExprKind::Callable {
            params: params.iter().map(shape_expr).collect(),
            ret: Box::new(shape_expr(ret)),
        },
        Shape::Result { ok, err } => ShapeExprKind::Generic {
            name: "Result".into(),
            args: vec![shape_expr(ok), shape_expr(err)],
        },
        Shape::Task(inner) => ShapeExprKind::Generic {
            name: "Task".into(),
            args: vec![shape_expr(inner)],
        },
        Shape::Apply { nominal, args } => ShapeExprKind::Generic {
            name: nominal.name.clone(),
            args: args.iter().map(shape_expr).collect(),
        },
        Shape::Struct(nominal) | Shape::Enum(nominal) => ShapeExprKind::Named(nominal.name.clone()),
        Shape::Range(inner) => ShapeExprKind::Generic {
            name: "Range".into(),
            args: vec![shape_expr(inner)],
        },
        Shape::Literal(_) | Shape::Param(_) | Shape::Structural(_) => ShapeExprKind::Missing,
    }
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

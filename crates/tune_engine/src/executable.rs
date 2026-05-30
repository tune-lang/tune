use crate::diagnostics::{diagnostic_from_bytecode_lower_error, diagnostic_from_ir_lower_error};
use crate::reachable::reachable_functions;
use crate::{CompileReport, EngineError, ExecutableReport, has_error_diagnostics};

pub(crate) fn executable_from_compile(
    compile: CompileReport,
) -> Result<ExecutableReport, EngineError> {
    if has_error_diagnostics(&compile.check.diagnostics) {
        return Err(EngineError::Diagnostics(compile.check.diagnostics.clone()));
    }
    let entry_plan = compile
        .module_plan
        .entry
        .as_ref()
        .ok_or(EngineError::MissingEntry)?;
    let reachable = reachable_functions(&compile.module_plan.functions, entry_plan);
    let planned = core::iter::once(entry_plan)
        .chain(
            reachable
                .iter()
                .map(|index| &compile.module_plan.functions[*index]),
        )
        .collect::<Vec<_>>();
    let mut ir = Vec::new();
    for plan in planned {
        let function = tune_ir::lower_plan_function(plan).map_err(|error| {
            EngineError::Diagnostics(vec![diagnostic_from_ir_lower_error(
                &plan.name, plan.span, &error,
            )])
        })?;
        ir.push(function);
    }
    let _report = tune_opt::optimize_functions(&mut ir);
    let host_value_types = host_value_types(&compile.check.module);
    let mut bytecode = tune_bytecode::lower_ir_functions(&ir).map_err(|error| {
        EngineError::Diagnostics(vec![diagnostic_from_bytecode_lower_error(&error)])
    })?;
    bytecode.entry_function = Some(0);
    Ok(ExecutableReport {
        compile,
        ir,
        bytecode,
        host_value_types,
    })
}

fn host_value_types(module: &tune_hir::module::Module) -> Vec<tune_vm::VmHostValueType> {
    module
        .items
        .iter()
        .filter_map(|item| {
            let Some(tune_hir::item::ExternalItem::HostValueType { type_name }) = &item.external
            else {
                return None;
            };
            Some(tune_vm::VmHostValueType::new(
                type_name.clone(),
                item.id.0,
                item.fields
                    .iter()
                    .filter_map(|field| field.name.clone())
                    .collect(),
            ))
        })
        .collect()
}

use tune_diagnostics::render::SourceProvider;
use tune_diagnostics::{Diagnostic, FactEntry, Span};
use tune_runtime::value::{RuntimeVariant, Value};

pub(crate) fn diagnostic_from_ir_lower_error(
    function_name: &str,
    function_span: Option<Span>,
    error: &tune_ir::IrLowerError,
) -> Diagnostic {
    let span = function_span.unwrap_or_else(Span::synthetic);
    Diagnostic::error(
        tune_diagnostics::codes::EXECUTABLE_LOWERING_ERROR,
        "executable lowering failed",
        span,
        "this planned function could not be lowered to IR",
    )
    .with_fact_entries(
        "lowering context",
        vec![
            FactEntry::spanned(span, format!("function: `{function_name}`")),
            FactEntry::new(format!("IR lowering error: {error:?}")),
        ],
    )
    .with_note("the semantic plan reached the backend with an operation this executable slice does not yet support")
    .build()
}

pub(crate) fn diagnostic_from_bytecode_lower_error(
    error: &tune_bytecode::BytecodeLowerError,
) -> Diagnostic {
    let span = Span::synthetic();
    Diagnostic::error(
        tune_diagnostics::codes::EXECUTABLE_LOWERING_ERROR,
        "bytecode lowering failed",
        span,
        "IR could not be lowered to typed bytecode",
    )
    .with_fact_entries(
        "lowering context",
        vec![FactEntry::new(format!(
            "bytecode lowering error: {error:?}"
        ))],
    )
    .with_note(
        "bytecode lowering should preserve Tune meaning already made explicit by earlier phases",
    )
    .build()
}

#[must_use]
pub fn diagnostic_from_vm_fault(fault: &tune_vm::VmFault) -> Diagnostic {
    vm_fault_diagnostic(fault, |_| None)
}

#[must_use]
pub fn diagnostic_from_vm_fault_with_sources(
    fault: &tune_vm::VmFault,
    sources: &impl SourceProvider,
) -> Diagnostic {
    vm_fault_diagnostic(fault, |span| source_summary(sources, span))
}

fn vm_fault_diagnostic(
    fault: &tune_vm::VmFault,
    source_summary: impl Fn(Span) -> Option<String>,
) -> Diagnostic {
    let span = fault
        .location
        .as_ref()
        .and_then(|location| location.span)
        .unwrap_or_else(Span::synthetic);
    let mut facts = vec![FactEntry::new(format!("VM error: {:?}", fault.error))];
    if let Some(location) = &fault.location {
        facts.push(location_fact(location, &source_summary));
        facts.push(FactEntry::new(format!(
            "bytecode function: {}",
            location.function
        )));
        if let Some(instruction) = location.instruction {
            facts.push(FactEntry::new(format!(
                "bytecode instruction: {instruction}"
            )));
        }
    }
    let (code, title, primary, help) = match &fault.error {
        tune_vm::VmError::RecursiveStructState => (
            tune_diagnostics::codes::SELF_STATE_ERROR,
            "owned struct field would create a receiver-state cycle",
            "ordinary struct fields own their values; this assignment would create cyclic ownership",
            Some(
                "use an explicit handle, resource, or future weak reference for graph back-references",
            ),
        ),
        _ => (
            tune_diagnostics::codes::RUNTIME_ERROR,
            "runtime execution failed",
            "execution failed here",
            None,
        ),
    };
    let mut diagnostic = Diagnostic::error(code, title, span, primary)
        .with_fact_entries("runtime provenance", facts)
        .with_note("this diagnostic was produced from a VM fault");
    if let Some(help) = help {
        diagnostic = diagnostic.with_help(help);
    }
    diagnostic.build()
}

fn location_fact(
    location: &tune_vm::VmLocation,
    source_summary: impl Fn(Span) -> Option<String>,
) -> FactEntry {
    let function = location.function_name.as_deref().unwrap_or("<unknown>");
    let message = location.span.and_then(&source_summary).map_or_else(
        || format!("fault in `{function}`"),
        |summary| format!("fault in `{function}` at `{summary}`"),
    );
    match location.span {
        Some(span) => FactEntry::spanned(span, message),
        None => FactEntry::new(message),
    }
}

#[must_use]
pub fn diagnostics_from_runtime_value(value: &Value) -> Vec<Diagnostic> {
    diagnostic_from_result_error(value).into_iter().collect()
}

#[must_use]
pub fn diagnostics_from_runtime_value_with_sources(
    value: &Value,
    sources: &impl SourceProvider,
) -> Vec<Diagnostic> {
    diagnostic_from_result_error_with_sources(value, sources)
        .into_iter()
        .collect()
}

#[must_use]
pub fn diagnostic_from_result_error(value: &Value) -> Option<Diagnostic> {
    result_error_diagnostic(value, |_| None)
}

#[must_use]
pub fn diagnostic_from_result_error_with_sources(
    value: &Value,
    sources: &impl SourceProvider,
) -> Option<Diagnostic> {
    result_error_diagnostic(value, |span| source_summary(sources, span))
}

fn result_error_diagnostic(
    value: &Value,
    source_summary: impl Fn(Span) -> Option<String>,
) -> Option<Diagnostic> {
    let Value::Variant {
        variant: RuntimeVariant::ResultError,
        propagation_frames,
        ..
    } = value
    else {
        return None;
    };
    if propagation_frames.is_empty() {
        return None;
    }

    let primary_span = propagation_frames
        .iter()
        .rev()
        .find_map(|frame| frame.span)
        .unwrap_or_else(Span::synthetic);
    let facts = propagation_frames
        .iter()
        .map(|frame| {
            let message = frame.span.and_then(&source_summary).map_or_else(
                || format!("propagated through `{}`", frame.function_name),
                |summary| {
                    format!(
                        "propagated through `{}` at `{summary}`",
                        frame.function_name
                    )
                },
            );
            match frame.span {
                Some(span) => FactEntry::spanned(span, message),
                None => FactEntry::new(message),
            }
        })
        .collect::<Vec<_>>();

    Some(
        Diagnostic::error(
            tune_diagnostics::codes::RESULT_PROPAGATION_ERROR,
            "result error propagated",
            primary_span,
            "unhandled Result error reached this boundary",
        )
        .with_fact_entries("Result propagation trace", facts)
        .with_note("each propagation frame comes from a `!` site on the cold Error path")
        .build(),
    )
}

fn source_summary(sources: &impl SourceProvider, span: Span) -> Option<String> {
    let source = sources.source(span.file)?;
    let start = usize::try_from(span.start.get()).ok()?;
    let end = usize::try_from(span.end.get()).ok()?;
    let text = source.text.get(start..end)?;
    let summary = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!summary.is_empty()).then_some(summary)
}

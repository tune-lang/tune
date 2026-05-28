use tune_db::TuneDb;
use tune_diagnostics::{Diagnostic, FactEntry, Span};
use tune_runtime::value::{RuntimeVariant, Value};

#[must_use]
pub fn diagnostic_from_vm_fault(fault: &tune_vm::VmFault) -> Diagnostic {
    let span = fault
        .location
        .and_then(|location| location.span)
        .unwrap_or_else(Span::synthetic);
    let mut facts = vec![FactEntry::new(format!("VM error: {:?}", fault.error))];
    if let Some(location) = fault.location {
        facts.push(FactEntry::new(format!(
            "bytecode function: {}",
            location.function
        )));
        if let Some(instruction) = location.instruction {
            facts.push(FactEntry::new(format!(
                "bytecode instruction: {instruction}"
            )));
        }
        if let Some(span) = location.span {
            facts.push(FactEntry::spanned(
                span,
                "source location from bytecode provenance",
            ));
        }
    }
    Diagnostic::error(
        tune_diagnostics::codes::RUNTIME_ERROR,
        "runtime execution failed",
        span,
        "execution failed here",
    )
    .with_fact_entries("runtime provenance", facts)
    .with_note("this diagnostic was produced from a VM fault")
    .build()
}

#[must_use]
pub fn diagnostics_from_runtime_value(value: &Value) -> Vec<Diagnostic> {
    diagnostic_from_result_error(value).into_iter().collect()
}

#[must_use]
pub fn diagnostics_from_runtime_value_with_sources(value: &Value, db: &TuneDb) -> Vec<Diagnostic> {
    diagnostic_from_result_error_with_sources(value, db)
        .into_iter()
        .collect()
}

#[must_use]
pub fn diagnostic_from_result_error(value: &Value) -> Option<Diagnostic> {
    result_error_diagnostic(value, |_| None)
}

#[must_use]
pub fn diagnostic_from_result_error_with_sources(value: &Value, db: &TuneDb) -> Option<Diagnostic> {
    result_error_diagnostic(value, |span| source_summary(db, span))
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

fn source_summary(db: &TuneDb, span: Span) -> Option<String> {
    let source = db.source(span.file)?;
    let start = usize::try_from(span.start.get()).ok()?;
    let end = usize::try_from(span.end.get()).ok()?;
    let text = source.text.get(start..end)?;
    let summary = text.split_whitespace().collect::<Vec<_>>().join(" ");
    (!summary.is_empty()).then_some(summary)
}

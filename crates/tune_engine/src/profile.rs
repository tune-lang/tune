mod quality;

use std::time::{Duration, Instant};

use quality::{bytecode_quality, ir_quality, optimizer_quality, plan_quality};
use tune_bytecode::Opcode;
use tune_db::FileId;
use tune_diagnostics::Diagnostic;

use crate::diagnostics::{diagnostic_from_bytecode_lower_error, diagnostic_from_ir_lower_error};
use crate::reachable::reachable_functions;
use crate::{CheckReport, EngineError, ProjectEntry, Tune};

#[derive(Debug, Clone)]
pub struct ProfileReport {
    pub file: FileId,
    pub diagnostics: Vec<Diagnostic>,
    pub timings: Vec<StageTiming>,
    pub plan: PlanQuality,
    pub ir: IrQuality,
    pub optimizer: OptimizerQuality,
    pub bytecode: BytecodeQuality,
    pub stop_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageTiming {
    pub stage: &'static str,
    pub duration: Duration,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PlanQuality {
    pub functions: usize,
    pub ops: usize,
    pub dynamic_bound_calls: usize,
    pub direct_calls: usize,
    pub unresolved_member_calls: usize,
    pub witness_calls: usize,
    pub host_calls: usize,
    pub struct_index_gets: usize,
    pub struct_index_sets: usize,
    pub finite_for_sequence: usize,
    pub finite_for_range: usize,
    pub finite_for_member_access: usize,
    pub finite_for_unknown: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IrQuality {
    pub functions: usize,
    pub ops: usize,
    pub shape_holes: usize,
    pub sequence_build_holes: usize,
    pub checked_sequence_ops: usize,
    pub unchecked_sequence_ops: usize,
    pub generic_finite_for_ops: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OptimizerQuality {
    pub changed_passes: usize,
    pub stack: usize,
    pub direct_drop: usize,
    pub non_atomic_rc: usize,
    pub cow: usize,
    pub shared_atomic: usize,
    pub host_retained: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BytecodeQuality {
    pub functions: usize,
    pub instructions: usize,
    pub registers: usize,
    pub locals: usize,
    pub constants: usize,
    pub direct_calls: usize,
    pub bound_calls: usize,
    pub callable_values: usize,
    pub checked_sequence_ops: usize,
    pub unchecked_sequence_ops: usize,
    pub field_accesses: usize,
    pub variant_field_accesses: usize,
    pub generic_finite_for_ops: usize,
    pub runtime_type_guard_pressure: usize,
    pub unsupported_reserved_opcodes: usize,
    pub opcodes: Vec<OpcodeCount>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpcodeCount {
    pub opcode: Opcode,
    pub count: usize,
}

impl Tune {
    pub fn profile_project(
        &mut self,
        manifest_path: impl AsRef<std::path::Path>,
    ) -> Result<ProfileReport, EngineError> {
        let entry = self.load_project_manifest(manifest_path)?;
        self.profile_project_entry(entry)
    }

    pub fn profile_project_entry(&self, entry: ProjectEntry) -> Result<ProfileReport, EngineError> {
        let mut timings = Vec::new();
        let (check, duration) = timed(|| self.check_project_entry(entry));
        timings.push(stage("project-check", duration));
        finish_profile(check?, timings, ProfileScope::Full)
    }

    pub fn profile_file_frontend(&self, file: FileId) -> Result<ProfileReport, EngineError> {
        self.profile_file_with_scope(file, ProfileScope::Frontend)
    }

    pub fn profile_file(&self, file: FileId) -> Result<ProfileReport, EngineError> {
        self.profile_file_with_scope(file, ProfileScope::Full)
    }

    fn profile_file_with_scope(
        &self,
        file: FileId,
        scope: ProfileScope,
    ) -> Result<ProfileReport, EngineError> {
        let source = self
            .db()
            .source(file)
            .ok_or(EngineError::FileNotFound(file))?;
        let mut timings = Vec::new();

        let (parsed, duration) = timed(|| tune_syntax::parse_with_file(file, &source.text));
        timings.push(stage("parse", duration));

        let (module, duration) = timed(|| tune_hir::lower::lower_module(&source.text, &parsed.cst));
        timings.push(stage("hir", duration));

        let (resolved, duration) = timed(|| tune_resolve::resolve_module(&module));
        timings.push(stage("resolve", duration));

        let (shape, duration) = timed(|| tune_shape::analyze_module(&module, &resolved));
        timings.push(stage("shape", duration));

        let diagnostics = parsed
            .diagnostics
            .iter()
            .chain(resolved.diagnostics.iter())
            .chain(
                shape
                    .iter()
                    .flat_map(|analysis| analysis.diagnostics.iter()),
            )
            .cloned()
            .collect::<Vec<_>>();

        finish_profile(
            CheckReport {
                file,
                diagnostics,
                module,
                resolved,
                shape,
            },
            timings,
            scope,
        )
    }
}

fn finish_profile(
    check: CheckReport,
    mut timings: Vec<StageTiming>,
    scope: ProfileScope,
) -> Result<ProfileReport, EngineError> {
    let (module_plan, duration) = timed(|| {
        tune_plan::lower_analyzed_module_to_plan(&check.module, &check.resolved, &check.shape)
    });
    timings.push(stage("plan", duration));

    let plan_quality = plan_quality(module_plan.entry.as_ref(), &module_plan.functions);

    let Some(entry_plan) = module_plan.entry.as_ref() else {
        return Ok(ProfileReport {
            file: check.file,
            diagnostics: check.diagnostics,
            timings,
            plan: plan_quality,
            ir: IrQuality::default(),
            optimizer: OptimizerQuality::default(),
            bytecode: BytecodeQuality::default(),
            stop_reason: Some("missing entry plan".to_owned()),
        });
    };

    if !check.diagnostics.is_empty() {
        return Ok(ProfileReport {
            file: check.file,
            diagnostics: check.diagnostics,
            timings,
            plan: plan_quality,
            ir: IrQuality::default(),
            optimizer: OptimizerQuality::default(),
            bytecode: BytecodeQuality::default(),
            stop_reason: Some("frontend diagnostics".to_owned()),
        });
    }

    let (reachable, duration) = timed(|| reachable_functions(&module_plan.functions, entry_plan));
    timings.push(stage("reachability", duration));

    let planned = core::iter::once(entry_plan)
        .chain(reachable.iter().map(|index| &module_plan.functions[*index]))
        .collect::<Vec<_>>();

    let (ir_result, duration) = timed(|| {
        let mut ir = Vec::new();
        for plan in &planned {
            let function = tune_ir::lower_plan_function(plan).map_err(|error| {
                Box::new(diagnostic_from_ir_lower_error(
                    &plan.name, plan.span, &error,
                ))
            })?;
            ir.push(function);
        }
        Ok::<_, Box<Diagnostic>>(ir)
    });
    timings.push(stage("ir", duration));
    let ir = match ir_result {
        Ok(ir) => ir,
        Err(diagnostic) => {
            return Ok(ProfileReport {
                file: check.file,
                diagnostics: vec![*diagnostic],
                timings,
                plan: plan_quality,
                ir: IrQuality::default(),
                optimizer: OptimizerQuality::default(),
                bytecode: BytecodeQuality::default(),
                stop_reason: Some("ir lowering failed".to_owned()),
            });
        }
    };
    let ir_quality = ir_quality(&ir);

    let (optimizer_quality, duration) = timed(|| optimizer_quality(&mut ir.clone()));
    timings.push(stage("opt", duration));

    if scope == ProfileScope::Frontend {
        return Ok(ProfileReport {
            file: check.file,
            diagnostics: check.diagnostics,
            timings,
            plan: plan_quality,
            ir: ir_quality,
            optimizer: optimizer_quality,
            bytecode: BytecodeQuality::default(),
            stop_reason: Some("frontend profiling skipped bytecode".to_owned()),
        });
    }

    let (bytecode_result, duration) = timed(|| tune_bytecode::lower_ir_functions(&ir));
    timings.push(stage("bytecode", duration));
    let mut bytecode = match bytecode_result {
        Ok(bytecode) => bytecode,
        Err(error) => {
            return Ok(ProfileReport {
                file: check.file,
                diagnostics: vec![diagnostic_from_bytecode_lower_error(&error)],
                timings,
                plan: plan_quality,
                ir: ir_quality,
                optimizer: optimizer_quality,
                bytecode: BytecodeQuality::default(),
                stop_reason: Some("bytecode lowering failed".to_owned()),
            });
        }
    };
    bytecode.entry_function = Some(0);
    let bytecode_quality = bytecode_quality(&bytecode);

    let (validation, duration) = timed(|| tune_bytecode::validate_artifact(&bytecode));
    timings.push(stage("validate", duration));
    let stop_reason = validation
        .err()
        .map(|error| format!("bytecode validation failed: {error:?}"));

    Ok(ProfileReport {
        file: check.file,
        diagnostics: check.diagnostics,
        timings,
        plan: plan_quality,
        ir: ir_quality,
        optimizer: optimizer_quality,
        bytecode: bytecode_quality,
        stop_reason,
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProfileScope {
    Frontend,
    Full,
}

fn timed<T>(f: impl FnOnce() -> T) -> (T, Duration) {
    let start = Instant::now();
    let value = f();
    (value, start.elapsed())
}

const fn stage(stage: &'static str, duration: Duration) -> StageTiming {
    StageTiming { stage, duration }
}

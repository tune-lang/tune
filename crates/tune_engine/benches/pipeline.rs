use std::hint::black_box;

use criterion::{BenchmarkId, Criterion, Throughput, criterion_group, criterion_main};

const BENCH_CASES: &[(&str, &str)] = &[
    ("arithmetic", include_str!("fixtures/arithmetic.tn")),
    ("struct_methods", include_str!("fixtures/struct_methods.tn")),
    ("finite_for", include_str!("fixtures/finite_for.tn")),
    (
        "sequence_access",
        include_str!("fixtures/sequence_access.tn"),
    ),
    (
        "generic_identity",
        include_str!("fixtures/generic_identity.tn"),
    ),
    ("tuple_expr", include_str!("fixtures/tuple_expr.tn")),
    ("spawn_join", include_str!("fixtures/spawn_join.tn")),
    (
        "structural_match",
        include_str!("fixtures/structural_match.tn"),
    ),
];

fn run_frontend_profile(source: &str) -> usize {
    let mut tune = tune_engine::Tune::new().with_std();
    let Some(file) = tune.add_source("case.tn", source) else {
        exit_benchmark("fixture should load");
    };
    let report = match tune.profile_source_frontend(file) {
        Ok(report) => report,
        Err(error) => exit_benchmark(&format!("frontend profile should succeed: {error:?}")),
    };
    report.ir.ops + report.optimizer.stack + report.plan.ops
}

fn run_full_profile(source: &str) -> usize {
    let mut tune = tune_engine::Tune::new().with_std();
    let Some(file) = tune.add_source("case.tn", source) else {
        exit_benchmark("fixture should load");
    };
    let report = match tune.profile_source(file) {
        Ok(report) => report,
        Err(error) => exit_benchmark(&format!("full pipeline profile should succeed: {error:?}")),
    };
    report.bytecode.instructions
}

fn compile_vm_artifact(source: &str) -> tune_bytecode::artifact::BytecodeArtifact {
    let mut tune = tune_engine::Tune::new().with_std();
    let Some(file) = tune.add_source("case.tn", source) else {
        exit_benchmark("fixture should load");
    };
    let executable = match tune.executable_source(file) {
        Ok(executable) => executable,
        Err(error) => exit_benchmark(&format!("executable should build: {error:?}")),
    };
    executable.bytecode
}

fn run_vm_execution(bytecode: tune_bytecode::artifact::BytecodeArtifact) -> tune_runtime::Value {
    let mut vm = tune_vm::Vm::new(bytecode);
    vm.run_entry()
        .unwrap_or_else(|error| exit_benchmark(&format!("vm execution should succeed: {error:?}")))
}

fn exit_benchmark(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}

fn bench_pipeline_frontend(c: &mut Criterion) {
    let mut group = c.benchmark_group("tune_pipeline_frontend_profile");
    for (name, source) in BENCH_CASES {
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("frontend", name),
            source,
            |bench, source| {
                bench.iter(|| {
                    black_box(run_frontend_profile(source));
                });
            },
        );
    }
    group.finish();
}

fn bench_pipeline_full(c: &mut Criterion) {
    let mut group = c.benchmark_group("tune_pipeline_full_profile");
    for (name, source) in BENCH_CASES {
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_with_input(BenchmarkId::new("full", name), source, |bench, source| {
            bench.iter(|| {
                black_box(run_full_profile(source));
            });
        });
    }
    group.finish();
}

fn bench_vm_execution(c: &mut Criterion) {
    let mut group = c.benchmark_group("tune_vm_execution");
    for (name, source) in BENCH_CASES {
        let bytecode = compile_vm_artifact(source);
        group.throughput(Throughput::Bytes(source.len() as u64));
        group.bench_with_input(
            BenchmarkId::new("execute", name),
            &bytecode,
            |bench, bytecode| {
                bench.iter(|| {
                    black_box(run_vm_execution(bytecode.clone()));
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    pipeline_benches,
    bench_pipeline_frontend,
    bench_pipeline_full,
    bench_vm_execution
);
criterion_main!(pipeline_benches);

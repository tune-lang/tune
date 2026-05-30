# Tune

Tune is a typed programming language for scripts, automation, and embeddable
application logic.

This is the Tune repository. It contains the language implementation, runtime,
standard library, host API, package/project tooling, LSP work, examples, and the
Dyno command-line tool. Dyno is Tune's bundled CLI and default platform embedder;
it is one consumer of the same public Tune engine API that other hosts can embed.

Tune is designed for code that should stay easy to read while still giving tools
and runtimes enough information to understand what the program means. The guiding
rule is:

```text
runtime values may be unknown
runtime meaning may not be unknown
```

Tune is pre-1.0. The language is usable for small programs and compiler/runtime
development, but the platform is still changing.

## Hello Tune

```tn
let score: Int = 37
let passed: Bool = score > 30

let status: String = if passed {
  "pass"
} else {
  "retry"
}

let report: String = "{status}:{score}"
```

Run it with Dyno:

```sh
cargo run -p dyno_cli -- run examples/language/01_values_and_flow.tn
```

Check without running:

```sh
cargo run -p dyno_cli -- check examples/language/01_values_and_flow.tn
```

Create a project:

```sh
cargo run -p dyno_cli -- new hello-tune
cd hello-tune
cargo run --manifest-path ../Cargo.toml -p dyno_cli -- run
```

## Learn By Example

The language examples under [examples/language](examples/language) are small
programs that teach one concept at a time. They are checked by the test suite so
the examples stay aligned with the implementation.

```sh
cargo test -p dyno_cli --test language_examples
```

Start with [examples/README.md](examples/README.md), then use
[docs/language-tour.md](docs/language-tour.md) for short explanations beside the
example files.

## Language Surface

Current implemented areas include:

- typed bindings, expressions, blocks, and control flow
- structs, fields, methods, and struct literals
- enums, variants, tuples, pattern matching, and structural match checks
- generic callables and generic structs
- sequences, ranges, finite `for`, string indexing, and interpolation
- optional values with narrowing through `none`
- `Result<T, E>` values and postfix `!` propagation
- `Task<T>`, `spawn`, and `join`
- host modules, authorities, task-safety metadata, and host resources

## Workspace

The workspace is split into focused crates:

- `tune_syntax`, `tune_ast`, `tune_hir`, `tune_db`
- `tune_resolve`, `tune_shape`, `tune_plan`
- `tune_ir`, `tune_opt`, `tune_bytecode`, `tune_vm`
- `tune_runtime`, `tune_host`, `tune_std`, `tune_meta`
- `tune_diagnostics`, `tune_lsp`
- `dyno_project`, `dyno_pkg`, `dyno_cli`

The public engine facade is `tune_engine::Tune`. Dyno CLI, tests, and embedders
should go through the facade instead of stitching compiler internals together.

## Standard Library And Hosts

Tune ships a default host/std profile through `tune_std`. Current modules include
`io`, `math`, `bits`, `parse`, `text`, `path`, `env`, `fs`, `hash`, `json`,
`process`, `random`, and `time`.

Outside-world operations are explicit host calls with known shapes, authority
requirements, and task-safety metadata. Core behavior such as `Result`, `Never`,
panic flow, task safety, and resource authority is represented as compiler/runtime
facts rather than hard-coded standard-library names.

## Development

Rust is pinned by [rust-toolchain.toml](rust-toolchain.toml).

Core checks:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

Public examples:

```sh
cargo test -p dyno_cli --test language_examples
cargo test -p dyno_cli --test std_examples
```

Performance and IR-quality checks:

```sh
cargo bench -p tune_engine --bench pipeline
cargo run --package tune_engine --bin profile_trace -- <path/to/source.tn>...
cargo run --package tune_engine --bin profile_trace -- --full --csv <path/to/source.tn>...
cargo run --package tune_engine --bin quality_check -- <path/to/source.tn>...
```

Benchmark fixtures live in `crates/tune_engine/benches/fixtures`.

## Implementation Notes

Tune preserves language meaning through this pipeline:

```text
syntax -> AST/HIR -> resolve -> shape/state -> semantic plan -> IR -> optimizer -> bytecode -> VM
```

That pipeline is an implementation detail, but it matters for the project: errors,
editor features, bytecode, optimization, host calls, and future native backends
should consume the same facts rather than rediscovering Tune semantics.

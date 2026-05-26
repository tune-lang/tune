# Dyno

Dyno is the Rust workspace for Tune.

The core compiler pipeline is:

```text
syntax -> HIR -> resolve -> shape/state -> semantic plan -> IR -> optimizer -> bytecode -> VM
```

The platform/runtime split lives beside that pipeline. Platform, Dyno, stdlib, host APIs, and LSP consume the same compiler facts instead of duplicating analysis.

## Design Rule

```text
runtime values may be unknown
runtime meaning may not be unknown
```

## Workspace

The workspace is split into focused crates under `crates/`:

- `tune_syntax`, `tune_ast`, `tune_hir`, `tune_db`
- `tune_resolve`, `tune_shape`, `tune_plan`
- `tune_ir`, `tune_opt`, `tune_bytecode`, `tune_vm`
- `tune_runtime`, `tune_host`, `tune_std`, `tune_meta`
- `tune_diagnostics`, `tune_lsp`
- `dyno_project`, `dyno_pkg`, `dyno_cli`

Each crate keeps integration tests in a sibling `tests/` directory next to `src/`.

## Checks

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

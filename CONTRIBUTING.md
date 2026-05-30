# Contributing

Tune is pre-1.0, but the repository is open for focused issues, examples,
tooling fixes, documentation, and small compiler/runtime improvements.

## Setup

```sh
rustup show
cargo check --workspace --all-targets --all-features
```

The Rust toolchain is pinned by `rust-toolchain.toml`.

## Useful Checks

For normal Rust changes:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

For examples, LSP, formatter, and editor packaging:

```sh
sh scripts/check_tooling.sh
```

## Expectations

- Keep changes small and focused.
- Route user-visible behavior through the normal compiler pipeline.
- Do not add editor, CLI, or stdlib shortcuts that rediscover compiler facts.
- Prefer adding or updating examples when changing language behavior.
- Keep generated/package artifacts out of commits.

The core design rule is:

```text
runtime values may be unknown
runtime meaning may not be unknown
```

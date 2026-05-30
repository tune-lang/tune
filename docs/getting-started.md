# Getting Started

Tune is currently distributed from source. The fastest way to try it is to run
Dyno from this checkout.

## Install Prerequisites

Install Rust. The repository pins the exact toolchain in
[`rust-toolchain.toml`](../rust-toolchain.toml), so `cargo` will use the right
compiler once Rustup is installed.

Node.js is only needed for VS Code extension packaging and editor tooling checks.

## Run A Program

Run the first language example:

```sh
cargo run -p dyno_cli -- run examples/language/01_values_and_flow.tn
```

Expected output:

```text
pass:37:false
```

Check the same file without running it:

```sh
cargo run -p dyno_cli -- check examples/language/01_values_and_flow.tn
```

Machine-readable diagnostics are available for editor and CI integrations:

```sh
cargo run -p dyno_cli -- check --json examples/language/01_values_and_flow.tn
```

## Create A Project

```sh
cargo run -p dyno_cli -- new hello-tune
cd hello-tune
cargo run --manifest-path ../Cargo.toml -p dyno_cli -- run
```

The generated project writes:

```text
hello from Dyno
```

## Install Dyno Locally

From the repository root:

```sh
cargo install --path crates/dyno_cli
```

After that, use `dyno` directly:

```sh
dyno run examples/language/03_structs_and_methods.tn
dyno check examples/language/06_result_propagation.tn
dyno fmt --check
```

## Learn The Language

Use these in order:

- [examples/README.md](../examples/README.md)
- [language-tour.md](language-tour.md)

The language examples are executable and covered by tests:

```sh
cargo test -p dyno_cli --test language_examples
```

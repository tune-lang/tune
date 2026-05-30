# Getting Started

This guide assumes you have never used Tune before.

Tune programs live in `.tn` files. Dyno is the command-line tool that checks,
runs, formats, and serves editor features for Tune code.

Tune is currently distributed from source. Build Dyno once, then use the `dyno`
command directly.

## Install Prerequisites

Install Rust. The repository pins the exact toolchain in
[`rust-toolchain.toml`](../rust-toolchain.toml), so `cargo` will use the right
compiler once Rustup is installed.

Node.js is only needed for VS Code extension packaging and editor tooling checks.

## Install Dyno

From the repository root:

```sh
cargo install --path crates/dyno_cli
```

That installs `dyno` into Cargo's bin directory, usually `~/.cargo/bin`.

If `dyno --help` is not found, add Cargo's bin directory to your `PATH`.

macOS/Linux, current shell:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

macOS/Linux, future shells:

```sh
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
```

Windows PowerShell, current shell:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
```

Windows PowerShell, future shells:

```powershell
[Environment]::SetEnvironmentVariable("Path", "$env:USERPROFILE\.cargo\bin;$env:Path", "User")
```

Check the command:

```sh
dyno --help
```

## Run A Program

Run the first language example:

```sh
dyno run examples/language/01_values_and_flow.tn
```

Expected output:

```text
pass:37:false
```

Open the file and read it:

```tn
let score = 37
let passed = score > 30
let retry = not passed or score < 10

let status = if passed => "pass" else "retry"

let report = "{status}:{score}:{retry}"
print(report)
```

What this shows:

- `let` creates a binding.
- Tune infers many shapes, but the compiler still knows meaning.
- `if` produces a value.
- Strings can interpolate `{name}`.
- `print(...)` is how a program writes visible output.

Check the same file without running it:

```sh
dyno check examples/language/01_values_and_flow.tn
```

Machine-readable diagnostics are available for editor and CI integrations:

```sh
dyno check --json examples/language/01_values_and_flow.tn
```

## Create A Project

```sh
dyno new hello-tune
cd hello-tune
dyno run
```

The generated project writes:

```text
hello from Dyno
```

Use `dyno` directly for day-to-day work:

```sh
dyno run examples/language/03_structs_and_methods.tn
dyno check examples/language/06_result_propagation.tn
dyno fmt --check
```

## Learn The Language

Use these in order:

- [why-tune.md](why-tune.md)
- [tutorial.md](tutorial.md)
- [from-other-languages.md](from-other-languages.md)
- [examples/README.md](../examples/README.md)
- [language-tour.md](language-tour.md)

The language examples are executable and covered by tests:

```sh
cargo test -p dyno_cli --test language_examples
```

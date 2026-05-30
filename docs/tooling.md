# Tooling

Tune tooling is built around one rule: tools consume compiler facts from Dyno and
`tune_engine`; they should not reimplement parsing, resolution, or inference.

## Dyno CLI

Dyno is the command-line frontend:

```sh
dyno help
dyno new hello-tune
dyno check src/main.tn
dyno run src/main.tn
dyno fmt --check
dyno explain T0301
```

When no file is passed, project commands use the current directory and
`dyno.toml`.

## Formatter

The formatter is intentionally conservative for multiline files while the CST
formatter matures. Single-line cleanup is useful today:

```sh
dyno fmt src/main.tn
dyno fmt --check src/main.tn
```

Project-wide formatting works from a project root:

```sh
dyno fmt
dyno fmt --check
```

## Diagnostics

Human-readable diagnostics are printed by default:

```sh
dyno check src/main.tn
```

Machine-readable diagnostics:

```sh
dyno check --json src/main.tn
```

Diagnostic explanations:

```sh
dyno explain
dyno explain T0301
```

The public diagnostic pages live in [diagnostics](diagnostics).

## Language Server

Start the LSP server:

```sh
dyno lsp
```

The LSP exposes diagnostics, hover, completion, signature help, go-to-definition,
references, rename, document symbols, document links, semantic tokens, inlay
hints, code actions, and formatting.

## VS Code

The VS Code extension lives in [`../editors/vscode`](../editors/vscode).

```sh
cd editors/vscode
npm install
npm run check
npm run package
code --install-extension tune-vscode-0.1.0.vsix
```

Set `tune.dynoPath` if `dyno` is not on `PATH`.

## Zed

The Zed extension lives in [`../editors/zed`](../editors/zed). For local testing,
use Zed's `zed: install dev extension` command and select that directory.

```sh
cargo check --manifest-path editors/zed/Cargo.toml
```

Zed publishing also needs the Tree-sitter grammar split to
`https://github.com/tune-lang/tree-sitter-tune`.

## Tooling Checks

Run the combined local check:

```sh
sh scripts/check_tooling.sh
```

This covers LSP tests, formatter tests, public examples, the Zed extension
crate, the Tree-sitter grammar seed, and VS Code extension packaging.

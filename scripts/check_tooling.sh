#!/usr/bin/env sh
set -eu

cargo test -p tune_lsp --all-targets
cargo test -p tune_fmt --all-targets
cargo test -p dyno_cli --test language_examples
cargo check --manifest-path editors/zed/Cargo.toml
node --check editors/tree-sitter-tune/grammar.js

(
  cd editors/vscode
  npm run check
  npm run package
)

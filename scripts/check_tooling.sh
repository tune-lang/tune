#!/usr/bin/env sh
set -eu

cargo test -p tune_lsp --all-targets
cargo test -p tune_fmt --all-targets
cargo test -p dyno_cli --test language_examples
cargo check --manifest-path editors/zed/Cargo.toml

(
  cd editors/tree-sitter-tune
  npm run check
  npm_config_cache="${TMPDIR:-/tmp}/dyno-npm-cache" npm run pack:dry
)

(
  cd editors/vscode
  npm run check
  npm_config_cache="${TMPDIR:-/tmp}/dyno-npm-cache" npm run package
)

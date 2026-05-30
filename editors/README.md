# Tune Editor Support

Editor integrations share one rule: they consume `dyno lsp` and compiler facts
from the Tune engine. They should not implement their own parser, resolver, or
shape inference for semantic features.

## VS Code

The VS Code extension lives in [vscode](vscode). It registers `.tn` files,
starts `dyno lsp`, and exposes the Tune formatter, diagnostics, hover,
completion, semantic tokens, inlay hints, rename, references, and code actions
through the LSP server.

## Zed

The Zed extension scaffold lives in [zed](zed). It starts `dyno lsp` through
Zed's extension API and includes Tune language metadata/query files. Zed also
requires a Tree-sitter grammar, so the seed grammar lives in
[tree-sitter-tune](tree-sitter-tune) until it is split into its own public
repository.

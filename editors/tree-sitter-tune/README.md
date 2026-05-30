# tree-sitter-tune

This is the seed Tree-sitter grammar for Tune editor integrations.

It is intentionally kept under `editors/` until the grammar is ready to split to
`tune-lang/tree-sitter-tune`, which the Zed extension manifest references.
Until then, Zed users can use this directory as the local grammar repository for
dev-extension testing.

## Checks

```sh
npm run check
npm run pack:dry
```

This directory is meant to split cleanly into
`https://github.com/tune-lang/tree-sitter-tune` before the Zed extension is
published.

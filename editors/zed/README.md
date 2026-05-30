# Tune for Zed

This is the Zed extension for Tune. It registers `.tn` files as Tune and starts
the language server through:

```sh
dyno lsp
```

Install it locally with Zed's `zed: install dev extension` command and select
this `editors/zed` directory. `dyno` must be available on the worktree `PATH`.

Check the extension crate with:

```sh
cargo check --manifest-path editors/zed/Cargo.toml
```

## Grammar

Zed language extensions require a Tree-sitter grammar. The extension manifest
points at the public grammar repository location:

```toml
[grammars.tune]
repository = "https://github.com/tune-lang/tree-sitter-tune"
rev = "main"
```

The split-ready grammar seed currently lives at
[`../tree-sitter-tune`](../tree-sitter-tune). Move that directory into the
grammar repository before publishing the extension.

# Tune Diagnostics

Each Tune diagnostic has a stable code, a short title, and a structured payload shared by CLI, LSP, JSON output, and docs.

Use `dyno explain <code>` for the local explanation printed by the installed toolchain. These pages exist so editor diagnostics can link to stable public documentation.

For machine-readable output, use:

```sh
dyno check --json path/to/file.tn
```

The JSON form is intended for editors, CI, and agent tooling. Do not scrape the
human CLI renderer when the JSON form or `tune_engine` facts are available.

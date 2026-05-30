# Tune for VS Code

This extension registers `.tn` files and starts the Tune language server through
the Dyno CLI:

```sh
dyno lsp
```

## Local Development

Install the extension dependencies from this directory:

```sh
npm install
```

Then open this directory in VS Code and run the extension host. The extension
expects `dyno` to be on `PATH`; set `tune.dynoPath` if you use a local binary.

```json
{
  "tune.dynoPath": "/path/to/dyno"
}
```

## Commands

- `Tune: Restart Language Server`
- `Tune: Check Current File`
- `Tune: Check Formatting`

`Check Current File` runs `dyno check <file>`. `Check Formatting` runs
`dyno fmt --check <file>` and does not modify the source.

## Packaging

```sh
npm run package
```

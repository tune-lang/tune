# Why Tune?

Tune exists because a lot of useful code lives between two uncomfortable choices:

- dynamic scripts that are easy to start but hard to trust later
- systems languages that are powerful but heavy for small embedded workflows

Tune tries to keep the first-day experience small while making the compiler
understand the program from the beginning.

## The Main Idea

Tune separates runtime value from runtime meaning.

The value of `user_input` might not be known until the program runs. But the
compiler should know whether it is a `String`, whether it can be indexed, whether
it has a `trim()` member, whether it can be sent to a spawned task, and what a
failed operation would return.

That is the project rule:

```text
runtime values may be unknown
runtime meaning may not be unknown
```

This matters because tools and runtimes can use the same facts:

- the CLI can produce useful diagnostics
- the LSP can provide hover, completion, rename, and references
- the compiler can plan calls and bytecode without dynamic lookup
- host APIs can expose typed resources and authorities
- future backends can consume the same semantic plan

## What Tune Feels Like

Tune is meant to read plainly:

```tn
import "text"
import "parse"

let parse_score(raw: String): Result<Int, String> = {
  let cleaned: String = text.trim(raw)
  let score: Int = parse.int(cleaned)!
  Ok(score)
}

let score: Result<Int, String> = parse_score(" 42 ")
```

The `!` is not exception magic. It is result propagation: unwrap `Ok(value)`, or
return/propagate `Error(error)` from the current callable.

Structs and methods are similarly direct:

```tn
struct Counter {
  value: Int

  bump(amount: Int): Int = {
    self.value = self.value + amount
    self.value
  }
}
```

The important part is that `self.value` is not a string lookup. It is known
meaning in the compiler pipeline.

## What Tune Is Not

Tune is not trying to be a large enterprise platform yet. It is pre-1.0 and the
ecosystem is early.

Tune is also not trying to win by hiding all complexity in a black-box backend.
The frontend is expected to preserve enough meaning that bytecode, editor tools,
host integrations, and future native codegen all consume the same semantic facts.

## Current State

Today, Tune has:

- a Rust implementation
- the `dyno` CLI
- executable language examples
- a standard library/host module surface
- diagnostics with public code pages
- an LSP used by VS Code and Zed extensions
- typed bytecode and VM execution

The next goal is making the language pleasant enough for early community use:
better docs, better examples, better editor polish, and a stdlib that covers
real scripting workflows.

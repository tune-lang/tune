# Tune Language Tour

This tour is made of `.tn` files in `examples/language`. Each file is small on
purpose: it introduces one idea and can be checked or run with Dyno.

```sh
cargo run -p dyno_cli -- check examples/language/01_values_and_flow.tn
cargo run -p dyno_cli -- run examples/language/01_values_and_flow.tn
```

## 1. Values And Flow

[01_values_and_flow.tn](../examples/language/01_values_and_flow.tn) shows typed
bindings, `if` expressions, string interpolation, and boolean operators.

Tune bindings use `let`; annotations are written after a colon:

```tn
let score: Int = 37
let passed: Bool = score > 30
```

`if` is an expression, so both branches contribute a value:

```tn
let status: String = if passed { "pass" } else { "retry" }
```

## 2. Functions And Blocks

[02_functions_and_blocks.tn](../examples/language/02_functions_and_blocks.tn)
uses callables and block bodies. The final expression in a block is the block's
value unless an explicit `return` exits earlier.

## 3. Structs And Methods

[03_structs_and_methods.tn](../examples/language/03_structs_and_methods.tn)
defines a struct, creates a struct literal, and mutates receiver state from a
method through `self`.

## 4. Sequences And Finite For

[04_sequences_and_for.tn](../examples/language/04_sequences_and_for.tn) uses a
sequence literal, indexed access, and finite `for`. Tune's v1 finite iteration
contract is intentionally typed: sequences expose `len(): Size` plus indexed
access.

## 5. Enums And Match

[05_enums_and_match.tn](../examples/language/05_enums_and_match.tn) defines an
enum and destructures variants with `match`.

## 6. Result And Propagation

[06_result_propagation.tn](../examples/language/06_result_propagation.tn) shows
`Result<T, E>` and postfix `!`. `!` unwraps `Ok(value)` and propagates
`Error(err)` from the current callable.

`Result`, `Ok`, and `Error` are standard core meaning, not special parser-only
syntax.

## 7. Generics

[07_generics.tn](../examples/language/07_generics.tn) shows a generic function
and a generic struct. Type arguments are inferred from how values are used.

## 8. Std Imports

[08_std_imports.tn](../examples/language/08_std_imports.tn) imports selected
standard host functions. Today host-module imports are selected by member:

```tn
import "parse".int
import "text".trim
```

The selected-import form is the stable form used by the current examples.

## 9. Tasks

[09_tasks.tn](../examples/language/09_tasks.tn) shows `spawn` and `join`. Host
calls inside spawned work must be marked task-safe.

## Current Limits

Examples intentionally avoid unsupported surfaces:

- `Map` and `Set` are known shapes, but runtime storage and materialization are
  still future work.
- JSON typed encode/decode needs compiler metadata and host value modeling.
- File handles need concrete host resource object storage before `fs.open` can
  be a real example.
- Host module namespace imports need importer support before docs should teach
  them as syntax.

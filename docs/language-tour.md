# Tune Language Tour

This tour follows the executable examples in
[examples/language](../examples/language). Each `.tn` file introduces one idea and
is kept small enough to read in one sitting.

```sh
cargo run -p dyno_cli -- check examples/language/01_values_and_flow.tn
cargo run -p dyno_cli -- run examples/language/01_values_and_flow.tn
```

The examples are part of the repository test suite:

```sh
cargo test -p dyno_cli --test language_examples
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

```tn
let clamp(value: Int, low: Int, high: Int): Int = {
  if value < low {
    low
  } elif value > high {
    high
  } else {
    value
  }
}
```

## 3. Structs And Methods

[03_structs_and_methods.tn](../examples/language/03_structs_and_methods.tn)
defines a struct, creates a struct literal, and mutates receiver state from a
method through `self`.

```tn
struct Counter {
  value: Int

  bump(amount: Int): Int = {
    self.value = self.value + amount
    self.value
  }
}
```

## 4. Sequences And Finite For

[04_sequences_and_for.tn](../examples/language/04_sequences_and_for.tn) uses a
sequence literal, indexed access, and finite `for`. Tune's v1 finite iteration
contract is intentionally typed: sequences expose `len(): Size` plus indexed
access.

```tn
for item in items {
  total = total + item
}
```

## 5. Enums And Match

[05_enums_and_match.tn](../examples/language/05_enums_and_match.tn) defines an
enum and destructures variants with `match`.

```tn
let cost(command: Command): Int = match command {
  Move(x, y) => x + y
  Wait(seconds) => seconds
}
```

## 6. Result And Propagation

[06_result_propagation.tn](../examples/language/06_result_propagation.tn) shows
`Result<T, E>` and postfix `!`. `!` unwraps `Ok(value)` and propagates
`Error(err)` from the current callable.

`Result`, `Ok`, and `Error` are standard core meaning, not special parser-only
syntax.

```tn
let value: Int = choose(okay)!
Ok(value + 1)
```

## 7. Generics

[07_generics.tn](../examples/language/07_generics.tn) shows a generic function
and a generic struct. Type arguments are inferred from how values are used.

```tn
struct Box<T> {
  value: T
}

let id<T>(value: T): T = value
```

## 8. Std Imports

[08_std_imports.tn](../examples/language/08_std_imports.tn) imports standard
modules and calls their members through module namespaces.

```tn
import "parse"
import "text"

let cleaned: String = text.trim(raw)
let value: Int = parse.int(cleaned)!
```

Host calls are ordinary typed calls with declared shapes, authority requirements,
and task-safety metadata.

## 9. Tasks

[09_tasks.tn](../examples/language/09_tasks.tn) shows `spawn` and `join`. Host
calls inside spawned work must be marked task-safe.

```tn
let task: Task<Int> = spawn compute(41)
let answer: Int = task.join()
```

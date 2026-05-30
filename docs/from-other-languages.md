# Tune From Other Languages

This page gives quick landmarks if you are coming from Python, JavaScript,
TypeScript, Go, or Rust.

Tune is still early, but the language direction is already clear: write scripts
plainly, while keeping program meaning known to the compiler.

## If You Come From Python

Python:

```py
score = int(raw.strip())
print(f"score={score}")
```

Tune:

```tn
import "text"
import "parse"

let cleaned: String = text.trim(raw)
let score: Int = parse.int(cleaned)!
let shown: () = print("score={score}")
```

The main difference is that failure is visible in the shape. `parse.int(...)`
returns `Result<Int, String>`, and `!` propagates the error if parsing fails.

## If You Come From JavaScript Or TypeScript

JavaScript:

```js
const status = score > 30 ? "pass" : "retry";
```

Tune:

```tn
let status: String = if score > 30 {
  "pass"
} else {
  "retry"
}
```

Tune uses expression-oriented flow. `if`, blocks, and `match` can produce values.

TypeScript types are mostly checked at compile time and erased. Tune shapes are
also compile-time meaning, but they are used throughout the compiler pipeline:
diagnostics, editor facts, semantic planning, bytecode, and future backends.

## If You Come From Go

Go often returns `(value, error)`.

Tune uses `Result<T, E>`:

```tn
let read_number(raw: String): Result<Int, String> = {
  let value: Int = parse.int(raw)!
  Ok(value)
}
```

The `!` operator is the common “if error, return it” path.

Tune’s long-term performance goal is to make the frontend prove enough meaning
that backends have less guessing to do.

## If You Come From Rust

Rust:

```rust
let value: i64 = raw.trim().parse()?;
```

Tune:

```tn
let cleaned: String = text.trim(raw)
let value: Int = parse.int(cleaned)!
```

Tune borrows ideas from expression-oriented languages and `Result` propagation,
but it is not trying to expose Rust’s ownership model directly to script authors.
The compiler should plan ownership, task safety, and resource meaning from the
program’s shapes and state facts.

## Common Syntax Landmarks

Bindings:

```tn
let name: String = "Tune"
```

Functions:

```tn
let add(left: Int, right: Int): Int = left + right
```

Blocks return their final expression:

```tn
let total: Int = {
  let first: Int = 1
  let second: Int = 2
  first + second
}
```

Structs:

```tn
struct Counter {
  value: Int

  bump(): Int = {
    self.value = self.value + 1
    self.value
  }
}
```

Enums and match:

```tn
enum Command {
  Move(Int, Int)
  Wait(Int)
}

let cost(command: Command): Int = match command {
  Move(x, y) => x + y
  Wait(seconds) => seconds
}
```

Imports:

```tn
import "text"

let cleaned: String = text.trim(" Tune ")
```

Output:

```tn
let shown: () = print("hello")
```

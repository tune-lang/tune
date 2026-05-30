# Tutorial: A Small Tune Script

This tutorial builds a tiny script that cleans up text, parses a number, handles
failure with `Result`, and prints a final message.

It is intentionally small. The goal is to learn how Tune code reads.

## Step 1: A Value And Some Output

Create a file named `score.tn`:

```tn
let score: Int = 37
print("score={score}")
```

Run it:

```sh
dyno run score.tn
```

Output:

```text
score=37
```

`let` creates a binding. `score: Int` says the binding has integer meaning.

`print(...)` writes visible program output. `dyno run` does not print the last
value automatically.

## Step 2: Use An Expression

Tune `if` produces a value:

```tn
let score: Int = 37

let status: String = if score > 30 {
  "pass"
} else {
  "retry"
}

print("{status}:{score}")
```

This is different from languages where `if` is only a statement. Here, the
selected branch becomes the value assigned to `status`.

## Step 3: Make A Function

Tune callable declarations start with `let` too:

```tn
let label(score: Int): String = if score > 30 {
  "pass"
} else {
  "retry"
}

print("{label(37)}")
```

The shape after `:` is the return shape. The body can be a single expression or
a block.

## Step 4: Import Standard Modules

Standard modules are imported by name:

```tn
import "text"
import "parse"

let raw: String = " 37 "
let cleaned: String = text.trim(raw)
let score: Result<Int, String> = parse.int(cleaned)

print("parsed")
```

`text.trim(raw)` is a normal typed call. `parse.int(cleaned)` returns
`Result<Int, String>` because parsing can fail.

## Step 5: Handle Failure With `!`

Postfix `!` unwraps an `Ok(value)` or propagates an `Error(error)` from the
current callable:

```tn
import "text"
import "parse"

let parse_score(raw: String): Result<Int, String> = {
  let cleaned: String = text.trim(raw)
  let score: Int = parse.int(cleaned)!
  Ok(score)
}

let score: Result<Int, String> = parse_score(" 37 ")
print("score parsed")
```

`!` is not exceptions. It is shorthand for the normal `Result` flow you could
write by hand.

## Step 6: Put State In A Struct

Structs group fields and methods:

```tn
struct Counter {
  value: Int

  bump(amount: Int): Int = {
    self.value = self.value + amount
    self.value
  }
}

let counter: Counter = Counter { value = 10 }
let value: Int = counter.bump(5)
print("counter={value}")
```

`self.value` is known by the compiler as a field access on `Counter`, not a
runtime string lookup.

## Step 7: Run A Checked Example

The repository examples are the next step:

```sh
dyno run examples/language/01_values_and_flow.tn
dyno run examples/language/03_structs_and_methods.tn
dyno check examples/language/06_result_propagation.tn
```

Use [language-tour.md](language-tour.md) when you want a concept-by-concept map
of those examples.

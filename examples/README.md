# Tune Examples

These examples are small Tune programs meant to be read and run. If you are new
to Tune, read the language examples in order. They are intentionally explicit:
annotations are included even where inference could solve them, because the goal
is to teach the language.

The examples are part of the test suite, so they should stay aligned with the
implementation.

Run a language example:

```sh
cargo run -p dyno_cli -- run examples/language/03_structs_and_methods.tn
```

Check without running:

```sh
cargo run -p dyno_cli -- check examples/language/06_result_propagation.tn
```

Run the example regression tests:

```sh
cargo test -p dyno_cli --test language_examples
cargo test -p dyno_cli --test std_examples
```

## Language Examples

Start here if you are learning Tune.

Suggested path:

1. Run an example.
2. Read the source.
3. Change one line.
4. Run `dyno check`.
5. Run it again.

- [01_values_and_flow.tn](language/01_values_and_flow.tn): typed bindings, `if`
  expressions, interpolation, and boolean operators. Prints a compact status
  report.
- [02_functions_and_blocks.tn](language/02_functions_and_blocks.tn): callables
  and block result values. Prints the clamped and adjusted values.
- [03_structs_and_methods.tn](language/03_structs_and_methods.tn): structs,
  fields, methods, and receiver state. Prints a method-generated label.
- [04_sequences_and_for.tn](language/04_sequences_and_for.tn): sequence literals,
  indexing, and finite `for`. Prints the first item and checksum.
- [05_enums_and_match.tn](language/05_enums_and_match.tn): enum payloads and
  `match`. Prints the combined command cost.
- [06_result_propagation.tn](language/06_result_propagation.tn):
  `Result<T, E>` and postfix `!` propagation.
- [07_generics.tn](language/07_generics.tn): generic callables and generic
  structs. Prints values that flowed through generic functions.
- [08_std_imports.tn](language/08_std_imports.tn): standard module imports and
  namespace calls.
- [09_tasks.tn](language/09_tasks.tn): `spawn`, `Task<T>`, and `join`.

## Standard Library Examples

Std examples are mostly checked rather than run by default because some modules
touch the host environment, filesystem, process table, time, or terminal I/O.

- [bits.tn](std/bits.tn): integer and size bit helpers.
- [encoding.tn](std/encoding.tn): hex text and byte encoding.
- [env.tn](std/env.tn): process arguments, environment, and runtime paths.
- [fs.tn](std/fs.tn): filesystem text, metadata, and path helpers.
- [hash.tn](std/hash.tn): stable text and byte hashing.
- [io.tn](std/io.tn): standard input/output host calls.
- [json.tn](std/json.tn): JSON validation and formatting helpers.
- [math.tn](std/math.tn): floating-point numeric helpers.
- [path.tn](std/path.tn): path component and extension helpers.
- [parse.tn](std/parse.tn): typed parsing into primitive values.
- [process.tn](std/process.tn): process execution result helpers.
- [random.tn](std/random.tn): deterministic pseudo-random values.
- [text.tn](std/text.tn): text splitting, joining, and slicing.
- [time.tn](std/time.tn): clock reads and millisecond sleep.

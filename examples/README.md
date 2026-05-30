# Tune Examples

These examples are Tune programs. Each file focuses on one concept and is checked
by the test suite as public documentation.

Run one example:

```sh
cargo run -p dyno_cli -- run examples/language/03_structs_and_methods.tn
```

Check one example:

```sh
cargo run -p dyno_cli -- check examples/language/06_result_propagation.tn
```

Run the example regression test:

```sh
cargo test -p dyno_cli --test language_examples
cargo test -p dyno_cli --test std_examples
```

## Language Examples

- [01_values_and_flow.tn](language/01_values_and_flow.tn): typed bindings, `if`
  expressions, interpolation, and boolean operators.
- [02_functions_and_blocks.tn](language/02_functions_and_blocks.tn): callables
  and block result values.
- [03_structs_and_methods.tn](language/03_structs_and_methods.tn): structs,
  fields, methods, and receiver state.
- [04_sequences_and_for.tn](language/04_sequences_and_for.tn): sequence literals,
  indexing, and finite `for`.
- [05_enums_and_match.tn](language/05_enums_and_match.tn): enum payloads and
  `match`.
- [06_result_propagation.tn](language/06_result_propagation.tn):
  `Result<T, E>` and postfix `!` propagation.
- [07_generics.tn](language/07_generics.tn): generic callables and generic
  structs.
- [08_std_imports.tn](language/08_std_imports.tn): standard module imports and
  namespace calls.
- [09_tasks.tn](language/09_tasks.tn): `spawn`, `Task<T>`, and `join`.

## Standard Library Examples

- [fs.tn](std/fs.tn): filesystem text, metadata, and path helpers.
- [hash.tn](std/hash.tn): stable text and byte hashing.
- [json.tn](std/json.tn): JSON validation and formatting helpers.
- [math.tn](std/math.tn): floating-point numeric helpers.
- [random.tn](std/random.tn): deterministic pseudo-random values.
- [time.tn](std/time.tn): clock reads and millisecond sleep.

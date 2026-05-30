# Giving Feedback

Tune is early. Useful feedback is not limited to bug reports. Reports about code
that feels too heavy, syntax that reads strangely, editor behavior that gets in
the way, or examples that fail to explain the language are all valuable.

## What To Try

Good first experiments:

```sh
dyno run examples/language/01_values_and_flow.tn
dyno check examples/language/03_structs_and_methods.tn
dyno new hello-tune
```

Then change the code and run `dyno check` again. Tune should feel like a
scripting language with compiler help, not like a ceremony-heavy systems
language.

## What To Report

Open an issue when:

- valid-looking Tune code is rejected
- invalid-looking Tune code is accepted
- diagnostics are confusing or point at the wrong thing
- editor completion, hover, rename, formatting, or go-to-definition behaves badly
- an example makes Tune look more complicated than it is
- a language rule feels surprising after you write real code

Small examples are best. A good report usually includes:

```tn
-- What you wrote.
let status = if score > 30 => "pass" else "retry"
print(status)
```

And:

```text
what you expected
what happened instead
the dyno command you ran
```

## Design Feedback

For language design questions, show the code you wanted to write first. Tune's
goal is a small set of primitives that compose naturally, so examples are more
useful than abstract feature requests.

It is especially useful to call out places where you had to add a temporary
binding, type annotation, import, or block that felt redundant. Those are often
signs that the compiler, docs, or language surface needs attention.

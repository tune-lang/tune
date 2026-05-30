# Tune

**Script-sized code. Systems-level help.**

Tune is a programming language for scripts, tools, plugins, workflows, and app logic that should stay easy to write without leaving basic meaning until the unlucky path runs.

Python showed how good low-ceremony code can feel. Tune keeps that direct style, then gives Dyno enough understanding to catch mistakes early, explain them clearly, power editor tooling, and make the fast path fast.

Tune is not “figure it out while running” with better checks. Runtime data can be unknown. A file can contain unknown text. A branch can depend on live input. A list can have a length that is only known while the program runs. But the meaning of the code cannot stay vague. If code reads a field, calls a function, compares a number, builds a string, propagates a result, or crosses a host boundary, Dyno must know what that operation means before the program runs.

That is the safety model:

```text
Unknown data is fine.
Unknown meaning is not.
```

This repository contains the Tune implementation, runtime, standard library, host API, package/project tooling, LSP work, examples, and the Dyno command-line tool.

Tune is pre-1.0. It is usable for small programs, examples, compiler/runtime development, and early feedback, but the language and platform are still changing.

## A small example

Say you are formatting a deploy event for a webhook, CI step, log line, or internal tool.

```tn
struct DeployEvent {
    service: String
    version: String
    ok: Bool
    actor: String
    duration_ms: Int
}

let deploy_line(event) = {
    let status = if event.ok => "deployed" else "failed"
    let speed = if event.duration_ms < 1000 => "fast" else "slow"

    "{event.service}@{event.version} {status} by {event.actor} in {event.duration_ms}ms ({speed})"
}

print(deploy_line(DeployEvent {
    service = "api"
    version = "a17c9e"
    ok = true
    actor = "Mira"
    duration_ms = 184
}))
```

Output:

```text
api@a17c9e deployed by Mira in 184ms (fast)
```

The function does not need a long signature. It just uses the event.

Dyno can still understand what the function needs:

```text
deploy_line(event) -> String

event must provide:
  service: String
  version: String
  ok: Bool
  actor: String
  duration_ms: Int
```

That is not extra code you have to write. It is what Tune learns from the code you already wrote.

## The same style in Python

Python is loved for a reason. You can write the same idea directly:

```py
from types import SimpleNamespace


def deploy_line(event):
    status = "deployed" if event.ok else "failed"
    speed = "fast" if event.duration_ms < 1000 else "slow"

    return f"{event.service}@{event.version} {status} by {event.actor} in {event.duration_ms}ms ({speed})"


print(deploy_line(SimpleNamespace(
    service="api",
    version="a17c9e",
    ok=True,
    actor="Mira",
    duration_ms=184,
)))
```

That directness is the bar. Tune is not trying to make Python look bad. Tune is trying to keep that lightweight feeling while changing what happens when the data is wrong.

## When a field is missing

Python happily creates this object. That flexibility is useful.

```py
print(deploy_line(SimpleNamespace(
    service="api",
    version="a17c9e",
    ok=True,
    actor="Mira",
    dur_ms=184,
)))
```

The bug appears when `deploy_line` reaches `event.duration_ms`:

```text
AttributeError: 'types.SimpleNamespace' object has no attribute 'duration_ms'
```

Tune can reject the event before `deploy_line` runs:

```tn
print(deploy_line(DeployEvent {
    service = "api"
    version = "a17c9e"
    ok = true
    actor = "Mira"
    dur_ms = 184
}))
```

Tune diagnostics are designed to include a code, source spans, what was expected, what was found, why Dyno believed that, and a concrete next step.

```text
error[T0301]: shape mismatch

  --> deploy.tn:20:19
   |
20 | print(deploy_line(DeployEvent {
   |                   ^^^^^^^^^^^ value does not fit DeployEvent
...
25 |     dur_ms = 184
   |     ------ found this field

missing:
  duration_ms: Int

unknown:
  dur_ms: Int

help:
  did you mean `duration_ms`?
```

The exact wording of diagnostics can evolve while Tune is pre-1.0. The important part is the model: Dyno diagnostics are structured, explainable, testable, and useful in the CLI, editors, JSON output, docs, and `dyno explain`.

## When a value has the wrong kind

Python keeps the call lightweight:

```py
print(deploy_line(SimpleNamespace(
    service="api",
    version="a17c9e",
    ok=True,
    actor="Mira",
    duration_ms="184",
)))
```

The mistake appears when the comparison runs:

```text
TypeError: '<' not supported between instances of 'str' and 'int'
```

Tune can connect the bad value to the exact expectation:

```tn
print(deploy_line(DeployEvent {
    service = "api"
    version = "a17c9e"
    ok = true
    actor = "Mira"
    duration_ms = "184"
}))
```

```text
error[T0301]: shape mismatch

  --> deploy.tn:25:19
   |
25 |     duration_ms = "184"
   |                   ^^^^^ String cannot materialize as Int

required by:
  DeployEvent.duration_ms: Int

used here:
  let speed = if event.duration_ms < 1000 => "fast" else "slow"
                 ---------------- compared with 1000 here

help:
  use a number:
    duration_ms = 184

help:
  or parse text before building the event:
    duration_ms = parse.int(raw_duration_ms)!
```

## Ask Dyno what an error means

Diagnostic codes are meant to be stable enough for docs, tests, tools, and search.

```sh
dyno explain T0301
```

```text
T0301: shape mismatch
A value's compile-time meaning does not satisfy the expected shape.
```

List all known diagnostic codes:

```sh
dyno explain
```

Check without running:

```sh
dyno check deploy.tn
```

Emit machine-readable diagnostics for tools and CI:

```sh
dyno check --json deploy.tn
```

## The core idea in plain English

Tune lets the compiler leave something unknown only while nothing depends on it.

An empty sequence can start out as “some sequence literal.” A number can start out as “the literal value `20`.” An ignored pattern can stay unnamed. A function parameter can start without a written type.

But the second code uses that thing, Tune has to know what the use means.

```tn
let failed_services = []

failed_services.push("api")
failed_services.push("worker")
```

At first, `[]` has not chosen its final representation. After `push("api")`, Dyno has enough information: this is a sequence of `String` values.

That is not runtime guessing. That is Tune delaying the decision until the code gives enough information, then locking the meaning down.

If the code later tries this:

```tn
failed_services.push(404)
```

Dyno can explain why the value no longer fits:

```text
error[T0302]: materialization failed

failed_services was materialized as [String]

expected:
  String

found:
  Int

help:
  use a String value here
  or make the sequence explicit if it is meant to hold more than one kind of value
```

## More small examples

Tune is built around a few ideas that should feel small in code. The technical names matter to the compiler, but the user-facing experience should stay simple.

### Use what you need

A Tune function can be useful without declaring a big interface up front.

```tn
struct Person {
    name: String
    email: String
}

struct ServiceAccount {
    name: String
    email: String
    key_id: String
}

let contact_line(contact) = {
    "{contact.name} <{contact.email}>"
}

print(contact_line(Person {
    name = "Mira"
    email = "mira@example.com"
}))

print(contact_line(ServiceAccount {
    name = "deploy-bot"
    email = "deploy@example.com"
    key_id = "svc_42"
}))
```

`contact_line` only uses `name` and `email`, so that is what Tune needs. `Person` works. `ServiceAccount` works. Extra fields do not make the call heavier.

This is not runtime duck typing. Dyno knows the required members before the call runs. The author just did not have to write the full generic form by hand.

<details>
<summary>Compiler view</summary>

The author writes:

```tn
let contact_line(contact) = {
    "{contact.name} <{contact.email}>"
}
```

Dyno can understand it like this:

```tn
let contact_line<T: {
    name: String
    email: String
}>(contact: T): String
```

That expanded form is not what you have to write. It is what Dyno can show in hovers, diagnostics, and public API warnings.

</details>

### Ignore what does not matter

`_` means “there is something here, but this code does not need a name for it.”

```tn
let count_items(items) = {
    let count = 0

    for _ in items {
        count = count + 1
    }

    count
}
```

The same idea works in patterns:

```tn
let message = match load_config(path) {
    Ok(_) => "config loaded"
    Error(reason) => "failed: {reason}"
}
```

`_` is not a magic fallback. In Tune, fallback branches use `else`:

```tn
let label = match status {
    Success(_) => "ok"
    else "not ok"
}
```

That keeps “ignore this value” separate from “handle every remaining case.”

If `_` is used somewhere that needs real meaning and Dyno cannot solve it, that is an error. Holes are allowed while unused or solvable. They are not a way to smuggle unknown behavior into the program.

### Let literals become the right thing

A literal can start as a fact and become a concrete value when the code needs one.

```tn
let delay = 20

sleep_ms(delay)      -- can use 20 as a Size
write_byte(delay)    -- can use 20 as a Byte if it fits
print(delay)         -- can display 20
```

The source still says `20`. Dyno can materialize that literal for each use when the expected meaning is known.

Sequences work the same way:

```tn
let failed_services = []

failed_services.push("api")
failed_services.push("worker")
```

The empty literal becomes `[String]` because the first meaningful use requires a sequence of strings.

Literals can also materialize into user-defined shapes when the shape explains how to build itself:

```tn
struct RetryQueue<T> {
    data: [T]

    [items] = {
        RetryQueue {
            data = items
        }
    }

    push(value: T) = {
        self.data.push(value)
    }
}

let delays: RetryQueue<Int> = [100, 250, 500]
```

The right side still looks like a simple script literal. The target shape explains how that literal becomes a `RetryQueue<Int>`.

Materializers are construction recipes. They should not perform outside-world work like file IO, time, randomness, or host effects.

### Safe absence instead of null surprises

Tune uses `T?` when a value may be absent.

```tn
let token = env.get("DEPLOY_TOKEN")
```

If `env.get` returns `String?`, then `token` is not a `String`. It is either `none` or a `String`.

This is not allowed:

```tn
let header = "Bearer {token}"
```

Dyno can point out the missing check:

```text
error[T0301]: optional value must be handled

expected:
  String

found:
  String?

note:
  `env.get("DEPLOY_TOKEN")` can return none

help:
  check the value before using it as a String
```

Handle the possible `none` case and the code becomes direct again:

```tn
enum ConfigError {
    MissingDeployToken
}

let auth_header(): Result<String, ConfigError> = {
    let token = env.get("DEPLOY_TOKEN")

    if token {
        Ok("Bearer {token}")
    } else {
        Error(MissingDeployToken)
    }
}
```

Inside the true branch, Tune knows `token` is present. Outside that branch, it remains `String?`.

### Propagate errors without hiding the path

Outside-world operations return ordinary results. `!` means “continue with the success value, or return the error from this function.”

```tn
let load_event(path: String): Result<DeployEvent, FsError | JsonError> = {
    let text = fs.read(path)!
    json.decode<DeployEvent>(text)!
}
```

If this fails at runtime, the error path can include the `!` sites that carried it upward:

```text
error: file not found: deploy.json
  propagated through:
    deploy.tn:2  fs.read(path)!                 in load_event
    main.tn:6    load_event("deploy.json")!     in main
```

The success path stays direct. The context is attached when the error path is taken.

### Choose concurrency at the call site

A Tune function does not have to be marked async forever. The caller decides when work should run independently.

```tn
let event_task = spawn load_event("deploy.json")
let user_task = spawn load_user("mira")

let event = event_task.join()!
let user = user_task.join()!

print("{user.name} deployed {event.service}")
```

If `load_event` returns a `Result`, then `join()` gives that result back naturally. `!` can propagate it like any other result.

## Built for fast paths

Tune is being built for more than better errors.

The same understanding that powers diagnostics also lets Tune make direct execution plans. In the deploy example, Dyno wants to know before execution:

```text
event.ok           -> the Bool field used by the branch
event.duration_ms  -> the Int field used by the comparison
event.service      -> the String field used in interpolation
deploy_line        -> the callable being invoked
```

That gives the runtime less guessing to do.

Tune's performance goal is practical, Go-like speed for script, tool, and app workloads without asking the author to write systems-language ceremony. The current implementation path starts with Dyno and a bytecode VM, while the compiler/runtime work is designed around direct calls and field access, predictable ownership, no tracing-GC requirement for normal execution, and reference counting only when values actually cross a sharing or ownership boundary.

In ordinary local code, Tune is designed so values do not pay reference-counting costs just for existing. RC/COW appears when the compiler needs a shared or escaping representation: returning a value, storing it longer term, capturing it in an escaping callable, crossing `spawn`, crossing a host boundary, or taking a lazy snapshot.

Ask Dyno to show what it planned:

```sh
dyno profile deploy.tn
```

A profile report can show pipeline and IR-quality information such as direct calls, witness/shared calls, host calls, ownership choices, field accesses, bytecode calls, and runtime guard pressure.

The pitch is not magic speed. The pitch is fewer surprises: write script-sized code, let the compiler keep track of what the code means, and make the fast path visible.

## Why Tune exists

Useful code often gets stuck between two worlds.

On one side, scripts are fast to write and easy to change, but mistakes often show up only when the unlucky path runs. On the other side, systems languages can give great performance and strong guarantees, but they may feel too heavy for deploy hooks, app scripting, internal tools, workflow glue, plugins, and host-embedded logic.

Tune is exploring a different split:

```text
Keep the author experience small.
Make the compiler understanding big.
```

Tune is aimed at:

- automation scripts
- app and plugin scripting
- CI/deploy tooling
- data and workflow glue
- host-embedded business logic
- tools that need editor help without becoming a large systems-language project

The goal is not “Python with types” or “Rust but easier.” Tune's bet is that a small language can feel direct to write while still giving Dyno enough information to explain mistakes, drive editor tooling, and plan execution seriously.

## What Dyno provides

Dyno is Tune's bundled command-line tool and default platform.

```sh
dyno new        # create a Tune project
dyno run        # run a file or project
dyno check      # check without running
dyno build      # compile and validate executable artifacts
dyno profile    # print pipeline and IR-quality metrics
dyno fmt        # format source
dyno explain    # explain diagnostic codes
dyno lsp        # start the language server
```

Create a project:

```sh
dyno new hello-tune
cd hello-tune
dyno run
```

Run a single file:

```sh
dyno run deploy.tn
```

Run one of the checked examples:

```sh
dyno run examples/language/01_values_and_flow.tn
```

Format or check formatting:

```sh
dyno fmt deploy.tn
dyno fmt --check deploy.tn
```

## Install

Tune is currently distributed from source.

Build and install Dyno from this checkout:

```sh
git clone https://github.com/tune-lang/tune.git
cd tune
cargo install --path crates/dyno_cli
```

That installs the `dyno` command into Cargo's bin directory, usually `~/.cargo/bin`.

If your shell cannot find `dyno`, add Cargo's bin directory to your `PATH`.

macOS/Linux, current shell:

```sh
export PATH="$HOME/.cargo/bin:$PATH"
```

macOS/Linux, future shells:

```sh
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
```

Windows PowerShell, current shell:

```powershell
$env:Path = "$env:USERPROFILE\.cargo\bin;$env:Path"
```

Windows PowerShell, future shells:

```powershell
[Environment]::SetEnvironmentVariable("Path", "$env:USERPROFILE\.cargo\bin;$env:Path", "User")
```

Check the install:

```sh
dyno --help
```

Prebuilt Dyno downloads are the next adoption milestone so new users can try Tune without installing Rust first.

## Learn by example

Start here:

- [Why Tune?](docs/why-tune.md)
- [Setup and Getting Started](docs/getting-started.md)
- [Tutorial](docs/tutorial.md)
- [Tune From Other Languages](docs/from-other-languages.md)
- [Language Tour](docs/language-tour.md)
- [Examples](examples/README.md)
- [Tooling](docs/tooling.md)
- [Giving Feedback](docs/feedback.md)

The language examples under [`examples/language`](examples/language) are small programs that teach one concept at a time and print a small result. They are checked and run by the test suite so the examples stay aligned with the implementation.

```sh
cargo test -p dyno_cli --test language_examples
```

The current standard library surface is summarized in [docs/stdlib.md](docs/stdlib.md).

If something feels awkward, surprising, or heavier than it should be, see [docs/feedback.md](docs/feedback.md). Tune is early enough that usage feedback can still change the shape of the language.

## Language surface

Current implemented areas include:

- typed bindings, expressions, blocks, and control flow
- structs, fields, methods, and struct literals
- enums, variants, tuples, pattern matching, and structural match checks
- generic callables and generic structs
- sequences, ranges, finite `for`, string indexing, and interpolation
- optional values with narrowing through `none`
- `Result` values and postfix `!` propagation
- `Task`, `spawn`, and `join`
- host modules, authorities, task-safety metadata, and host resources

A few Tune details that matter in practice:

```tn
let status = if ok => "deployed" else "failed"  -- if is an expression
let token = env.get("DEPLOY_TOKEN")              -- String? means absence must be handled
let text = fs.read(path)!                         -- ! propagates Result errors with context
let task = spawn load_user(id)                    -- callers choose concurrency
let user = task.join()!                           -- join returns the task's result
```

## Editor support

Editor integrations live in [`editors`](editors).

- VS Code support is in [`editors/vscode`](editors/vscode).
- Zed support is in [`editors/zed`](editors/zed).

Both start `dyno lsp` and consume the same Tune language server. Set `tune.dynoPath` in VS Code if `dyno` is not on `PATH`; Zed resolves `dyno` from the worktree `PATH`.

The same compiler facts should feed CLI diagnostics, LSP hovers, Problems panels, JSON output, snapshot tests, `dyno explain`, and documentation examples. That is why the editor story is part of Tune's core pitch, not an add-on.

## Standard library and hosts

Tune ships a default host/std profile through `tune_std`.

Current modules include:

```text
io, math, bits, encoding, parse, text, path, env, fs, hash, json, process, random, time
```

Outside-world operations are explicit host calls with known shapes, authority requirements, and task-safety metadata.

That matters for embedding. Host applications can provide modules, functions, opaque resources, IO surfaces, authority policies, target support, and executor integration. Hosts complete Tune; they should not casually mutate Tune's core language rules.

The public engine facade is `tune_engine::Tune`. Dyno CLI, tests, and embedders should go through the facade instead of stitching compiler internals together.

## Repository layout

The workspace is split into focused crates:

```text
tune_syntax, tune_ast, tune_hir, tune_db
tune_resolve, tune_shape, tune_plan
tune_ir, tune_opt, tune_bytecode, tune_vm
tune_runtime, tune_host, tune_std, tune_meta
tune_diagnostics, tune_lsp
dyno_project, dyno_pkg, dyno_cli
```

The short version:

```text
source -> AST/HIR -> resolve -> shape/state -> plan -> IR -> optimizer -> bytecode -> VM
```

That pipeline is an implementation detail for users, but it matters for the project. Errors, editor features, bytecode, profiling, optimization, host calls, and future native/WASM/JS backends should consume the same facts rather than rediscovering Tune semantics in separate systems.

<details>
<summary>Compiler view of the deploy example</summary>

The author writes this:

```tn
let deploy_line(event) = {
    let status = if event.ok => "deployed" else "failed"
    let speed = if event.duration_ms < 1000 => "fast" else "slow"

    "{event.service}@{event.version} {status} by {event.actor} in {event.duration_ms}ms ({speed})"
}
```

A technical view can look like this:

```tn
let deploy_line<T: {
    service: String
    version: String
    ok: Bool
    actor: String
    duration_ms: Int
}>(event: T): String
```

That technical form is not the pitch. The pitch is that Tune can learn this from the obvious code and use it for diagnostics, editor help, and execution planning.

</details>

## Development

Rust is pinned by [`rust-toolchain.toml`](rust-toolchain.toml).

Install the local Dyno CLI from this checkout:

```sh
cargo install --path crates/dyno_cli
```

Core checks:

```sh
cargo fmt --all -- --check
cargo check --workspace --all-targets --all-features
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-targets --all-features
```

Public examples:

```sh
cargo test -p dyno_cli --test language_examples
cargo test -p dyno_cli --test std_examples
```

Tooling and editor checks:

```sh
sh scripts/check_tooling.sh
```

Performance and IR-quality checks:

```sh
cargo bench -p tune_engine --bench pipeline
cargo run --package tune_engine --bin profile_trace -- ...
cargo run --package tune_engine --bin profile_trace -- --full --csv ...
cargo run --package tune_engine --bin quality_check -- ...
```

Benchmark fixtures live in `crates/tune_engine/benches/fixtures`.

See [CONTRIBUTING.md](CONTRIBUTING.md) for contribution checks and repository expectations.

## Status

Tune is early, but it is not just a syntax sketch.

The repository currently includes the Dyno CLI, checked examples, compiler/runtime work, diagnostics, a bytecode VM, standard library and host APIs, profiling/reporting work, LSP/editor integrations, and project tooling.

Use Tune today for examples, experiments, compiler/runtime work, tooling work, and early feedback. Do not treat it as a stable production language yet.

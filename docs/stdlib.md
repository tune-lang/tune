# Standard Library

Tune ships a default std/host profile through `tune_std`. Import modules by
name:

```tn
import "text"
import "parse"

let raw: String = text.trim(" 42 ")
let value: Result<Int, String> = parse.int(raw)
```

Std modules are ordinary typed host modules. Calls have declared shapes,
authority requirements, and task-safety metadata.

## Core Output

`print(text: String): Unit` is available from the default prelude/corestd and is
the normal way for examples and scripts to write visible output.

For explicit terminal I/O, import `io`.

## Modules

### `bits`

Deterministic bit helpers for `Int` and `Size`.

Example: [examples/std/bits.tn](../examples/std/bits.tn)

Common calls:

- `bits.count_ones(value: Int): Size`
- `bits.leading_zeros(value: Int): Size`
- `bits.trailing_zeros(value: Int): Size`
- `bits.rotate_left(value: Int, amount: Size): Int`
- `bits.rotate_right(value: Int, amount: Size): Int`
- `bits.size_count_ones(value: Size): Size`
- `bits.size_rotate_left(value: Size, amount: Size): Size`

### `encoding`

Byte/text encoding helpers.

Example: [examples/std/encoding.tn](../examples/std/encoding.tn)

- `encoding.hex(bytes: [Byte]): String`
- `encoding.from_hex(text: String): Result<[Byte], String>`

### `env`

Process environment and runtime path helpers. Requires environment-read host
authority.

Example: [examples/std/env.tn](../examples/std/env.tn)

- `env.args(): [String]`
- `env.var(name: String): Result<String, String>`
- `env.cwd(): Result<String, String>`
- `env.temp_dir(): String`
- `env.current_exe(): Result<String, String>`
- `env.var_names(): [String]`

### `fs`

Filesystem metadata, text/byte reads and writes, resource-backed file handles,
directory listing, and path mutation. Requires filesystem host authorities.

Example: [examples/std/fs.tn](../examples/std/fs.tn)

Representative calls:

- `fs.exists(path: String): Bool`
- `fs.is_file(path: String): Bool`
- `fs.is_dir(path: String): Bool`
- `fs.metadata(path: String): Result<fs.Metadata, String>`
- `fs.read_text(path: String): Result<String, String>`
- `fs.open(path: String): Result<fs.File, String>`
- `fs.read_chunk(file: fs.File, size: Size): Result<[Byte], String>`
- `fs.close(file: fs.File): Result<Unit, String>`
- `fs.write_text(path: String, text: String): Result<Unit, String>`
- `fs.append_text(path: String, text: String): Result<Unit, String>`
- `fs.read_dir(path: String): Result<[fs.DirEntry], String>`

### `hash`

Stable hashing for text, bytes, and combined hash values.

Example: [examples/std/hash.tn](../examples/std/hash.tn)

- `hash.text(text: String): Size`
- `hash.bytes(bytes: [Byte]): Size`
- `hash.combine(left: Size, right: Size): Size`

### `io`

Terminal I/O. These calls can write to stdout/stderr or read from stdin.

Example: [examples/std/io.tn](../examples/std/io.tn)

- `io.write(text: String): Result<Unit, String>`
- `io.write_line(text: String): Result<Unit, String>`
- `io.error(text: String): Result<Unit, String>`
- `io.error_line(text: String): Result<Unit, String>`
- `io.flush(): Result<Unit, String>`
- `io.read_line(): Result<String, String>`

### `json`

JSON validation, parsing, construction, inspection, and encoding. JSON values are
host-defined value structs, not stringly compiler magic.

Example: [examples/std/json.tn](../examples/std/json.tn)

- `json.valid(text: String): Bool`
- `json.decode(text: String): Result<json.Value, String>`
- `json.encode(value: json.Value): Result<String, String>`
- `json.format(text: String): Result<String, String>`
- `json.minify(text: String): Result<String, String>`
- `json.null(): json.Value`
- `json.bool(value: Bool): json.Value`
- `json.number(value: Float): json.Value`
- `json.string(value: String): json.Value`
- `json.array(items: [json.Value]): json.Value`
- `json.field(name: String, value: json.Value): json.Field`
- `json.object(fields: [json.Field]): json.Value`
- `json.kind(value: json.Value): String`

### `math`

Floating-point math helpers.

Example: [examples/std/math.tn](../examples/std/math.tn)

- `math.pi(): Float`
- `math.e(): Float`
- `math.pow(left: Float, right: Float): Float`
- `math.sin(value: Float): Float`
- `math.cos(value: Float): Float`
- `math.sqrt(value: Float): Float`
- `math.abs(value: Float): Float`
- `math.clamp(value: Float, min: Float, max: Float): Float`
- `math.is_finite(value: Float): Bool`

### `parse`

Parse primitive values from text.

Example: [examples/std/parse.tn](../examples/std/parse.tn)

- `parse.int(text: String): Result<Int, String>`
- `parse.int_radix(text: String, radix: Size): Result<Int, String>`
- `parse.float(text: String): Result<Float, String>`
- `parse.size(text: String): Result<Size, String>`
- `parse.size_radix(text: String, radix: Size): Result<Size, String>`
- `parse.byte(text: String): Result<Byte, String>`
- `parse.byte_radix(text: String, radix: Size): Result<Byte, String>`
- `parse.bool(text: String): Result<Bool, String>`

### `path`

Path manipulation without filesystem access.

Example: [examples/std/path.tn](../examples/std/path.tn)

- `path.join(base: String, next: String): String`
- `path.join_all(parts: [String]): String`
- `path.components(path: String): [String]`
- `path.file_name(path: String): String?`
- `path.stem(path: String): String?`
- `path.ext(path: String): String?`
- `path.with_ext(path: String, ext: String): String`
- `path.is_absolute(path: String): Bool`
- `path.is_relative(path: String): Bool`
- `path.separator(): String`

### `process`

Process execution and result inspection. Running processes requires process host
authority.

Example: [examples/std/process.tn](../examples/std/process.tn)

- `process.run(command: String, args: [String]): Result<process.ProcessResult, String>`
- `process.shell(command: String): Result<process.ProcessResult, String>`
- `process.success(result: process.ProcessResult): Bool`
- `process.code(result: process.ProcessResult): Int`
- `process.stdout(result: process.ProcessResult): String`
- `process.stderr(result: process.ProcessResult): String`

### `random`

Deterministic pseudo-random helpers keyed by seed and index. These are useful for
tests and reproducible scripts, not cryptographic randomness.

Example: [examples/std/random.tn](../examples/std/random.tn)

- `random.int(seed: Size, index: Size, min: Int, max: Int): Result<Int, String>`
- `random.float(seed: Size, index: Size): Float`
- `random.bool(seed: Size, index: Size): Bool`
- `random.index(seed: Size, index: Size, len: Size): Result<Size, String>`
- `random.bytes(seed: Size, count: Size): Result<[Byte], String>`

### `text`

Unicode-scalar text helpers.

Example: [examples/std/text.tn](../examples/std/text.tn)

- `text.bytes(text: String): [Byte]`
- `text.from_bytes(bytes: [Byte]): Result<String, String>`
- `text.split(text: String, delimiter: String): [String]`
- `text.join(items: [String], separator: String): String`
- `text.char_count(text: String): Size`
- `text.trim(text: String): String`
- `text.find(text: String, needle: String): Size?`
- `text.slice(text: String, start: Size, count: Size): Result<String, String>`
- `text.repeat(text: String, count: Size): Result<String, String>`

### `time`

Clock and sleep helpers. Reading clocks and sleeping require time host
authorities.

Example: [examples/std/time.tn](../examples/std/time.tn)

- `time.now_millis(): Result<Int, String>`
- `time.monotonic_millis(): Size`
- `time.sleep_millis(duration: Size): Result<Unit, String>`

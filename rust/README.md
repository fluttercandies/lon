# LON for Rust

Compact, canonical, lossless [LON v1](https://github.com/fluttercandies/lon/blob/main/spec/lon-v1.md) codec for the JSON data model. It converts between strict JSON text and canonical LON text while preserving object-member order, duplicate members, string code units (including escaped lone surrogates), and number lexemes such as `1e+02` or `-0`.

This crate tracks the shared [conformance fixtures](https://github.com/fluttercandies/lon/blob/main/conformance/v1.json) and mirrors the semantics of the [Dart reference implementation](https://github.com/fluttercandies/lon/tree/main/dart).

## Installation

```sh
cargo add lon
```

The crate is not yet on crates.io. Until it is published, depend on it with a `path` dependency inside a checkout of the monorepo:

```toml
[dependencies]
lon = { path = "path/to/lon/rust" }
```

A plain `git` dependency does not resolve: the crate lives in `rust/` and the repository root has no workspace manifest for cargo to discover it through.

## Text-to-text API

The lossless core: `json_to_lon` accepts strict JSON text and emits canonical LON; `lon_to_json` emits compact JSON. For any valid JSON `J`, `lon_to_json(json_to_lon(J))` yields the same JSON value, and for canonical LON `L`, `json_to_lon(lon_to_json(L)) == L`.

```rust
let lon = lon::json_to_lon(r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}"#)?;
assert_eq!(lon, "{users:[id name;1 Ada;2 Bob]}");

let json = lon::lon_to_json(&lon)?;
assert_eq!(json, r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}"#);
```

## Pretty rendering

`LonCodec::pretty(true)` renders one object member or table row per line, indented two spaces per nesting depth. The multi-line output decodes to the identical value and re-encodes to the identical canonical document; the same flag makes `lon_to_json` emit standard two-space-indented JSON:

```rust
let pretty = lon::LonCodec::new().pretty(true);
assert_eq!(
    pretty.json_to_lon(r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}"#)?,
    "{\n  users: [\n    id name;\n    1 Ada;\n    2 Bob\n  ]\n}",
);
```

## Value API

A `serde_json::Value` convenience layer is built on serde_json's `preserve_order` feature, so object order is kept and duplicate keys decode with the last value winning:

```rust
use lon::LonCodec;
use serde_json::json;

let codec = LonCodec::new();
let value = codec.decode("[id name;1 Ada;2 Bob]")?;
assert_eq!(value, json!([{"id": 1, "name": "Ada"}, {"id": 2, "name": "Bob"}]));
assert_eq!(codec.encode(&value)?, "[id name;1 Ada;2 Bob]");
```

Unpaired surrogates decode to U+FFFD in the value API, exactly like `serde_json`'s own parser, because a Rust `String` cannot hold them; the text-to-text API preserves them as `\uD800`-style escapes.

## Serde integration

Any `Serialize`/`Deserialize` type converts directly to and from LON, and sequences of uniform structs hit the record-table form automatically:

```rust
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct User {
    id: u32,
    name: String,
}

let users = vec![
    User { id: 1, name: "Ada".into() },
    User { id: 2, name: "Bob".into() },
];

let encoded = lon::to_string(&users)?;
assert_eq!(encoded, "[id name;1 Ada;2 Bob]");
assert_eq!(lon::from_str::<Vec<User>>(&encoded)?, users);

// Pretty multi-line form and codec options work the same way:
let pretty = lon::to_string_pretty(&users)?;
let compact = lon::LonCodec::new().use_tables(false).to_string(&users)?;
```

Semantics follow the value API: enums use `serde_json`'s conventions (unit variants become the bare variant-name string), `HashMap`/`BTreeMap` keys serialize as strings, duplicate keys decode with the last value winning (structs still reject duplicates, as serde itself does), non-finite floats are rejected, and `i128`/`u128` values beyond the `i64`/`u64` ranges have no JSON representation and fail. `#[serde(untagged)]` and `serde_json::Value` targets are supported through a complete `deserialize_any`.

## CLI

The crate ships a `lon` binary. It reads a file or standard input, writes one complete document followed by a newline, rejects malformed UTF-8 instead of replacing it, and never echoes untrusted input in diagnostics.

```sh
echo '{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}' | lon encode
# {users:[id name;1 Ada;2 Bob]}

echo '{users:[id name;1 Ada;2 Bob]}' | lon decode
# {"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}

echo '{users:[id name;1 Ada;2 Bob]}' | lon check
# (no output; exit 0 means valid)
```

`lon check` validates that the input is well-formed LON and writes nothing to stdout, suitable for save-time editor checks and lint pipelines. `--pretty` renders multi-line LON on `encode` and two-space-indented JSON on `decode`; it has no effect on `check`.

Exit codes: `64` usage error, `65` invalid input (including malformed UTF-8 and limit breaches), `66` file-system error.

## Limits

`LonCodec::new()` starts from the reference defaults; every limit is configurable with `const` builder methods.

| Option | Default | Meaning |
|---|---|---|
| `use_tables` | `true` | Emit self-describing record tables |
| `pretty` | `false` | Render multi-line LON and indented JSON |
| `max_depth` | `512` | Maximum container nesting depth |
| `max_expanded_elements` | 1,000,000 | Maximum parsed or encoded element occurrences (bounds table schema fields independently) |
| `max_input_code_units` | 67,108,864 | Maximum input length, counted in UTF-16 code units like the Dart codec |
| `max_output_code_units` | 67,108,864 | Maximum output length, counted in UTF-16 code units |

The CLI independently caps streamed input at 67,108,864 bytes. No operation returns a partial document after a limit breach.

Rust-specific notes: limits are `usize`, so the Dart codec's runtime `RangeError` for negative limits is unrepresentable at the type level, and error offsets count bytes of the UTF-8 input rather than UTF-16 code units.

## Performance

The codec is written for throughput: parsers scan ASCII bytes in bulk, string checks and canonical writers avoid intermediate allocations, and output buffers are pre-sized. On an Apple-Silicon runner (`cargo bench`, criterion, ~1 MB mixed payload: uniform record tables, nested objects, dense text), the text-to-text paths sustain roughly 230 MB/s JSON-to-LON and 200 MB/s LON-to-JSON; the value, serde, and table-form serde paths run at 55-125 MB/s. `cargo bench` reproduces the measurements.

## Running the tests

```sh
cd rust
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
```

The suite asserts the shared conformance fixtures (via `../conformance/v1.json`) plus ports of the Dart reference tests for grammar acceptance and rejection, table normalization and malformed-table rejection, limit boundaries, lone surrogates, and ambiguous-string quoting rules.

## License

BSD 3-Clause, see [LICENSE](LICENSE).

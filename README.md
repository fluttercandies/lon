<h1 align="center">LON</h1>

<p align="center">
  <a href="spec/lon-v1.md">
    <img src="assets/lon-logo.svg" alt="LON — compact, lossless, AI-first object notation" width="640">
  </a>
</p>

<p align="center">
  <a href="https://pub.dev/packages/lon"><img alt="Package version 1.0.0" src="https://img.shields.io/badge/version-1.0.0-8B5CF6?style=flat-square"></a>
  <a href="spec/lon-v1.md"><img alt="LON format v1" src="https://img.shields.io/badge/format-LON_v1-22D3EE?style=flat-square"></a>
  <a href="pubspec.yaml"><img alt="Dart SDK 3.3 or later" src="https://img.shields.io/badge/Dart-%5E3.3.0-0175C2?style=flat-square&amp;logo=dart&amp;logoColor=white"></a>
  <a href="spec/lon-v1.md"><img alt="Lossless JSON" src="https://img.shields.io/badge/JSON-lossless-2EA44F?style=flat-square&amp;logo=json&amp;logoColor=white"></a>
  <a href="benchmark/results/token-efficiency.md"><img alt="o200k benchmark uses LON as baseline" src="https://img.shields.io/badge/o200k-LON_baseline-F59E0B?style=flat-square"></a>
  <a href="https://github.com/fluttercandies/lon"><img alt="GitHub repository" src="https://img.shields.io/badge/source-GitHub-181717?style=flat-square&amp;logo=github&amp;logoColor=white"></a>
</p>

<p align="center">
  <strong>English</strong> · <a href="README.zh-CN.md">简体中文</a>
</p>

Language Object Notation (LON) is a compact, canonical, and lossless text encoding of the JSON data model. It is designed for AI-first data exchange without making humans or models guess types, boundaries, schemas, or omitted structure.

> Package version: `1.0.0` · Format version: `LON v1` · Dart SDK: `^3.3.0`

## Why LON

- **Lossless JSON text conversion.** Preserves object-member order, duplicate members, string code units, and number lexemes.
- **AI-first, not AI-only.** Removes redundant syntax while retaining familiar JSON delimiters and explicit type boundaries.
- **Self-describing tables.** Repeated record keys may be factored into a visible schema followed by semicolon-delimited rows.
- **One canonical encoding.** Different accepted spellings normalize to one deterministic output.
- **No hidden context.** No aliases, key dictionaries, numeric references, external schemas, or model instructions.
- **Resource-bounded.** Parsing, expansion, nesting, input, and output are limited before unsafe allocation or multiplication.

## Quick example

JSON:

```json
{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"},{"id":3,"name":"Lin"}]}
```

Canonical LON:

```lon
{users:[id name;1 Ada;2 Bob;3 Lin]}
```

The payload remains self-describing:

- `[` and `]` remain the decoded array boundary.
- `id name` is the visible record schema; the first `;` ends it.
- Each later `;` starts another row whose width is validated against the schema.
- Spaces separate fields without adding row parentheses or another delimiter.

For small or non-uniform data, LON stays close to JSON:

```json
{"id":7,"name":"Ada","active":true}
```

```lon
{id:7 name:Ada active:true}
```

## Syntax in one minute

### Objects and arrays

Objects retain braces and colons. Whitespace replaces object-member commas:

```lon
{name:Ada age:37 active:true}
```

Arrays retain brackets. Only arrays made entirely of safely bare strings use spaces:

```lon
[Ada Bob Lin]
```

All other arrays retain commas so mixed types and nested boundaries remain explicit:

```lon
[1,Ada,true,null]
[[1,2],[3,4]]
[1,{a:b},{a:[1,{b:c}]}]
```

### Deterministic value types

Value classification has one exhaustive precedence order:

1. Exact `null`, `true`, and `false` tokens are JSON literals.
2. Tokens matching JSON number syntax are numbers.
3. Every other permitted bare token is a string.
4. Anything unsafe or ambiguous must use a JSON-quoted string.

LON has no inferred identifier, enum, symbol, date, or custom scalar type. For example, both values below are strings:

```lon
[Ada 2026-08-02]
```

These ambiguous strings remain quoted:

```lon
["01" ".5" "NaN" "undefined" "None" "yes" "off"]
```

Whitespace, structural punctuation, double quotes, backslashes, controls, invisible separators, and lone surrogates also require JSON quoting or escaping. LON does not normalize Unicode case, width, direction, NFC, or NFD.
The same rules apply to object member names and table-schema field names, including empty names and arbitrary JSON string code-unit sequences.

### Self-describing tables

Uniform record arrays may use:

```text
[SCHEMA;ROW;ROW]
```

Example with a nested schema:

```lon
[id name addr{city zip} active;1 Ada London W1 true;2 Bob Paris "75001" false]
```

Equivalent JSON:

```json
[
  {"id":1,"name":"Ada","addr":{"city":"London","zip":"W1"},"active":true},
  {"id":2,"name":"Bob","addr":{"city":"Paris","zip":"75001"},"active":false}
]
```

Schemas and tables require at least two leaf-value fields and two rows. Smaller records remain ordinary arrays because forms such as `[id;1;2]` and `[id name;1 Ada]` are easily mistaken for scalar arrays or objects without a format primer.

Keyed objects and arrays with varying object-cell shapes stay explicit:

```lon
{production:{region:eu-central-1 replicas:6 debug:false} staging:{region:eu-central-1 replicas:2 debug:true}}
```

The outer brackets always decode to an array. A valid schema followed by `;` uniquely selects table grammar because ordinary arrays never use semicolons as separators. Spaces separate cells and later semicolons separate rows, so no count, row parentheses, or duplicate delimiter is needed. A complete-row deletion cannot be detected without a declared count; use a transport checksum or authenticated envelope when omission detection is required.

See the normative [LON v1 specification](spec/lon-v1.md) for the complete grammar and canonical table-selection rules.

## Use from Dart

Install the package from pub.dev:

```sh
dart pub add lon
```

Or add the dependency directly:

```yaml
dependencies:
  lon: ^1.0.0
```

For local development, replace the hosted constraint with `path: /path/to/lon`.

Import the public API:

```dart
import 'package:lon/lon.dart';
```

### Lossless text-to-text API

Use the text API when duplicate object members and exact number lexemes matter:

```dart
const json = '{"value":1e+02,"value":-0,"name":"Ada"}';

final encoded = jsonToLon(json);
final decodedJson = lonToJson(encoded);

print(encoded);     // {value:1e+02 value:-0 name:Ada}
print(decodedJson); // {"value":1e+02,"value":-0,"name":"Ada"}
```

`jsonToLon` accepts strict JSON text and emits canonical LON. `lonToJson` accepts LON and emits compact JSON while preserving member order, duplicate members, and number lexemes.

### Dart value API

Use the default codec for ordinary JSON-compatible Dart values:

```dart
final encoded = lon.encode({
  'name': 'Ada',
  'roles': ['admin', 'editor'],
  'active': true,
});

final value = lon.decode(encoded);
```

The Dart value API follows normal Dart/JSON map behavior: duplicate keys cannot be represented, and decoding duplicate members keeps the last value. Use the text-to-text API when those distinctions matter.

### Custom limits

```dart
const codec = LonCodec(
  useTables: true,
  maxDepth: 128,
  maxExpandedElements: 100000,
  maxInputCodeUnits: 8 * 1024 * 1024,
  maxOutputCodeUnits: 8 * 1024 * 1024,
);
```

Default limits:

| Limit | Default |
|---|---:|
| Container nesting depth | 512 |
| Parsed or expanded elements | 1,000,000 |
| Table schema fields | 1,000,000, through the element limit |
| Input UTF-16 code units | 67,108,864 |
| Output UTF-16 code units | 67,108,864 |

Text parse and limit failures throw `FormatException`. Invalid negative codec limits throw `RangeError`. Dart value encoding rejects unsupported, cyclic, non-finite, or over-budget values with argument errors rather than returning a partial document.

## Command line

From this repository:

```sh
dart pub get
dart run bin/lon.dart encode input.json
dart run bin/lon.dart decode input.lon
```

Standard input is used when no file is supplied:

```sh
printf '%s' '{"name":"Ada","active":true}' | dart run bin/lon.dart encode
printf '%s' '{name:Ada active:true}' | dart run bin/lon.dart decode
```

Install the executable from pub.dev:

```sh
dart pub global activate lon
lon encode input.json
lon decode input.lon
```

To activate a local checkout instead, run `dart pub global activate --source path .`.

CLI behavior:

| Exit code | Meaning |
|---:|---|
| 64 | Invalid command or argument count |
| 65 | Invalid JSON/LON, invalid UTF-8, or input limit exceeded |
| 66 | File-system failure |

The CLI caps streamed UTF-8 input at 67,108,864 bytes and does not echo untrusted input in diagnostics.

### UTF-8 and network transport

Encode one complete LON document as strict UTF-8 without a BOM, and reject malformed UTF-8 instead of replacement-decoding it:

```dart
import 'dart:convert';

final body = utf8.encode(lon.encode(value));
final received = lon.decode(utf8.decode(body, allowMalformed: false));
```

For HTTP or another byte-preserving protocol, send LON in the message body with an explicit content length or frame boundary. This package does not register a media type; use `application/octet-stream` when byte identity matters, or `text/plain; charset=utf-8` only when intermediaries will not normalize or transcode the body.

Canonical LON contains no raw C0/C1 controls, CR, LF, Unicode line separators, BOM, or visibility-sensitive code points from string data; those are visible JSON escapes. The CLI adds one final LF after its complete document. LON is not an HTTP-header, URL, HTML, JavaScript, shell, or SQL escaping format, so apply the destination context's encoding rather than embedding untrusted LON directly.

## Token benchmark

The reproducible benchmark compares LON, Compact JSON, TOON 4.1.0, YAML 2.9.0, and Pretty JSON across 27 deterministic small, medium, and large scenarios. Every payload must decode to the original JSON value before it is counted.

`o200k_base`, with LON as the comparison baseline:

| Scope | LON | Compact JSON | TOON | YAML | Pretty JSON |
|---|---:|---:|---:|---:|---:|
| Small, 14 cases | **149** | 193 (+29.5%) | 186 (+24.8%) | 202 (+35.6%) | 347 (+132.9%) |
| Medium, 8 cases | **3,964** | 5,516 (+39.2%) | 5,119 (+29.1%) | 7,526 (+89.9%) | 9,146 (+130.7%) |
| Large, 5 cases | **79,694** | 107,636 (+35.1%) | 104,247 (+30.8%) | 134,543 (+68.8%) | 168,891 (+111.9%) |
| All, 27 cases | **83,807** | 113,345 (+35.2%) | 109,552 (+30.7%) | 142,271 (+69.8%) | 178,384 (+112.9%) |

A positive percentage means that format consumes more tokens than LON. LON is not forced to win every tiny payload: the report retains per-case ties and cases where TOON or YAML is smaller.

Reproduce the benchmark:

```sh
cd benchmark
bun install --frozen-lockfile
bun run benchmark
```

Outputs:

- [Human-readable benchmark report](benchmark/results/token-efficiency.md)
- [Machine-readable benchmark results](benchmark/results/token-efficiency.json)

Token count is not a model-comprehension score. The benchmark measures payload cost; semantic round-trip validation and the serialized examples provide the objective correctness and inspectability checks.

## Correctness and conformance

For valid JSON text `J`:

```text
lonToJson(jsonToLon(J))
```

produces compact JSON with the same JSON value, member order, duplicate members, string code units, and number lexemes. JSON whitespace and equivalent escape spellings are canonicalized.

For canonical LON `L`:

```text
jsonToLon(lonToJson(L)) == L
```

Run the quality gates:

```sh
dart analyze
dart test
```

The suite includes:

- [LON v1 conformance cases](conformance/v1.json)
- deterministic generated round trips
- JSONTestSuite valid, invalid, and implementation-defined inputs
- Unicode and surrogate round trips
- malformed and truncated table rejection
- nesting, schema, expansion, input, and output limits
- CLI diagnostic safety

## Project layout

| Path | Purpose |
|---|---|
| `lib/` | Public Dart package and codec implementation |
| `bin/lon.dart` | `lon encode` / `lon decode` CLI |
| `assets/` | Scalable LON logo and standalone mark |
| `example/` | Runnable package example |
| `spec/lon-v1.md` | Normative format specification |
| `conformance/v1.json` | Cross-implementation conformance fixtures |
| `benchmark/` | Reproducible multi-format token benchmark |
| `.github/workflows/benchmark.yml` | Push-triggered benchmark CI |
| `test/` | Unit, round-trip, security, and JSON conformance tests |
| `CHANGELOG.md` | Versioned release notes |
| `LICENSE` | BSD 3-Clause license |

## Design boundary

LON encodes the JSON data model. It does not add comments, timestamps, binary blobs, references, arbitrary object types, or inferred application schemas. Keeping that boundary is what makes canonical round trips and unambiguous AI/human interpretation enforceable.

## License

LON is available under the [BSD 3-Clause License](LICENSE).

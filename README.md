<h1 align="center">LON</h1>

<p align="center">
  <a href="spec/lon-v1.md">
    <img src="assets/lon-logo.svg" alt="LON — compact, lossless, AI-first object notation" width="640">
  </a>
</p>

<p align="center">
  <a href="spec/lon-v1.md"><img alt="LON format v1" src="https://img.shields.io/badge/format-LON_v1-22D3EE?style=flat-square"></a>
  <a href="spec/lon-v1.md"><img alt="Lossless JSON" src="https://img.shields.io/badge/JSON-lossless-2EA44F?style=flat-square&amp;logo=json&amp;logoColor=white"></a>
  <a href="benchmark/results/token-efficiency.md"><img alt="o200k benchmark uses LON as baseline" src="https://img.shields.io/badge/o200k-LON_baseline-F59E0B?style=flat-square"></a>
  <a href="https://github.com/fluttercandies/lon"><img alt="GitHub repository" src="https://img.shields.io/badge/source-GitHub-181717?style=flat-square&amp;logo=github&amp;logoColor=white"></a>
</p>

<p align="center">
  <strong>English</strong> · <a href="README.zh-CN.md">简体中文</a>
</p>

Language Object Notation (LON) is a compact, canonical, and lossless text encoding of the JSON data model. It is designed for AI-first data exchange without making humans or models guess types, boundaries, schemas, or omitted structure.

> Format version: `LON v1`

This repository is a multi-language monorepo: the normative specification, conformance fixtures, benchmarks, and branding are language independent, and each implementation lives in its own directory.

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

See the normative [LON v1 specification](spec/lon-v1.md) for the complete grammar and canonical table-selection rules.

## Implementations

| Language | Status | Entry |
|---|---|---|
| Dart | Ready | [`dart/`](dart/README.md) · [lon on pub.dev](https://pub.dev/packages/lon) |
| Rust | Ready | [`rust/`](rust/README.md) · [lon on crates.io](https://crates.io/crates/lon) |

Each implementation is validated against the shared [conformance fixtures](conformance/v1.json).

## Repository layout

| Path | Purpose |
|---|---|
| `spec/lon-v1.md` | Normative format specification |
| `conformance/v1.json` | Cross-implementation conformance fixtures ([schema](conformance/schema.json)) |
| `benchmark/` | Reproducible multi-format token benchmark |
| `editors/` | Editor support (VS Code syntax highlighting and language configuration) |
| `assets/` | Scalable LON logo and standalone mark |
| `dart/` | Dart implementation, CLI, and tests |
| `rust/` | Rust implementation, CLI, and tests |

## Token benchmark

The reproducible benchmark compares LON, Compact JSON, TOON, YAML, and Pretty JSON across 27 deterministic small, medium, and large scenarios. Every payload must decode to the original JSON value before it is counted, and LON is the comparison baseline.

Read the [human-readable benchmark report](benchmark/results/token-efficiency.md) or the [machine-readable results](benchmark/results/token-efficiency.json). Reproduce it:

```sh
cd benchmark
bun install --frozen-lockfile
bun run benchmark
```

## Design boundary

LON encodes the JSON data model. It does not add comments, timestamps, binary blobs, references, arbitrary object types, or inferred application schemas. Keeping that boundary is what makes canonical round trips and unambiguous AI/human interpretation enforceable.

## License

LON is available under the [BSD 3-Clause License](LICENSE).

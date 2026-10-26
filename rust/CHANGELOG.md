# Changelog

## 0.1.0 - 2026-10-26

First release of the Rust implementation of the canonical Language Object Notation v1 format.

- Add lossless JSON text conversion: `json_to_lon` accepts strict JSON text and emits canonical LON; `lon_to_json` emits compact JSON while preserving member order, duplicate members, string code units (including escaped lone surrogates), and number lexemes.
- Add a `serde_json::Value` convenience layer via `LonCodec::encode`/`LonCodec::decode`, built on serde_json's `preserve_order` feature.
- Add direct serde integration: `lon::to_string`, `lon::to_string_pretty`, and `lon::from_str` serialize any `Serialize` type to canonical LON (record tables included) and deserialize any `DeserializeOwned` type back, with `LonCodec::to_string`/`LonCodec::from_str` for custom limits; all entry points budget object graphs identically (one expanded element per member).
- Add optional multi-line rendering: `LonCodec::pretty(true)` places each object member and table row on its own line with two-space-per-depth indentation and makes `lon_to_json` emit standard indented JSON.
- Add the `lon` command-line binary with `lon encode`, `lon decode`, and `lon check` subcommands, a `--pretty` flag on `encode`/`decode`, and sysexits-style exit codes: 64 for usage errors, 65 for invalid input (including malformed UTF-8 and limit breaches), 66 for file-system errors. Diagnostics never include untrusted source text.
- Add configurable nesting, expansion, input, and output resource limits through `LonCodec` builder methods; the table compactness gate uses saturating arithmetic so cost comparison stays safe on 32-bit targets.
- Deliver 1.3x-3.4x headroom throughput through ASCII bulk scanning in the parsers, allocation-free reserved-token and bare-string checks, widened-run string construction, and pre-sized byte-oriented writers, with byte-identical output.
- Validate against the shared LON v1 conformance fixtures, ports of the Dart reference test suite, and a differential harness that asserts byte-identical behavior with the Dart codec.

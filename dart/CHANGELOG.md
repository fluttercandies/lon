# Changelog

## 1.1.0 - 2026-10-26

- Add optional multi-line rendering: `LonCodec(pretty: true)` and `jsonToLon(..., pretty: true)` place each object member and table row on its own line with two-space-per-depth indentation; `LonCodec(pretty: true).decodeJson` emits standard indented JSON.
- Add `--pretty` to the `lon encode` and `lon decode` CLI commands.
- Add `lon check` to validate that input is well-formed LON: silent exit 0 on success, 65 on invalid input, 66 on file-system errors.
- Fix the CLI to reject a leading UTF-8 byte-order mark instead of silently stripping it, matching codec behavior.
- Improve codec throughput 1.4x-2.9x (AOT, `tool/bench.dart`): replace the per-token regex classifiers with hand-rolled scanners behind an exact non-ASCII folding fallback, add fast paths for printable-ASCII visibility checks, bulk-write unescaped string runs, cache hot parser loops' locals, preallocate decoded arrays, and read CLI input through `BytesBuilder`. Canonical output is byte-for-byte unchanged.
- Add `tool/bench.dart`, a zero-dependency Stopwatch throughput benchmark (small/medium/large payloads, median-of-9 rounds, `dart run` or AOT-compiled).
- Document that `useTables` applies to encoding only, and tighten static analysis lints.
- Expand boundary, limit, and round-trip test coverage.

## 1.0.1 - 2026-08-02

- Add 20,000-case multi-seed structural property tests, exact ordinary-container depth boundaries, dynamic value and record-shape matrices, duplicate-key records, deterministic LON mutations, and exhaustive complex-container truncations.

## 1.0.0 - 2026-08-02

- Publish the canonical Language Object Notation v1 format and Dart codec.
- Add lossless JSON text conversion that preserves member order, duplicate members, string code units, and number lexemes.
- Add JSON-compatible Dart value encoding and decoding.
- Add self-describing array record tables with deterministic compactness selection.
- Add strict ambiguity, Unicode visibility, malformed input, and trailing content checks.
- Add configurable nesting, expansion, input, and output resource limits.
- Add the `lon encode` and `lon decode` command-line interface.
- Add conformance fixtures, JSONTestSuite coverage, generated round trips, and security regression tests.
- Add a reproducible LON, Compact JSON, TOON, YAML, and Pretty JSON token benchmark.
- Replace tagged and counted table headers with the minimal `[SCHEMA;ROW;ROW]` array form; require at least two rows and leaf values, and keep keyed and varying object data explicit for zero-prompt readability.
- Add exhaustive UTF-16 value and member-name coverage, grammar-conflict table-schema cases, strict chunked UTF-8 transport tests, and explicit network framing and embedding rules.
- Gate pushes and pull requests on Dart 3.3 and stable formatting, analysis, tests, and publish validation; run benchmarks on every push and retain SHA-named reports for 30 days.
- Add English and Simplified Chinese documentation and scalable project branding.

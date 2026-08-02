# Changelog

## Unreleased

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

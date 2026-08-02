# LON Multi-format Token Efficiency Benchmark

## Scope

This report compares the serialized payloads of the same JSON data in LON, TOON, YAML, Compact JSON, and Pretty JSON. Before token counting, every payload is decoded and checked for deep equality with the original JSON value. The benchmark fails instead of reporting a result if any format changes a type or structure.

Token count does not measure model comprehension. This report measures payload cost and includes small examples for human inspection. Code fences, format instructions, prompts, and transport-protocol overhead are excluded.

## Measured highlights

- Small JSON covers 14 scenarios and 548 Compact JSON UTF-8 input bytes: LON 149 tokens (baseline), Compact JSON 193 (+29.5%), TOON 186 (+24.8%), and YAML 202 (+35.6%).
- LON has the lowest token count, including ties, in 11/14 small scenarios. LON is not guaranteed to win on tiny objects; inspect the per-scenario results.
- Across all 27 scenarios, LON uses 83807 tokens (baseline), Compact JSON 113345 (+35.2%), TOON 109552 (+30.7%), and YAML 142271 (+69.8%).
- Every number is measured after a semantic round trip; lower is more token-efficient.

## Reproduce

Dart SDK and Bun are required. Dependency versions are fixed by `benchmark/bun.lock`.

```sh
cd benchmark
bun install --frozen-lockfile
bun run benchmark
```

The command overwrites `benchmark/results/token-efficiency.json` and `benchmark/results/token-efficiency.md`.

## Versions and methodology

- LON 1.0.0, syntax v1, canonical encoder/decoder from this repository
- TOON 4.1.0, default encoder/decoder
- YAML 2.9.0, default stringify/parse
- Compact JSON: `JSON.stringify`
- Pretty JSON: `JSON.stringify(value, null, 2)`
- Tokenizer: gpt-tokenizer 3.4.0 with `o200k_base` and `cl100k_base`
- Runtime: Bun 1.3.14; Dart SDK version: 3.12.0 (stable) (Fri May 8 01:51:14 2026 -0700) on "macos_arm64"
- Every format has one trailing serializer line break removed; internal payload whitespace is unchanged
- Scenarios are generated deterministically without random, time-based, or network data
- Small scenarios cover primitives, empty containers, single fields, mixed types, ambiguous strings, two- and three-row tables, nesting, non-uniform structures, Unicode, and escapes
- Medium and large scenarios cover uniform records, nested orders, keyed maps, semi-uniform logs, configuration, string lists, numeric matrices, and time-series metrics

## o200k_base summary

Parenthesized values show token change relative to LON. Positive values use more tokens; negative values use fewer.

| Scope | Scenarios | LON | Compact JSON | TOON | YAML | Pretty JSON |
|---|---:|---:|---:|---:|---:|---:|
| Small | 14 | 149 | 193 (+29.5%) | 186 (+24.8%) | 202 (+35.6%) | 347 (+132.9%) |
| Medium | 8 | 3,964 | 5,516 (+39.2%) | 5,119 (+29.1%) | 7,526 (+89.9%) | 9,146 (+130.7%) |
| Large | 5 | 79,694 | 107,636 (+35.1%) | 104,247 (+30.8%) | 134,543 (+68.8%) | 168,891 (+111.9%) |
| All | 27 | 83,807 | 113,345 (+35.2%) | 109,552 (+30.7%) | 142,271 (+69.8%) | 178,384 (+112.9%) |

## cl100k_base summary

| Scope | Scenarios | LON | Compact JSON | TOON | YAML | Pretty JSON |
|---|---:|---:|---:|---:|---:|---:|
| Small | 14 | 155 | 195 (+25.8%) | 192 (+23.9%) | 208 (+34.2%) | 353 (+127.7%) |
| Medium | 8 | 3,892 | 5,361 (+37.7%) | 5,056 (+29.9%) | 7,445 (+91.3%) | 9,064 (+132.9%) |
| Large | 5 | 77,192 | 103,033 (+33.5%) | 102,046 (+32.2%) | 131,843 (+70.8%) | 166,191 (+115.3%) |
| All | 27 | 81,239 | 108,589 (+33.7%) | 107,294 (+32.1%) | 139,496 (+71.7%) | 175,608 (+116.2%) |

## Small JSON: per-scenario o200k_base

| Scenario | JSON bytes | LON | Compact JSON | TOON | YAML | Pretty JSON | Fewest tokens |
|---|---:|---:|---:|---:|---:|---:|---|
| Null root | 4 | 1 | 1 | 1 | 1 | 1 | LON / Compact JSON / TOON / YAML / Pretty JSON |
| Short string root | 5 | 1 | 3 (+200.0%) | 1 | 1 | 3 (+200.0%) | LON / TOON / YAML |
| Empty object and array | 24 | 6 | 8 (+33.3%) | 5 (−16.7%) | 6 | 12 (+100.0%) | TOON |
| Single-field object | 11 | 4 | 5 (+25.0%) | 3 (−25.0%) | 3 (−25.0%) | 8 (+100.0%) | TOON / YAML |
| Flat mixed-type object | 35 | 9 | 13 (+44.4%) | 12 (+33.3%) | 12 (+33.3%) | 22 (+144.4%) | LON |
| Ambiguous strings | 76 | 25 | 26 (+4.0%) | 29 (+16.0%) | 27 (+8.0%) | 39 (+56.0%) | LON |
| Safe short-string array | 22 | 5 | 7 (+40.0%) | 8 (+60.0%) | 8 (+60.0%) | 14 (+180.0%) | LON |
| Mixed scalar array | 19 | 7 | 8 (+14.3%) | 9 (+28.6%) | 12 (+71.4%) | 16 (+128.6%) | LON |
| Two uniform short records | 55 | 12 | 21 (+75.0%) | 19 (+58.3%) | 25 (+108.3%) | 45 (+275.0%) | LON |
| Three uniform short records | 77 | 15 | 29 (+93.3%) | 25 (+66.7%) | 37 (+146.7%) | 63 (+320.0%) | LON |
| Small nested object | 57 | 15 | 19 (+26.7%) | 20 (+33.3%) | 22 (+46.7%) | 38 (+153.3%) | LON |
| Non-uniform record array | 25 | 9 | 11 (+22.2%) | 15 (+66.7%) | 10 (+11.1%) | 24 (+166.7%) | LON |
| Multilingual text and emoji | 69 | 15 | 17 (+13.3%) | 12 (−20.0%) | 12 (−20.0%) | 27 (+80.0%) | TOON / YAML |
| Newline and structural characters | 69 | 25 | 25 | 27 (+8.0%) | 26 (+4.0%) | 35 (+40.0%) | LON / Compact JSON |

## Medium JSON: per-scenario o200k_base

| Scenario | JSON bytes | LON | Compact JSON | TOON | YAML | Pretty JSON | Fewest tokens |
|---|---:|---:|---:|---:|---:|---:|---|
| 10 uniform user records | 1038 | 171 | 306 (+78.9%) | 195 (+14.0%) | 392 (+129.2%) | 520 (+204.1%) | LON |
| 50 uniform user records | 5210 | 814 | 1,509 (+85.4%) | 918 (+12.8%) | 1,955 (+140.2%) | 2,563 (+214.9%) | LON |
| 10 nested orders | 2866 | 663 | 979 (+47.7%) | 1,060 (+59.9%) | 1,224 (+84.6%) | 1,637 (+146.9%) | LON |
| 20 keyed configuration entries | 1733 | 484 | 603 (+24.6%) | 415 (−14.3%) | 701 (+44.8%) | 909 (+87.8%) | TOON |
| 25 semi-uniform event records | 3335 | 976 | 1,113 (+14.0%) | 1,392 (+42.6%) | 1,390 (+42.4%) | 1,669 (+71.0%) | LON |
| Deep application configuration | 712 | 148 | 197 (+33.1%) | 213 (+43.9%) | 242 (+63.5%) | 350 (+136.5%) | LON |
| 100-item string list | 902 | 304 | 404 (+32.9%) | 403 (+32.6%) | 601 (+97.7%) | 609 (+100.3%) | LON |
| 20 x 10 numeric matrix | 742 | 404 | 405 (+0.2%) | 523 (+29.5%) | 1,021 (+152.7%) | 889 (+120.0%) | LON |

## Large JSON: per-scenario o200k_base

| Scenario | JSON bytes | LON | Compact JSON | TOON | YAML | Pretty JSON | Fewest tokens |
|---|---:|---:|---:|---:|---:|---:|---|
| 500 uniform user records | 52952 | 8,051 | 15,046 (+86.9%) | 9,055 (+12.5%) | 19,542 (+142.7%) | 25,550 (+217.4%) | LON |
| 200 nested orders | 57779 | 12,803 | 19,485 (+52.2%) | 20,990 (+63.9%) | 24,574 (+91.9%) | 32,873 (+156.8%) | LON |
| 500 keyed configuration entries | 43091 | 12,004 | 15,003 (+25.0%) | 10,015 (−16.6%) | 17,501 (+45.8%) | 22,509 (+87.5%) | TOON |
| 1,000 semi-uniform event records | 131990 | 38,438 | 43,870 (+14.1%) | 55,051 (+43.2%) | 55,048 (+43.2%) | 65,693 (+70.9%) | LON |
| 365 days of uniform analytics metrics | 36804 | 8,398 | 14,232 (+69.5%) | 9,136 (+8.8%) | 17,878 (+112.9%) | 22,266 (+165.1%) | LON |

## UTF-8 byte summary

| Scope | LON | Compact JSON | TOON | YAML | Pretty JSON |
|---|---:|---:|---:|---:|---:|
| Small | 410 | 548 (+33.7%) | 441 (+7.6%) | 496 (+21.0%) | 849 (+107.1%) |
| Medium | 11,131 | 16,538 (+48.6%) | 12,736 (+14.4%) | 19,602 (+76.1%) | 27,405 (+146.2%) |
| Large | 221,651 | 322,616 (+45.6%) | 261,292 (+17.9%) | 360,198 (+62.5%) | 503,845 (+127.3%) |
| All | 233,192 | 339,702 (+45.7%) | 274,469 (+17.7%) | 380,296 (+63.1%) | 532,099 (+128.2%) |

## Fewest-token scenario counts

Ties count for every winning format, so the total can exceed 27.

| Format | o200k_base fewest-token scenarios | cl100k_base fewest-token scenarios |
|---|---:|---:|
| LON | 22 | 22 |
| Compact JSON | 2 | 2 |
| TOON | 7 | 7 |
| YAML | 4 | 4 |
| Pretty JSON | 1 | 1 |

## Small payload examples

### Flat mixed-type object

**LON**

```lon
{id:7 name:Ada active:true}
```

**Compact JSON**

```json
{"id":7,"name":"Ada","active":true}
```

**TOON**

```toon
id: 7
name: Ada
active: true
```

**YAML**

```yaml
id: 7
name: Ada
active: true
```

**Pretty JSON**

```json
{
  "id": 7,
  "name": "Ada",
  "active": true
}
```

### Ambiguous strings

**LON**

```lon
{code:"01" word:"null" date:2026-08-02 url:"https://lon.dev/docs"}
```

**Compact JSON**

```json
{"code":"01","word":"null","date":"2026-08-02","url":"https://lon.dev/docs"}
```

**TOON**

```toon
code: "01"
word: "null"
date: 2026-08-02
url: "https://lon.dev/docs"
```

**YAML**

```yaml
code: "01"
word: "null"
date: 2026-08-02
url: https://lon.dev/docs
```

**Pretty JSON**

```json
{
  "code": "01",
  "word": "null",
  "date": "2026-08-02",
  "url": "https://lon.dev/docs"
}
```

### Three uniform short records

**LON**

```lon
{users:[id name;1 Ada;2 Bob;3 Lin]}
```

**Compact JSON**

```json
{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"},{"id":3,"name":"Lin"}]}
```

**TOON**

```toon
users[3]{id,name}:
  1,Ada
  2,Bob
  3,Lin
```

**YAML**

```yaml
users:
  - id: 1
    name: Ada
  - id: 2
    name: Bob
  - id: 3
    name: Lin
```

**Pretty JSON**

```json
{
  "users": [
    {
      "id": 1,
      "name": "Ada"
    },
    {
      "id": 2,
      "name": "Bob"
    },
    {
      "id": 3,
      "name": "Lin"
    }
  ]
}
```

### Small nested object

**LON**

```lon
{user:{id:1 name:Ada} roles:[admin editor]}
```

**Compact JSON**

```json
{"user":{"id":1,"name":"Ada"},"roles":["admin","editor"]}
```

**TOON**

```toon
user:
  id: 1
  name: Ada
roles[2]: admin,editor
```

**YAML**

```yaml
user:
  id: 1
  name: Ada
roles:
  - admin
  - editor
```

**Pretty JSON**

```json
{
  "user": {
    "id": 1,
    "name": "Ada"
  },
  "roles": [
    "admin",
    "editor"
  ]
}
```

## Fairness boundaries

- YAML, TOON, and LON use the default text encoder from each pinned version; the outputs are not hand-optimized examples.
- Pretty JSON is a readability reference, not a different data model.
- CSV is excluded because it only represents flat tables and loses JSON scalar types without an external schema, which fails this benchmark's unambiguous round-trip requirement.
- XML is excluded because it has no unique, schema-free mapping for JSON arrays, nulls, numbers, and booleans. Any custom mapping would add schema overhead or implicit rules to the comparison.
- MessagePack and CBOR are binary formats. Counting language-model tokens after base64 encoding is not equivalent to comparing native text formats.
- Totals directly sum payload tokens across scenarios, so large inputs dominate the All row. Inspect both the per-scenario small table and grouped summaries.

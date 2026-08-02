#!/usr/bin/env bun

import { spawnSync } from 'node:child_process'
import { readFileSync, rmSync, mkdirSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { dirname, join, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import { isDeepStrictEqual } from 'node:util'

import { decode as decodeToon, encode as encodeToon } from '@toon-format/toon'
import { encode as encodeCl100k } from 'gpt-tokenizer/encoding/cl100k_base'
import { encode as encodeO200k } from 'gpt-tokenizer/encoding/o200k_base'
import { parse as parseYaml, stringify as stringifyYaml } from 'yaml'

const BENCHMARK_DIR = dirname(fileURLToPath(import.meta.url))
const ROOT_DIR = resolve(BENCHMARK_DIR, '..')
const RESULTS_DIR = join(BENCHMARK_DIR, 'results')
const BINARY_PATH = join(
  tmpdir(),
  process.platform === 'win32'
    ? `lon-token-benchmark-${process.pid}.exe`
    : `lon-token-benchmark-${process.pid}`,
)
const MAX_BUFFER_BYTES = 64 * 1024 * 1024

const FORMAT_ORDER = ['lon', 'json-compact', 'toon', 'yaml', 'json-pretty']
const COMPARISON_BASELINE = 'lon'
const FORMAT_META = {
  'json-compact': { label: 'Compact JSON', fence: 'json' },
  'lon': { label: 'LON', fence: 'lon' },
  'toon': { label: 'TOON', fence: 'toon' },
  'yaml': { label: 'YAML', fence: 'yaml' },
  'json-pretty': { label: 'Pretty JSON', fence: 'json' },
}
const TOKENIZERS = {
  o200k_base: encodeO200k,
  cl100k_base: encodeCl100k,
}
const SIZE_LABELS = {
  small: 'Small',
  medium: 'Medium',
  large: 'Large',
}

function scenario(size, id, description, data) {
  return { size, id, description, data }
}

function makeUsers(count) {
  const names = ['Ada Lovelace', 'Grace Hopper', 'Alan Turing', 'Lin Chen']
  const regions = ['eu-west', 'us-east', 'ap-south']
  return {
    users: Array.from({ length: count }, (_, index) => ({
      id: index + 1,
      name: names[index % names.length],
      email: `user${index + 1}@example.com`,
      active: index % 5 !== 0,
      score: 70 + ((index * 17) % 31),
      region: regions[index % regions.length],
    })),
  }
}

function makeOrders(count) {
  const statuses = ['pending', 'paid', 'shipped', 'delivered']
  return {
    orders: Array.from({ length: count }, (_, index) => {
      const itemCount = (index % 3) + 1
      const items = Array.from({ length: itemCount }, (_, itemIndex) => ({
        sku: `SKU-${(index * 3) + itemIndex + 1}`,
        name: `Product ${(itemIndex % 5) + 1}`,
        quantity: (itemIndex % 3) + 1,
        price: 9.99 + (((index + itemIndex) % 20) * 2.5),
      }))
      const subtotal = Number(
        items.reduce((sum, item) => sum + (item.quantity * item.price), 0).toFixed(2),
      )
      return {
        orderId: `ORD-${String(index + 1).padStart(5, '0')}`,
        customer: {
          id: (index % 97) + 1,
          name: `Customer ${(index % 97) + 1}`,
          tier: index % 8 === 0 ? 'pro' : 'standard',
        },
        items,
        subtotal,
        tax: Number((subtotal * 0.08).toFixed(2)),
        total: Number((subtotal * 1.08).toFixed(2)),
        status: statuses[index % statuses.length],
      }
    }),
  }
}

function makeFlags(count) {
  return {
    flags: Object.fromEntries(
      Array.from({ length: count }, (_, index) => [
        `feature_${String(index + 1).padStart(3, '0')}`,
        {
          enabled: index % 4 !== 0,
          rollout: (index * 7) % 101,
          owner: `team-${(index % 8) + 1}`,
          updatedAt: `2026-07-${String((index % 28) + 1).padStart(2, '0')}`,
        },
      ]),
    ),
  }
}

function makeEvents(count) {
  const levels = ['info', 'info', 'warn', 'error']
  const endpoints = ['/v1/users', '/v1/orders', '/health', '/v1/search']
  return {
    events: Array.from({ length: count }, (_, index) => {
      const event = {
        timestamp: `2026-08-${String((index % 28) + 1).padStart(2, '0')}T${String(index % 24).padStart(2, '0')}:00:00Z`,
        level: levels[index % levels.length],
        endpoint: endpoints[index % endpoints.length],
        statusCode: index % 11 === 0 ? 500 : index % 7 === 0 ? 404 : 200,
        responseTimeMs: 18 + ((index * 13) % 500),
        userId: (index % 250) + 1,
      }
      if (event.statusCode === 500) {
        event.error = {
          message: 'Database connection timed out',
          retryable: index % 2 === 0,
        }
      }
      return event
    }),
  }
}

function makeAnalytics(days) {
  return {
    metrics: Array.from({ length: days }, (_, index) => ({
      date: `2026-${String(Math.floor(index / 28) + 1).padStart(2, '0')}-${String((index % 28) + 1).padStart(2, '0')}`,
      views: 4000 + ((index * 97) % 3500),
      clicks: 180 + ((index * 31) % 420),
      conversions: 12 + ((index * 11) % 70),
      revenue: Number((1200 + ((index * 53.17) % 4200)).toFixed(2)),
      bounceRate: Number((0.31 + ((index % 25) / 100)).toFixed(2)),
    })),
  }
}

function makeConfig() {
  return {
    environment: 'production',
    version: '2.4.1',
    database: {
      host: 'db.internal.example.com',
      port: 5432,
      name: 'application',
      pool: { min: 4, max: 30, idleTimeoutMs: 30000 },
      replicas: [
        { host: 'db-eu-1.internal', port: 5432, priority: 1 },
        { host: 'db-eu-2.internal', port: 5432, priority: 2 },
      ],
    },
    authentication: {
      providers: [
        { name: 'github', clientId: 'client-public-1', scopes: ['read:user', 'user:email'] },
        { name: 'google', clientId: 'client-public-2', scopes: ['openid', 'email', 'profile'] },
      ],
      session: { durationSeconds: 86400, refreshThresholdSeconds: 3600 },
    },
    features: {
      searchV2: { enabled: true, rollout: 100 },
      billingPortal: { enabled: true, rollout: 50 },
      experimentalEditor: { enabled: false, rollout: 5 },
    },
  }
}

const SCENARIOS = [
  scenario('small', 'null-root', 'Null root', null),
  scenario('small', 'short-string-root', 'Short string root', 'Ada'),
  scenario('small', 'empty-containers', 'Empty object and array', { object: {}, array: [] }),
  scenario('small', 'one-field-object', 'Single-field object', { ok: true }),
  scenario('small', 'flat-mixed-object', 'Flat mixed-type object', { id: 7, name: 'Ada', active: true }),
  scenario('small', 'ambiguous-strings', 'Ambiguous strings', {
    code: '01',
    word: 'null',
    date: '2026-08-02',
    url: 'https://lon.dev/docs',
  }),
  scenario('small', 'bare-string-array', 'Safe short-string array', ['red', 'green', 'blue']),
  scenario('small', 'mixed-scalar-array', 'Mixed scalar array', [1, 'Ada', true, null]),
  scenario('small', 'two-uniform-records', 'Two uniform short records', {
    users: [{ id: 1, name: 'Ada' }, { id: 2, name: 'Bob' }],
  }),
  scenario('small', 'three-uniform-records', 'Three uniform short records', {
    users: [{ id: 1, name: 'Ada' }, { id: 2, name: 'Bob' }, { id: 3, name: 'Lin' }],
  }),
  scenario('small', 'nested-object', 'Small nested object', {
    user: { id: 1, name: 'Ada' },
    roles: ['admin', 'editor'],
  }),
  scenario('small', 'heterogeneous-records', 'Non-uniform record array', [{ id: 1 }, { name: 'Ada' }]),
  scenario('small', 'unicode-text', 'Multilingual text and emoji', { message: '안녕하세요 세계', emoji: '🙂', city: 'München' }),
  scenario('small', 'escaped-text', 'Newline and structural characters', {
    note: 'line 1\nline 2',
    path: 'C:\\tmp\\data',
    expression: 'a:b,c',
  }),

  scenario('medium', 'users-10', '10 uniform user records', makeUsers(10)),
  scenario('medium', 'users-50', '50 uniform user records', makeUsers(50)),
  scenario('medium', 'orders-10', '10 nested orders', makeOrders(10)),
  scenario('medium', 'flags-20', '20 keyed configuration entries', makeFlags(20)),
  scenario('medium', 'events-25', '25 semi-uniform event records', makeEvents(25)),
  scenario('medium', 'nested-config', 'Deep application configuration', makeConfig()),
  scenario('medium', 'string-list-100', '100-item string list', {
    tags: Array.from({ length: 100 }, (_, index) => `tag-${index + 1}`),
  }),
  scenario('medium', 'numeric-matrix', '20 x 10 numeric matrix', {
    matrix: Array.from({ length: 20 }, (_, row) =>
      Array.from({ length: 10 }, (_, column) => (row * 10) + column),
    ),
  }),

  scenario('large', 'users-500', '500 uniform user records', makeUsers(500)),
  scenario('large', 'orders-200', '200 nested orders', makeOrders(200)),
  scenario('large', 'flags-500', '500 keyed configuration entries', makeFlags(500)),
  scenario('large', 'events-1000', '1,000 semi-uniform event records', makeEvents(1000)),
  scenario('large', 'analytics-365', '365 days of uniform analytics metrics', makeAnalytics(365)),
]

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: ROOT_DIR,
    encoding: 'utf8',
    maxBuffer: MAX_BUFFER_BYTES,
    ...options,
  })
  if (result.error) {
    throw result.error
  }
  if (result.status !== 0) {
    const detail = (result.stderr || result.stdout || '').trim()
    throw new Error(`${command} ${args.join(' ')} failed (${result.status}): ${detail}`)
  }
  return result
}

function stripTerminalLineBreak(value) {
  return value.replace(/\r?\n$/, '')
}

function runLon(command, input) {
  return stripTerminalLineBreak(
    run(BINARY_PATH, [command], { input }).stdout,
  )
}

function compileLon() {
  run('dart', ['compile', 'exe', 'bin/lon.dart', '-o', BINARY_PATH])
}

function assertEquivalent(expected, actual, scenarioId, formatId) {
  if (!isDeepStrictEqual(actual, expected)) {
    throw new Error(
      `${scenarioId}: ${formatId} round-trip mismatch\nexpected=${JSON.stringify(expected)}\nactual=${JSON.stringify(actual)}`,
    )
  }
}

function serializeAndValidate(entry) {
  const compactJson = JSON.stringify(entry.data)
  const outputs = {
    'json-compact': compactJson,
    'lon': runLon('encode', compactJson),
    'toon': encodeToon(entry.data),
    'yaml': stripTerminalLineBreak(stringifyYaml(entry.data)),
    'json-pretty': JSON.stringify(entry.data, null, 2),
  }

  assertEquivalent(entry.data, JSON.parse(outputs['json-compact']), entry.id, 'json-compact')
  assertEquivalent(entry.data, JSON.parse(outputs['json-pretty']), entry.id, 'json-pretty')
  assertEquivalent(entry.data, JSON.parse(runLon('decode', outputs.lon)), entry.id, 'lon')
  assertEquivalent(entry.data, decodeToon(outputs.toon), entry.id, 'toon')
  assertEquivalent(entry.data, parseYaml(outputs.yaml), entry.id, 'yaml')

  return outputs
}

function measure(text) {
  return {
    bytes: Buffer.byteLength(text, 'utf8'),
    lines: text.length === 0 ? 0 : text.split('\n').length,
    tokens: Object.fromEntries(
      Object.entries(TOKENIZERS).map(([name, tokenize]) => [name, tokenize(text).length]),
    ),
  }
}

function aggregate(entries) {
  return {
    cases: entries.length,
    inputJsonBytes: entries.reduce((sum, entry) => sum + entry.inputJsonBytes, 0),
    formats: Object.fromEntries(
      FORMAT_ORDER.map(formatId => [
        formatId,
        {
          bytes: entries.reduce((sum, entry) => sum + entry.metrics[formatId].bytes, 0),
          lines: entries.reduce((sum, entry) => sum + entry.metrics[formatId].lines, 0),
          tokens: Object.fromEntries(
            Object.keys(TOKENIZERS).map(tokenizer => [
              tokenizer,
              entries.reduce(
                (sum, entry) => sum + entry.metrics[formatId].tokens[tokenizer],
                0,
              ),
            ]),
          ),
        },
      ]),
    ),
  }
}

function signedPercent(value) {
  if (Math.abs(value) < 0.05) {
    return '0.0%'
  }
  return `${value > 0 ? '+' : '−'}${Math.abs(value).toFixed(1)}%`
}

function relativePercent(value, baseline) {
  return ((value - baseline) / baseline) * 100
}

function metricCell(value, baseline) {
  if (value === baseline) {
    return value.toLocaleString('en-US')
  }
  return `${value.toLocaleString('en-US')} (${signedPercent(relativePercent(value, baseline))})`
}

function aggregateTable(summary, tokenizer) {
  const header = `| Scope | Scenarios | ${FORMAT_ORDER.map(formatId => FORMAT_META[formatId].label).join(' | ')} |`
  const divider = '|---|---:|---:|---:|---:|---:|---:|'
  const rows = ['small', 'medium', 'large', 'all'].map((size) => {
    const group = summary[size]
    const baseline = group.formats[COMPARISON_BASELINE].tokens[tokenizer]
    const cells = FORMAT_ORDER.map(formatId =>
      metricCell(group.formats[formatId].tokens[tokenizer], baseline))
    return `| ${size === 'all' ? 'All' : SIZE_LABELS[size]} | ${group.cases} | ${cells.join(' | ')} |`
  })
  return [header, divider, ...rows].join('\n')
}

function scenarioTable(results, tokenizer, size) {
  const header = `| Scenario | JSON bytes | ${FORMAT_ORDER.map(formatId => FORMAT_META[formatId].label).join(' | ')} | Fewest tokens |`
  const divider = '|---|---:|---:|---:|---:|---:|---:|---|'
  const rows = results.filter(result => result.size === size).map((result) => {
    const values = Object.fromEntries(
      FORMAT_ORDER.map(formatId => [formatId, result.metrics[formatId].tokens[tokenizer]]),
    )
    const baseline = values[COMPARISON_BASELINE]
    const minimum = Math.min(...Object.values(values))
    const winners = FORMAT_ORDER
      .filter(formatId => values[formatId] === minimum)
      .map(formatId => FORMAT_META[formatId].label)
      .join(' / ')
    return `| ${result.description} | ${result.inputJsonBytes} | ${FORMAT_ORDER.map(formatId => metricCell(values[formatId], baseline)).join(' | ')} | ${winners} |`
  })
  return [header, divider, ...rows].join('\n')
}

function byteTable(summary) {
  const header = `| Scope | ${FORMAT_ORDER.map(formatId => FORMAT_META[formatId].label).join(' | ')} |`
  const divider = '|---|---:|---:|---:|---:|---:|'
  const rows = ['small', 'medium', 'large', 'all'].map((size) => {
    const group = summary[size]
    const baseline = group.formats[COMPARISON_BASELINE].bytes
    return `| ${size === 'all' ? 'All' : SIZE_LABELS[size]} | ${FORMAT_ORDER.map(formatId => metricCell(group.formats[formatId].bytes, baseline)).join(' | ')} |`
  })
  return [header, divider, ...rows].join('\n')
}

function winCounts(results, tokenizer) {
  const counts = Object.fromEntries(FORMAT_ORDER.map(formatId => [formatId, 0]))
  for (const result of results) {
    const minimum = Math.min(
      ...FORMAT_ORDER.map(formatId => result.metrics[formatId].tokens[tokenizer]),
    )
    for (const formatId of FORMAT_ORDER) {
      if (result.metrics[formatId].tokens[tokenizer] === minimum) {
        counts[formatId] += 1
      }
    }
  }
  return counts
}

function renderExamples(results) {
  const selected = new Set([
    'flat-mixed-object',
    'ambiguous-strings',
    'three-uniform-records',
    'nested-object',
  ])
  return results
    .filter(result => selected.has(result.id))
    .map((result) => {
      const formats = FORMAT_ORDER.map((formatId) => {
        const meta = FORMAT_META[formatId]
        return `**${meta.label}**\n\n\`\`\`${meta.fence}\n${result.outputs[formatId]}\n\`\`\``
      }).join('\n\n')
      return `### ${result.description}\n\n${formats}`
    })
    .join('\n\n')
}

function renderReport(snapshot) {
  const o200kWins = winCounts(snapshot.scenarios, 'o200k_base')
  const cl100kWins = winCounts(snapshot.scenarios, 'cl100k_base')
  const smallScenarios = snapshot.scenarios.filter(result => result.size === 'small')
  const smallWins = winCounts(smallScenarios, 'o200k_base')
  const smallSummary = snapshot.summary.small.formats
  const allSummary = snapshot.summary.all.formats
  const smallLon = smallSummary.lon.tokens.o200k_base
  const allLon = allSummary.lon.tokens.o200k_base
  const winTable = [
    '| Format | o200k_base fewest-token scenarios | cl100k_base fewest-token scenarios |',
    '|---|---:|---:|',
    ...FORMAT_ORDER.map(formatId =>
      `| ${FORMAT_META[formatId].label} | ${o200kWins[formatId]} | ${cl100kWins[formatId]} |`),
  ].join('\n')

  return `# LON Multi-format Token Efficiency Benchmark

## Scope

This report compares the serialized payloads of the same JSON data in LON, TOON, YAML, Compact JSON, and Pretty JSON. Before token counting, every payload is decoded and checked for deep equality with the original JSON value. The benchmark fails instead of reporting a result if any format changes a type or structure.

Token count does not measure model comprehension. This report measures payload cost and includes small examples for human inspection. Code fences, format instructions, prompts, and transport-protocol overhead are excluded.

## Measured highlights

- Small JSON covers ${snapshot.summary.small.cases} scenarios and ${snapshot.summary.small.inputJsonBytes} Compact JSON UTF-8 input bytes: LON ${smallLon} tokens (baseline), Compact JSON ${smallSummary['json-compact'].tokens.o200k_base} (${signedPercent(relativePercent(smallSummary['json-compact'].tokens.o200k_base, smallLon))}), TOON ${smallSummary.toon.tokens.o200k_base} (${signedPercent(relativePercent(smallSummary.toon.tokens.o200k_base, smallLon))}), and YAML ${smallSummary.yaml.tokens.o200k_base} (${signedPercent(relativePercent(smallSummary.yaml.tokens.o200k_base, smallLon))}).
- LON has the lowest token count, including ties, in ${smallWins.lon}/${smallScenarios.length} small scenarios. LON is not guaranteed to win on tiny objects; inspect the per-scenario results.
- Across all ${snapshot.summary.all.cases} scenarios, LON uses ${allLon} tokens (baseline), Compact JSON ${allSummary['json-compact'].tokens.o200k_base} (${signedPercent(relativePercent(allSummary['json-compact'].tokens.o200k_base, allLon))}), TOON ${allSummary.toon.tokens.o200k_base} (${signedPercent(relativePercent(allSummary.toon.tokens.o200k_base, allLon))}), and YAML ${allSummary.yaml.tokens.o200k_base} (${signedPercent(relativePercent(allSummary.yaml.tokens.o200k_base, allLon))}).
- Every number is measured after a semantic round trip; lower is more token-efficient.

## Reproduce

Dart SDK and Bun are required. Dependency versions are fixed by \`benchmark/bun.lock\`.

\`\`\`sh
cd benchmark
bun install --frozen-lockfile
bun run benchmark
\`\`\`

The command overwrites \`benchmark/results/token-efficiency.json\` and \`benchmark/results/token-efficiency.md\`.

## Versions and methodology

- LON ${snapshot.versions.lon}, syntax v1, canonical encoder/decoder from this repository
- TOON ${snapshot.versions.toon}, default encoder/decoder
- YAML ${snapshot.versions.yaml}, default stringify/parse
- Compact JSON: \`JSON.stringify\`
- Pretty JSON: \`JSON.stringify(value, null, 2)\`
- Tokenizer: gpt-tokenizer ${snapshot.versions.tokenizer} with \`o200k_base\` and \`cl100k_base\`
- Runtime: Bun ${snapshot.versions.bun}; ${snapshot.versions.dart}
- Every format has one trailing serializer line break removed; internal payload whitespace is unchanged
- Scenarios are generated deterministically without random, time-based, or network data
- Small scenarios cover primitives, empty containers, single fields, mixed types, ambiguous strings, two- and three-row tables, nesting, non-uniform structures, Unicode, and escapes
- Medium and large scenarios cover uniform records, nested orders, keyed maps, semi-uniform logs, configuration, string lists, numeric matrices, and time-series metrics

## o200k_base summary

Parenthesized values show token change relative to LON. Positive values use more tokens; negative values use fewer.

${aggregateTable(snapshot.summary, 'o200k_base')}

## cl100k_base summary

${aggregateTable(snapshot.summary, 'cl100k_base')}

## Small JSON: per-scenario o200k_base

${scenarioTable(snapshot.scenarios, 'o200k_base', 'small')}

## Medium JSON: per-scenario o200k_base

${scenarioTable(snapshot.scenarios, 'o200k_base', 'medium')}

## Large JSON: per-scenario o200k_base

${scenarioTable(snapshot.scenarios, 'o200k_base', 'large')}

## UTF-8 byte summary

${byteTable(snapshot.summary)}

## Fewest-token scenario counts

Ties count for every winning format, so the total can exceed ${snapshot.scenarios.length}.

${winTable}

## Small payload examples

${renderExamples(snapshot.scenarios)}

## Fairness boundaries

- YAML, TOON, and LON use the default text encoder from each pinned version; the outputs are not hand-optimized examples.
- Pretty JSON is a readability reference, not a different data model.
- CSV is excluded because it only represents flat tables and loses JSON scalar types without an external schema, which fails this benchmark's unambiguous round-trip requirement.
- XML is excluded because it has no unique, schema-free mapping for JSON arrays, nulls, numbers, and booleans. Any custom mapping would add schema overhead or implicit rules to the comparison.
- MessagePack and CBOR are binary formats. Counting language-model tokens after base64 encoding is not equivalent to comparing native text formats.
- Totals directly sum payload tokens across scenarios, so large inputs dominate the All row. Inspect both the per-scenario small table and grouped summaries.
`
}

function extractLonVersion() {
  const pubspec = readFileSync(join(ROOT_DIR, 'pubspec.yaml'), 'utf8')
  const match = /^version:\s*(\S+)\s*$/m.exec(pubspec)
  if (!match) {
    throw new Error('Unable to read LON version from pubspec.yaml')
  }
  return match[1]
}

function dependencyVersions() {
  const packageJson = JSON.parse(readFileSync(join(BENCHMARK_DIR, 'package.json'), 'utf8'))
  const dartVersion = run('dart', ['--version'])
  return {
    lon: extractLonVersion(),
    toon: packageJson.dependencies['@toon-format/toon'],
    yaml: packageJson.dependencies.yaml,
    tokenizer: packageJson.dependencies['gpt-tokenizer'],
    bun: Bun.version,
    dart: `${dartVersion.stdout}${dartVersion.stderr}`.trim(),
  }
}

function main() {
  const ids = new Set()
  for (const entry of SCENARIOS) {
    if (ids.has(entry.id)) {
      throw new Error(`Duplicate scenario id: ${entry.id}`)
    }
    ids.add(entry.id)
  }

  compileLon()
  try {
    const scenarios = SCENARIOS.map((entry) => {
      const outputs = serializeAndValidate(entry)
      return {
        id: entry.id,
        size: entry.size,
        description: entry.description,
        inputJsonBytes: Buffer.byteLength(JSON.stringify(entry.data), 'utf8'),
        metrics: Object.fromEntries(
          FORMAT_ORDER.map(formatId => [formatId, measure(outputs[formatId])]),
        ),
        ...(entry.size === 'small' ? { outputs } : {}),
      }
    })

    const summary = {
      small: aggregate(scenarios.filter(entry => entry.size === 'small')),
      medium: aggregate(scenarios.filter(entry => entry.size === 'medium')),
      large: aggregate(scenarios.filter(entry => entry.size === 'large')),
      all: aggregate(scenarios),
    }
    const snapshot = {
      versions: dependencyVersions(),
      methodology: {
        payloadOnly: true,
        comparisonBaseline: COMPARISON_BASELINE,
        semanticRoundTripRequired: true,
        trailingLineBreakRemoved: true,
        tokenizers: Object.keys(TOKENIZERS),
        formats: FORMAT_ORDER,
      },
      summary,
      scenarios,
    }

    mkdirSync(RESULTS_DIR, { recursive: true })
    writeFileSync(
      join(RESULTS_DIR, 'token-efficiency.json'),
      `${JSON.stringify(snapshot, null, 2)}\n`,
    )
    writeFileSync(
      join(RESULTS_DIR, 'token-efficiency.md'),
      renderReport(snapshot),
    )

    const total = summary.all
    const baseline = total.formats[COMPARISON_BASELINE].tokens.o200k_base
    console.log(`Validated ${scenarios.length} scenarios across ${FORMAT_ORDER.length} formats.`)
    for (const formatId of FORMAT_ORDER) {
      const tokens = total.formats[formatId].tokens.o200k_base
      console.log(
        `${FORMAT_META[formatId].label.padEnd(12)} ${String(tokens).padStart(9)} o200k tokens  ${signedPercent(relativePercent(tokens, baseline))} vs LON`,
      )
    }
    console.log('Wrote benchmark/results/token-efficiency.{json,md}')
  } finally {
    rmSync(BINARY_PATH, { force: true })
  }
}

main()

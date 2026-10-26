// Throughput benchmark for the LON Dart codec.
//
// Pure `Stopwatch` timing, no external dependencies beyond this package.
// Runs both via `dart run tool/bench.dart` (JIT) and `dart compile exe`
// (AOT, the numbers to trust). Prints a markdown table; exit code 0.
//
// Throughput basis per operation:
//   jsonToLon -> JSON input bytes, lonToJson -> LON input bytes,
//   lon.decode -> LON input bytes, lon.encode -> LON output bytes.
import 'dart:io';

import 'package:lon/lon.dart';

const _rounds = 9;
const _warmupRounds = 3;
const _targetRoundMillis = 120;
const _isAot = bool.fromEnvironment('dart.vm.product');

int _sink = 0; // Folded identity hashes keep results observable.

void _consume(Object? value) {
  _sink ^= value == null ? 0 : identityHashCode(value);
}

int _seed = 0x2F6E2B1;

int _next() => _seed = (_seed * 1103515245 + 12345) & 0x7FFFFFFF;

String _smallJson() => '{"id":42,"name":"alice","active":true,"score":9.5,'
    '"tags":["alpha","beta"],"meta":{"lang":"en","count":7},"note":null}';

String _mediumJson() {
  final buffer = StringBuffer('[');
  for (var row = 0; row < 100; row++) {
    if (row != 0) {
      buffer.write(',');
    }
    buffer
      ..write('{"id":$row,"name":"user_${row.toString().padLeft(4, '0')}",')
      ..write('"email":"user${row.toString().padLeft(4, '0')}@example.com",')
      ..write('"active":${row % 2 == 0},')
      ..write('"score":${(row * 7 % 100) / 10},')
      ..write('"city":"city${row % 20}"}');
  }
  return (buffer..write(']')).toString();
}

const _words = [
  'lon',
  'codec',
  'table',
  'record',
  'field',
  'schema',
  'value',
  'token',
  'parser',
  'writer',
  'encode',
  'decode',
  'stream',
  'column',
  'metric',
  'signal',
  'buffer',
  'packet',
  'index',
  'query',
  'vector',
  'matrix',
  'shadow',
  'beacon',
  'lantern',
  'harbor',
  'meadow',
  'summit',
  'canyon',
];

String _sentence() {
  final wordCount = 8 + _next() % 12;
  final buffer = StringBuffer();
  for (var word = 0; word < wordCount; word++) {
    if (word != 0) {
      buffer.write(' ');
    }
    buffer.write(_words[_next() % _words.length]);
  }
  return buffer.toString();
}

String _nested(int depth) {
  if (depth == 0) {
    return '"kind":"leaf","weight":${_next() % 1000},'
        '"label":"node_${_next() % 10000}"';
  }
  final buffer = StringBuffer();
  for (var child = 0; child < 3; child++) {
    if (child != 0) {
      buffer.write(',');
    }
    buffer.write('"child$child":{${_nested(depth - 1)}}');
  }
  return buffer.toString();
}

String _largeJson() {
  final buffer = StringBuffer('[');

  // Uniform record table: 1400 rows x 6 fields.
  buffer.write('[');
  for (var row = 0; row < 1400; row++) {
    if (row != 0) {
      buffer.write(',');
    }
    buffer
      ..write('{"id":$row,"name":"user_${row.toString().padLeft(5, '0')}",')
      ..write('"email":"user${row.toString().padLeft(5, '0')}@example.com",')
      ..write('"active":${row % 3 != 0},')
      ..write('"score":${(row * 13 % 1000) / 10},')
      ..write('"city":"city${row % 50}"}');
  }
  buffer.write('],');

  // Nested object tree.
  buffer.write('{"root":{${_nested(5)}},"depth":6,"tag":"tree"},');

  // String-dense array.
  buffer.write('[');
  for (var index = 0; index < 6500; index++) {
    if (index != 0) {
      buffer.write(',');
    }
    buffer.write('"${_sentence()}"');
  }
  buffer.write('],');

  // Numeric array.
  buffer.write('[');
  for (var index = 0; index < 32000; index++) {
    if (index != 0) {
      buffer.write(',');
    }
    buffer.write(index % 7 == 0 ? '${index / 100}' : '${index * 3 + 1}');
  }
  buffer.write('],');

  // Mixed scalar array.
  buffer.write('[');
  for (var index = 0; index < 6500; index++) {
    if (index != 0) {
      buffer.write(',');
    }
    switch (index % 4) {
      case 0:
        buffer.write('${_next() % 100000}');
      case 1:
        buffer.write('"word_${_next() % 10000}"');
      case 2:
        buffer.write(index % 8 == 1);
      case 3:
        buffer.write('null');
    }
  }
  buffer.write(']');

  return (buffer..write(']')).toString();
}

final class _Case {
  _Case(this.name, this.bytes, this.run);

  final String name;
  final int bytes;
  final Object? Function() run;
}

void _report(_Case case_, List<double> roundMillis, int iterations) {
  final sorted = [...roundMillis]..sort();
  final medianMillis = sorted[sorted.length ~/ 2] / iterations;
  final megaBytesPerSecond = case_.bytes / medianMillis / 1000;
  stdout.writeln(
    '| ${case_.name.padRight(9)} | ${case_.bytes} | '
    '${iterations.toString().padLeft(6)} | '
    '${_formatMicros(medianMillis * 1000).padLeft(10)} | '
    '${megaBytesPerSecond.toStringAsFixed(1).padLeft(7)} |',
  );
}

String _formatMicros(double micros) => micros >= 10000
    ? '${(micros / 1000).toStringAsFixed(1)} ms'
    : micros >= 1000
        ? '${(micros / 1000).toStringAsFixed(2)} ms'
        : '${micros.toStringAsFixed(1)} us';

double _calibrate(_Case case_) {
  for (var index = 0; index < 3; index++) {
    _consume(case_.run());
  }
  final watch = Stopwatch()..start();
  _consume(case_.run());
  return watch.elapsed.inMicroseconds / 1000;
}

void _measure(_Case case_) {
  final singleMillis = _calibrate(case_);
  final iterations = singleMillis <= 0
      ? 1000000
      : (_targetRoundMillis / singleMillis).round().clamp(1, 1000000);

  final roundMillis = <double>[];
  for (var round = 0; round < _warmupRounds + _rounds; round++) {
    final watch = Stopwatch()..start();
    for (var index = 0; index < iterations; index++) {
      _consume(case_.run());
    }
    roundMillis.add(watch.elapsed.inMicroseconds / 1000);
  }
  _report(case_, roundMillis.sublist(_warmupRounds), iterations);
}

void _runPayload(String name, String json) {
  final lonText = jsonToLon(json);
  final value = lon.decode(lonText);
  if (lon.encode(value) != lonText) {
    stderr.writeln('round-trip mismatch on $name payload');
    exitCode = 1;
    return;
  }

  stdout.writeln();
  stdout.writeln('## $name (${json.length} B JSON / ${lonText.length} B LON)');
  stdout.writeln('| operation | basis B | iter/round | median/iter | MB/s |');
  stdout.writeln('|---|---:|---:|---:|---:|');
  _measure(_Case('jsonToLon', json.length, () => jsonToLon(json)));
  _measure(_Case('lonToJson', lonText.length, () => lonToJson(lonText)));
  _measure(_Case('lon.decode', lonText.length, () => lon.decode(lonText)));
  _measure(_Case('lon.encode', lonText.length, () => lon.encode(value)));
}

void main() {
  stdout.writeln('# LON Dart codec benchmark');
  stdout.writeln();
  stdout.writeln(
    '- executable: ${Platform.script.pathSegments.last} '
    '(${Platform.operatingSystem}-aot=$_isAot)',
  );
  stdout.writeln('- rounds: $_rounds (median of), iterations target '
      '$_targetRoundMillis ms/round');

  _runPayload('small', _smallJson());
  _runPayload('medium', _mediumJson());
  _runPayload('large', _largeJson());

  stdout.writeln();
  stdout.writeln('result sink: $_sink');
}

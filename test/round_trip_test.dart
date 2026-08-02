import 'dart:convert';
import 'dart:math';

import 'package:lon/lon.dart';
import 'package:test/test.dart';

void main() {
  group('structural round trips', () {
    test('deterministic generated JSON values round-trip', () {
      final random = Random(20260802);

      for (var index = 0; index < 500; index++) {
        final value = _value(random, 0);
        _expectAllRoundTripInvariants(value, reason: 'case $index');
      }
    });

    test('ordinary arrays and objects honor exact depth boundaries', () {
      const zeroDepth = LonCodec(
        maxDepth: 0,
        maxExpandedElements: 2048,
      );
      _expectAllRoundTripInvariants(const <Object?>[], codec: zeroDepth);
      _expectAllRoundTripInvariants(
        const <String, Object?>{},
        codec: zeroDepth,
      );

      final builders = <String, Object? Function(int)>{
        'array': _nestedArray,
        'object': _nestedObject,
      };
      const depths = <int>[1, 2, 31, 32, 127, 511, 512];

      for (final builder in builders.entries) {
        for (final depth in depths) {
          final value = builder.value(depth);
          final codec = LonCodec(
            maxDepth: depth,
            maxExpandedElements: 2048,
          );
          final encoded = _expectAllRoundTripInvariants(
            value,
            codec: codec,
            reason: '${builder.key} depth $depth',
          );
          final tooShallow = LonCodec(
            maxDepth: depth - 1,
            maxExpandedElements: 2048,
          );
          final json = jsonEncode(value);

          expect(
            () => tooShallow.encode(value),
            throwsArgumentError,
            reason: '${builder.key} Dart depth ${depth - 1}',
          );
          expect(
            () => tooShallow.encodeJson(json),
            throwsFormatException,
            reason: '${builder.key} JSON depth ${depth - 1}',
          );
          expect(
            () => tooShallow.decode(encoded),
            throwsFormatException,
            reason: '${builder.key} LON depth ${depth - 1}',
          );
        }
      }

      expect(() => lon.encode(_nestedArray(513)), throwsArgumentError);
      expect(() => lon.encode(_nestedObject(513)), throwsArgumentError);
      expect(() => jsonToLon(_nestedArrayJson(513)), throwsFormatException);
      expect(() => jsonToLon(_nestedObjectJson(513)), throwsFormatException);
    });

    test('round-trips every ordered dynamic field type transition', () {
      final transitions = <MapEntry<String, Object?>>[
        const MapEntry<String, Object?>('null', null),
        const MapEntry<String, Object?>('boolean', true),
        const MapEntry<String, Object?>('integer', -7),
        const MapEntry<String, Object?>('double', 1.25),
        const MapEntry<String, Object?>('string', 'a b;[]{}'),
        const MapEntry<String, Object?>('empty array', <Object?>[]),
        const MapEntry<String, Object?>('array', <Object?>[1, 'x']),
        const MapEntry<String, Object?>(
          'empty object',
          <String, Object?>{},
        ),
        const MapEntry<String, Object?>(
          'flat object',
          <String, Object?>{'a': 1},
        ),
        const MapEntry<String, Object?>(
          'nested object',
          <String, Object?>{
            'x': <Object?>[
              1,
              <String, Object?>{'y': 'z'},
            ],
          },
        ),
      ];

      for (final left in transitions) {
        for (final right in transitions) {
          final value = <Object?>[
            <String, Object?>{'id': 1, 'payload': left.value},
            <String, Object?>{'id': 2, 'payload': right.value},
          ];
          final reason = '${left.key} -> ${right.key}';
          final encoded = _expectAllRoundTripInvariants(
            value,
            reason: reason,
          );
          final leftIsObject = left.value is Map<Object?, Object?>;
          final rightIsObject = right.value is Map<Object?, Object?>;
          if (leftIsObject != rightIsObject) {
            expect(encoded, startsWith('[{'), reason: reason);
          }
        }
      }
    });

    test('selects tables only for recursively uniform record structures', () {
      final cases = <({String name, Object? value, bool usesTable})>[
        (
          name: 'same keys and order',
          value: <Object?>[
            <String, Object?>{'a': 1, 'b': 2, 'c': 3},
            <String, Object?>{'a': 4, 'b': 5, 'c': 6},
          ],
          usesTable: true,
        ),
        (
          name: 'same keys in different order',
          value: <Object?>[
            <String, Object?>{'a': 1, 'b': 2, 'c': 3},
            <String, Object?>{'b': 4, 'a': 5, 'c': 6},
          ],
          usesTable: false,
        ),
        (
          name: 'missing key',
          value: <Object?>[
            <String, Object?>{'a': 1, 'b': 2, 'c': 3},
            <String, Object?>{'a': 4, 'b': 5},
          ],
          usesTable: false,
        ),
        (
          name: 'extra key',
          value: <Object?>[
            <String, Object?>{'a': 1, 'b': 2},
            <String, Object?>{'a': 3, 'b': 4, 'c': 5},
          ],
          usesTable: false,
        ),
        (
          name: 'recursively uniform nested objects',
          value: <Object?>[
            <String, Object?>{
              'id': 1,
              'meta': <String, Object?>{'x': 2, 'y': 3},
              'ok': true,
            },
            <String, Object?>{
              'id': 4,
              'meta': <String, Object?>{'x': 5, 'y': 6},
              'ok': false,
            },
          ],
          usesTable: true,
        ),
        (
          name: 'different nested keys',
          value: <Object?>[
            <String, Object?>{
              'id': 1,
              'meta': <String, Object?>{'x': 2, 'y': 3},
            },
            <String, Object?>{
              'id': 4,
              'meta': <String, Object?>{'x': 5, 'z': 6},
            },
          ],
          usesTable: false,
        ),
        (
          name: 'different nested key order',
          value: <Object?>[
            <String, Object?>{
              'id': 1,
              'meta': <String, Object?>{'x': 2, 'y': 3},
            },
            <String, Object?>{
              'id': 4,
              'meta': <String, Object?>{'y': 5, 'x': 6},
            },
          ],
          usesTable: false,
        ),
        (
          name: 'empty versus non-empty nested object',
          value: <Object?>[
            <String, Object?>{
              'id': 1,
              'meta': <String, Object?>{},
            },
            <String, Object?>{
              'id': 2,
              'meta': <String, Object?>{'x': 3},
            },
          ],
          usesTable: false,
        ),
        (
          name: 'varying array cells remain table values',
          value: <Object?>[
            <String, Object?>{
              'id': 1,
              'payload': <Object?>[1],
              'kind': 'short',
            },
            <String, Object?>{
              'id': 2,
              'payload': <Object?>[
                <String, Object?>{'x': 3},
              ],
              'kind': 'nested',
            },
          ],
          usesTable: true,
        ),
        (
          name: 'record and non-record mixture',
          value: <Object?>[
            <String, Object?>{'a': 1},
            2,
            <String, Object?>{'a': 3},
          ],
          usesTable: false,
        ),
      ];

      for (final testCase in cases) {
        final encoded = _expectAllRoundTripInvariants(
          testCase.value,
          reason: testCase.name,
        );
        expect(
          _hasTopLevelTable(encoded),
          testCase.usesTable,
          reason: testCase.name,
        );
      }
    });

    test('preserves nested duplicate record keys and order in text APIs', () {
      const uniform = '[{"a":1,"a":2,"meta":{"x":3}},'
          '{"a":4,"a":5,"meta":{"x":6}}]';
      const reordered = '[{"a":1,"a":2,"meta":{"x":3}},'
          '{"a":4,"meta":{"x":6},"a":5}]';

      final uniformLon = _expectJsonTextRoundTrip(uniform);
      final reorderedLon = _expectJsonTextRoundTrip(reordered);

      expect(_hasTopLevelTable(uniformLon), isTrue);
      expect(_hasTopLevelTable(reorderedLon), isFalse);
    });
  });

  group('property and mutation fuzzing', () {
    test(
      '20,000 multi-seed dynamic JSON structures satisfy all invariants',
      () {
        const seedCount = 20;
        const casesPerSeed = 1000;

        for (var seedIndex = 0; seedIndex < seedCount; seedIndex++) {
          final seed = 0x5EED0000 + seedIndex * 7919;
          final random = Random(seed);
          for (var caseIndex = 0; caseIndex < casesPerSeed; caseIndex++) {
            final factory = _GeneratedValueFactory(
              random,
              maxDepth: 4 + random.nextInt(9),
              nodeBudget: 64 + random.nextInt(193),
            );
            final value = factory.next(
              0,
              forceContainer: caseIndex % 4 != 0,
            );
            _expectAllRoundTripInvariants(
              value,
              reason: 'seed $seed case $caseIndex',
            );
          }
        }
      },
      timeout: const Timeout(Duration(minutes: 2)),
    );

    test('deterministic LON mutations reject or canonicalize safely', () {
      final random = Random(0x10F0F00D);
      final corpus = <String>{
        lon.encode(<String, Object?>{
          'root': <Object?>[
            1,
            <String, Object?>{
              'a': 'b',
              'nested': <Object?>[true, null]
            },
          ],
        }),
        lon.encode(<Object?>[
          <String, Object?>{'id': 1, 'name': 'Ada', 'active': true},
          <String, Object?>{'id': 2, 'name': 'Bob', 'active': false},
        ]),
      };

      for (var index = 0; index < 128; index++) {
        final factory = _GeneratedValueFactory(
          random,
          maxDepth: 6,
          nodeBudget: 96,
        );
        corpus.add(lon.encode(factory.next(0, forceContainer: true)));
      }

      var accepted = 0;
      var rejected = 0;
      var mutations = 0;
      for (final source in corpus) {
        for (final mutation in _mutations(source, random)) {
          mutations++;
          if (_acceptsCanonically(mutation)) {
            accepted++;
          } else {
            rejected++;
          }
        }
      }

      expect(mutations, greaterThan(1000));
      expect(accepted, greaterThan(0));
      expect(rejected, greaterThan(0));
    });

    test('rejects every truncation of complex root containers', () {
      final sources = <String>[
        lon.encode(<String, Object?>{
          'users': <Object?>[
            <String, Object?>{
              'id': 1,
              'payload': <String, Object?>{
                'items': <Object?>[1, true, null, 'x'],
              },
            },
            <String, Object?>{
              'id': 2,
              'payload': <String, Object?>{
                'items': <Object?>[2, false, null, 'y'],
              },
            },
          ],
        }),
        lon.encode(<Object?>[
          1,
          <String, Object?>{
            'a': <Object?>[2, 3]
          },
          <Object?>[
            4,
            <String, Object?>{'b': 5}
          ],
        ]),
      ];

      for (final source in sources) {
        for (var end = 0; end < source.length; end++) {
          expect(
            () => lon.decode(source.substring(0, end)),
            throwsFormatException,
            reason: 'end $end of $source',
          );
        }
      }
    });
  });
}

String _expectAllRoundTripInvariants(
  Object? value, {
  LonCodec codec = lon,
  String? reason,
}) {
  final sourceJson = jsonEncode(value);
  final dartEncoded = codec.encode(value);
  expect(
    jsonEncode(codec.decode(dartEncoded)),
    sourceJson,
    reason: _invariantReason(reason, 'Dart value round-trip'),
  );
  final textEncoded = codec.encodeJson(sourceJson);
  expect(
    textEncoded,
    dartEncoded,
    reason: _invariantReason(reason, 'Dart and JSON encoders agree'),
  );

  final decodedJson = codec.decodeJson(textEncoded);
  expect(
    jsonEncode(jsonDecode(decodedJson)),
    jsonEncode(jsonDecode(sourceJson)),
    reason: _invariantReason(reason, 'JSON semantic round-trip'),
  );
  expect(
    codec.encodeJson(decodedJson),
    textEncoded,
    reason: _invariantReason(reason, 'canonical LON is idempotent'),
  );
  return textEncoded;
}

String _expectJsonTextRoundTrip(String source) {
  final encoded = jsonToLon(source);
  final decoded = lonToJson(encoded);

  expect(decoded, source);
  expect(jsonToLon(decoded), encoded);
  return encoded;
}

String _invariantReason(String? reason, String invariant) =>
    reason == null ? invariant : '$reason: $invariant';

Object? _nestedArray(int depth) {
  Object? value = 0;
  for (var index = 0; index < depth; index++) {
    value = <Object?>[value];
  }
  return value;
}

Object? _nestedObject(int depth) {
  Object? value = 0;
  for (var index = 0; index < depth; index++) {
    value = <String, Object?>{'value': value};
  }
  return value;
}

String _nestedArrayJson(int depth) =>
    '${List<String>.filled(depth, '[').join()}0'
    '${List<String>.filled(depth, ']').join()}';

String _nestedObjectJson(int depth) =>
    '${List<String>.filled(depth, '{"value":').join()}0'
    '${List<String>.filled(depth, '}').join()}';

bool _hasTopLevelTable(String source) {
  if (!source.startsWith('[')) {
    return false;
  }

  var nestedDepth = 0;
  var quoted = false;
  var escaped = false;
  for (var index = 1; index < source.length; index++) {
    final code = source.codeUnitAt(index);
    if (quoted) {
      if (escaped) {
        escaped = false;
      } else if (code == 0x5C) {
        escaped = true;
      } else if (code == 0x22) {
        quoted = false;
      }
      continue;
    }

    if (code == 0x22) {
      quoted = true;
    } else if (code == 0x5B || code == 0x7B) {
      nestedDepth++;
    } else if (code == 0x5D || code == 0x7D) {
      if (nestedDepth == 0) {
        return false;
      }
      nestedDepth--;
    } else if (code == 0x3B && nestedDepth == 0) {
      return true;
    }
  }
  return false;
}

final class _GeneratedValueFactory {
  _GeneratedValueFactory(
    this.random, {
    required this.maxDepth,
    required int nodeBudget,
  }) : _remainingNodes = nodeBudget;

  final Random random;
  final int maxDepth;
  int _remainingNodes;

  Object? next(int depth, {bool forceContainer = false}) {
    if (_remainingNodes <= 0 || depth >= maxDepth) {
      return _scalar();
    }
    _remainingNodes--;

    final kind = forceContainer ? 5 + random.nextInt(5) : random.nextInt(10);
    return switch (kind) {
      0 || 1 || 2 || 3 || 4 => _scalar(),
      5 => <Object?>[
          for (var index = 0; index < random.nextInt(5); index++)
            next(depth + 1),
        ],
      6 => <String, Object?>{
          for (var index = 0; index < random.nextInt(5); index++)
            '${_keys[random.nextInt(_keys.length)]}.$depth.$index':
                next(depth + 1),
        },
      7 => _sameKeyRecords(depth),
      8 => _varyingKeyRecords(depth),
      _ => random.nextBool()
          ? <Object?>[next(depth + 1, forceContainer: true)]
          : <String, Object?>{
              'value': next(depth + 1, forceContainer: true),
            },
    };
  }

  Object? _scalar() => switch (random.nextInt(6)) {
        0 => null,
        1 => random.nextBool(),
        2 => random.nextInt(200000) - 100000,
        3 => (random.nextInt(200000) - 100000) / 16,
        4 => random.nextBool() ? -0.0 : 0.0,
        _ => _strings[random.nextInt(_strings.length)],
      };

  List<Object?> _sameKeyRecords(int depth) {
    final length = 2 + random.nextInt(4);
    return <Object?>[
      for (var row = 0; row < length; row++)
        <String, Object?>{
          'id': row,
          'payload': next(depth + 2),
          'meta': <String, Object?>{
            'active': random.nextBool(),
            'value': next(depth + 3),
          },
        },
    ];
  }

  List<Object?> _varyingKeyRecords(int depth) {
    final length = 2 + random.nextInt(4);
    return <Object?>[
      for (var row = 0; row < length; row++)
        <String, Object?>{
          'id': row,
          if (row.isEven) 'left': next(depth + 2),
          if (row.isOdd) 'right': next(depth + 2),
          if (row % 3 == 0) 'extra': next(depth + 2),
        },
    ];
  }
}

Iterable<String> _mutations(String source, Random random) sync* {
  const atoms = <String>[
    ' ',
    ',',
    ';',
    ':',
    '[',
    ']',
    '{',
    '}',
    '"',
    '\\',
    '0',
    'n',
    '\u0000',
    '\u202E',
  ];

  for (var round = 0; round < 12; round++) {
    final atom = atoms[random.nextInt(atoms.length)];
    final insertion = random.nextInt(source.length + 1);
    yield source.substring(0, insertion) + atom + source.substring(insertion);

    if (source.isNotEmpty) {
      final deletion = random.nextInt(source.length);
      yield source.substring(0, deletion) + source.substring(deletion + 1);

      final replacement = random.nextInt(source.length);
      yield source.substring(0, replacement) +
          atom +
          source.substring(replacement + 1);
    }
  }
}

bool _acceptsCanonically(String source) {
  late final String decodedJson;
  try {
    decodedJson = lonToJson(source);
  } on FormatException {
    return false;
  }

  final canonical = jsonToLon(decodedJson);
  final reparsedJson = lonToJson(canonical);
  if (reparsedJson != decodedJson || jsonToLon(reparsedJson) != canonical) {
    fail('Accepted mutation did not canonicalize stably: $source');
  }
  return true;
}

Object? _value(Random random, int depth) {
  final scalarOnly = depth >= 4;
  final kind = random.nextInt(scalarOnly ? 5 : 8);
  return switch (kind) {
    0 => null,
    1 => random.nextBool(),
    2 => random.nextInt(200000) - 100000,
    3 => (random.nextDouble() * 2000 - 1000) / 7,
    4 => _strings[random.nextInt(_strings.length)],
    5 => <Object?>[
        for (var index = 0; index < random.nextInt(5); index++)
          _value(random, depth + 1),
      ],
    6 => <String, Object?>{
        for (var index = 0; index < random.nextInt(5); index++)
          '${_keys[random.nextInt(_keys.length)]}_$index':
              _value(random, depth + 1),
      },
    _ => _uniformRecords(random, depth + 1),
  };
}

List<Object?> _uniformRecords(Random random, int depth) {
  final length = random.nextInt(4) + 2;
  return <Object?>[
    for (var index = 0; index < length; index++)
      <String, Object?>{
        'id': index,
        'name': _strings[random.nextInt(_strings.length)],
        'meta': <String, Object?>{
          'active': random.nextBool(),
          'score': random.nextInt(100),
        },
        'tags': <Object?>[
          if (random.nextBool()) _strings[random.nextInt(_strings.length)],
        ],
      },
  ];
}

const _strings = <String>[
  '',
  'Ada',
  'a b',
  'null',
  'true',
  '01',
  '+1',
  'table',
  'https://example.com/a',
  'line\nbreak',
  'quote"slash\\',
  '\u4F60\u597D',
  '😀',
  '[brackets]',
  'NaN',
  'undefined',
  'None',
  'yes',
  'off',
  '.5',
  '1.',
  '1e',
  '0x10',
  '1_000',
  'a\u0000b',
  'a\u0085b',
  'line\u2028break',
  'zero\u200Bwidth',
];

const _keys = <String>[
  'plain',
  'first name',
  'null',
  '01',
  'a:b',
  'table',
  '\u952E',
];

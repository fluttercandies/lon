import 'dart:convert';
import 'dart:math';

import 'package:lon/lon.dart';
import 'package:test/test.dart';

void main() {
  test('deterministic generated JSON values round-trip', () {
    final random = Random(20260802);

    for (var index = 0; index < 500; index++) {
      final value = _value(random, 0);
      final json = jsonEncode(value);
      final encoded = jsonToLon(json);
      final decodedJson = lonToJson(encoded);

      expect(jsonDecode(decodedJson), jsonDecode(json), reason: 'case $index');
      expect(jsonToLon(decodedJson), encoded, reason: 'case $index canonical');
    }
  });
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

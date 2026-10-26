import 'dart:convert';

import 'package:lon/lon.dart';
import 'package:test/test.dart';

void main() {
  const pretty = LonCodec(pretty: true);

  group('pretty edge cases', () {
    test('empty containers stay inline at deep nesting', () {
      expect(
        pretty.encodeJson('{"o":{"e":{},"a":[]},"m":{"x":1}}'),
        '{\n'
        '  o: {\n'
        '    e: {}\n'
        '    a: []\n'
        '  }\n'
        '  m: {\n'
        '    x: 1\n'
        '  }\n'
        '}',
      );
      expect(pretty.encodeJson('{"e":{},"l":[]}'), '{\n  e: {}\n  l: []\n}');
      expect(pretty.encodeJson('[[{}]]'), '[\n  [\n    {}\n  ]\n]');
      expect(
        pretty.encodeJson('{"a":[[],[{}]]}'),
        '{\n'
        '  a: [\n'
        '    [],\n'
        '    [\n'
        '      {}\n'
        '    ]\n'
        '  ]\n'
        '}',
      );
    });

    test('pure scalar arrays stay on one line in multiline contexts', () {
      expect(
        pretty.encodeJson('{"row":[1,2,3],"next":[{"a":1}]}'),
        '{\n'
        '  row: [1,2,3]\n'
        '  next: [\n'
        '    {\n'
        '      a: 1\n'
        '    }\n'
        '  ]\n'
        '}',
      );
      expect(
        pretty.encodeJson('{"tags":["x","y"],"nums":[0.5,-1,2e3]}'),
        '{\n'
        '  tags: [x,"y"]\n'
        '  nums: [0.5,-1,2e3]\n'
        '}',
      );
    });

    test('nested table inside a multiline array keeps cells on one row', () {
      expect(
        pretty.encodeJson(
          '{"outer":{"rows":[[{"s":"x","q":1},{"s":"y","q":2}],{"k":"v"}],"z":1}}',
        ),
        '{\n'
        '  outer: {\n'
        '    rows: [\n'
        '      [\n'
        '        s q;\n'
        '        x 1;\n'
        '        "y" 2\n'
        '      ],\n'
        '      {\n'
        '        k: v\n'
        '      }\n'
        '    ]\n'
        '    z: 1\n'
        '  }\n'
        '}',
      );
    });

    test('table cells holding nested tables render inline', () {
      expect(
        pretty.encodeJson(
          '{"o":[{"id":1,"items":[{"s":"x","q":2},{"s":"y","q":1},{"s":"z","q":3}]},'
          '{"id":2,"items":[{"s":"w","q":4},{"s":"v","q":5},{"s":"u","q":6}]}]}',
        ),
        '{\n'
        '  o: [\n'
        '    id items;\n'
        '    1 [s q;x 2;"y" 1;z 3];\n'
        '    2 [s q;w 4;v 5;u 6]\n'
        '  ]\n'
        '}',
      );
    });

    test('object inside a table cell renders inline', () {
      expect(
        pretty.encodeJson(
          '[{"id":1,"payload":{"a":{"b":2}}},{"id":2,"payload":{"a":{"b":3}}}]',
        ),
        '[\n'
        '  id payload{a{b}};\n'
        '  1 2;\n'
        '  2 3\n'
        ']',
      );
    });

    test('pretty decodeJson emits two-space indented JSON at every depth', () {
      expect(pretty.decodeJson('[]'), '[]');
      expect(pretty.decodeJson('{}'), '{}');
      expect(pretty.decodeJson('null'), 'null');
      expect(pretty.decodeJson('"a"'), '"a"');
      expect(
        pretty.decodeJson('[a b;1 2;3 4]'),
        '[\n'
        '  {\n'
        '    "a": 1,\n'
        '    "b": 2\n'
        '  },\n'
        '  {\n'
        '    "a": 3,\n'
        '    "b": 4\n'
        '  }\n'
        ']',
      );
      expect(
        pretty.decodeJson('{a:[1 2] b:{}}'),
        '{\n'
        '  "a": [\n'
        '    1,\n'
        '    2\n'
        '  ],\n'
        '  "b": {}\n'
        '}',
      );
      expect(
        pretty.decodeJson('[[a b;1 2;3 4]]'),
        '[\n'
        '  [\n'
        '    {\n'
        '      "a": 1,\n'
        '      "b": 2\n'
        '    },\n'
        '    {\n'
        '      "a": 3,\n'
        '      "b": 4\n'
        '    }\n'
        '  ]\n'
        ']',
      );
    });

    test('pretty output decodes to the same values as compact output', () {
      const source =
          '{"outer":{"rows":[[{"s":"x","q":1},{"s":"y","q":2}],{"k":"v"}],"z":1},'
          '"empty":{"o":{},"a":[]},"scalars":[1,2.5,"x"],"t":[{"a":1,"b":{"c":2}},'
          '{"a":3,"b":{"c":4}}]}';
      final compactLon = lon.encodeJson(source);
      final prettyLon = pretty.encodeJson(source);

      expect(jsonEncode(lon.decode(prettyLon)),
          jsonEncode(lon.decode(compactLon)));
      expect(jsonEncode(lon.decode(prettyLon)), jsonEncode(jsonDecode(source)));
      // pretty re-encoded by the compact codec is byte-identical canonical LON.
      expect(lon.encodeJson(lonToJson(prettyLon)), compactLon);
      // and the pretty codec re-encodes to the identical pretty text (idempotent).
      expect(pretty.encodeJson(lonToJson(prettyLon)), prettyLon);
    });

    test('LON whitespace members parse identically to a plain space', () {
      const whitespace = <int>[
        0x09,
        0x0A,
        0x0B,
        0x0C,
        0x0D,
        0x20,
        0x85,
        0xA0,
        0x1680,
        0x2000,
        0x2001,
        0x2002,
        0x2003,
        0x2004,
        0x2005,
        0x2006,
        0x2007,
        0x2008,
        0x2009,
        0x200A,
        0x2028,
        0x2029,
        0x202F,
        0x205F,
        0x3000,
      ];
      const expected = '{"a":1,"b":[true,{"c":null}],"d":{"e":2,"f":3}}';
      for (final code in whitespace) {
        final ws = String.fromCharCode(code);
        final source =
            '{a:${ws}1${ws}b:[true,$ws{c:null}]${ws}d:{e:${ws}2${ws}f:3}}';
        expect(
          lonToJson(source),
          expected,
          reason: 'whitespace U+${code.toRadixString(16).padLeft(4, '0')}',
        );
      }

      // A multiline pretty document re-parses to the same value with any
      // mix of TAB, CRLF, and Unicode whitespace substituted for spaces.
      const mixedPretty =
          '{\n\touter:\u00A0{\r\n  rows:\u2028[\n    a\u3000b;\n    1\u202F2;\n    3\u20074\n  ]\n}\n}';
      expect(lon.decode(mixedPretty), <String, Object?>{
        'outer': <String, Object?>{
          'rows': <Object?>[
            <String, Object?>{'a': 1, 'b': 2},
            <String, Object?>{'a': 3, 'b': 4},
          ],
        },
      });
    });
  });
}

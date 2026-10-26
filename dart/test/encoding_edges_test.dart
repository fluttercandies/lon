import 'dart:convert';

import 'package:lon/lon.dart';
import 'package:test/test.dart';

void main() {
  group('table selection thresholds', () {
    test('exactly two rows and two fields still select a table', () {
      // The byte-cost gate compares table bytes against the ordinary form;
      // for scalar-valued 2x2 records the table form always wins.
      expect(jsonToLon('[{"a":1,"b":2},{"a":3,"b":4}]'), '[a b;1 2;3 4]');
      // Long keys and values keep the 2x2 shape a table.
      const wide = '[{"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa":1,'
          '"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb":2},'
          '{"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa":3,'
          '"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb":4}]';
      expect(
        jsonToLon(wide),
        '[aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa '
        'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb;1 2;3 4]',
      );
      // Multi-byte UTF-8 keys compare on UTF-8 cost, still a table.
      expect(
        jsonToLon('[{"简":1,"体":2},{"简":3,"体":4}]'),
        '[简 体;1 2;3 4]',
      );
      // Long string values do not change the structural decision.
      final heavy = jsonEncode(<Object?>[
        <String, Object?>{'a': 'x' * 60, 'b': 'y' * 60},
        <String, Object?>{'a': 'z' * 60, 'b': 'w' * 60},
      ]);
      expect(jsonToLon(heavy), startsWith('[a b;'));
    });

    test('three rows or three fields always select a table', () {
      expect(
        jsonToLon('[{"a":1,"b":2},{"a":3,"b":4},{"a":5,"b":6}]'),
        '[a b;1 2;3 4;5 6]',
      );
      expect(
        jsonToLon('[{"a":1,"b":2,"c":3},{"a":4,"b":5,"c":6}]'),
        '[a b c;1 2 3;4 5 6]',
      );
      // Nested schema fields count toward the 3-field threshold:
      // id + m{x y} is fieldCount 3.
      expect(
        jsonToLon(
          '[{"id":1,"m":{"x":2,"y":3}},{"id":4,"m":{"x":5,"y":6}}]',
        ),
        '[id m{x y};1 2 3;4 5 6]',
      );
      // Deeper nesting contributes recursively: a{x{p q}} b is 4 fields.
      expect(
        jsonToLon(
          '[{"a":{"x":{"p":1,"q":2}},"b":3},'
          '{"a":{"x":{"p":4,"q":5}},"b":6}]',
        ),
        '[a{x{p q}} b;1 2 3;4 5 6]',
      );
    });

    test('single-field or single-leaf schemas never select a table', () {
      // One member per record: valueCount < 2 keeps objects explicit.
      expect(jsonToLon('[{"a":1},{"a":2}]'), '[{a:1},{a:2}]');
      expect(jsonToLon('[{"a":1},{"a":2},{"a":3}]'), '[{a:1},{a:2},{a:3}]');
      // A single nested-object member still totals one leaf value.
      expect(
        jsonToLon('[{"a":{"b":1}},{"a":{"b":2}}]'),
        '[{a:{b:1}},{a:{b:2}}]',
      );
      expect(
        jsonToLon('[{"a":{"b":{"c":1}}},{"a":{"b":{"c":2}}}]'),
        '[{a:{b:{c:1}}},{a:{b:{c:2}}}]',
      );
      // One leaf field plus an empty nested object also stays explicit.
      expect(
        jsonToLon('[{"a":{},"c":2},{"a":{},"c":4}]'),
        '[{a:{} c:2},{a:{} c:4}]',
      );
    });

    test('non-uniform member sets never select a table', () {
      // Mixed object/non-object members.
      expect(
        jsonToLon('[{"a":1,"b":2},3,{"a":4,"b":5}]'),
        '[{a:1 b:2},3,{a:4 b:5}]',
      );
      expect(
        jsonToLon('[{"a":1,"b":2},[1],{"a":3,"b":4}]'),
        '[{a:1 b:2},[1],{a:3 b:4}]',
      );
      // Varying nested object shapes.
      expect(
        jsonToLon('[{"a":1,"m":{"x":1}},{"a":2,"m":{"x":1,"y":2}}]'),
        '[{a:1 m:{x:1}},{a:2 m:{x:1 y:2}}]',
      );
      expect(
        jsonToLon('[{"a":1,"m":{"x":1,"y":2}},{"a":2,"m":{"x":1,"z":2}}]'),
        '[{a:1 m:{x:1 y:2}},{a:2 m:{x:1 z:2}}]',
      );
      // Same nested keys, different order.
      expect(
        jsonToLon('[{"a":1,"m":{"x":1,"y":2}},{"a":2,"m":{"y":1,"x":2}}]'),
        '[{a:1 m:{x:1 y:2}},{a:2 m:{y:1 x:2}}]',
      );
      // Nested member present in one row only.
      expect(
        jsonToLon('[{"a":1,"m":{}},{"a":2,"m":{"x":1}}]'),
        '[{a:1 m:{}},{a:2 m:{x:1}}]',
      );
      // Nested object vs non-object in the same member position.
      expect(
        jsonToLon('[{"a":{"x":1},"b":2},{"a":7,"b":8}]'),
        '[{a:{x:1} b:2},{a:7 b:8}]',
      );
      // Single-record array is never a table.
      expect(jsonToLon('[{"a":1,"b":2}]'), '[{a:1 b:2}]');
      // A record array nested inside another container stays a table.
      expect(
        jsonToLon('{"r":[{"a":1,"b":2},{"a":3,"b":4}]}'),
        '{r:[a b;1 2;3 4]}',
      );
    });

    test('useTables: false keeps record arrays ordinary but still decodes', () {
      const explicit = LonCodec(useTables: false);
      const records = '[{"a":1,"b":2},{"a":3,"b":4}]';
      expect(explicit.encodeJson(records), '[{a:1 b:2},{a:3 b:4}]');
      expect(
        explicit.encode(const [
          {'a': 1, 'b': 2},
          {'a': 3, 'b': 4},
        ]),
        '[{a:1 b:2},{a:3 b:4}]',
      );
      // The flag gates encoding only; table syntax still decodes.
      expect(
        explicit.decodeJson('[a b;1 2;3 4]'),
        '[{"a":1,"b":2},{"a":3,"b":4}]',
      );
      // Non-table encodings are unchanged.
      expect(explicit.encodeJson('["a","b"]'), '[a b]');
    });
  });

  group('text API losslessness', () {
    test('preserves repeated members three or more times in order', () {
      const source = '{"a":1,"b":2,"a":3,"a":4,"c":5,"a":6}';
      final lon = jsonToLon(source);
      expect(lon, '{a:1 b:2 a:3 a:4 c:5 a:6}');
      expect(lonToJson(lon), source);
      expect(jsonToLon(lonToJson(lon)), lon);
      // The value API collapses duplicates with the last value winning.
      expect(
        lonToJson('{a:1 a:2 a:3}'),
        '{"a":1,"a":2,"a":3}',
      );
      expect(
        const LonCodec().decode('{a:1 b:2 a:3}'),
        <String, Object?>{'a': 3, 'b': 2},
      );
    });

    test('preserves extreme number lexemes verbatim', () {
      const source = '[{"a":-0,"b":0.0e0,"c":1e+09,"d":-0.0,"e":1E-3},'
          '{"a":1e+09,"b":-0,"c":0.0e0,"d":-0.0,"e":1E-3}]';
      final lon = jsonToLon(source);
      expect(
          lon, '[a b c d e;-0 0.0e0 1e+09 -0.0 1E-3;1e+09 -0 0.0e0 -0.0 1E-3]');
      expect(lonToJson(lon), source);
    });

    test('preserves a huge integer lexeme through the text pipeline', () {
      const source = '{"n":123456789012345678901234567890}';
      final lon = jsonToLon(source);
      expect(lon, '{n:123456789012345678901234567890}');
      expect(lonToJson(lon), source);
      expect(jsonToLon(lonToJson(lon)), lon);
      // The Dart value API degrades gracefully to double.
      expect(
        const LonCodec().decode(lon),
        <String, Object?>{'n': 1.2345678901234568e29},
      );
    });

    test('preserves empty member names', () {
      const source = '{"":1,"a":{"":2}}';
      final lon = jsonToLon(source);
      expect(lon, '{"":1 a:{"":2}}');
      expect(lonToJson(lon), source);
      expect(jsonToLon(lonToJson(lon)), lon);
      expect(
        const LonCodec().decode(lon),
        <String, Object?>{
          '': 1,
          'a': <String, Object?>{'': 2},
        },
      );
    });

    test('preserves escaped and surrogate-pair member names', () {
      const source = r'{"\ud800":1,"a\u0000b":2,"😀":3}';
      final lon = jsonToLon(source);
      expect(lon, '{"\\ud800":1 "a\\u0000b":2 😀:3}');
      expect(lonToJson(lon), source);
      expect(jsonToLon(lonToJson(lon)), lon);
    });

    test('interleaves escaped strings across nested object-in-array members',
        () {
      const source =
          r'[{"a":"x\ty","b":{"c":"u\nv"},"d":[1,{"e":"q\"r"}],"f":2},'
          r'{"a":"x\ty","b":{"c":"u\nv"},"d":[2,{"e":"q\"r"}],"f":3}]';
      final lon = jsonToLon(source);
      expect(
        lon,
        r'[a b{c} d f;"x\ty" "u\nv" [1,{e:"q\"r"}] 2;"x\ty" "u\nv" [2,{e:"q\"r"}] 3]',
      );
      expect(lonToJson(lon), source);
      expect(jsonToLon(lonToJson(lon)), lon);
    });
  });

  group('ambiguity and quoting rules', () {
    test('quotes every JSON-adjacent literal as a string value', () {
      const tokens = <String>[
        'NaN',
        'nan',
        'Infinity',
        'inf',
        '-Infinity',
        '+Infinity',
        '-inf',
        '.NaN',
        '+.inf',
        'undefined',
        'null',
        'NULL',
        'true',
        'TRUE',
        'false',
        'False',
        'None',
        'none',
        'nil',
        'yes',
        'YES',
        'no',
        'NO',
        'on',
        'ON',
        'off',
        'OFF',
        'y',
        'Y',
        'n',
        'N',
        '~',
      ];
      for (final token in tokens) {
        expect(
          jsonToLon(jsonEncode(token)),
          jsonEncode(token),
          reason: token,
        );
      }
      // Single-letter JSON-adjacent members not in the reserved set stay bare.
      expect(jsonToLon('"T"'), 'T');
      expect(jsonToLon('"F"'), 'F');
      expect(jsonToLon('"x"'), 'x');
    });

    test('applies the apostrophe rule to string values', () {
      // An apostrophe in the middle of a token stays bare.
      expect(jsonToLon('"don\'t"'), "don't");
      expect(jsonToLon('"O\'Reilly"'), "O'Reilly");
      // Leading or trailing apostrophes force quoting.
      expect(jsonToLon('"\'abc"'), '"\'abc"');
      expect(jsonToLon('"abc\'"'), '"abc\'"');
      expect(jsonToLon('"\'abc\'"'), '"\'abc\'"');
      expect(jsonToLon('"\'"'), '"\'"');
      // Round-trip through the bare-token side.
      expect(lonToJson("don't"), '"don\'t"');
      expect(() => lonToJson("'abc"), throwsFormatException);
      expect(() => lonToJson("abc'"), throwsFormatException);
    });

    test('quotes strings containing structural punctuation', () {
      for (final punctuation in <String>[
        '(',
        ')',
        '[',
        ']',
        '{',
        '}',
        ',',
        ':',
        ';',
        '|',
        '"',
        r'\',
      ]) {
        final value = 'a${punctuation}b';
        expect(
          jsonToLon(jsonEncode(value)),
          jsonEncode(value),
          reason: 'token $value',
        );
      }
      // An entire token of punctuation also quotes.
      expect(jsonToLon('","'), '","');
      expect(jsonToLon('"()"'), '"()"');
      expect(jsonToLon('"a:b;c,d|e[f]g{h}(i)"'), '"a:b;c,d|e[f]g{h}(i)"');
    });

    test('quotes numeric-looking strings that JSON would misread', () {
      const tokens = <String>[
        '1e',
        '1e+',
        '1.',
        '.5',
        '._',
        '0x10',
        '0X10',
        '0b101',
        '0o77',
        '+1',
        '-01',
        '01',
        '1_000',
        '1.e3',
      ];
      for (final token in tokens) {
        expect(
          jsonToLon(jsonEncode(token)),
          jsonEncode(token),
          reason: token,
        );
      }
      // Bare forms of those tokens must be rejected at parse time.
      for (final token in <String>['1e', '.5', '0x10', '+1', '01', '1_000']) {
        expect(
          () => lonToJson('[$token]'),
          throwsFormatException,
          reason: 'bare $token',
        );
      }
      // JSON number spellings still encode as numbers.
      for (final lexeme in <String>[
        '0',
        '-0',
        '1',
        '-12.5e+2',
        '1e+02',
        '10'
      ]) {
        expect(jsonToLon(lexeme), lexeme, reason: lexeme);
      }
      // Non-numeric punctuation-free tokens stay bare.
      for (final token in <String>[
        '1-2',
        'e',
        '0x',
        'Infinityx',
        '1x',
        'x1',
        'a1e',
        '1_',
        '1_.5',
        '1__2',
      ]) {
        expect(jsonToLon(jsonEncode(token)), token, reason: token);
      }
    });
  });
}

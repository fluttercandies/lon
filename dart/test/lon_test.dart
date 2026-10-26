import 'dart:convert';
import 'dart:io';

import 'package:lon/lon.dart';
import 'package:test/test.dart';

void main() {
  final cases = (jsonDecode(File('../conformance/v1.json').readAsStringSync())
          as List<Object?>)
      .cast<Map<String, Object?>>();

  group('v1 conformance', () {
    for (final testCase in cases) {
      final name = testCase['name']! as String;
      final json = testCase['json']! as String;
      final encoded = testCase['lon']! as String;
      final canonicalJson = testCase['canonicalJson']! as String;

      test(name, () {
        expect(jsonToLon(json), encoded);
        expect(lonToJson(encoded), canonicalJson);
        expect(jsonToLon(lonToJson(encoded)), encoded);
      });
    }
  });

  group('Dart value API', () {
    test('encodes and decodes JSON-compatible values', () {
      final value = <String, Object?>{
        'users': <Object?>[
          <String, Object?>{'id': 1, 'name': 'Ada'},
          <String, Object?>{'id': 2, 'name': 'Bob'},
        ],
      };

      final encoded = lon.encode(value);

      expect(encoded, '{users:[id name;1 Ada;2 Bob]}');
      expect(lon.decode(encoded), value);
    });

    test('rejects non-finite numbers', () {
      expect(() => lon.encode(double.nan), throwsArgumentError);
      expect(() => lon.encode(double.infinity), throwsArgumentError);
    });

    test('rejects cyclic values', () {
      final value = <Object?>[];
      value.add(value);

      expect(() => lon.encode(value), throwsArgumentError);
    });

    test('bounds expanded Dart object graphs', () {
      Object? value = 0;
      for (var depth = 0; depth < 20; depth++) {
        final child = value;
        value = <Object?>[child, child];
      }

      const constrained = LonCodec(maxExpandedElements: 64);
      expect(() => constrained.encode(value), throwsArgumentError);
    });

    test('bounds serialized output', () {
      const constrained = LonCodec(maxOutputCodeUnits: 3);
      expect(constrained.encode('Ada'), 'Ada');
      expect(() => constrained.encode('Alan'), throwsFormatException);
    });

    test('rejects negative resource limits', () {
      expect(() => LonCodec(maxDepth: -1).encode(null), throwsRangeError);
      expect(
        () => LonCodec(maxExpandedElements: -1).encode(null),
        throwsRangeError,
      );
      expect(
        () => LonCodec(maxInputCodeUnits: -1).encode(null),
        throwsRangeError,
      );
      expect(
        () => LonCodec(maxOutputCodeUnits: -1).encode(null),
        throwsRangeError,
      );
    });
  });

  group('strict decoding', () {
    test('rejects truncated or malformed table rows', () {
      for (final source in <String>[
        '[id name;1]',
        '[id name;1 Ada;2]',
        '[id;1 2]',
        '[id name;1 Ada 2 Bob]',
        '[id name;1 Ada;]',
        '[id name;(1 Ada)]',
        '[id name;[1 Ada][2 Bob]]',
      ]) {
        expect(() => lonToJson(source), throwsFormatException, reason: source);
      }
    });

    test('rejects superseded table syntaxes', () {
      for (final source in <String>[
        'table(1)',
        'table(1)[id;(1)]',
        'table[1:id;(1)]',
        'table{1:id;(key:1)}',
        '[2|id;(1)(2)]',
        '{2|id;(a:1)(b:2)}',
        '[|id;]',
      ]) {
        expect(() => lonToJson(source), throwsFormatException, reason: source);
      }
    });

    test('rejects tables with fewer than two rows', () {
      for (final source in <String>[
        '[a b;]',
        '[a b;1 2]',
      ]) {
        expect(() => lonToJson(source), throwsFormatException, reason: source);
      }
    });

    test('rejects ambiguous single-value table schemas', () {
      for (final source in <String>[
        '[a;1;2]',
        '[a{b};1;2]',
        '[a empty{};1;2]',
      ]) {
        expect(() => lonToJson(source), throwsFormatException, reason: source);
      }
    });

    test('retains schema fields and quoted cell boundaries', () {
      expect(
        lonToJson('[id id;1 2;3 4]'),
        '[{"id":1,"id":2},{"id":3,"id":4}]',
      );
      expect(
        lonToJson('["a b" c;1 2;3 4]'),
        '[{"a b":1,"c":2},{"a b":3,"c":4}]',
      );
      expect(
        lonToJson('[v w;"a;b" 1;"c" 2]'),
        '[{"v":"a;b","w":1},{"v":"c","w":2}]',
      );
    });

    test('accepts dynamic cells but keeps their encoding explicit', () {
      const source = '[id payload;1 {a:b};2 {x:[1,{b:c}]}]';
      const json =
          '[{"id":1,"payload":{"a":"b"}},{"id":2,"payload":{"x":[1,{"b":"c"}]}}]';
      expect(lonToJson(source), json);
      expect(
        jsonToLon(json),
        '[{id:1 payload:{a:b}},{id:2 payload:{x:[1,{b:c}]}}]',
      );
    });

    test('canonicalizes table whitespace and preserves ordinary prefixes', () {
      const spaced = '[ a b ; 1 2 ; 3 4 ; 5 6 ]';
      expect(jsonToLon(lonToJson(spaced)), '[a b;1 2;3 4;5 6]');
      expect(jsonToLon('{"1":"x"}'), '{1:x}');
      expect(lonToJson('{1:x}'), '{"1":"x"}');
      expect(lonToJson('table'), '"table"');
      expect(lonToJson('tablex'), '"tablex"');
    });

    test('accepts separators and canonicalizes explicit type boundaries', () {
      expect(lonToJson('[true false]'), '[true,false]');
      expect(lonToJson('[true,false]'), '[true,false]');
      expect(jsonToLon(lonToJson('[1 Ada true]')), '[1,Ada,true]');
      expect(jsonToLon(lonToJson('[Ada,Bob]')), '[Ada Bob]');
      expect(lonToJson('[Ada 2026-08-02]'), '["Ada","2026-08-02"]');
    });

    test('rejects a trailing array comma', () {
      expect(() => lonToJson('[true,]'), throwsFormatException);
    });

    test('rejects trailing content', () {
      expect(() => lonToJson('{a:1} {b:2}'), throwsFormatException);
    });

    test('enforces expansion limits', () {
      const constrained = LonCodec(maxExpandedElements: 1);
      expect(() => constrained.decodeJson('[1 2]'), throwsFormatException);
    });

    test('rejects ambiguous or invisible bare tokens', () {
      for (final source in <String>[
        '[01]',
        '[undefined]',
        '[NaN]',
        '[a\u0000b]',
        '[tr\u2063ue]',
        r'[a\b]',
        '{a"b:1}',
        '{a\u202Eb:1}',
        r'{a\b:1}',
        '[a"b;1]',
        '"a\u2063b"',
        '{"a\u202Eb":1}',
      ]) {
        expect(() => lonToJson(source), throwsFormatException, reason: source);
      }

      expect(
        lonToJson(r'["01" "undefined" "a\u0000b"]'),
        r'["01","undefined","a\u0000b"]',
      );
      expect(
        lonToJson(r'{"a\u202eb":1}'),
        r'{"a\u202eb":1}',
      );
      expect(
        lonToJson(r'["a\u2063b"]'),
        r'["a\u2063b"]',
      );
    });

    test('handles deeply nested table schemas in linear work', () {
      expect(lonToJson(_nestedTable(40)), startsWith('[{"a":'));
      expect(
        () => lonToJson(_nestedTable(2000), maxDepth: 32),
        throwsFormatException,
      );
    });

    test('enforces schema, element, input, and output limits', () {
      const elementConstrained = LonCodec(maxExpandedElements: 3);
      expect(
        () => elementConstrained.decodeJson('[a b;1 2;3 4]'),
        throwsFormatException,
      );

      const schemaConstrained = LonCodec(maxExpandedElements: 1);
      expect(
        () => schemaConstrained.decodeJson('[a b;]'),
        throwsFormatException,
      );

      const jsonConstrained = LonCodec(maxExpandedElements: 2);
      expect(jsonConstrained.encodeJson('[1,2]'), '[1,2]');
      expect(
        () => jsonConstrained.encodeJson('[1,2,3]'),
        throwsFormatException,
      );

      const inputConstrained = LonCodec(maxInputCodeUnits: 4);
      expect(inputConstrained.decodeJson('null'), 'null');
      expect(
        () => inputConstrained.decodeJson(' null'),
        throwsFormatException,
      );

      const outputConstrained = LonCodec(maxOutputCodeUnits: 3);
      expect(outputConstrained.decodeJson(r'"a"'), r'"a"');
      expect(
        () => outputConstrained.decodeJson(r'"ab"'),
        throwsFormatException,
      );

      const expansionConstrained = LonCodec(maxOutputCodeUnits: 20);
      expect(
        () => expansionConstrained.decodeJson(
          '[abcdefghij;1;2;3]',
        ),
        throwsFormatException,
      );
    });
  });

  group('pretty rendering', () {
    const prettyCodec = LonCodec(pretty: true);

    test('renders objects, tables, and container arrays across lines', () {
      expect(
        jsonToLon(
          '{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"},'
          '{"id":3,"name":"Lin"}],"meta":{"count":3,"tags":["x","y"]},'
          '"empty":{},"scalars":[1,2,3]}',
          pretty: true,
        ),
        '{\n'
        '  users: [\n'
        '    id name;\n'
        '    1 Ada;\n'
        '    2 Bob;\n'
        '    3 Lin\n'
        '  ]\n'
        '  meta: {\n'
        '    count: 3\n'
        '    tags: [x,"y"]\n'
        '  }\n'
        '  empty: {}\n'
        '  scalars: [1,2,3]\n'
        '}',
      );
    });

    test('indents nested containers by depth', () {
      expect(
        jsonToLon(
          '{"outer":{"rows":[{"id":1,"items":[{"sku":"x","qty":2},'
          '{"sku":"y","qty":1}]}],"leaf":{"a":{"b":1}}},'
          '"mix":[1,{"a":"b"},[{"c":"d"}]]}',
          pretty: true,
        ),
        '{\n'
        '  outer: {\n'
        '    rows: [\n'
        '      {\n'
        '        id: 1\n'
        '        items: [\n'
        '          sku qty;\n'
        '          x 2;\n'
        '          "y" 1\n'
        '        ]\n'
        '      }\n'
        '    ]\n'
        '    leaf: {\n'
        '      a: {\n'
        '        b: 1\n'
        '      }\n'
        '    }\n'
        '  }\n'
        '  mix: [\n'
        '    1,\n'
        '    {\n'
        '      a: b\n'
        '    },\n'
        '    [\n'
        '      {\n'
        '        c: d\n'
        '      }\n'
        '    ]\n'
        '  ]\n'
        '}',
      );
    });

    test('keeps table cells inline even when they hold containers', () {
      expect(
        jsonToLon(
          '{"orders":[{"id":"A1","customer":{"id":1,"name":"Ada"},'
          '"items":[{"sku":"X","qty":2},{"sku":"Y","qty":1}]},'
          '{"id":"A2","customer":{"id":2,"name":"Bob"},"items":[]}]}',
          pretty: true,
        ),
        '{\n'
        '  orders: [\n'
        '    id customer{id name} items;\n'
        '    A1 1 Ada [sku qty;X 2;"Y" 1];\n'
        '    A2 2 Bob []\n'
        '  ]\n'
        '}',
      );
    });

    test('scalar roots stay compact', () {
      expect(jsonToLon('42', pretty: true), '42');
      expect(jsonToLon('"Ada"', pretty: true), 'Ada');
      expect(jsonToLon('[1,2,3]', pretty: true), '[1,2,3]');
      expect(jsonToLon('["a","b"]', pretty: true), '[a b]');
      expect(jsonToLon('{}', pretty: true), '{}');
      expect(jsonToLon('[]', pretty: true), '[]');
    });

    test('decodes to the same values as the compact form', () {
      const source =
          '{"users":[{"id":1,"name":"Ada","meta":{"x":true,"y":null}},'
          '{"id":2,"name":"Bob","meta":{"x":false,"y":1}}],"mix":[1,{"a":"b"},'
          '[{"c":"d"}]]}';
      final compact = jsonToLon(source);
      final pretty = prettyCodec.encodeJson(source);

      expect(lonToJson(pretty), lonToJson(compact));
      expect(jsonToLon(lonToJson(pretty)), compact);
      expect(
        jsonDecode(prettyCodec.decodeJson(compact)),
        jsonDecode(lonToJson(compact)),
      );
      expect(
        prettyCodec.decodeJson('{users:[id name;1 Ada;2 Bob]}'),
        '{\n'
        '  "users": [\n'
        '    {\n'
        '      "id": 1,\n'
        '      "name": "Ada"\n'
        '    },\n'
        '    {\n'
        '      "id": 2,\n'
        '      "name": "Bob"\n'
        '    }\n'
        '  ]\n'
        '}',
      );
    });

    test('pretty re-encodes to identical canonical output', () {
      const source = '{"a":{"b":{"c":[1,{"d":"e"}]}},"r":[{"x":1,"y":{"z":2}},'
          '{"x":3,"y":{"z":4}}]}';
      final canonical = jsonToLon(source);
      final pretty = jsonToLon(source, pretty: true);
      expect(jsonToLon(lonToJson(pretty)), canonical);
    });

    test('line endings contain no trailing whitespace', () {
      final pretty = jsonToLon(
        '{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}],'
        '"o":{"a":[1,{"b":2}]}}',
        pretty: true,
      );
      for (final line in pretty.split('\n')) {
        expect(line.endsWith(' '), isFalse, reason: line);
        expect(line.endsWith('\t'), isFalse, reason: line);
      }
      expect(pretty, isNot(endsWith('\n')));
    });
  });

  group('CLI', () {
    test('round-trips split UTF-8 byte streams', () async {
      final value = <String, Object?>{
        '空 白;"[]{}': 'line one\r\nline two,;[]{}"\\😀\u202E',
      };
      final process = await Process.start(
        Platform.resolvedExecutable,
        <String>['run', 'bin/lon.dart', 'encode'],
      );
      final stdoutBytesFuture =
          process.stdout.expand((chunk) => chunk).toList();
      final stderrBytesFuture =
          process.stderr.expand((chunk) => chunk).toList();

      final inputBytes = utf8.encode(jsonEncode(value));
      for (final byte in inputBytes) {
        process.stdin.add(<int>[byte]);
      }
      await process.stdin.close();

      expect(await process.exitCode, 0);
      final stdoutText = utf8.decode(
        await stdoutBytesFuture,
        allowMalformed: false,
      );
      expect(
        utf8.decode(await stderrBytesFuture, allowMalformed: false),
        isEmpty,
      );
      expect(stdoutText.endsWith('\n'), isTrue);
      final payload = stdoutText.substring(0, stdoutText.length - 1);
      expect(payload, isNot(contains('\n')));
      expect(jsonDecode(lonToJson(payload)), value);
    });

    test('rejects malformed UTF-8 byte streams', () async {
      final process = await Process.start(
        Platform.resolvedExecutable,
        <String>['run', 'bin/lon.dart', 'decode'],
      );
      final stdoutBytesFuture =
          process.stdout.expand((chunk) => chunk).toList();
      final stderrBytesFuture =
          process.stderr.expand((chunk) => chunk).toList();

      process.stdin.add(<int>[0x22, 0xC3, 0x28, 0x22]);
      await process.stdin.close();

      expect(await process.exitCode, 65);
      expect(await stdoutBytesFuture, isEmpty);
      final diagnostic = utf8.decode(
        await stderrBytesFuture,
        allowMalformed: false,
      );
      expect(diagnostic, contains('Invalid input'));
      expect(diagnostic, isNot(contains('\uFFFD')));
    });
    test('encodes and decodes with --pretty', () async {
      final process = await Process.start(
        Platform.resolvedExecutable,
        <String>['run', 'bin/lon.dart', 'encode', '--pretty'],
      );
      final stdoutFuture = process.stdout.transform(utf8.decoder).join();
      final stderrFuture = process.stderr.transform(utf8.decoder).join();
      process.stdin.write(
        '{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}',
      );
      await process.stdin.close();

      expect(await process.exitCode, 0);
      expect(await stderrFuture, isEmpty);
      final encoded = (await stdoutFuture).trimRight();
      expect(
        encoded,
        '{\n'
        '  users: [\n'
        '    id name;\n'
        '    1 Ada;\n'
        '    2 Bob\n'
        '  ]\n'
        '}',
      );

      final decode = await Process.start(
        Platform.resolvedExecutable,
        <String>['run', 'bin/lon.dart', 'decode', '--pretty'],
      );
      final decodeOut = decode.stdout.transform(utf8.decoder).join();
      decode.stdin.write(encoded);
      await decode.stdin.close();

      expect(await decode.exitCode, 0);
      expect(
        jsonDecode(await decodeOut),
        jsonDecode(
          '{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}',
        ),
      );
    });

    test('rejects unknown flags with exit code 64', () async {
      final process = await Process.start(
        Platform.resolvedExecutable,
        <String>['run', 'bin/lon.dart', 'encode', '--bogus'],
      );
      await process.stdin.close();
      expect(await process.exitCode, 64);
    });

    test('does not echo untrusted input in diagnostics', () async {
      final process = await Process.start(
        Platform.resolvedExecutable,
        <String>['run', 'bin/lon.dart', 'decode'],
      );
      final stdoutFuture = process.stdout.transform(utf8.decoder).join();
      final stderrFuture = process.stderr.transform(utf8.decoder).join();

      process.stdin.write('ok \x1B]0;PWNED\x07');
      await process.stdin.close();

      expect(await process.exitCode, 65);
      expect(await stdoutFuture, isEmpty);
      final diagnostic = await stderrFuture;
      expect(diagnostic, contains('Unexpected trailing content'));
      expect(diagnostic, isNot(contains('PWNED')));
      expect(diagnostic.codeUnits.contains(0x1B), isFalse);
      expect(diagnostic.codeUnits.contains(0x07), isFalse);
    });
  });
}

String _nestedTable(int depth) {
  final openings = List.filled(depth - 1, 'a{').join();
  final closings = List.filled(depth - 1, '}').join();
  return '[${openings}x$closings y;0 1;2 3]';
}

import 'package:lon/lon.dart';
import 'package:test/test.dart';

void main() {
  group('table row width mismatches', () {
    for (final source in <String>[
      // Fewer cells than the schema declares.
      '[a b;1 2;3]',
      '[a b;1;2 3]',
      '[a b c;1 2 3;4 5]',
      '[a{x y} b;1 2;3 4]',
      // More cells than the schema declares.
      '[a b;1 2;3 4 5]',
      '[a b;1 2 3;4 5]',
      '[a{x y} b;1 2 3 4;5 6 7]',
      // Empty row or double semicolons.
      '[a b;;1 2;3 4]',
      '[a b;1 2;;3 4]',
      '[a b;1 2;3 4;]',
      // A nested table-shaped row in a schema row position.
      '[a b;[1 2];[3 4]]',
      '[a b;{x:1} {y:2}]',
    ]) {
      test('rejects $source', () {
        expect(() => lonToJson(source), throwsFormatException);
      });
    }
  });

  group('unterminated structures', () {
    for (final source in <String>[
      // A table header run hits the closing bracket: the schema ends without
      // the ';' terminator, and one row is never enough.
      '[a b;]',
      '[a b;1 2;]',
      '[a{x y} b;]',
      // Unterminated schema or row.
      '[a b;1 2',
      '[a b;1 2;3 4',
      '[a{x y} b;1 2 3',
      // Unterminated containers.
      '{a:1',
      '{a:1 b:2',
      '[1,2',
      '[[1],[2',
      '{a:{b:{c:1}}',
      // Missing separators between adjacent values.
      '[1 2}3]',
      '{a:1,b:2}',
      // Stray semicolons outside tables.
      '{a:1;b:2}',
      '[1;2]',
    ]) {
      test('rejects $source', () {
        expect(() => lonToJson(source), throwsFormatException);
      });
    }
  });

  group('malformed quoted strings', () {
    for (final source in <String>[
      // Unterminated quote at various positions.
      '{a:"unterminated',
      '[a b;"x" 1;"y 2]',
      '"unterminated',
      '{"key:',
      // Invalid escape sequences.
      r'{a:"bad\x"}',
      r'[a b;"x\x" 1;"y" 2]',
      r'"bad\q"',
      r'"\u12"',
      r'"\uzzzz"',
      // Unescaped control characters.
      '"raw\nnewline"',
      '"raw\u0000control"',
      // A dangling backslash at end of input.
      '"trailing\\',
    ]) {
      test('rejects ${source.replaceAll('\n', r'\n')}', () {
        expect(() => lonToJson(source), throwsFormatException);
      });
    }
  });

  group('deeply nested truncation', () {
    test('every prefix of a deep container is rejected', () {
      final encoded = jsonToLon(
        '{"a":{"b":{"c":{"d":{"e":[1,{"f":{"g":2}}],'
        '"h":[{"i":1,"j":2},{"i":3,"j":4}]}}}}}',
      );
      for (var end = 0; end < encoded.length; end++) {
        expect(
          () => lon.decode(encoded.substring(0, end)),
          throwsFormatException,
          reason: 'prefix of length $end of $encoded',
        );
      }
      // Sanity-check the source itself parses.
      expect(lon.decode(encoded), isA<Map<String, Object?>>());
    });

    test('nested table headers reject their truncations', () {
      const source = '[a{b{c{x}}} y;0 1 2 3;4 5 6 7]';
      for (var end = 0; end < source.length; end++) {
        expect(
          () => lonToJson(source.substring(0, end)),
          throwsFormatException,
          reason: 'prefix of length $end of $source',
        );
      }
    });
  });
}

import 'package:lon/lon.dart';
import 'package:test/test.dart';

void main() {
  group('depth limits', () {
    // Table rows expand into real objects: the schema's deepest field path
    // adds to the depth of the table itself.
    test('counts expanded table schema depth against maxDepth', () {
      // Schema a{b{c{d{x y}}} z} w has a maximum field path of 4
      // (a -> b -> c -> d -> x/y); the table sits at depth 0, so the
      // deepest leaf value lands at depth 5.
      const source = '[a{b{c{d{x y}}} z} w;0 1 2 3;4 5 6 7]';
      const expected = '[{"a":{"b":{"c":{"d":{"x":0,"y":1}}},"z":2},"w":3},'
          '{"a":{"b":{"c":{"d":{"x":4,"y":5}}},"z":6},"w":7}]';

      for (var allowed = 0; allowed <= 5; allowed++) {
        expect(
          () => LonCodec(maxDepth: allowed).decodeJson(source),
          throwsFormatException,
          reason: 'maxDepth $allowed',
        );
      }
      expect(const LonCodec(maxDepth: 6).decodeJson(source), expected);
      expect(const LonCodec(maxDepth: 7).decodeJson(source), expected);
    });

    test('applies maxDepth identically on all three entry points', () {
      const three = LonCodec(maxDepth: 3);
      expect(three.decode('[[[1]]]'), <Object?>[
        <Object?>[
          <Object?>[1],
        ],
      ]);
      expect(() => three.decode('[[[[1]]]]'), throwsFormatException);
      expect(three.decodeJson('[[[1]]]'), '[[[1]]]');
      expect(() => three.encodeJson('[[[[1]]]]'), throwsFormatException);
      expect(
        three.encode(const [
          [
            [2],
          ],
        ]),
        '[[[2]]]',
      );
      expect(
        () => three.encode(const [
          [
            [
              [2],
            ],
          ],
        ]),
        throwsArgumentError,
      );
    });

    test('zero depth accepts scalars and empty containers only', () {
      const flat = LonCodec(maxDepth: 0);
      expect(flat.decode('null'), isNull);
      expect(flat.decode('[]'), isEmpty);
      expect(flat.decode('{}'), isEmpty);
      expect(() => flat.decodeJson('[a b;1 2;3 4]'), throwsFormatException);
      expect(() => flat.decode('[[1]]'), throwsFormatException);
      expect(() => flat.decode('{a:1}'), throwsFormatException);
      expect(() => flat.encode(const [1]), throwsArgumentError);
      expect(() => flat.encode(const {'a': 1}), throwsArgumentError);
      expect(() => flat.encodeJson('[1]'), throwsFormatException);
      // Table schema nesting counts against depth even at 0.
      expect(() => flat.decodeJson('[a{x} b;1 2;3 4]'), throwsFormatException);
    });
  });

  group('expanded element limits', () {
    test('bounds ordinary LON array elements exactly at the limit', () {
      const two = LonCodec(maxExpandedElements: 2);
      expect(two.decodeJson('[1 2]'), '[1,2]');
      expect(() => two.decodeJson('[1 2 3]'), throwsFormatException);
      const three = LonCodec(maxExpandedElements: 3);
      expect(three.decodeJson('[1 2 3]'), '[1,2,3]');
    });

    test('bounds ordinary LON object members exactly at the limit', () {
      const two = LonCodec(maxExpandedElements: 2);
      expect(two.decodeJson('{a:1 b:2}'), '{"a":1,"b":2}');
      expect(() => two.decodeJson('{a:1 b:2 c:3}'), throwsFormatException);
    });

    test('bounds JSON-parsed elements on encode exactly at the limit', () {
      const two = LonCodec(maxExpandedElements: 2);
      expect(two.encodeJson('[1,2]'), '[1,2]');
      expect(() => two.encodeJson('[1,2,3]'), throwsFormatException);
      expect(two.encodeJson('{"a":1,"b":2}'), '{a:1 b:2}');
      expect(
          () => two.encodeJson('{"a":1,"b":2,"c":3}'), throwsFormatException);
      // Nested elements count too.
      const five = LonCodec(maxExpandedElements: 5);
      expect(five.encodeJson('[1,[2,[3]]]'), '[1,[2,[3]]]');
      const four = LonCodec(maxExpandedElements: 4);
      expect(() => four.encodeJson('[1,[2,[3]]]'), throwsFormatException);
    });

    test('bounds Dart object graph elements on encode exactly at the limit',
        () {
      const three = LonCodec(maxExpandedElements: 3);
      expect(three.encode(const [1, 2, 3]), '[1,2,3]');
      const two = LonCodec(maxExpandedElements: 2);
      expect(two.encode(const {'a': 1, 'b': 2}), '{a:1 b:2}');
      expect(() => two.encode(const [1, 2, 3]), throwsArgumentError);
      expect(
        () => two.encode(const {'a': 1, 'b': 2, 'c': 3}),
        throwsArgumentError,
      );
    });

    test('counts each table row as fieldCount + 1 expanded elements', () {
      // [a b;...] → 3 slots per row: two rows fit in 6, the third needs 9.
      const six = LonCodec(maxExpandedElements: 6);
      expect(
        six.decodeJson('[a b;1 2;3 4]'),
        '[{"a":1,"b":2},{"a":3,"b":4}]',
      );
      expect(() => six.decodeJson('[a b;1 2;3 4;5 6]'), throwsFormatException);
      const nine = LonCodec(maxExpandedElements: 9);
      expect(
        nine.decodeJson('[a b;1 2;3 4;5 6]'),
        '[{"a":1,"b":2},{"a":3,"b":4},{"a":5,"b":6}]',
      );
      const eight = LonCodec(maxExpandedElements: 8);
      expect(
          () => eight.decodeJson('[a b;1 2;3 4;5 6]'), throwsFormatException);
    });

    test('counts schema fields against their own element budget', () {
      // Schema fields are tracked by a separate counter bounded by the same
      // limit: a 3-field schema already fails at budget 2 even before rows.
      const two = LonCodec(maxExpandedElements: 2);
      expect(
          () => two.decodeJson('[a b c;1 2 3;4 5 6]'), throwsFormatException);
      const three = LonCodec(maxExpandedElements: 3);
      expect(
          () => three.decodeJson('[a b c;1 2 3;4 5 6]'), throwsFormatException);
      // The schema fits the field counter at 3, but each row then needs
      // fieldCount + 1 = 4 expanded elements, so 2 rows need 8.
      const eight = LonCodec(maxExpandedElements: 8);
      expect(
        eight.decodeJson('[a b c;1 2 3;4 5 6]'),
        '[{"a":1,"b":2,"c":3},{"a":4,"b":5,"c":6}]',
      );
      const seven = LonCodec(maxExpandedElements: 7);
      expect(
        () => seven.decodeJson('[a b c;1 2 3;4 5 6]'),
        throwsFormatException,
      );
      // Nested schema fields count recursively: a{x y} b is 4 schema fields
      // and 4 + 1 = 5 expanded elements per row.
      const fourFields = LonCodec(maxExpandedElements: 3);
      expect(
        () => fourFields.decodeJson('[a{x y} b;1 2 3;4 5 6]'),
        throwsFormatException,
      );
      const nestedBudget = LonCodec(maxExpandedElements: 2 * 5);
      expect(
        nestedBudget.decodeJson('[a{x y} b;1 2 3;4 5 6]'),
        '[{"a":{"x":1,"y":2},"b":3},{"a":{"x":4,"y":5},"b":6}]',
      );
      const nestedShort = LonCodec(maxExpandedElements: 2 * 5 - 1);
      expect(
        () => nestedShort.decodeJson('[a{x y} b;1 2 3;4 5 6]'),
        throwsFormatException,
      );
    });

    test('zero element budget accepts only empty containers and scalars', () {
      const zero = LonCodec(maxExpandedElements: 0);
      expect(zero.decodeJson('null'), 'null');
      expect(zero.decodeJson('[]'), '[]');
      expect(zero.decodeJson('{}'), '{}');
      expect(zero.encode(null), 'null');
      expect(zero.encode(const []), '[]');
      expect(zero.encode(const {}), '{}');
      expect(() => zero.decodeJson('[1]'), throwsFormatException);
      expect(() => zero.decodeJson('{a:1}'), throwsFormatException);
      expect(() => zero.decodeJson('[a b;1 2;3 4]'), throwsFormatException);
      expect(() => zero.encode(const [1]), throwsArgumentError);
      expect(() => zero.encode(const {'a': 1}), throwsArgumentError);
      expect(() => zero.encodeJson('[1]'), throwsFormatException);
      expect(() => zero.encodeJson('{"a":1}'), throwsFormatException);
    });
  });

  group('input and output code-unit limits', () {
    test('accepts input exactly at the limit and rejects limit + 1', () {
      const four = LonCodec(maxInputCodeUnits: 4);
      expect(four.decodeJson('null'), 'null');
      expect(four.decode('null'), isNull);
      expect(four.encodeJson('null'), 'null');
      expect(() => four.decodeJson(' null'), throwsFormatException);
      expect(() => four.decode(' null'), throwsFormatException);
      expect(() => four.encodeJson(' null'), throwsFormatException);
    });

    test('counts input in UTF-16 code units, not characters', () {
      // The emoji is 2 code units: 'a' + emoji = 3 units inside quotes.
      const five = LonCodec(maxInputCodeUnits: 5);
      expect(five.decode('"a😀"'), 'a😀');
      expect(() => five.decode('"ab😀"'), throwsFormatException);
    });

    test('accepts output exactly at the limit and rejects limit + 1', () {
      const three = LonCodec(maxOutputCodeUnits: 3);
      expect(three.encode('Ada'), 'Ada');
      expect(() => three.encode('Alan'), throwsFormatException);
      expect(three.decodeJson('"a"'), '"a"');
      expect(() => three.decodeJson('"ab"'), throwsFormatException);
      // Output escaping also counts against the budget: '"\\t"' is 4 units.
      const four = LonCodec(maxOutputCodeUnits: 4);
      expect(four.decodeJson(r'"\t"'), r'"\t"');
      const tight = LonCodec(maxOutputCodeUnits: 3);
      expect(() => tight.decodeJson(r'"\t"'), throwsFormatException);
    });

    test('zero limits permit only empty output and empty input', () {
      const zeroIn = LonCodec(maxInputCodeUnits: 0);
      expect(() => zeroIn.decodeJson('null'), throwsFormatException);
      expect(() => zeroIn.decode('null'), throwsFormatException);
      expect(() => zeroIn.encodeJson('null'), throwsFormatException);
      expect(() => zeroIn.decodeJson(''), throwsFormatException);

      const zeroOut = LonCodec(maxOutputCodeUnits: 0);
      expect(() => zeroOut.encode(''), throwsFormatException);
      expect(() => zeroOut.encode(null), throwsFormatException);
      expect(() => zeroOut.decodeJson('""'), throwsFormatException);
      expect(() => zeroOut.decodeJson('null'), throwsFormatException);
      expect(() => zeroOut.encodeJson('""'), throwsFormatException);
    });
  });

  group('configuration validation', () {
    test('rejects negative limits on every entry point', () {
      for (final codec in <LonCodec>[
        const LonCodec(maxDepth: -1),
        const LonCodec(maxExpandedElements: -1),
        const LonCodec(maxInputCodeUnits: -1),
        const LonCodec(maxOutputCodeUnits: -1),
      ]) {
        expect(() => codec.encode(null), throwsRangeError, reason: '$codec');
        expect(() => codec.decode('null'), throwsRangeError, reason: '$codec');
        expect(
          () => codec.encodeJson('null'),
          throwsRangeError,
          reason: '$codec',
        );
        expect(
          () => codec.decodeJson('null'),
          throwsRangeError,
          reason: '$codec',
        );
      }
    });
  });
}

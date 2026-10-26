import 'dart:convert';
import 'dart:math';

import 'package:lon/lon.dart';
import 'package:test/test.dart';

void main() {
  group('JSON string domain', () {
    test('preserves complex embedded whitespace exactly', () {
      const value = 'abc 12 34 \nxxx\nb \txx';

      final encoded = lon.encode(value);

      expect(encoded, r'"abc 12 34 \nxxx\nb \txx"');
      expect(lon.decode(encoded), value);
      expect(jsonDecode(lonToJson(jsonToLon(jsonEncode(value)))), value);
    });

    test('preserves empty, leading, middle, and trailing whitespace', () {
      const values = <String>[
        '',
        ' ',
        '  leading',
        'middle  space',
        'trailing  ',
        '\tleading tab',
        'line one\nline two',
        'carriage\rreturn',
        'form\ffeed',
        'back\bspace',
        'mixed \n\r\t whitespace',
      ];

      _expectValueAndJsonRoundTrip(values);
    });

    test('preserves every C0 and C1 control code unit', () {
      final values = <String>[
        for (var code = 0; code <= 0x9F; code++) String.fromCharCode(code),
      ];

      final encoded = lon.encode(values);

      expect(lon.decode(encoded), values);
      expect(jsonDecode(lonToJson(jsonToLon(jsonEncode(values)))), values);
      for (final code in encoded.codeUnits) {
        expect(
          code < 0x20 || (code >= 0x7F && code <= 0x9F),
          isFalse,
          reason: 'Raw control U+${code.toRadixString(16).padLeft(4, '0')}',
        );
      }
    });

    test('preserves punctuation, delimiters, and JSON escapes', () {
      const values = <String>[
        '"',
        r'\',
        '/',
        ':',
        ',',
        ';',
        '[',
        ']',
        '{',
        '}',
        '(',
        ')',
        '|',
        "'",
        r'quote"slash\solidus/',
        'https://example.com/a?x=1&y=2#part',
        '[not a container]',
        '{not:an object}',
      ];

      _expectValueAndJsonRoundTrip(values);
    });

    test('preserves exhaustive short grammar-conflict combinations', () {
      final values = _grammarConflictStrings().toList();
      values.addAll(const <String>[
        'key:value;next,[array],{object}',
        r'quote\"slash\\comma,semicolon;',
        '// comment-like',
        '/* block-comment-like */',
        '# yaml-comment-like',
        '&anchor *alias !tag',
        '```json\n{"x":"</script>"}\n```',
        r'${template} %0A %0D ?x=1&y=2#fragment',
        '<script>alert("x")</script>',
      ]);

      _expectValueAndJsonRoundTrip(values);
    });

    test('preserves delimiter-heavy table schema keys and cells', () {
      final value = <Object?>[
        <String, Object?>{
          'a;b': 'x;y',
          'q"x': '[one]',
          'line\nkey': '{one}',
        },
        <String, Object?>{
          'a;b': 'x,y',
          'q"x': '[two]',
          'line\nkey': '{two}',
        },
        <String, Object?>{
          'a;b': 'x:y',
          'q"x': '[three]',
          'line\nkey': '{three}',
        },
      ];

      final encoded = lon.encode(value);

      expect(encoded, startsWith(r'["a;b" "q\"x" "line\nkey";'));
      _expectValueAndJsonRoundTrip(value);
    });

    test('round-trips every UTF-16 code unit', () {
      final values = List<String>.generate(
        0x10000,
        String.fromCharCode,
        growable: false,
      );

      final encoded = lon.encode(values);

      expect(lon.decode(encoded), values);
      expect(utf8.decode(utf8.encode(encoded), allowMalformed: false), encoded);
    });

    test('preserves JSON-adjacent and numeric-looking strings', () {
      const values = <String>[
        'null',
        'NULL',
        'true',
        'False',
        'undefined',
        'None',
        'nil',
        'NaN',
        '+Infinity',
        '-inf',
        '.NaN',
        '+.inf',
        'yes',
        'NO',
        'on',
        'Off',
        'y',
        'N',
        '~',
        '01',
        '+1',
        '.5',
        '1.',
        '1e',
        '1e+',
        '0x10',
        '0b101',
        '0o77',
        '1_000',
        '2026-08-02',
        '123abc',
        'table',
      ];

      _expectValueAndJsonRoundTrip(values);
      final encoded = lon.encode(values);
      for (var index = 0; index < values.length; index++) {
        if (values[index] == '2026-08-02' ||
            values[index] == '123abc' ||
            values[index] == 'table') {
          continue;
        }
        expect(encoded, contains(jsonEncode(values[index])));
      }
      expect(encoded, isNot(contains('"table"')));
    });

    test('preserves multilingual text without Unicode normalization', () {
      const values = <String>[
        'English: Hello world',
        'Español: ¿Qué tal?',
        'Français : Bonjour le monde',
        'Deutsch: Grüße aus Köln',
        'Português: Olá mundo',
        'Русский: Привет, мир',
        'Українська: Привіт, світе',
        'Ελληνικά: Γειά σου κόσμε',
        '\u7B80\u4F53\u4E2D\u6587\uFF1A\u4F60\u597D\uFF0C\u4E16\u754C',
        '\u7E41\u9AD4\u4E2D\u6587\uFF1A\u4F60\u597D\uFF0C\u4E16\u754C',
        '\u65E5\u672C\u8A9E\uFF1A\u3053\u3093\u306B\u3061\u306F\u4E16\u754C',
        '한국어: 안녕하세요 세계',
        'العربية: مرحبًا بالعالم',
        'עברית: שלום עולם',
        'فارسی: می‌روم',
        'हिन्दी: नमस्ते दुनिया',
        'বাংলা: নমস্কার বিশ্ব',
        'தமிழ்: வணக்கம் உலகம்',
        'తెలుగు: నమస్కారం ప్రపంచం',
        'ไทย: สวัสดีชาวโลก',
        'Tiếng Việt: Xin chào thế giới',
        'ქართული: გამარჯობა მსოფლიო',
        'Հայերեն: Բարև աշխարհ',
        'አማርኛ: ሰላም ዓለም',
        'ᐃᓄᒃᑎᑐᑦ: ᐊᐃ',
        'e\u0301',
        '\u00E9',
        '👩‍💻',
        '👨‍👩‍👧‍👦',
        '🇨🇳🇯🇵🇰🇷',
      ];

      _expectValueAndJsonRoundTrip(values);
      expect(values[25], isNot(values[26]));
    });

    test('preserves Unicode whitespace and directional formatting', () {
      const values = <String>[
        'a\u0085b',
        'a\u00A0b',
        'a\u1680b',
        'a\u2000b',
        'a\u200Ab',
        'a\u2028b',
        'a\u2029b',
        'a\u202Fb',
        'a\u205Fb',
        'a\u3000b',
        'a\u200Bb',
        'a\u00ADb',
        'a\u034Fb',
        'a\u061Cb',
        'a\u180Eb',
        'a\uFDD0b',
        'a\uFFFEb',
        'a\u200Eb',
        'a\u200Fb',
        'a\u202Ab',
        'a\u202Eb',
        'a\u2060b',
        'a\u2066b',
        'a\u2069b',
        'a\uFEFFb',
      ];

      _expectValueAndJsonRoundTrip(values);
      final encoded = lon.encode(values);
      for (final escape in <String>[
        r'\u0085',
        r'\u2028',
        r'\u2029',
        r'\u200b',
        r'\u00ad',
        r'\u034f',
        r'\u061c',
        r'\u180e',
        r'\ufdd0',
        r'\ufffe',
        r'\u202a',
        r'\u202e',
        r'\u2060',
        r'\u2066',
        r'\ufeff',
      ]) {
        expect(encoded, contains(escape));
      }
    });

    test('preserves lone surrogates and supplementary characters', () {
      final values = <String>[
        String.fromCharCode(0xD800),
        String.fromCharCode(0xDFFF),
        String.fromCharCodes(<int>[0xD800, 0x61]),
        String.fromCharCodes(<int>[0x61, 0xDFFF]),
        '😀',
        '𓀀',
        '𝄞',
        String.fromCharCode(0xE0001),
        String.fromCharCode(0x10FFFF),
      ];

      final valueEncoded = lon.encode(values);
      expect(lon.decode(valueEncoded), values);
      expect(valueEncoded, contains(r'\udb40\udc01'));
      expect(valueEncoded, contains(r'\udbff\udfff'));
      const json = r'["\ud800","\udfff","\ud83d\ude00"]';
      final encoded = jsonToLon(json);
      expect(lonToJson(encoded), r'["\ud800","\udfff","😀"]');
      expect(jsonToLon(lonToJson(encoded)), encoded);
    });

    test('preserves strings used as object keys', () {
      final value = <String, Object?>{
        'a b': 'space key',
        'line\nkey': 'newline key',
        'a:b': 'colon key',
        'مرحبا بالعالم': 'Arabic key',
        '\u952E \u540D': 'CJK key',
        'a\u202Eb': 'directional key',
      };

      _expectValueAndJsonRoundTrip(value);
    });

    test('round-trips every UTF-16 code unit as an object key', () {
      final value = <String, Object?>{
        '': -1,
        for (var code = 0; code <= 0xFFFF; code++)
          String.fromCharCode(code): code,
      };

      final encoded = lon.encode(value);

      expect(lon.decode(encoded), value);
      expect(utf8.decode(utf8.encode(encoded), allowMalformed: false), encoded);
      final encodedJson = jsonToLon(jsonEncode(value));
      expect(jsonDecode(lonToJson(encodedJson)), value);
      expect(jsonToLon(lonToJson(encodedJson)), encodedJson);
    });

    test('round-trips grammar-conflict keys through table schemas', () {
      final keys = _grammarConflictStrings().toList(growable: false);
      Map<String, Object?> record(int offset) => <String, Object?>{
            for (var index = 0; index < keys.length; index++)
              keys[index]: offset + index,
          };
      final value = <Object?>[
        record(0),
        record(keys.length),
      ];

      final encoded = lon.encode(value);

      expect(encoded, startsWith('['));
      expect(encoded, contains(';'));
      expect(lon.decode(encoded), value);
      expect(utf8.decode(utf8.encode(encoded), allowMalformed: false), encoded);
      final encodedJson = jsonToLon(jsonEncode(value));
      expect(jsonDecode(lonToJson(encodedJson)), value);
      expect(jsonToLon(lonToJson(encodedJson)), encodedJson);
    });

    test('preserves duplicate delimiter-heavy keys in the text API', () {
      const json = r'{"a;b":1,"a;b":2,"q\"x":3,"line\nkey":4,"":5}';

      final encoded = jsonToLon(json);

      expect(lonToJson(encoded), json);
      expect(jsonToLon(lonToJson(encoded)), encoded);
    });

    test('preserves a long adversarial string', () {
      const fragment =
          'abc 12 34 \nxxx\nb \txx 😀 مرحبًا \u4F60\u597D \\ " \u202E';
      final value = List<String>.filled(4096, fragment).join();

      _expectValueAndJsonRoundTrip(value);
    });

    test('round-trips deterministic generated Unicode strings', () {
      final random = Random(20260802);
      const atoms = <String>[
        'a',
        'Z',
        '0',
        ' ',
        '\t',
        '\n',
        '\r',
        '\u0000',
        '\u001F',
        '\u0085',
        '\u00A0',
        '"',
        r'\',
        ':',
        ',',
        ';',
        '[',
        ']',
        '{',
        '}',
        '(',
        ')',
        '|',
        "'",
        'é',
        '\u0301',
        '\u4E2D',
        'あ',
        '한',
        'م',
        'ש',
        'न',
        'ก',
        '😀',
        '👩‍💻',
        '\u200B',
        '\u202E',
        '\uFEFF',
      ];

      for (var iteration = 0; iteration < 1000; iteration++) {
        final buffer = StringBuffer();
        final length = random.nextInt(40);
        for (var index = 0; index < length; index++) {
          buffer.write(atoms[random.nextInt(atoms.length)]);
        }
        final value = buffer.toString();
        expect(
          lon.decode(lon.encode(value)),
          value,
          reason: 'Generated string $iteration',
        );
      }
    });
  });

  group('UTF-8 transport', () {
    test('survives arbitrary byte chunk boundaries without normalization',
        () async {
      final value = <String, Object?>{
        '空 白;"[]{}': <String>[
          '',
          'line one\r\nline two\nline three',
          'quote"slash\\comma,semicolon;brackets[]{}',
          'e\u0301',
          'é',
          '👩‍💻',
          'a\u202Eb',
          '\u0000\u0085\u2028\u2029\uFEFF',
          String.fromCharCode(0xD800),
          String.fromCharCode(0xDFFF),
        ],
      };
      final payload = lon.encode(value);
      final bytes = utf8.encode(payload);
      final chunks = <List<int>>[];
      for (var offset = 0; offset < bytes.length;) {
        final end = (offset + (offset % 7) + 1).clamp(0, bytes.length);
        chunks.add(bytes.sublist(offset, end));
        offset = end;
      }

      final received = await Stream<List<int>>.fromIterable(chunks)
          .transform(utf8.decoder)
          .join();

      expect(received, payload);
      expect(lon.decode(received), value);
      final jsonBytes = utf8.encode(lonToJson(received));
      expect(jsonDecode(utf8.decode(jsonBytes, allowMalformed: false)), value);
      for (final code in payload.codeUnits) {
        expect(
          code < 0x20 || (code >= 0x7F && code <= 0x9F),
          isFalse,
          reason: 'Raw transport control U+${code.toRadixString(16)}',
        );
      }
    });

    test('strict UTF-8 decoding rejects malformed transport bytes', () {
      const malformed = <List<int>>[
        <int>[0xC3, 0x28],
        <int>[0xC0, 0xAF],
        <int>[0xE2, 0x82],
        <int>[0xED, 0xA0, 0x80],
        <int>[0xF4, 0x90, 0x80, 0x80],
      ];

      for (final bytes in malformed) {
        expect(
          () => utf8.decode(bytes, allowMalformed: false),
          throwsFormatException,
          reason: bytes.toString(),
        );
      }
    });

    test('rejects a byte-order mark at the document boundary', () {
      expect(() => lonToJson('\uFEFF{a:1}'), throwsFormatException);
      expect(() => jsonToLon('\uFEFF{"a":1}'), throwsFormatException);
    });
  });

  group('malformed quoted strings', () {
    for (final source in <String>[
      '"unterminated',
      r'"bad\x20"',
      r'"bad\u12"',
      r'"bad\u0xz0"',
      '"raw\nnewline"',
      '"raw\u0000control"',
      '"trailing\\',
    ]) {
      test('rejects ${jsonEncode(source)}', () {
        expect(() => lonToJson(source), throwsFormatException);
      });
    }
  });
}

void _expectValueAndJsonRoundTrip(Object? value) {
  final encoded = lon.encode(value);
  expect(lon.decode(encoded), value);

  final json = jsonEncode(value);
  final encodedJson = jsonToLon(json);
  expect(jsonDecode(lonToJson(encodedJson)), value);
  expect(jsonToLon(lonToJson(encodedJson)), encodedJson);
}

const _grammarConflictAtoms = <String>[
  ' ',
  '\t',
  '\r',
  '\n',
  '"',
  r'\',
  ',',
  ';',
  ':',
  '[',
  ']',
  '{',
  '}',
  '(',
  ')',
  '|',
  "'",
];

Iterable<String> _grammarConflictStrings() sync* {
  yield '';
  for (final first in _grammarConflictAtoms) {
    for (final second in _grammarConflictAtoms) {
      for (final third in _grammarConflictAtoms) {
        yield '$first$second$third';
      }
    }
  }
}

import 'dart:convert';
import 'dart:io';

import 'package:lon/lon.dart';
import 'package:test/test.dart';

// nst/JSONTestSuite revision 1ef36fa01286573e846ac449e8683f8833c5b26a.
// The vendored corpus is MIT licensed; see its adjacent LICENSE file.
void main() {
  final fixtureRoot = Directory('test/fixtures/json_test_suite');
  final parsingFiles =
      _jsonFiles(Directory('${fixtureRoot.path}/test_parsing'));
  final transformFiles = _jsonFiles(
    Directory('${fixtureRoot.path}/test_transform'),
  );

  test('vendored corpus is complete', () {
    expect(parsingFiles, hasLength(318));
    expect(parsingFiles.where(_isRequiredValid), hasLength(95));
    expect(parsingFiles.where(_isRequiredInvalid), hasLength(188));
    expect(parsingFiles.where(_isImplementationDefined), hasLength(35));
    expect(transformFiles, hasLength(22));
    expect(File('${fixtureRoot.path}/LICENSE').existsSync(), isTrue);
  });

  group('JSONTestSuite required-valid', () {
    for (final file in parsingFiles.where(_isRequiredValid)) {
      test(_name(file), () {
        final source = _readStrictUtf8(file);
        _expectCanonicalRoundTrip(jsonToLon(source));
      });
    }
  });

  group('JSONTestSuite required-invalid', () {
    for (final file in parsingFiles.where(_isRequiredInvalid)) {
      test(_name(file), () {
        final source = _tryReadStrictUtf8(file);
        if (source == null) {
          return;
        }
        expect(() => jsonToLon(source), throwsFormatException);
      });
    }
  });

  group('JSONTestSuite implementation-defined', () {
    for (final file in parsingFiles.where(_isImplementationDefined)) {
      test(_name(file), () {
        final source = _tryReadStrictUtf8(file);
        if (source == null) {
          return;
        }

        String encoded;
        try {
          encoded = jsonToLon(source);
        } on FormatException {
          return;
        }
        _expectCanonicalRoundTrip(encoded);
      });
    }
  });

  group('JSONTestSuite transforms', () {
    const invalidUtf8 = <String>{
      'string_1_invalid_codepoint.json',
      'string_2_invalid_codepoints.json',
      'string_3_invalid_codepoints.json',
    };

    for (final file in transformFiles) {
      test(_name(file), () {
        final source = _tryReadStrictUtf8(file);
        if (invalidUtf8.contains(_name(file))) {
          expect(source, isNull);
          return;
        }

        expect(source, isNotNull);
        _expectCanonicalRoundTrip(jsonToLon(source!));
      });
    }
  });
}

List<File> _jsonFiles(Directory directory) {
  final files = directory
      .listSync()
      .whereType<File>()
      .where((file) => file.path.endsWith('.json'))
      .toList();
  files.sort((left, right) => left.path.compareTo(right.path));
  return files;
}

bool _isRequiredValid(File file) => _name(file).startsWith('y_');

bool _isRequiredInvalid(File file) => _name(file).startsWith('n_');

bool _isImplementationDefined(File file) => _name(file).startsWith('i_');

String _name(File file) => file.uri.pathSegments.last;

String _readStrictUtf8(File file) =>
    utf8.decode(file.readAsBytesSync(), allowMalformed: false);

String? _tryReadStrictUtf8(File file) {
  try {
    return _readStrictUtf8(file);
  } on FormatException {
    return null;
  }
}

void _expectCanonicalRoundTrip(String encoded) {
  final canonicalJson = lonToJson(encoded);
  expect(jsonToLon(canonicalJson), encoded);
}

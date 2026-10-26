import 'json_parser.dart';
import 'json_value_converter.dart';
import 'json_writer.dart';
import 'lon_parser.dart';
import 'lon_writer.dart';

/// The normative Language Object Notation version implemented by this package.
const lonFormatVersion = 1;

/// Lossless JSON-value codec for Language Object Notation v1.
///
/// ```dart
/// final encoded = lon.encode({
///   'users': [
///     {'id': 1, 'name': 'Ada'},
///     {'id': 2, 'name': 'Bob'},
///   ],
/// }); // {users:[id name;1 Ada;2 Bob]}
/// final value = lon.decode(encoded);
/// ```
final class LonCodec {
  const LonCodec({
    this.useTables = true,
    this.pretty = false,
    this.maxDepth = 512,
    this.maxExpandedElements = 1000000,
    this.maxInputCodeUnits = 64 * 1024 * 1024,
    this.maxOutputCodeUnits = 64 * 1024 * 1024,
  });

  /// Enables self-describing fixed-schema record tables when encoding.
  /// Decoding accepts table syntax regardless of this flag.
  final bool useTables;

  /// Renders encoded LON across lines, indented two spaces per nesting
  /// depth: every object member, table row, and container-array element
  /// occupies its own line between the container's brackets, while table
  /// rows and scalar arrays stay inline. Also selects standard
  /// two-space-indented JSON from [decodeJson]. Both outputs decode to
  /// exactly the same values as the compact forms.
  final bool pretty;

  /// Maximum accepted or encoded container nesting depth.
  final int maxDepth;

  /// Maximum parsed or encoded container-element occurrences. A table's
  /// schema-field count is bounded independently by the same value.
  final int maxExpandedElements;

  /// Maximum accepted source length in UTF-16 code units.
  final int maxInputCodeUnits;

  /// Maximum serialized output length in UTF-16 code units.
  final int maxOutputCodeUnits;

  /// Encodes a JSON-compatible Dart value as canonical LON.
  String encode(Object? value) {
    _validateConfiguration();
    return _newWriter().write(
      jsonNodeFromDart(
        value,
        maxDepth: maxDepth,
        maxExpandedElements: maxExpandedElements,
      ),
    );
  }

  /// Decodes LON into ordinary Dart JSON values.
  ///
  /// Duplicate object keys follow Dart/JSON decoding behavior: the last value
  /// wins. Use [decodeJson] when duplicate-key and number-lexeme preservation is
  /// required.
  Object? decode(String source) {
    _validateConfiguration();
    _validateInput(source);
    return jsonNodeToDart(
      LonTextParser(
        source,
        maxDepth: maxDepth,
        maxExpandedElements: maxExpandedElements,
      ).parse(),
    );
  }

  /// Converts JSON text to canonical LON while preserving member order,
  /// duplicate keys, and number lexemes.
  String encodeJson(String source) {
    _validateConfiguration();
    _validateInput(source);
    return _newWriter().write(
      JsonTextParser(
        source,
        maxDepth: maxDepth,
        maxExpandedElements: maxExpandedElements,
      ).parse(),
    );
  }

  LonWriter _newWriter() => LonWriter(
        useTables: useTables,
        pretty: pretty,
        maxOutputCodeUnits: maxOutputCodeUnits,
      );

  /// Converts LON to JSON while preserving member order, duplicate keys,
  /// and number lexemes. Output is compact unless [pretty] selects standard
  /// two-space indentation.
  String decodeJson(String source) {
    _validateConfiguration();
    _validateInput(source);
    final node = LonTextParser(
      source,
      maxDepth: maxDepth,
      maxExpandedElements: maxExpandedElements,
    ).parse();
    if (pretty) {
      return writePrettyJson(node, maxCodeUnits: maxOutputCodeUnits);
    }
    return writeJson(node, maxCodeUnits: maxOutputCodeUnits);
  }

  void _validateConfiguration() {
    RangeError.checkNotNegative(maxDepth, 'maxDepth');
    RangeError.checkNotNegative(maxExpandedElements, 'maxExpandedElements');
    RangeError.checkNotNegative(maxInputCodeUnits, 'maxInputCodeUnits');
    RangeError.checkNotNegative(maxOutputCodeUnits, 'maxOutputCodeUnits');
  }

  void _validateInput(String source) {
    if (source.length > maxInputCodeUnits) {
      throw FormatException(
        'Input code-unit limit of $maxInputCodeUnits exceeded',
      );
    }
  }
}

/// Default canonical LON v1 codec.
const lon = LonCodec();

/// Converts JSON text to canonical Language Object Notation.
String jsonToLon(
  String source, {
  bool useTables = true,
  bool pretty = false,
  int maxDepth = 512,
  int maxExpandedElements = 1000000,
  int maxInputCodeUnits = 64 * 1024 * 1024,
  int maxOutputCodeUnits = 64 * 1024 * 1024,
}) =>
    LonCodec(
      useTables: useTables,
      pretty: pretty,
      maxDepth: maxDepth,
      maxExpandedElements: maxExpandedElements,
      maxInputCodeUnits: maxInputCodeUnits,
      maxOutputCodeUnits: maxOutputCodeUnits,
    ).encodeJson(source);

/// Converts Language Object Notation to compact JSON text.
String lonToJson(
  String source, {
  int maxDepth = 512,
  int maxExpandedElements = 1000000,
  int maxInputCodeUnits = 64 * 1024 * 1024,
  int maxOutputCodeUnits = 64 * 1024 * 1024,
}) =>
    LonCodec(
      maxDepth: maxDepth,
      maxExpandedElements: maxExpandedElements,
      maxInputCodeUnits: maxInputCodeUnits,
      maxOutputCodeUnits: maxOutputCodeUnits,
    ).decodeJson(source);

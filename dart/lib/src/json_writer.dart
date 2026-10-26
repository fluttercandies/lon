import 'json_node.dart';
import 'json_string.dart';

final class CodeUnitLimitedStringBuffer implements StringSink {
  CodeUnitLimitedStringBuffer(this.maxCodeUnits) {
    RangeError.checkNotNegative(maxCodeUnits, 'maxCodeUnits');
  }

  final int maxCodeUnits;
  final StringBuffer _buffer = StringBuffer();
  int _length = 0;

  @override
  void write(Object? object) {
    String value;
    if (object is String) {
      value = object;
    } else {
      value = object?.toString() ?? 'null';
    }
    _reserve(value.length);
    _buffer.write(value);
    _length += value.length;
  }

  @override
  void writeAll(Iterable<Object?> objects, [String separator = '']) {
    var isFirst = true;
    for (final object in objects) {
      if (!isFirst) {
        write(separator);
      }
      write(object);
      isFirst = false;
    }
  }

  @override
  void writeCharCode(int charCode) {
    final count = charCode > 0xFFFF ? 2 : 1;
    _reserve(count);
    _buffer.writeCharCode(charCode);
    _length += count;
  }

  @override
  void writeln([Object? object = '']) {
    final value = object?.toString() ?? 'null';
    _reserve(value.length + 1);
    _buffer
      ..write(value)
      ..writeCharCode(0x0A);
    _length += value.length + 1;
  }

  void _reserve(int count) {
    if (count > maxCodeUnits - _length) {
      throw FormatException(
        'Output code-unit limit of $maxCodeUnits exceeded',
      );
    }
  }

  @override
  String toString() => _buffer.toString();
}

String writeJson(JsonNode node, {required int maxCodeUnits}) {
  final buffer = CodeUnitLimitedStringBuffer(maxCodeUnits);
  writeJsonNode(buffer, node, 0, false);
  return buffer.toString();
}

/// Serializes the decoded tree as standard indented JSON for the
/// `lon decode --pretty` CLI mode; the lossless [writeJson] stays compact.
String writePrettyJson(JsonNode node, {required int maxCodeUnits}) {
  final buffer = CodeUnitLimitedStringBuffer(maxCodeUnits);
  writeJsonNode(buffer, node, 0, true);
  return buffer.toString();
}

void writeJsonNode(
  StringSink buffer,
  JsonNode node,
  int depth,
  bool pretty,
) {
  switch (node) {
    case JsonNullNode():
      buffer.write('null');
    case JsonBooleanNode(:final value):
      buffer.write(value ? 'true' : 'false');
    case JsonNumberNode(:final lexeme):
      buffer.write(lexeme);
    case JsonStringNode(:final value):
      writeJsonString(buffer, value);
    case JsonArrayNode(:final values):
      buffer.writeCharCode(0x5B);
      final multiline = pretty && values.isNotEmpty;
      for (var index = 0; index < values.length; index++) {
        if (index != 0) {
          buffer.writeCharCode(0x2C);
        }
        if (multiline) {
          _writeJsonLineBreak(buffer, depth + 1);
        }
        writeJsonNode(buffer, values[index], depth + 1, pretty);
      }
      if (multiline) {
        _writeJsonLineBreak(buffer, depth);
      }
      buffer.writeCharCode(0x5D);
    case JsonObjectNode(:final members):
      buffer.writeCharCode(0x7B);
      final multiline = pretty && members.isNotEmpty;
      for (var index = 0; index < members.length; index++) {
        if (index != 0) {
          buffer.writeCharCode(0x2C);
        }
        if (multiline) {
          _writeJsonLineBreak(buffer, depth + 1);
        }
        final member = members[index];
        writeJsonString(buffer, member.key);
        buffer.writeCharCode(0x3A);
        if (pretty) {
          buffer.writeCharCode(0x20);
        }
        writeJsonNode(buffer, member.value, depth + 1, pretty);
      }
      if (multiline) {
        _writeJsonLineBreak(buffer, depth);
      }
      buffer.writeCharCode(0x7D);
  }
}

void _writeJsonLineBreak(StringSink buffer, int depth) {
  buffer.writeCharCode(0x0A);
  for (var index = 0; index < depth; index++) {
    buffer.write('  ');
  }
}

void writeJsonString(StringSink buffer, String value) {
  buffer.writeCharCode(0x22);
  final length = value.length;
  var index = 0;
  while (index < length) {
    // Bulk-write the common run of plain, unescaped code units.
    final runStart = index;
    while (index < length) {
      final code = value.codeUnitAt(index);
      if (code < 0x20 ||
          code == 0x22 ||
          code == 0x5C ||
          (code >= 0xD800 && code <= 0xDFFF) ||
          requiresJsonEscapeForVisibility(code)) {
        break;
      }
      index++;
    }
    if (index > runStart) {
      buffer.write(value.substring(runStart, index));
    }
    if (index >= length) {
      break;
    }

    final code = value.codeUnitAt(index);
    switch (code) {
      case 0x22:
        buffer.write(r'\"');
      case 0x5C:
        buffer.write(r'\\');
      case 0x08:
        buffer.write(r'\b');
      case 0x09:
        buffer.write(r'\t');
      case 0x0A:
        buffer.write(r'\n');
      case 0x0C:
        buffer.write(r'\f');
      case 0x0D:
        buffer.write(r'\r');
      default:
        if (code >= 0xD800 && code <= 0xDBFF && index + 1 < length) {
          final low = value.codeUnitAt(index + 1);
          if (low >= 0xDC00 && low <= 0xDFFF) {
            final rune = 0x10000 + ((code - 0xD800) << 10) + low - 0xDC00;
            if (requiresJsonEscapeForVisibility(rune)) {
              _writeUnicodeEscape(buffer, code);
              _writeUnicodeEscape(buffer, low);
            } else {
              buffer.writeCharCode(rune);
            }
            // Advance past the high surrogate; the loop tail then skips the
            // low one.
            index++;
            break;
          }
        }
        // Lone surrogate, control, or visibility-escaped code unit.
        _writeUnicodeEscape(buffer, code);
    }
    index++;
  }
  buffer.writeCharCode(0x22);
}

int utf8Length(String value) {
  var length = 0;
  for (var index = 0; index < value.length; index++) {
    final code = value.codeUnitAt(index);
    if (code <= 0x7F) {
      length++;
    } else if (code <= 0x7FF) {
      length += 2;
    } else if (code >= 0xD800 && code <= 0xDBFF && index + 1 < value.length) {
      final low = value.codeUnitAt(index + 1);
      if (low >= 0xDC00 && low <= 0xDFFF) {
        length += 4;
        index++;
      } else {
        length += 3;
      }
    } else {
      length += 3;
    }
  }
  return length;
}

int jsonStringUtf8Length(String value) {
  var length = 2;
  for (var index = 0; index < value.length; index++) {
    final code = value.codeUnitAt(index);
    switch (code) {
      case 0x22 || 0x5C || 0x08 || 0x09 || 0x0A || 0x0C || 0x0D:
        length += 2;
      default:
        if (requiresJsonEscapeForVisibility(code)) {
          length += 6;
        } else if (code >= 0xD800 && code <= 0xDBFF) {
          if (index + 1 < value.length) {
            final low = value.codeUnitAt(index + 1);
            if (low >= 0xDC00 && low <= 0xDFFF) {
              final rune = 0x10000 + ((code - 0xD800) << 10) + low - 0xDC00;
              length += requiresJsonEscapeForVisibility(rune) ? 12 : 4;
              index++;
              continue;
            }
          }
          length += 6;
        } else if (code >= 0xDC00 && code <= 0xDFFF) {
          length += 6;
        } else if (code <= 0x7F) {
          length++;
        } else if (code <= 0x7FF) {
          length += 2;
        } else {
          length += 3;
        }
    }
  }
  return length;
}

void _writeUnicodeEscape(StringSink buffer, int code) {
  const hex = '0123456789abcdef';
  buffer
    ..write(r'\u')
    ..writeCharCode(hex.codeUnitAt((code >> 12) & 0xF))
    ..writeCharCode(hex.codeUnitAt((code >> 8) & 0xF))
    ..writeCharCode(hex.codeUnitAt((code >> 4) & 0xF))
    ..writeCharCode(hex.codeUnitAt(code & 0xF));
}

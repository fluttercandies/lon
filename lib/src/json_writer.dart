import 'json_node.dart';
import 'json_string.dart';

final class CodeUnitLimitedStringBuffer implements StringSink {
  CodeUnitLimitedStringBuffer(this.maxCodeUnits) {
    RangeError.checkNotNegative(maxCodeUnits, 'maxCodeUnits');
  }

  final int maxCodeUnits;
  final StringBuffer _buffer = StringBuffer();

  int get length => _buffer.length;

  @override
  void write(Object? object) {
    final value = object?.toString() ?? 'null';
    _reserve(value.length);
    _buffer.write(value);
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
    _reserve(charCode > 0xFFFF ? 2 : 1);
    _buffer.writeCharCode(charCode);
  }

  @override
  void writeln([Object? object = '']) {
    final value = object?.toString() ?? 'null';
    _reserve(value.length + 1);
    _buffer
      ..write(value)
      ..writeCharCode(0x0A);
  }

  void _reserve(int count) {
    if (count > maxCodeUnits - _buffer.length) {
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
  writeJsonNode(buffer, node);
  return buffer.toString();
}

void writeJsonNode(StringSink buffer, JsonNode node) {
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
      for (var index = 0; index < values.length; index++) {
        if (index != 0) {
          buffer.writeCharCode(0x2C);
        }
        writeJsonNode(buffer, values[index]);
      }
      buffer.writeCharCode(0x5D);
    case JsonObjectNode(:final members):
      buffer.writeCharCode(0x7B);
      for (var index = 0; index < members.length; index++) {
        if (index != 0) {
          buffer.writeCharCode(0x2C);
        }
        final member = members[index];
        writeJsonString(buffer, member.key);
        buffer.writeCharCode(0x3A);
        writeJsonNode(buffer, member.value);
      }
      buffer.writeCharCode(0x7D);
  }
}

void writeJsonString(StringSink buffer, String value) {
  buffer.writeCharCode(0x22);
  for (var index = 0; index < value.length; index++) {
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
        if (requiresJsonEscapeForVisibility(code)) {
          _writeUnicodeEscape(buffer, code);
        } else if (code >= 0xD800 && code <= 0xDBFF) {
          if (index + 1 < value.length) {
            final low = value.codeUnitAt(index + 1);
            if (low >= 0xDC00 && low <= 0xDFFF) {
              final rune = 0x10000 + ((code - 0xD800) << 10) + low - 0xDC00;
              if (requiresJsonEscapeForVisibility(rune)) {
                _writeUnicodeEscape(buffer, code);
                _writeUnicodeEscape(buffer, low);
              } else {
                buffer.writeCharCode(rune);
              }
              index++;
              continue;
            }
          }
          _writeUnicodeEscape(buffer, code);
        } else if (code >= 0xDC00 && code <= 0xDFFF) {
          _writeUnicodeEscape(buffer, code);
        } else {
          buffer.writeCharCode(code);
        }
    }
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

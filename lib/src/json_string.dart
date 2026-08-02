final class ParsedJsonString {
  const ParsedJsonString(this.value, this.nextOffset);

  final String value;
  final int nextOffset;
}

bool requiresJsonEscapeForVisibility(int rune) {
  if (rune < 0x20 || (rune >= 0x7F && rune <= 0x9F)) {
    return true;
  }
  if ((rune >= 0xFDD0 && rune <= 0xFDEF) ||
      (rune <= 0x10FFFF && (rune & 0xFFFE) == 0xFFFE)) {
    return true;
  }
  return rune == 0x00AD ||
      rune == 0x034F ||
      rune == 0x061C ||
      rune == 0x180E ||
      rune == 0x200B ||
      rune == 0x200E ||
      rune == 0x200F ||
      (rune >= 0x2028 && rune <= 0x202E) ||
      (rune >= 0x2060 && rune <= 0x206F) ||
      rune == 0xFEFF ||
      (rune >= 0xFFF9 && rune <= 0xFFFB) ||
      (rune >= 0x13430 && rune <= 0x13455) ||
      (rune >= 0x1BCA0 && rune <= 0x1BCA3) ||
      (rune >= 0x1D173 && rune <= 0x1D17A) ||
      (rune >= 0xE0000 && rune <= 0xE007F);
}

ParsedJsonString parseJsonStringAt(
  String source,
  int offset, {
  bool requireVisibleLiterals = false,
}) {
  if (offset >= source.length || source.codeUnitAt(offset) != 0x22) {
    throw FormatException('Expected a JSON string', source, offset);
  }

  var cursor = offset + 1;
  var segmentStart = cursor;
  StringBuffer? buffer;

  while (cursor < source.length) {
    final code = source.codeUnitAt(cursor);
    if (code == 0x22) {
      final end = cursor;
      cursor++;
      if (buffer == null) {
        return ParsedJsonString(source.substring(segmentStart, end), cursor);
      }
      buffer.write(source.substring(segmentStart, end));
      return ParsedJsonString(buffer.toString(), cursor);
    }
    if (code < 0x20) {
      throw FormatException(
        'Unescaped control character in JSON string',
        source,
        cursor,
      );
    }
    if (code != 0x5C) {
      if (requireVisibleLiterals && !_isVisibleLiteralAt(source, cursor)) {
        throw FormatException(
          'Invisible or unpaired code point must use a Unicode escape',
          source,
          cursor,
        );
      }
      cursor++;
      continue;
    }

    buffer ??= StringBuffer();
    buffer.write(source.substring(segmentStart, cursor));
    cursor++;
    if (cursor >= source.length) {
      throw FormatException('Incomplete JSON escape sequence', source, cursor);
    }

    final escape = source.codeUnitAt(cursor++);
    switch (escape) {
      case 0x22:
        buffer.writeCharCode(0x22);
      case 0x5C:
        buffer.writeCharCode(0x5C);
      case 0x2F:
        buffer.writeCharCode(0x2F);
      case 0x62:
        buffer.writeCharCode(0x08);
      case 0x66:
        buffer.writeCharCode(0x0C);
      case 0x6E:
        buffer.writeCharCode(0x0A);
      case 0x72:
        buffer.writeCharCode(0x0D);
      case 0x74:
        buffer.writeCharCode(0x09);
      case 0x75:
        if (cursor + 4 > source.length) {
          throw FormatException(
            'Incomplete Unicode escape sequence',
            source,
            cursor,
          );
        }
        var value = 0;
        for (var index = 0; index < 4; index++) {
          final digit = _hexValue(source.codeUnitAt(cursor++));
          if (digit < 0) {
            throw FormatException(
              'Invalid Unicode escape sequence',
              source,
              cursor - 1,
            );
          }
          value = (value << 4) | digit;
        }
        buffer.writeCharCode(value);
      default:
        throw FormatException(
            'Invalid JSON escape sequence', source, cursor - 1);
    }
    segmentStart = cursor;
  }

  throw FormatException('Unterminated JSON string', source, cursor);
}

bool _isVisibleLiteralAt(String source, int offset) {
  final code = source.codeUnitAt(offset);
  if (code >= 0xD800 && code <= 0xDBFF) {
    if (offset + 1 >= source.length) {
      return false;
    }
    final low = source.codeUnitAt(offset + 1);
    if (low < 0xDC00 || low > 0xDFFF) {
      return false;
    }
    final rune = 0x10000 + ((code - 0xD800) << 10) + low - 0xDC00;
    return !requiresJsonEscapeForVisibility(rune);
  }
  if (code >= 0xDC00 && code <= 0xDFFF) {
    return offset > 0 &&
        source.codeUnitAt(offset - 1) >= 0xD800 &&
        source.codeUnitAt(offset - 1) <= 0xDBFF;
  }
  return !requiresJsonEscapeForVisibility(code);
}

int _hexValue(int code) {
  if (code >= 0x30 && code <= 0x39) {
    return code - 0x30;
  }
  if (code >= 0x41 && code <= 0x46) {
    return code - 0x41 + 10;
  }
  if (code >= 0x61 && code <= 0x66) {
    return code - 0x61 + 10;
  }
  return -1;
}

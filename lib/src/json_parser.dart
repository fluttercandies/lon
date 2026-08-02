import 'json_node.dart';
import 'json_string.dart';

final class JsonTextParser {
  JsonTextParser(
    this.source, {
    this.maxDepth = 512,
    this.maxExpandedElements = 1000000,
  })  : assert(maxDepth >= 0),
        assert(maxExpandedElements >= 0);

  final String source;
  final int maxDepth;
  final int maxExpandedElements;

  int _offset = 0;
  int _expandedElements = 0;

  JsonNode parse() {
    _skipWhitespace();
    if (_isAtEnd) {
      _fail('Expected a JSON value');
    }

    final value = _parseValue(0);
    _skipWhitespace();
    if (!_isAtEnd) {
      _fail('Unexpected trailing content');
    }
    return value;
  }

  JsonNode _parseValue(int depth) {
    if (depth > maxDepth) {
      _fail('Maximum nesting depth of $maxDepth exceeded');
    }
    if (_isAtEnd) {
      _fail('Expected a JSON value');
    }

    return switch (source.codeUnitAt(_offset)) {
      0x22 => JsonStringNode(parseString()),
      0x5B => _parseArray(depth),
      0x7B => _parseObject(depth),
      0x74 => _parseLiteral('true', const JsonBooleanNode(true)),
      0x66 => _parseLiteral('false', const JsonBooleanNode(false)),
      0x6E => _parseLiteral('null', jsonNullNode),
      0x2D => _parseNumber(),
      final code when code >= 0x30 && code <= 0x39 => _parseNumber(),
      _ => _unexpectedValue(),
    };
  }

  JsonNode _unexpectedValue() {
    _fail('Expected a JSON value');
  }

  JsonNode _parseLiteral(String literal, JsonNode value) {
    if (!source.startsWith(literal, _offset)) {
      _fail('Invalid JSON literal');
    }
    _offset += literal.length;
    return value;
  }

  JsonArrayNode _parseArray(int depth) {
    _offset++;
    _skipWhitespace();

    final values = <JsonNode>[];
    if (_consumeIf(0x5D)) {
      return JsonArrayNode(values);
    }

    while (true) {
      _reserve();
      values.add(_parseValue(depth + 1));
      _skipWhitespace();
      if (_consumeIf(0x5D)) {
        return JsonArrayNode(values);
      }
      _expect(0x2C, 'Expected a comma or closing bracket');
      _skipWhitespace();
    }
  }

  JsonObjectNode _parseObject(int depth) {
    _offset++;
    _skipWhitespace();

    final members = <JsonMember>[];
    if (_consumeIf(0x7D)) {
      return JsonObjectNode(members);
    }

    while (true) {
      if (_isAtEnd || source.codeUnitAt(_offset) != 0x22) {
        _fail('Expected a quoted object key');
      }
      final key = parseString();
      _skipWhitespace();
      _expect(0x3A, 'Expected a colon after the object key');
      _skipWhitespace();
      _reserve();
      members.add(JsonMember(key, _parseValue(depth + 1)));
      _skipWhitespace();
      if (_consumeIf(0x7D)) {
        return JsonObjectNode(members);
      }
      _expect(0x2C, 'Expected a comma or closing brace');
      _skipWhitespace();
    }
  }

  JsonNumberNode _parseNumber() {
    final start = _offset;

    _consumeIf(0x2D);
    if (_isAtEnd) {
      _fail('Incomplete JSON number');
    }

    final first = source.codeUnitAt(_offset);
    if (first == 0x30) {
      _offset++;
      if (!_isAtEnd && _isDigit(source.codeUnitAt(_offset))) {
        _fail('Leading zeroes are not allowed in JSON numbers');
      }
    } else if (first >= 0x31 && first <= 0x39) {
      _offset++;
      while (!_isAtEnd && _isDigit(source.codeUnitAt(_offset))) {
        _offset++;
      }
    } else {
      _fail('Expected a digit in JSON number');
    }

    if (!_isAtEnd && source.codeUnitAt(_offset) == 0x2E) {
      _offset++;
      if (_isAtEnd || !_isDigit(source.codeUnitAt(_offset))) {
        _fail('Expected a digit after the decimal point');
      }
      while (!_isAtEnd && _isDigit(source.codeUnitAt(_offset))) {
        _offset++;
      }
    }

    if (!_isAtEnd) {
      final exponent = source.codeUnitAt(_offset);
      if (exponent == 0x65 || exponent == 0x45) {
        _offset++;
        if (!_isAtEnd) {
          final sign = source.codeUnitAt(_offset);
          if (sign == 0x2B || sign == 0x2D) {
            _offset++;
          }
        }
        if (_isAtEnd || !_isDigit(source.codeUnitAt(_offset))) {
          _fail('Expected an exponent digit');
        }
        while (!_isAtEnd && _isDigit(source.codeUnitAt(_offset))) {
          _offset++;
        }
      }
    }

    return JsonNumberNode(source.substring(start, _offset));
  }

  String parseString() {
    final parsed = parseJsonStringAt(source, _offset);
    _offset = parsed.nextOffset;
    return parsed.value;
  }

  void _skipWhitespace() {
    while (!_isAtEnd) {
      final code = source.codeUnitAt(_offset);
      if (code != 0x20 && code != 0x09 && code != 0x0A && code != 0x0D) {
        return;
      }
      _offset++;
    }
  }

  bool _consumeIf(int code) {
    if (!_isAtEnd && source.codeUnitAt(_offset) == code) {
      _offset++;
      return true;
    }
    return false;
  }

  void _expect(int code, String message) {
    if (!_consumeIf(code)) {
      _fail(message);
    }
  }

  void _reserve() {
    if (_expandedElements >= maxExpandedElements) {
      _fail('Parsed element limit of $maxExpandedElements exceeded');
    }
    _expandedElements++;
  }

  Never _fail(String message) {
    throw FormatException(message, source, _offset);
  }

  bool get _isAtEnd => _offset >= source.length;

  static bool _isDigit(int code) => code >= 0x30 && code <= 0x39;
}

import 'json_node.dart';
import 'json_string.dart';
import 'lon_syntax.dart';

final class LonTextParser {
  LonTextParser(
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
  int _schemaFields = 0;

  JsonNode parse() {
    _skipWhitespace();
    if (_isAtEnd) {
      _fail('Expected a LON value');
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
    _skipWhitespace();
    if (_isAtEnd) {
      _fail('Expected a LON value');
    }

    switch (source.codeUnitAt(_offset)) {
      case 0x22:
        return JsonStringNode(_parseQuotedString());
      case 0x5B:
        return _parseArray(depth);
      case 0x7B:
        return _parseObject(depth);
    }

    final tokenOffset = _offset;
    final token = _readBareToken();
    switch (token) {
      case 'null':
        return jsonNullNode;
      case 'true':
        return const JsonBooleanNode(true);
      case 'false':
        return const JsonBooleanNode(false);
    }

    if (isJsonNumberToken(token)) {
      return JsonNumberNode(token);
    }
    if (!canUseBareLonString(token)) {
      _failAt('Ambiguous or invalid bare string must be quoted', tokenOffset);
    }
    return JsonStringNode(token);
  }

  bool _hasRecordTableHeader() => _hasSchemaTerminator(_offset);

  bool _hasSchemaTerminator(int cursor) {
    var nestedSchemaDepth = 0;

    while (cursor < source.length) {
      final code = source.codeUnitAt(cursor);
      if (code == 0x22) {
        cursor++;
        while (cursor < source.length) {
          final quotedCode = source.codeUnitAt(cursor++);
          if (quotedCode == 0x5C && cursor < source.length) {
            cursor++;
          } else if (quotedCode == 0x22) {
            break;
          }
        }
        continue;
      }
      if (code == 0x7B) {
        nestedSchemaDepth++;
        cursor++;
        continue;
      }
      if (code == 0x7D) {
        if (nestedSchemaDepth == 0) {
          return false;
        }
        nestedSchemaDepth--;
        cursor++;
        continue;
      }
      if (nestedSchemaDepth == 0) {
        if (code == 0x3B) {
          return true;
        }
        if (code == 0x28 ||
            code == 0x29 ||
            code == 0x2C ||
            code == 0x3A ||
            code == 0x5B ||
            code == 0x5D ||
            code == 0x7C) {
          return false;
        }
      }
      cursor++;
    }
    return false;
  }

  JsonArrayNode _parseArray(int depth) {
    final tableOffset = _offset;
    _offset++;
    if (_hasRecordTableHeader()) {
      return _parseRecordTable(depth, tableOffset);
    }

    _skipWhitespace();
    if (_consumeIf(0x5D)) {
      return const JsonArrayNode(<JsonNode>[]);
    }

    final values = <JsonNode>[];
    while (true) {
      _reserve(1);
      values.add(_parseValue(depth + 1));

      final hadWhitespace = _skipWhitespace();
      if (_consumeIf(0x5D)) {
        return JsonArrayNode(values);
      }
      if (_consumeIf(0x2C)) {
        _skipWhitespace();
        if (_isAtEnd || source.codeUnitAt(_offset) == 0x5D) {
          _fail('Expected an array value after comma');
        }
        continue;
      }
      if (!hadWhitespace) {
        _fail('Expected whitespace, comma, or ] after array value');
      }
    }
  }

  JsonObjectNode _parseObject(int depth) {
    _offset++;
    _skipWhitespace();
    if (_consumeIf(0x7D)) {
      return const JsonObjectNode(<JsonMember>[]);
    }

    final members = <JsonMember>[];
    while (true) {
      final key = _parseObjectKey();
      _reserve(1);
      members.add(JsonMember(key, _parseValue(depth + 1)));

      final hadWhitespace = _skipWhitespace();
      if (_consumeIf(0x7D)) {
        return JsonObjectNode(members);
      }
      if (!hadWhitespace) {
        _fail('Expected whitespace or } after object field');
      }
    }
  }

  JsonArrayNode _parseRecordTable(int depth, int tableOffset) {
    final schema = _parseSchemaHeader(depth, tableOffset);
    final rows = <JsonNode>[];
    _skipWhitespace();
    if (_consumeIf(0x5D)) {
      _failAt('A table must contain at least two rows', tableOffset);
    }

    while (true) {
      _reserve(schema.fieldCount + 1);
      rows.add(_parseSchemaRow(schema, depth + 1));

      _skipWhitespace();
      if (_consumeIf(0x5D)) {
        if (rows.length < 2) {
          _failAt('A table must contain at least two rows', tableOffset);
        }
        return JsonArrayNode(rows);
      }
      _expect(0x3B, 'Expected a semicolon between table rows');
      _skipWhitespace();
      if (_isAtEnd || source.codeUnitAt(_offset) == 0x5D) {
        _fail('Expected a table row after semicolon');
      }
    }
  }

  _RecordSchema _parseSchemaHeader(int depth, int tableOffset) {
    final schema = _parseSchemaFields(0x3B, depth, 1, tableOffset);
    if (schema.fields.isEmpty || schema.valueCount < 2) {
      _failAt(
        'A table header must contain at least two value fields',
        tableOffset,
      );
    }
    if (depth + schema.maxPathDepth + 1 > maxDepth) {
      _failAt('Maximum nesting depth of $maxDepth exceeded', tableOffset);
    }
    return schema;
  }

  _RecordSchema _parseSchemaFields(
    int terminator,
    int tableDepth,
    int pathDepth,
    int tableOffset,
  ) {
    if (tableDepth + pathDepth > maxDepth) {
      _failAt('Maximum nesting depth of $maxDepth exceeded', tableOffset);
    }

    final fields = <_SchemaField>[];
    _skipWhitespace();

    while (!_isAtEnd && source.codeUnitAt(_offset) != terminator) {
      _reserveSchemaField(tableOffset);
      final key = _parseSchemaKey();
      _RecordSchema? nested;
      if (!_isAtEnd && source.codeUnitAt(_offset) == 0x7B) {
        _offset++;
        nested = _parseSchemaFields(
          0x7D,
          tableDepth,
          pathDepth + 1,
          tableOffset,
        );
      }
      fields.add(_SchemaField(key, nested));

      final hadWhitespace = _skipWhitespace();
      if (!_isAtEnd && source.codeUnitAt(_offset) == terminator) {
        break;
      }
      if (!hadWhitespace) {
        _fail('Schema fields must be separated by whitespace');
      }
    }

    _expect(terminator, 'Unterminated table schema');
    return _RecordSchema(fields);
  }

  String _parseSchemaKey() {
    _skipWhitespace();
    if (_isAtEnd) {
      _fail('Expected a table field name');
    }
    if (source.codeUnitAt(_offset) == 0x22) {
      return _parseQuotedString();
    }

    final keyOffset = _offset;
    final key = _readBareToken();
    if (!canUseBareLonKey(key)) {
      _failAt('Invalid bare table field name must be quoted', keyOffset);
    }
    return key;
  }

  JsonObjectNode _parseSchemaRow(_RecordSchema schema, int depth) =>
      _parseSchemaObject(schema, depth, _RowState());

  JsonObjectNode _parseSchemaObject(
    _RecordSchema schema,
    int depth,
    _RowState state,
  ) {
    final members = <JsonMember>[];
    for (final field in schema.fields) {
      final nested = field.nested;
      if (nested == null) {
        if (state.hasValue) {
          if (!_skipWhitespace()) {
            _fail('Table fields must be separated by whitespace');
          }
        } else {
          _skipWhitespace();
        }
        members.add(JsonMember(field.key, _parseValue(depth + 1)));
        state.hasValue = true;
      } else {
        members.add(
          JsonMember(
            field.key,
            _parseSchemaObject(nested, depth + 1, state),
          ),
        );
      }
    }
    return JsonObjectNode(members);
  }

  String _parseObjectKey() {
    _skipWhitespace();
    if (_isAtEnd) {
      _fail('Expected an object key');
    }

    final key = source.codeUnitAt(_offset) == 0x22
        ? _parseQuotedString()
        : _readKeyBeforeColon();
    _skipWhitespace();
    _expect(0x3A, 'Expected a colon after the object key');
    return key;
  }

  String _readKeyBeforeColon() {
    final start = _offset;
    while (!_isAtEnd) {
      final code = source.codeUnitAt(_offset);
      if (code == 0x3A || isLonWhitespace(code)) {
        break;
      }
      if (code == 0x28 ||
          code == 0x29 ||
          code == 0x2C ||
          code == 0x3B ||
          code == 0x5B ||
          code == 0x5D ||
          code == 0x7B ||
          code == 0x7D) {
        _fail('Invalid character in bare object key');
      }
      _offset++;
    }
    if (start == _offset) {
      _fail('Expected an object key');
    }
    final key = source.substring(start, _offset);
    if (!canUseBareLonKey(key)) {
      _failAt('Invalid bare object key must be quoted', start);
    }
    return key;
  }

  String _parseQuotedString() {
    final parsed = parseJsonStringAt(
      source,
      _offset,
      requireVisibleLiterals: true,
    );
    _offset = parsed.nextOffset;
    return parsed.value;
  }

  String _readBareToken() {
    final start = _offset;
    while (!_isAtEnd && !isLonTokenDelimiter(source.codeUnitAt(_offset))) {
      _offset++;
    }
    if (start == _offset) {
      _fail('Expected a LON token');
    }
    return source.substring(start, _offset);
  }

  bool _skipWhitespace() {
    final start = _offset;
    while (!_isAtEnd && isLonWhitespace(source.codeUnitAt(_offset))) {
      _offset++;
    }
    return _offset != start;
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

  void _reserveSchemaField(int tableOffset) {
    if (_schemaFields >= maxExpandedElements) {
      _failAt(
        'Schema field limit of $maxExpandedElements exceeded',
        tableOffset,
      );
    }
    _schemaFields++;
  }

  void _reserve(int count) {
    if (count > maxExpandedElements - _expandedElements) {
      _fail('Expanded element limit of $maxExpandedElements exceeded');
    }
    _expandedElements += count;
  }

  Never _fail(String message) {
    throw FormatException(message, source, _offset);
  }

  Never _failAt(String message, int offset) {
    throw FormatException(message, source, offset);
  }

  bool get _isAtEnd => _offset >= source.length;
}

final class _RowState {
  bool hasValue = false;
}

final class _RecordSchema {
  _RecordSchema(this.fields)
      : fieldCount = _countFields(fields),
        valueCount = _countValues(fields),
        maxPathDepth = _calculateMaxPathDepth(fields);

  final List<_SchemaField> fields;
  final int fieldCount;
  final int valueCount;
  final int maxPathDepth;

  static int _countFields(List<_SchemaField> fields) {
    var count = fields.length;
    for (final field in fields) {
      count += field.nested?.fieldCount ?? 0;
    }
    return count;
  }

  static int _countValues(List<_SchemaField> fields) {
    var count = 0;
    for (final field in fields) {
      final nested = field.nested;
      count += nested?.valueCount ?? 1;
    }
    return count;
  }

  static int _calculateMaxPathDepth(List<_SchemaField> fields) {
    var depth = fields.isEmpty ? 0 : 1;
    for (final field in fields) {
      final nested = field.nested;
      if (nested == null) {
        continue;
      }
      final candidate = nested.maxPathDepth + 1;
      if (candidate > depth) {
        depth = candidate;
      }
    }
    return depth;
  }
}

final class _SchemaField {
  const _SchemaField(this.key, this.nested);

  final String key;
  final _RecordSchema? nested;
}

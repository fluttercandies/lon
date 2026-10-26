import 'json_node.dart';
import 'json_writer.dart';
import 'lon_syntax.dart';

final class LonWriter {
  const LonWriter({
    this.useTables = true,
    this.pretty = false,
    required this.maxOutputCodeUnits,
  });

  final bool useTables;

  /// Renders containers across lines, indented two spaces per nesting
  /// depth: every object member, table row, and container-array element
  /// occupies its own line between the container's brackets. Table rows and
  /// scalar arrays stay inline. The output decodes to exactly the same value
  /// as the single-line form.
  final bool pretty;
  final int maxOutputCodeUnits;

  String write(JsonNode node) {
    final buffer = CodeUnitLimitedStringBuffer(maxOutputCodeUnits);
    _writeValue(buffer, node, 0, false);
    return buffer.toString();
  }

  void _writeValue(
    StringSink buffer,
    JsonNode node,
    int depth,
    bool inline,
  ) {
    switch (node) {
      case JsonNullNode():
        buffer.write('null');
      case JsonBooleanNode(:final value):
        buffer.write(value ? 'true' : 'false');
      case JsonNumberNode(:final lexeme):
        buffer.write(lexeme);
      case JsonStringNode(:final value):
        _writeString(buffer, value);
      case JsonArrayNode():
        if (useTables && _writeRecordTable(buffer, node, depth, inline)) {
          return;
        }
        _writeArray(buffer, node, depth, inline);
      case JsonObjectNode():
        _writeObject(buffer, node, depth, inline);
    }
  }

  void _writeArray(
    StringSink buffer,
    JsonArrayNode node,
    int depth,
    bool inline,
  ) {
    buffer.writeCharCode(0x5B);
    var allBareStrings = node.values.isNotEmpty;
    for (final value in node.values) {
      if (value is JsonStringNode && canUseBareLonString(value.value)) {
        continue;
      }
      allBareStrings = false;
      break;
    }
    if (allBareStrings) {
      // Space-separated bare strings; the scan already validated every
      // element, so values are written without revalidation.
      for (var index = 0; index < node.values.length; index++) {
        if (index != 0) {
          buffer.writeCharCode(0x20);
        }
        buffer.write((node.values[index] as JsonStringNode).value);
      }
      buffer.writeCharCode(0x5D);
      return;
    }

    final multiline = pretty && !inline && node.values.any(_isContainer);
    for (var index = 0; index < node.values.length; index++) {
      if (multiline) {
        _writeLineBreak(buffer, depth + 1);
      } else if (index != 0) {
        buffer.writeCharCode(0x2C);
      }
      _writeValue(buffer, node.values[index], depth + 1, inline);
      if (multiline && index != node.values.length - 1) {
        buffer.writeCharCode(0x2C);
      }
    }
    if (multiline) {
      _writeLineBreak(buffer, depth);
    }
    buffer.writeCharCode(0x5D);
  }

  void _writeObject(
    StringSink buffer,
    JsonObjectNode node,
    int depth,
    bool inline,
  ) {
    buffer.writeCharCode(0x7B);
    final multiline = pretty && !inline && node.members.isNotEmpty;
    for (var index = 0; index < node.members.length; index++) {
      if (multiline) {
        _writeLineBreak(buffer, depth + 1);
      } else if (index != 0) {
        buffer.writeCharCode(0x20);
      }
      final member = node.members[index];
      _writeKey(buffer, member.key);
      buffer.writeCharCode(0x3A);
      if (multiline) {
        buffer.writeCharCode(0x20);
      }
      _writeValue(buffer, member.value, depth + 1, inline);
    }
    if (multiline) {
      _writeLineBreak(buffer, depth);
    }
    buffer.writeCharCode(0x7D);
  }

  bool _writeRecordTable(
    StringSink buffer,
    JsonArrayNode node,
    int depth,
    bool inline,
  ) {
    if (node.values.length < 2) {
      return false;
    }
    final rows = <JsonObjectNode>[];
    for (final value in node.values) {
      if (value is! JsonObjectNode) {
        return false;
      }
      rows.add(value);
    }

    final schema = _RecordSchema.infer(rows);
    if (schema == null ||
        schema.fields.isEmpty ||
        schema.valueCount < 2 ||
        !_shouldUseTable(node.values.length, schema)) {
      return false;
    }

    buffer.writeCharCode(0x5B);
    if (pretty && !inline) {
      _writeLineBreak(buffer, depth + 1);
      _writeSchema(buffer, schema);
      for (var index = 0; index < rows.length; index++) {
        buffer.writeCharCode(0x3B);
        _writeLineBreak(buffer, depth + 1);
        _writeRow(buffer, schema, rows[index], depth + 1);
      }
      _writeLineBreak(buffer, depth);
      buffer.writeCharCode(0x5D);
      return true;
    }

    _writeSchema(buffer, schema);
    for (final value in rows) {
      buffer.writeCharCode(0x3B);
      _writeRow(buffer, schema, value, depth + 1);
    }
    buffer.writeCharCode(0x5D);
    return true;
  }

  bool _shouldUseTable(int rowCount, _RecordSchema schema) {
    if (rowCount >= 3 || schema.fieldCount >= 3) {
      return true;
    }

    final ordinaryBytes =
        rowCount + 1 + rowCount * schema.objectStructuralByteLength;
    final tableBytes =
        2 + schema.encodedByteLength + rowCount * schema.valueCount;
    return tableBytes <= ordinaryBytes;
  }

  void _writeSchema(StringSink buffer, _RecordSchema schema) {
    for (var index = 0; index < schema.fields.length; index++) {
      if (index != 0) {
        buffer.writeCharCode(0x20);
      }
      final field = schema.fields[index];
      _writeKey(buffer, field.key);
      final nested = field.nested;
      if (nested != null) {
        buffer.writeCharCode(0x7B);
        _writeSchema(buffer, nested);
        buffer.writeCharCode(0x7D);
      }
    }
  }

  void _writeRow(
    StringSink buffer,
    _RecordSchema schema,
    JsonObjectNode value,
    int depth,
  ) {
    _writeRowValues(buffer, schema, value, depth, false);
  }

  // Row cells always render inline: a table row stays on one line even when
  // a cell holds a nested table or object.
  bool _writeRowValues(
    StringSink buffer,
    _RecordSchema schema,
    JsonObjectNode value,
    int depth,
    bool hasValue,
  ) {
    for (var index = 0; index < schema.fields.length; index++) {
      final field = schema.fields[index];
      final memberValue = value.members[index].value;
      final nested = field.nested;
      if (nested == null) {
        if (hasValue) {
          buffer.writeCharCode(0x20);
        }
        _writeValue(buffer, memberValue, depth, true);
        hasValue = true;
      } else {
        hasValue = _writeRowValues(
          buffer,
          nested,
          memberValue as JsonObjectNode,
          depth,
          hasValue,
        );
      }
    }
    return hasValue;
  }

  void _writeLineBreak(StringSink buffer, int depth) {
    buffer.writeCharCode(0x0A);
    for (var index = 0; index < depth; index++) {
      buffer.write('  ');
    }
  }

  void _writeString(StringSink buffer, String value) {
    if (canUseBareLonString(value)) {
      buffer.write(value);
    } else {
      writeJsonString(buffer, value);
    }
  }

  void _writeKey(StringSink buffer, String value) {
    if (canUseBareLonKey(value)) {
      buffer.write(value);
    } else {
      writeJsonString(buffer, value);
    }
  }
}

bool _isContainer(JsonNode node) =>
    node is JsonArrayNode || node is JsonObjectNode;

int _lonKeyByteLength(String value) =>
    canUseBareLonKey(value) ? utf8Length(value) : jsonStringUtf8Length(value);

final class _RecordSchema {
  factory _RecordSchema(List<_SchemaField> fields) {
    var fieldCount = fields.length;
    var encodedByteLength = fields.isEmpty ? 0 : fields.length - 1;
    var objectStructuralByteLength = 2;
    var valueCount = 0;

    for (var index = 0; index < fields.length; index++) {
      final field = fields[index];
      final keyLength = _lonKeyByteLength(field.key);
      encodedByteLength += keyLength;
      objectStructuralByteLength += keyLength + 1;
      if (index != 0) {
        objectStructuralByteLength++;
      }

      final nested = field.nested;
      if (nested == null) {
        valueCount++;
      } else {
        fieldCount += nested.fieldCount;
        encodedByteLength += 2 + nested.encodedByteLength;
        objectStructuralByteLength += nested.objectStructuralByteLength;
        valueCount += nested.valueCount;
      }
    }

    return _RecordSchema._(
      fields,
      fieldCount,
      encodedByteLength,
      objectStructuralByteLength,
      valueCount,
    );
  }

  const _RecordSchema._(
    this.fields,
    this.fieldCount,
    this.encodedByteLength,
    this.objectStructuralByteLength,
    this.valueCount,
  );

  static _RecordSchema? infer(List<JsonObjectNode> values) => _infer(values);

  static _RecordSchema? _infer(List<JsonObjectNode> values) {
    if (values.isEmpty) {
      return null;
    }
    final first = values.first;

    for (var row = 1; row < values.length; row++) {
      final value = values[row];
      if (value.members.length != first.members.length) {
        return null;
      }
      for (var index = 0; index < first.members.length; index++) {
        if (value.members[index].key != first.members[index].key) {
          return null;
        }
      }
    }

    final fields = <_SchemaField>[];
    for (var index = 0; index < first.members.length; index++) {
      _RecordSchema? nested;
      final firstValue = first.members[index].value;
      if (firstValue is JsonObjectNode) {
        final nestedValues = <JsonObjectNode>[firstValue];
        for (var row = 1; row < values.length; row++) {
          final candidate = values[row].members[index].value;
          if (candidate is! JsonObjectNode) {
            return null;
          }
          nestedValues.add(candidate);
        }
        nested = _infer(nestedValues);
        if (nested == null) {
          return null;
        }
      } else {
        for (var row = 1; row < values.length; row++) {
          if (values[row].members[index].value is JsonObjectNode) {
            return null;
          }
        }
      }
      fields.add(_SchemaField(first.members[index].key, nested));
    }
    return _RecordSchema(fields);
  }

  final List<_SchemaField> fields;
  final int fieldCount;
  final int encodedByteLength;
  final int objectStructuralByteLength;
  final int valueCount;
}

final class _SchemaField {
  const _SchemaField(this.key, this.nested);

  final String key;
  final _RecordSchema? nested;
}

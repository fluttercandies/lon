import 'json_node.dart';
import 'json_writer.dart';
import 'lon_syntax.dart';

final class LonWriter {
  const LonWriter({
    this.useTables = true,
    required this.maxOutputCodeUnits,
  });

  final bool useTables;
  final int maxOutputCodeUnits;

  String write(JsonNode node) {
    final buffer = CodeUnitLimitedStringBuffer(maxOutputCodeUnits);
    _writeValue(buffer, node);
    return buffer.toString();
  }

  void _writeValue(StringSink buffer, JsonNode node) {
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
        if (useTables && _writeRecordTable(buffer, node)) {
          return;
        }
        _writeArray(buffer, node);
      case JsonObjectNode():
        _writeObject(buffer, node);
    }
  }

  void _writeArray(StringSink buffer, JsonArrayNode node) {
    buffer.writeCharCode(0x5B);
    var useSpaces = node.values.isNotEmpty;
    for (final value in node.values) {
      if (value is JsonStringNode && canUseBareLonString(value.value)) {
        continue;
      }
      useSpaces = false;
      break;
    }
    for (var index = 0; index < node.values.length; index++) {
      if (index != 0) {
        buffer.writeCharCode(useSpaces ? 0x20 : 0x2C);
      }
      _writeValue(buffer, node.values[index]);
    }
    buffer.writeCharCode(0x5D);
  }

  void _writeObject(StringSink buffer, JsonObjectNode node) {
    buffer.writeCharCode(0x7B);
    for (var index = 0; index < node.members.length; index++) {
      if (index != 0) {
        buffer.writeCharCode(0x20);
      }
      final member = node.members[index];
      _writeKey(buffer, member.key);
      buffer.writeCharCode(0x3A);
      _writeValue(buffer, member.value);
    }
    buffer.writeCharCode(0x7D);
  }

  bool _writeRecordTable(StringSink buffer, JsonArrayNode node) {
    if (node.values.length < 2) {
      return false;
    }
    for (final value in node.values) {
      if (value is! JsonObjectNode) {
        return false;
      }
    }

    final schema = _RecordSchema.infer(
      node.values.cast<JsonObjectNode>(),
    );
    if (schema == null ||
        schema.fields.isEmpty ||
        schema.valueCount < 2 ||
        !_shouldUseTable(node.values.length, schema)) {
      return false;
    }

    buffer.writeCharCode(0x5B);
    _writeSchema(buffer, schema);
    for (final value in node.values.cast<JsonObjectNode>()) {
      buffer.writeCharCode(0x3B);
      _writeRow(buffer, schema, value);
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
  ) {
    _writeRowValues(buffer, schema, value, false);
  }

  bool _writeRowValues(
    StringSink buffer,
    _RecordSchema schema,
    JsonObjectNode value,
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
        _writeValue(buffer, memberValue);
        hasValue = true;
      } else {
        hasValue = _writeRowValues(
          buffer,
          nested,
          memberValue as JsonObjectNode,
          hasValue,
        );
      }
    }
    return hasValue;
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

  static _RecordSchema? infer(Iterable<JsonObjectNode> values) =>
      _infer(values.toList(growable: false));

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

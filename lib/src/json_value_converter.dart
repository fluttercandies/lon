import 'dart:collection';

import 'json_node.dart';

JsonNode jsonNodeFromDart(
  Object? value, {
  int maxDepth = 512,
  int maxExpandedElements = 1000000,
}) =>
    _DartValueReader(maxDepth, maxExpandedElements).convert(value, 0);

Object? jsonNodeToDart(JsonNode node) {
  switch (node) {
    case JsonNullNode():
      return null;
    case JsonBooleanNode(:final value):
      return value;
    case JsonNumberNode(:final lexeme):
      if (!lexeme.contains('.') &&
          !lexeme.contains('e') &&
          !lexeme.contains('E')) {
        final integer = int.tryParse(lexeme);
        if (integer != null) {
          return integer;
        }
      }
      return double.parse(lexeme);
    case JsonStringNode(:final value):
      return value;
    case JsonArrayNode(:final values):
      return values.map(jsonNodeToDart).toList(growable: false);
    case JsonObjectNode(:final members):
      final result = <String, Object?>{};
      for (final member in members) {
        result[member.key] = jsonNodeToDart(member.value);
      }
      return result;
  }
}

final class _DartValueReader {
  _DartValueReader(this.maxDepth, this.maxExpandedElements);

  final int maxDepth;
  final int maxExpandedElements;
  int _expandedElements = 0;
  final Set<Object> _activeContainers = HashSet<Object>.identity();

  JsonNode convert(Object? value, int depth) {
    if (depth > maxDepth) {
      throw ArgumentError('Maximum nesting depth of $maxDepth exceeded');
    }
    if (value == null) {
      return jsonNullNode;
    }
    if (value is bool) {
      return JsonBooleanNode(value);
    }
    if (value is String) {
      return JsonStringNode(value);
    }
    if (value is int) {
      return JsonNumberNode(value.toString());
    }
    if (value is double) {
      if (!value.isFinite) {
        throw ArgumentError.value(
            value, 'value', 'JSON numbers must be finite');
      }
      return JsonNumberNode(value.toString());
    }
    if (value is List<Object?>) {
      return _convertList(value, depth);
    }
    if (value is Map<Object?, Object?>) {
      return _convertMap(value, depth);
    }
    throw ArgumentError.value(
      value,
      'value',
      'Expected null, bool, num, String, List, or Map<String, Object?>',
    );
  }

  JsonArrayNode _convertList(List<Object?> value, int depth) {
    _enter(value);
    try {
      final values = <JsonNode>[];
      for (final item in value) {
        _reserve();
        values.add(convert(item, depth + 1));
      }
      return JsonArrayNode(values);
    } finally {
      _activeContainers.remove(value);
    }
  }

  JsonObjectNode _convertMap(Map<Object?, Object?> value, int depth) {
    _enter(value);
    try {
      final members = <JsonMember>[];
      for (final entry in value.entries) {
        final key = entry.key;
        if (key is! String) {
          throw ArgumentError.value(
              key, 'key', 'JSON object keys must be strings');
        }
        _reserve();
        members.add(JsonMember(key, convert(entry.value, depth + 1)));
      }
      return JsonObjectNode(members);
    } finally {
      _activeContainers.remove(value);
    }
  }

  void _reserve() {
    if (_expandedElements >= maxExpandedElements) {
      throw ArgumentError(
        'Expanded element limit of $maxExpandedElements exceeded',
      );
    }
    _expandedElements++;
  }

  void _enter(Object container) {
    if (!_activeContainers.add(container)) {
      throw ArgumentError('Cyclic JSON value');
    }
  }
}

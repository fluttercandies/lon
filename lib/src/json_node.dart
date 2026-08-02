sealed class JsonNode {
  const JsonNode();
}

final class JsonNullNode extends JsonNode {
  const JsonNullNode();
}

final class JsonBooleanNode extends JsonNode {
  const JsonBooleanNode(this.value);

  final bool value;
}

final class JsonNumberNode extends JsonNode {
  const JsonNumberNode(this.lexeme);

  final String lexeme;
}

final class JsonStringNode extends JsonNode {
  const JsonStringNode(this.value);

  final String value;
}

final class JsonArrayNode extends JsonNode {
  const JsonArrayNode(this.values);

  final List<JsonNode> values;
}

final class JsonObjectNode extends JsonNode {
  const JsonObjectNode(this.members);

  final List<JsonMember> members;
}

final class JsonMember {
  const JsonMember(this.key, this.value);

  final String key;
  final JsonNode value;
}

const jsonNullNode = JsonNullNode();

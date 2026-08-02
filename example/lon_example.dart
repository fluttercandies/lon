import 'package:lon/lon.dart';

void main() {
  const json =
      '{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"},{"id":3,"name":"Lin"}]}';

  final encoded = jsonToLon(json);
  final decodedJson = lonToJson(encoded);

  print(encoded);
  print(decodedJson);

  final value = lon.decode(encoded) as Map<String, Object?>;
  print(value['users']);
}

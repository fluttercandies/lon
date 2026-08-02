import 'dart:convert';
import 'dart:io';

import 'package:lon/lon.dart';

const _maxInputBytes = 64 * 1024 * 1024;

Future<void> main(List<String> arguments) async {
  stdout.encoding = utf8;
  stderr.encoding = utf8;
  if (arguments.isEmpty || arguments.length > 2) {
    _usage();
    exitCode = 64;
    return;
  }

  final command = arguments.first;
  if (command != 'encode' && command != 'decode') {
    _usage();
    exitCode = 64;
    return;
  }

  try {
    final input = arguments.length == 2 ? File(arguments[1]).openRead() : stdin;
    final source = await _readUtf8(input);
    final output = command == 'encode' ? jsonToLon(source) : lonToJson(source);
    stdout.writeln(output);
  } on FormatException catch (error) {
    final offset = error.offset;
    stderr.writeln(
      offset == null
          ? 'Invalid input: ${error.message}'
          : 'Invalid input at offset $offset: ${error.message}',
    );
    exitCode = 65;
  } on FileSystemException catch (error) {
    stderr.writeln(error.message);
    exitCode = 66;
  }
}

Future<String> _readUtf8(Stream<List<int>> input) =>
    _limitInputBytes(input).transform(utf8.decoder).join();

Stream<List<int>> _limitInputBytes(Stream<List<int>> input) async* {
  var byteCount = 0;
  await for (final chunk in input) {
    if (chunk.length > _maxInputBytes - byteCount) {
      throw FormatException('Input byte limit of $_maxInputBytes exceeded');
    }
    byteCount += chunk.length;
    yield chunk;
  }
}

void _usage() {
  stderr.writeln('Usage: lon encode [json-file] | lon decode [lon-file]');
}

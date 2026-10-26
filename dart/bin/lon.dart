import 'dart:convert';
import 'dart:io';
import 'dart:typed_data';

import 'package:lon/lon.dart';

const _maxInputBytes = 64 * 1024 * 1024;
const _prettyFlag = '--pretty';

Future<void> main(List<String> arguments) async {
  stdout.encoding = utf8;
  stderr.encoding = utf8;
  var pretty = false;
  final positional = <String>[];
  var valid = true;
  for (final argument in arguments) {
    if (argument == _prettyFlag) {
      pretty = true;
    } else if (argument.startsWith('-')) {
      valid = false;
    } else {
      positional.add(argument);
    }
  }
  if (!valid || positional.isEmpty || positional.length > 2) {
    _usage();
    exitCode = 64;
    return;
  }

  final command = positional.first;
  if (command != 'encode' && command != 'decode' && command != 'check') {
    _usage();
    exitCode = 64;
    return;
  }

  try {
    final input =
        positional.length == 2 ? File(positional[1]).openRead() : stdin;
    final source = await _readUtf8(input);
    if (command == 'check') {
      // Validation only: a fully decoded document exits 0 with no output.
      const LonCodec().decodeJson(source);
      return;
    }
    final codec = LonCodec(pretty: pretty);
    final output = command == 'encode'
        ? codec.encodeJson(source)
        : codec.decodeJson(source);
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

Future<String> _readUtf8(Stream<List<int>> input) async {
  final builder = BytesBuilder(copy: false);
  await for (final chunk in _limitInputBytes(input)) {
    builder.add(chunk);
  }
  final bytes = builder.toBytes();
  // Reject a byte-order mark explicitly: utf8.decode would silently strip it.
  if (bytes.length >= 3 &&
      bytes[0] == 0xEF &&
      bytes[1] == 0xBB &&
      bytes[2] == 0xBF) {
    throw const FormatException('Byte-order mark is not allowed');
  }
  return utf8.decode(bytes, allowMalformed: false);
}

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
  stderr.writeln(
    'Usage: lon encode [--pretty] [json-file] | lon decode [--pretty] '
    '[lon-file] | lon check [lon-file]',
  );
}

import 'dart:convert';
import 'dart:io';

import 'package:test/test.dart';

void main() {
  Future<Process> lon(List<String> args) => Process.start(
        Platform.resolvedExecutable,
        <String>['run', 'bin/lon.dart', ...args],
      );

  group('usage errors', () {
    test('exits 64 with no arguments, extra positionals, or unknown flag',
        () async {
      for (final args in <List<String>>[
        <String>[],
        <String>['bogus'],
        <String>['encode', 'a', 'b'],
        <String>['encode', 'a', 'b', 'c'],
        <String>['encode', '--bogus'],
        <String>['--pretty'],
        <String>['--pretty', 'encode', '--bogus'],
        <String>['encode', 'a', '--pretty', 'extra'],
      ]) {
        final process = await lon(args);
        await process.stdin.close();
        final stderr = await process.stderr.transform(utf8.decoder).join();
        expect(await process.exitCode, 64, reason: args.join(' '));
        expect(stderr, contains('Usage:'), reason: args.join(' '));
        expect(await process.stdout.expand((c) => c).toList(), isEmpty);
      }
    });

    test('exits 66 when the input file does not exist', () async {
      for (final command in <String>['encode', 'decode']) {
        final process = await lon(<String>[
          command,
          '/nonexistent/lon-cli-edge-test.json',
        ]);
        await process.stdin.close();
        final stderr = await process.stderr.transform(utf8.decoder).join();
        expect(await process.exitCode, 66, reason: command);
        expect(stderr, isNotEmpty);
        expect(await process.stdout.expand((c) => c).toList(), isEmpty);
      }
    });
  });

  group('stdin handling', () {
    test('rejects empty input with exit 65', () async {
      for (final command in <String>['encode', 'decode']) {
        final process = await lon(<String>[command]);
        await process.stdin.close();
        final stderr = await process.stderr.transform(utf8.decoder).join();
        expect(await process.exitCode, 65, reason: command);
        expect(stderr, contains('Invalid input'), reason: command);
        expect(await process.stdout.expand((c) => c).toList(), isEmpty);
      }
    });

    test('accepts input without a trailing newline', () async {
      final encode = await lon(<String>['encode']);
      encode.stdin.write('{"a":1}');
      await encode.stdin.close();
      expect(await encode.exitCode, 0);
      expect(
        (await encode.stdout.transform(utf8.decoder).join()).trimRight(),
        '{a:1}',
      );

      final decode = await lon(<String>['decode']);
      decode.stdin.write('{a:1}');
      await decode.stdin.close();
      expect(await decode.exitCode, 0);
      expect(
        (await decode.stdout.transform(utf8.decoder).join()).trimRight(),
        '{"a":1}',
      );
    });

    test('rejects a byte-order mark with exit 65 and no echoed content',
        () async {
      for (final command in <String>['encode', 'decode']) {
        final process = await lon(<String>[command]);
        process.stdin.add(const <int>[
          0xEF,
          0xBB,
          0xBF,
          0x7B,
          0x61,
          0x3A,
          0x31,
          0x7D,
        ]);
        await process.stdin.close();
        final stderrBytes = await process.stderr.expand((c) => c).toList();
        expect(await process.exitCode, 65, reason: command);
        expect(utf8.decode(stderrBytes), contains('Invalid input'));
        expect(await process.stdout.expand((c) => c).toList(), isEmpty);
      }
    });

    test('rejects invalid UTF-8 with exit 65 and a sanitized diagnostic',
        () async {
      final process = await lon(<String>['decode']);
      process.stdin.add(const <int>[0xFF, 0xFE, 0x22, 0x61, 0x22]);
      await process.stdin.close();
      final stderrBytes = await process.stderr.expand((c) => c).toList();
      expect(await process.exitCode, 65);
      final diagnostic = utf8.decode(stderrBytes, allowMalformed: false);
      expect(diagnostic, contains('Invalid input'));
      expect(diagnostic, isNot(contains('\uFFFD')));
      expect(await process.stdout.expand((c) => c).toList(), isEmpty);
    });
  });

  group('file mode', () {
    test('produces identical output to stdin mode', () async {
      final jsonFile = File(
        '${Directory.systemTemp.path}/lon_cli_edge_${pid}_in.json',
      );
      final lonFile = File(
        '${Directory.systemTemp.path}/lon_cli_edge_${pid}_in.lon',
      );
      try {
        jsonFile.writeAsBytesSync(
          utf8.encode(
            '{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}',
          ),
        );
        lonFile.writeAsBytesSync(
          utf8.encode('{users:[id name;1 Ada;2 Bob]}'),
        );

        final fileEncode = await lon(<String>['encode', jsonFile.path]);
        await fileEncode.stdin.close();
        final fileEncodeOut =
            await fileEncode.stdout.transform(utf8.decoder).join();
        expect(await fileEncode.exitCode, 0);
        expect(fileEncodeOut, '{users:[id name;1 Ada;2 Bob]}\n');

        final stdinEncode = await lon(<String>['encode']);
        stdinEncode.stdin.write(
          '{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}',
        );
        await stdinEncode.stdin.close();
        final stdinEncodeOut =
            await stdinEncode.stdout.transform(utf8.decoder).join();
        expect(await stdinEncode.exitCode, 0);
        expect(stdinEncodeOut, fileEncodeOut);

        final fileDecode = await lon(<String>['decode', lonFile.path]);
        await fileDecode.stdin.close();
        final fileDecodeOut =
            await fileDecode.stdout.transform(utf8.decoder).join();
        expect(await fileDecode.exitCode, 0);
        expect(
          fileDecodeOut,
          '{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}\n',
        );

        final stdinDecode = await lon(<String>['decode']);
        stdinDecode.stdin.write('{users:[id name;1 Ada;2 Bob]}');
        await stdinDecode.stdin.close();
        final stdinDecodeOut =
            await stdinDecode.stdout.transform(utf8.decoder).join();
        expect(await stdinDecode.exitCode, 0);
        expect(stdinDecodeOut, fileDecodeOut);
      } finally {
        if (jsonFile.existsSync()) jsonFile.deleteSync();
        if (lonFile.existsSync()) lonFile.deleteSync();
      }
    });

    test('file input with --pretty matches stdin --pretty output', () async {
      final file = File(
        '${Directory.systemTemp.path}/lon_cli_edge_${pid}_pretty.json',
      );
      try {
        file.writeAsBytesSync(
          utf8.encode(
              '{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}'),
        );
        final fromFile = await lon(<String>['encode', '--pretty', file.path]);
        await fromFile.stdin.close();
        final fileOut = await fromFile.stdout.transform(utf8.decoder).join();
        expect(await fromFile.exitCode, 0);

        final fromStdin = await lon(<String>['encode', '--pretty']);
        fromStdin.stdin.write(
          '{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}',
        );
        await fromStdin.stdin.close();
        final stdinOut = await fromStdin.stdout.transform(utf8.decoder).join();
        expect(await fromStdin.exitCode, 0);
        expect(fileOut, stdinOut);
      } finally {
        if (file.existsSync()) file.deleteSync();
      }
    });
  });

  group('check', () {
    test('exits 0 with no output for valid LON from stdin and file', () async {
      const source = '{users:[id name;1 Ada;2 Bob]}';

      final fromStdin = await lon(<String>['check']);
      fromStdin.stdin.write(source);
      await fromStdin.stdin.close();
      expect(await fromStdin.exitCode, 0);
      expect(await fromStdin.stdout.expand((c) => c).toList(), isEmpty);
      expect(await fromStdin.stderr.expand((c) => c).toList(), isEmpty);

      final file = File(
        '${Directory.systemTemp.path}/lon_cli_edge_${pid}_check.lon',
      );
      try {
        file.writeAsBytesSync(utf8.encode(source));
        final fromFile = await lon(<String>['check', file.path]);
        await fromFile.stdin.close();
        expect(await fromFile.exitCode, 0);
        expect(await fromFile.stdout.expand((c) => c).toList(), isEmpty);
      } finally {
        if (file.existsSync()) file.deleteSync();
      }
    });

    test('exits 65 for invalid LON, empty input, BOM, and bad UTF-8', () async {
      final cases = <List<int>>[
        utf8.encode('{a:1'),
        utf8.encode('{a:1 b:[x y]}'), // 'y' is a reserved literal
        const <int>[],
        const <int>[0xEF, 0xBB, 0xBF, 0x7B, 0x7D],
        const <int>[0xFF, 0xFE, 0x22],
      ];
      for (final bytes in cases) {
        final process = await lon(<String>['check']);
        process.stdin.add(bytes);
        await process.stdin.close();
        expect(await process.exitCode, 65, reason: '$bytes');
        expect(await process.stdout.expand((c) => c).toList(), isEmpty);
      }
    });

    test('exits 66 for a missing file and 64 for extra arguments', () async {
      final missing = await lon(
        <String>['check', '/nonexistent/lon-cli-check-test.lon'],
      );
      await missing.stdin.close();
      expect(await missing.exitCode, 66);
      expect(await missing.stdout.expand((c) => c).toList(), isEmpty);

      final extra = await lon(<String>['check', 'a', 'b']);
      await extra.stdin.close();
      expect(await extra.exitCode, 64);
      expect(await extra.stdout.expand((c) => c).toList(), isEmpty);
    });
  });
}

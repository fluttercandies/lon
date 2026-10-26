// Differential harness: feeds a battery of inputs through the Dart reference
// codec and prints one JSON result line per case:
//   {"kind":"jsonToLon"|"lonToJson","input":...,"ok":true,"output":...}
//   {"kind":...,"input":...,"ok":false}
// The Rust side (tests/differential.rs) compares behavior. This file lives in
// the Rust crate only and is executed via `dart run`; it is not part of the
// Dart package's own tests.
import 'dart:convert';
import 'dart:io';
import 'package:lon/lon.dart';

void main() {
  final conformance = (jsonDecode(File('../conformance/v1.json').readAsStringSync())
      as List<Object?>)
      .cast<Map<String, Object?>>();

  final cases = <Map<String, String>>[
    for (final case_ in conformance) ...[
      {'kind': 'jsonToLon', 'input': case_['json']! as String},
      {'kind': 'lonToJson', 'input': case_['lon']! as String},
    ],
    // Grammar acceptance and rejection edges.
    for (final source in [
      '[1,2 3]', '[1 2,3]', '[]', '{}', 'null', 'true', 'false', '0', '-0',
      '1e+02', '0.5e-3', '"quoted"', 'bare', '01', 'table', 'NULL',
      '[01]', '[undefined]', '[NaN]', '[a\u0000b]', '[tr\u2063ue]',
      '{a\u202Eb:1}', '{a"b:1}', '[a"b;1]', '"a\u2063b"', '{"a\u202Eb":1}',
      '[id name;1]', '[id name;1 Ada;2]', '[id;1 2]', '[id name;(1 Ada)]',
      '[a b;]', '[a b;1 2]', '[a;1;2]', '[a{b};1;2]', '[a empty{};1;2]',
      '[id id;1 2;3 4]', '["a b" c;1 2;3 4]', '[v w;"a;b" 1;"c" 2]',
      '[ a b ; 1 2 ; 3 4 ; 5 6 ]', '{"1":"x"}', 'tablex', '[true false]',
      '[true,false]', '[true,]', '[1 Ada true]', '[Ada,Bob]',
      '[Ada 2026-08-02]', '{a:1} {b:2}', '"unterminated', r'"bad\x20"',
      r'"bad\u12"', r'"bad\u0xz0"', '"raw\nnewline"', '"raw\u0000control"',
      '"trailing\\', '\uFEFF{a:1}', '\uFEFF{"a":1}', '{a\u00A0b:1}',
      '[a\u1680b]', '{"k\u2028v":1}', '[a\u3000b]', '{null:null true:true}',
      '{01:2}', '{1:2 0x10:3}', '[[1,2],[3,4]]', '[{a:1},{b:2}]',
      '[{a:1},{a:2}]', '[a b;1 2;3 4]', '[简 体;1 2;3 4]',
      '[a b;1 2;3 4;5 6]', '[a{b c};1 2 3;4 5 6]',
      '[id name addr{city zip} tags;1 Ada London W1 [math first];2 Bob Paris "75001" []]',
      '[a b c;1 2 3;4 5 6;7 8 9]', '[a b;{} {};1 2]', '[aa bb;1 2;3 4]',
      '[aaaaaaa b;1 2;3 4]', '[a b cc;1 2;3 4]', '["aa" b;1 2;3 4]',
      '[x{y z};1 2 3;4 5 6]',
      '{"x":1e+02,"x":-0,"s":"A"}', '[1e999]', '[123456789012345678901234567890]',
      '[-1234567890123456789]', '[18446744073709551615]',
    ])
      {'kind': source.startsWith('{') && source.contains('"') || _looksLikeJson(source)
          ? 'jsonToLon'
          : 'lonToJson', 'input': source},
    // Lone surrogates through the JSON path.
    {'kind': 'jsonToLon', 'input': r'["\ud800","\udfff","\ud83d\ude00"]'},
    {'kind': 'jsonToLon', 'input': r'{"\ud800":1}'},
    {'kind': 'lonToJson', 'input': r'["\ud800" "\udfff"]'},
    {'kind': 'jsonToLon', 'input': r'["\udb40\udc01","\udbff\udfff"]'},
  ];

  final results = <Map<String, Object?>>[];
  for (final testCase in cases) {
    final kind = testCase['kind']!;
    final input = testCase['input']!;
    String? output;
    var ok = false;
    try {
      output = kind == 'jsonToLon'
          ? jsonToLon(input)
          : lonToJson(input);
      ok = true;
    } on FormatException {
      ok = false;
    } on RangeError {
      ok = false;
    } on ArgumentError {
      ok = false;
    }
    results.add({
      'kind': kind,
      'input': input,
      'ok': ok,
      if (ok) 'output': output,
    });
  }
  stdout.write(jsonEncode(results));
}

bool _looksLikeJson(String source) {
  // Inputs that both decoders should treat as JSON: strict JSON shape with
  // commas or quoted keys and no bare tokens.
  final trimmed = source.trimLeft();
  if (trimmed.isEmpty) return false;
  if (trimmed.startsWith('{') && source.contains('"')) return true;
  if (source.contains(',')) return true;
  return false;
}

import 'json_string.dart';

final RegExp _jsonNumberPattern = RegExp(
  r'^-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?$',
);
final RegExp _numericLikePattern = RegExp(
  r'^[+-]?(?:(?:[0-9]+(?:_[0-9]+)*(?:\.[0-9_]*)?(?:[eE][+-]?[0-9_]*)?)|(?:\.[0-9_]+)|(?:0[xX][0-9a-fA-F_]+)|(?:0[bB][01_]+)|(?:0[oO][0-7_]+))$',
);
final RegExp _nonFiniteLikePattern = RegExp(
  r'^[+-]?\.?(?:nan|inf(?:inity)?)$',
);

const _ambiguousLiteralTokens = <String>{
  'null',
  'true',
  'false',
  'undefined',
  'none',
  'nil',
  'yes',
  'no',
  'on',
  'off',
  'y',
  'n',
  '~',
};

bool isJsonNumberToken(String token) => _jsonNumberPattern.hasMatch(token);

bool isReservedLonToken(String token) {
  final folded = token.toLowerCase();
  return _ambiguousLiteralTokens.contains(folded) ||
      _nonFiniteLikePattern.hasMatch(folded) ||
      _numericLikePattern.hasMatch(token);
}

bool canUseBareLonString(String value) =>
    _canUseBare(value) && !isReservedLonToken(value);

bool canUseBareLonKey(String value) => _canUseBare(value);

bool _canUseBare(String value) {
  if (value.isEmpty) {
    return false;
  }

  for (var index = 0; index < value.length; index++) {
    final code = value.codeUnitAt(index);
    if (isLonWhitespace(code) ||
        requiresJsonEscapeForVisibility(code) ||
        code == 0x22 ||
        (code == 0x27 && (index == 0 || index == value.length - 1)) ||
        code == 0x28 ||
        code == 0x29 ||
        code == 0x2C ||
        code == 0x3A ||
        code == 0x3B ||
        code == 0x5B ||
        code == 0x5C ||
        code == 0x5D ||
        code == 0x7B ||
        code == 0x7C ||
        code == 0x7D) {
      return false;
    }
    if (code >= 0xD800 && code <= 0xDBFF) {
      if (index + 1 >= value.length) {
        return false;
      }
      final low = value.codeUnitAt(index + 1);
      if (low < 0xDC00 || low > 0xDFFF) {
        return false;
      }
      final rune = 0x10000 + ((code - 0xD800) << 10) + low - 0xDC00;
      if (requiresJsonEscapeForVisibility(rune)) {
        return false;
      }
      index++;
    } else if (code >= 0xDC00 && code <= 0xDFFF) {
      return false;
    }
  }
  return true;
}

bool isLonWhitespace(int code) =>
    (code >= 0x09 && code <= 0x0D) ||
    code == 0x20 ||
    code == 0x85 ||
    code == 0xA0 ||
    code == 0x1680 ||
    (code >= 0x2000 && code <= 0x200A) ||
    code == 0x2028 ||
    code == 0x2029 ||
    code == 0x202F ||
    code == 0x205F ||
    code == 0x3000;

bool isLonTokenDelimiter(int code) =>
    isLonWhitespace(code) ||
    code == 0x28 ||
    code == 0x29 ||
    code == 0x2C ||
    code == 0x3A ||
    code == 0x3B ||
    code == 0x5B ||
    code == 0x5D ||
    code == 0x7B ||
    code == 0x7C ||
    code == 0x7D;

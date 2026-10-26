import 'json_string.dart';

// Rare-path fallbacks: exotic Unicode lowercases (e.g. U+0130 folds to `i`)
// must keep matching the reserved-token rules through full-string folding.
// ASCII inputs never hit them.
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

bool isJsonNumberToken(String token) {
  final length = token.length;
  var index = 0;
  if (index < length && token.codeUnitAt(index) == 0x2D) {
    index++;
  }
  if (index >= length) {
    return false;
  }
  final first = token.codeUnitAt(index);
  if (first == 0x30) {
    index++;
  } else if (first >= 0x31 && first <= 0x39) {
    index++;
    while (index < length && _isAsciiDigit(token.codeUnitAt(index))) {
      index++;
    }
  } else {
    return false;
  }
  if (index < length && token.codeUnitAt(index) == 0x2E) {
    index++;
    if (index >= length || !_isAsciiDigit(token.codeUnitAt(index))) {
      return false;
    }
    while (index < length && _isAsciiDigit(token.codeUnitAt(index))) {
      index++;
    }
  }
  if (index < length) {
    final exponent = token.codeUnitAt(index);
    if (exponent == 0x65 || exponent == 0x45) {
      index++;
      if (index < length) {
        final sign = token.codeUnitAt(index);
        if (sign == 0x2B || sign == 0x2D) {
          index++;
        }
      }
      if (index >= length || !_isAsciiDigit(token.codeUnitAt(index))) {
        return false;
      }
      while (index < length && _isAsciiDigit(token.codeUnitAt(index))) {
        index++;
      }
    }
  }
  return index == length;
}

bool isReservedLonToken(String token) {
  for (var index = 0; index < token.length; index++) {
    if (token.codeUnitAt(index) >= 0x80) {
      final folded = token.toLowerCase();
      return _ambiguousLiteralTokens.contains(folded) ||
          _nonFiniteLikePattern.hasMatch(folded) ||
          _numericLikePattern.hasMatch(token);
    }
  }
  return _isAmbiguousAsciiLiteral(token) ||
      _isNonFiniteAsciiLiteral(token) ||
      _isNumericLike(token);
}

bool canUseBareLonString(String value) =>
    _canUseBare(value) && !isReservedLonToken(value);

bool canUseBareLonKey(String value) => _canUseBare(value);

bool _canUseBare(String value) {
  final length = value.length;
  if (length == 0) {
    return false;
  }

  for (var index = 0; index < length; index++) {
    final code = value.codeUnitAt(index);
    if (code < 0x20) {
      return false;
    }
    if (code <= 0x7E) {
      if (_isForbiddenInBareAscii(code) ||
          (code == 0x27 && (index == 0 || index == length - 1))) {
        return false;
      }
      continue;
    }
    if (code == 0x7F || code >= 0xDC00 && code <= 0xDFFF) {
      return false;
    }
    if (code >= 0xD800 && code <= 0xDBFF) {
      if (index + 1 >= length) {
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
    } else if (isLonWhitespace(code) || requiresJsonEscapeForVisibility(code)) {
      return false;
    }
  }
  return true;
}

/// 0x20..0x7E forbidden set: space, `"`, `(`, `)`, `,`, `:`, `;`, `[`, `\`,
/// `]`, `{`, `|`, `}`. Quotes are additionally rejected at the edges.
bool _isForbiddenInBareAscii(int code) =>
    code == 0x20 ||
    code == 0x22 ||
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
    code == 0x7D;

bool isLonWhitespace(int code) {
  if (code > 0x20) {
    return code == 0x85 ||
        code == 0xA0 ||
        code == 0x1680 ||
        (code >= 0x2000 && code <= 0x200A) ||
        code == 0x2028 ||
        code == 0x2029 ||
        code == 0x202F ||
        code == 0x205F ||
        code == 0x3000;
  }
  return code == 0x20 || code >= 0x09 && code <= 0x0D;
}

bool isLonTokenDelimiter(int code) {
  if (code < 0x21) {
    return code == 0x20 || code >= 0x09 && code <= 0x0D;
  }
  if (code < 0x28 || code > 0x7D) {
    return code > 0x7D && isLonWhitespace(code);
  }
  return code == 0x28 ||
      code == 0x29 ||
      code == 0x2C ||
      code == 0x3A ||
      code == 0x3B ||
      code == 0x5B ||
      code == 0x5D ||
      code == 0x7B ||
      code == 0x7C ||
      code == 0x7D;
}

bool _isAmbiguousAsciiLiteral(String token) => switch (token.length) {
      1 => _asciiLowerEquals(token, 0, 'n') ||
          _asciiLowerEquals(token, 0, 'y') ||
          token.codeUnitAt(0) == 0x7E,
      2 =>
        _asciiLowerEquals(token, 0, 'no') || _asciiLowerEquals(token, 0, 'on'),
      3 => _asciiLowerEquals(token, 0, 'nil') ||
          _asciiLowerEquals(token, 0, 'yes') ||
          _asciiLowerEquals(token, 0, 'off'),
      4 => _asciiLowerEquals(token, 0, 'null') ||
          _asciiLowerEquals(token, 0, 'none') ||
          _asciiLowerEquals(token, 0, 'true'),
      5 => _asciiLowerEquals(token, 0, 'false'),
      9 => _asciiLowerEquals(token, 0, 'undefined'),
      _ => false,
    };

bool _isNonFiniteAsciiLiteral(String token) {
  var start = 0;
  if (token.isEmpty) {
    return false;
  }
  final sign = token.codeUnitAt(0);
  if (sign == 0x2B || sign == 0x2D) {
    start++;
  }
  if (start < token.length && token.codeUnitAt(start) == 0x2E) {
    start++;
  }
  return switch (token.length - start) {
    3 => _asciiLowerEquals(token, start, 'nan') ||
        _asciiLowerEquals(token, start, 'inf'),
    8 => _asciiLowerEquals(token, start, 'infinity'),
    _ => false,
  };
}

/// Compares `token[start..]` with the ASCII literal, case-insensitively.
/// Callers guarantee the token holds ASCII only.
bool _asciiLowerEquals(String token, int start, String literal) {
  if (token.length - start != literal.length) {
    return false;
  }
  for (var index = 0; index < literal.length; index++) {
    var code = token.codeUnitAt(start + index);
    if (code >= 0x41 && code <= 0x5A) {
      code += 0x20;
    }
    if (code != literal.codeUnitAt(index)) {
      return false;
    }
  }
  return true;
}

/// Mirrors `_numericLikePattern` (see above) without the regex engine.
bool _isNumericLike(String token) {
  final length = token.length;
  if (length == 0) {
    return false;
  }
  var index = 0;
  if (token.codeUnitAt(0) == 0x2B || token.codeUnitAt(0) == 0x2D) {
    index++;
  }
  if (index >= length) {
    return false;
  }
  final first = token.codeUnitAt(index);
  if (first == 0x30 && index + 1 < length) {
    final radix = token.codeUnitAt(index + 1);
    if (radix == 0x78 || radix == 0x58) {
      return _allOf(token, index + 2, _isAsciiHex);
    }
    if (radix == 0x62 || radix == 0x42) {
      return _allOf(token, index + 2, _isAsciiBinary);
    }
    if (radix == 0x6F || radix == 0x4F) {
      return _allOf(token, index + 2, _isAsciiOctal);
    }
  }

  if (first == 0x2E) {
    // `\.[0-9_]+`
    index++;
    if (index >= length || !_isDigitOrUnderscore(token.codeUnitAt(index))) {
      return false;
    }
    while (index < length && _isDigitOrUnderscore(token.codeUnitAt(index))) {
      index++;
    }
    return index == length;
  }
  if (!_isAsciiDigit(first)) {
    return false;
  }
  index++;
  while (index < length && _isAsciiDigit(token.codeUnitAt(index))) {
    index++;
  }
  while (index < length && token.codeUnitAt(index) == 0x5F) {
    index++;
    if (index >= length || !_isAsciiDigit(token.codeUnitAt(index))) {
      return false;
    }
    while (index < length && _isAsciiDigit(token.codeUnitAt(index))) {
      index++;
    }
  }
  if (index < length && token.codeUnitAt(index) == 0x2E) {
    index++;
    while (index < length && _isDigitOrUnderscore(token.codeUnitAt(index))) {
      index++;
    }
  }
  if (index < length) {
    final exponent = token.codeUnitAt(index);
    if (exponent == 0x65 || exponent == 0x45) {
      index++;
      if (index < length) {
        final sign = token.codeUnitAt(index);
        if (sign == 0x2B || sign == 0x2D) {
          index++;
        }
      }
      while (index < length && _isDigitOrUnderscore(token.codeUnitAt(index))) {
        index++;
      }
    }
  }
  return index == length;
}

/// True when [start] begins a non-empty run of [accept] chars reaching the end.
bool _allOf(String token, int start, bool Function(int) accept) {
  if (start >= token.length) {
    return false;
  }
  for (var index = start; index < token.length; index++) {
    if (!accept(token.codeUnitAt(index))) {
      return false;
    }
  }
  return true;
}

bool _isAsciiDigit(int code) => code >= 0x30 && code <= 0x39;

bool _isDigitOrUnderscore(int code) =>
    code == 0x5F || code >= 0x30 && code <= 0x39;

bool _isAsciiHex(int code) =>
    code == 0x5F ||
    code >= 0x30 && code <= 0x39 ||
    code >= 0x41 && code <= 0x46 ||
    code >= 0x61 && code <= 0x66;

bool _isAsciiBinary(int code) => code == 0x30 || code == 0x31 || code == 0x5F;

bool _isAsciiOctal(int code) => code == 0x5F || code >= 0x30 && code <= 0x37;

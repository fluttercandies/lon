//! LON token classification: bare-string eligibility, reserved tokens, and
//! whitespace rules shared by the parser and the writer (port of
//! `lon_syntax.dart`).

/// Code points that must appear as a `\uXXXX` escape inside any quoted
/// string: controls, invisible separators, directional marks, and noncharacters.
pub(crate) fn requires_visibility_escape(rune: u32) -> bool {
    rune < 0x20
        || (0x7F..=0x9F).contains(&rune)
        || (0xFDD0..=0xFDEF).contains(&rune)
        || (rune & 0xFFFE) == 0xFFFE
        || matches!(
            rune,
            0x00AD
                | 0x034F
                | 0x061C
                | 0x180E
                | 0x200B
                | 0x200E
                | 0x200F
                | 0xFEFF
                | 0x2028..=0x202E
                | 0x2060..=0x206F
                | 0xFFF9..=0xFFFB
                | 0x13430..=0x13455
                | 0x1BCA0..=0x1BCA3
                | 0x1D173..=0x1D17A
                | 0xE0000..=0xE007F
        )
}

/// The Unicode whitespace set that separates LON tokens.
pub(crate) fn is_lon_whitespace(code: u32) -> bool {
    (0x09..=0x0D).contains(&code)
        || (0x2000..=0x200A).contains(&code)
        || matches!(
            code,
            0x20 | 0x85 | 0xA0 | 0x1680 | 0x2028 | 0x2029 | 0x202F | 0x205F | 0x3000
        )
}

/// Code points that terminate a bare LON token: whitespace plus the reserved
/// punctuation `(`, `)`, `,`, `:`, `;`, `[`, `]`, `{`, `|`, `}`.
pub(crate) fn is_lon_token_delimiter(code: u32) -> bool {
    is_lon_whitespace(code)
        || matches!(
            code,
            0x28 | 0x29 | 0x2C | 0x3A | 0x3B | 0x5B | 0x5D | 0x7B | 0x7C | 0x7D
        )
}

/// ASCII whitespace bytes (0x09..=0x0D and 0x20), the only ASCII members of
/// the LON whitespace set.
pub(crate) fn is_ascii_lon_whitespace(byte: u8) -> bool {
    byte == b' ' || (0x09..=0x0D).contains(&byte)
}

/// ASCII bytes that terminate a bare LON token. Every non-ASCII delimiter is
/// Unicode whitespace, so the byte fast path only needs this set.
pub(crate) fn is_ascii_lon_delimiter(byte: u8) -> bool {
    matches!(
        byte,
        b' ' | b'\t'..=b'\r' | b'(' | b')' | b',' | b':' | b';' | b'[' | b']' | b'{' | b'|' | b'}'
    )
}

/// Tokens matching JSON number syntax, which parse as numbers instead of strings.
pub(crate) fn is_json_number_token(token: &str) -> bool {
    let bytes = token.as_bytes();
    let mut i = if bytes.first() == Some(&b'-') { 1 } else { 0 };

    match bytes.get(i) {
        Some(&b'0') => i += 1,
        Some(&d) if (b'1'..=b'9').contains(&d) => i = digits_end(bytes, i + 1),
        _ => return false,
    }

    if bytes.get(i) == Some(&b'.') {
        i = match bytes.get(i + 1) {
            Some(&d) if d.is_ascii_digit() => digits_end(bytes, i + 2),
            _ => return false,
        };
    }

    if matches!(bytes.get(i), Some(&b'e') | Some(&b'E')) {
        i += 1;
        if matches!(bytes.get(i), Some(&b'+') | Some(&b'-')) {
            i += 1;
        }
        i = match bytes.get(i) {
            Some(&d) if d.is_ascii_digit() => digits_end(bytes, i + 1),
            _ => return false,
        };
    }

    i == bytes.len()
}

/// Tokens that stay quoted because a model could confuse them with a literal,
/// a number, or a non-finite float spelling.
///
/// Pure-ASCII tokens (the common case) never fold outside ASCII, so the whole
/// check runs over bytes without the `to_lowercase` allocation.
pub(crate) fn is_reserved_lon_token(token: &str) -> bool {
    let bytes = token.as_bytes();
    if bytes.iter().all(u8::is_ascii) {
        is_reserved_ascii_bytes(bytes)
    } else {
        let folded = token.to_lowercase();
        AMBIGUOUS_LITERALS.contains(&folded.as_str())
            || is_non_finite_like(&folded)
            || is_numeric_like(bytes)
    }
}

/// The longest reserved literal spelling ("undefined").
const LONGEST_RESERVED_LITERAL: usize = 9;
/// The longest non-finite spelling with sign and dot (".infinity" is 9, a
/// sign plus dot plus "infinity" is 10).
const LONGEST_NON_FINITE: usize = 10;

pub(crate) fn is_reserved_ascii_bytes(bytes: &[u8]) -> bool {
    if bytes.len() <= LONGEST_RESERVED_LITERAL {
        for literal in AMBIGUOUS_LITERALS {
            if bytes.eq_ignore_ascii_case(literal.as_bytes()) {
                return true;
            }
        }
    }
    if bytes.len() <= LONGEST_NON_FINITE && is_non_finite_ascii(bytes) {
        return true;
    }
    is_numeric_like(bytes)
}

fn is_non_finite_ascii(bytes: &[u8]) -> bool {
    let mut body = bytes;
    if matches!(body.first(), Some(b'+') | Some(b'-')) {
        body = &body[1..];
    }
    if body.first() == Some(&b'.') {
        body = &body[1..];
    }
    body.eq_ignore_ascii_case(b"nan")
        || body.eq_ignore_ascii_case(b"inf")
        || body.eq_ignore_ascii_case(b"infinity")
}

const AMBIGUOUS_LITERALS: [&str; 13] = [
    "null",
    "true",
    "false",
    "undefined",
    "none",
    "nil",
    "yes",
    "no",
    "on",
    "off",
    "y",
    "n",
    "~",
];

fn is_non_finite_like(folded: &str) -> bool {
    let unsigned = folded
        .strip_prefix('+')
        .or_else(|| folded.strip_prefix('-'))
        .unwrap_or(folded);
    let bare = unsigned.strip_prefix('.').unwrap_or(unsigned);
    matches!(bare, "nan" | "inf" | "infinity")
}

fn is_numeric_like(bytes: &[u8]) -> bool {
    let body = match bytes.first() {
        Some(b'+') | Some(b'-') => &bytes[1..],
        _ => bytes,
    };
    decimal_like(body).is_some_and(|end| end == body.len())
        || is_underscore_fraction(body)
        || is_radix_like(body)
}

/// Matches `[0-9]+(?:_[0-9]+)*(?:\.[0-9_]*)?(?:[eE][+-]?[0-9_]*)?` and returns
/// the matched length.
fn decimal_like(body: &[u8]) -> Option<usize> {
    let mut i = digit_run(body, 0)?;
    while body.get(i) == Some(&b'_') {
        i = digit_run(body, i + 1)?;
    }
    if body.get(i) == Some(&b'.') {
        i = digit_or_underscore_run(body, i + 1);
    }
    if matches!(body.get(i), Some(&b'e') | Some(&b'E')) {
        i += 1;
        if matches!(body.get(i), Some(&b'+') | Some(&b'-')) {
            i += 1;
        }
        i = digit_or_underscore_run(body, i);
    }
    Some(i)
}

/// Matches `\.[0-9_]+`.
fn is_underscore_fraction(body: &[u8]) -> bool {
    body.first() == Some(&b'.')
        && body.len() > 1
        && body[1..].iter().all(|&d| d.is_ascii_digit() || d == b'_')
}

/// Matches `0[xX][0-9a-fA-F_]+`, `0[bB][01_]+`, and `0[oO][0-7_]+`.
fn is_radix_like(body: &[u8]) -> bool {
    if body.len() < 3 || body[0] != b'0' {
        return false;
    }
    let rest = &body[2..];
    match body[1] {
        b'x' | b'X' => rest.iter().all(|&d| d.is_ascii_hexdigit() || d == b'_'),
        b'b' | b'B' => rest.iter().all(|&d| matches!(d, b'0' | b'1' | b'_')),
        b'o' | b'O' => rest.iter().all(|&d| matches!(d, b'0'..=b'7' | b'_')),
        _ => false,
    }
}

fn digits_end(bytes: &[u8], mut i: usize) -> usize {
    while matches!(bytes.get(i), Some(&d) if d.is_ascii_digit()) {
        i += 1;
    }
    i
}

fn digit_run(bytes: &[u8], start: usize) -> Option<usize> {
    let end = digits_end(bytes, start);
    (end > start).then_some(end)
}

fn digit_or_underscore_run(bytes: &[u8], mut i: usize) -> usize {
    while matches!(bytes.get(i), Some(d) if d.is_ascii_digit() || *d == b'_') {
        i += 1;
    }
    i
}

/// Whether a bare token read from LON source text may stay unquoted as a
/// string value (reserved tokens excluded).
pub(crate) fn can_use_bare_lon_string(token: &str) -> bool {
    bare_eligible_token(token) && !is_reserved_lon_token(token)
}

/// Whether a bare token read from LON source text may stay unquoted as an
/// object key or schema field name. Reserved spellings are allowed because the
/// key context already fixes the value type.
pub(crate) fn can_use_bare_lon_key(token: &str) -> bool {
    bare_eligible_token(token)
}

fn is_bare_forbidden_code(code: u32) -> bool {
    code == 0x22 || code == 0x5C || is_lon_token_delimiter(code)
}

fn bare_eligible_token(token: &str) -> bool {
    if token.is_empty() {
        return false;
    }
    for (index, c) in token.char_indices() {
        let code = c as u32;
        if code < 0x80 {
            // ASCII fast path: the ASCII visibility escapes are just the
            // C0 controls and DEL, the forbidden quotes and backslashes
            // join the delimiter set, and `'` is only rejected at the edges.
            if code < 0x20
                || code == 0x7F
                || code == 0x22
                || code == 0x5C
                || is_ascii_lon_delimiter(code as u8)
                || (code == 0x27 && (index == 0 || index + c.len_utf8() == token.len()))
            {
                return false;
            }
            continue;
        }
        if is_bare_forbidden_code(code)
            || requires_visibility_escape(code)
            || (code == 0x27 && (index == 0 || index + c.len_utf8() == token.len()))
        {
            return false;
        }
    }
    true
}

pub(crate) fn bare_eligible_units(units: &[u16]) -> bool {
    if units.is_empty() {
        return false;
    }
    let mut index = 0;
    while index < units.len() {
        let unit = units[index];
        let at_edge = index == 0 || index == units.len() - 1;
        if unit < 0x80 {
            // ASCII fast path, mirroring `bare_eligible_token`.
            if unit < 0x20
                || unit == 0x7F
                || unit == 0x22
                || unit == 0x5C
                || is_ascii_lon_delimiter(unit as u8)
                || (unit == 0x27 && at_edge)
            {
                return false;
            }
            index += 1;
            continue;
        }
        if is_bare_forbidden_code(u32::from(unit))
            || requires_visibility_escape(u32::from(unit))
            || (unit == 0x27 && at_edge)
        {
            return false;
        }
        if (0xD800..=0xDBFF).contains(&unit) {
            let Some(&low) = units.get(index + 1) else {
                return false;
            };
            if !(0xDC00..=0xDFFF).contains(&low) {
                return false;
            }
            let rune = surrogate_pair_rune(unit, low);
            if requires_visibility_escape(rune) {
                return false;
            }
            index += 2;
            continue;
        }
        if (0xDC00..=0xDFFF).contains(&unit) {
            return false;
        }
        index += 1;
    }
    true
}

pub(crate) fn surrogate_pair_rune(high: u16, low: u16) -> u32 {
    0x10000 + ((u32::from(high) - 0xD800) << 10) + u32::from(low) - 0xDC00
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_number_tokens() {
        for token in ["0", "-0", "1", "1.5", "-12.5e+2", "1e+02", "1E-3", "10"] {
            assert!(is_json_number_token(token), "{token}");
        }
        for token in [
            "01", ".5", "1.", "1e", "1e+", "-", "+1", "0x10", "1_000", "", "nan", "1 2",
        ] {
            assert!(!is_json_number_token(token), "{token}");
        }
    }

    #[test]
    fn reserved_lon_tokens() {
        for token in [
            "null",
            "NULL",
            "true",
            "False",
            "undefined",
            "None",
            "nil",
            "yes",
            "NO",
            "on",
            "Off",
            "y",
            "N",
            "~",
            "NaN",
            "nan",
            "+Infinity",
            "-inf",
            ".NaN",
            "+.inf",
            "01",
            "+1",
            ".5",
            "1.",
            "1e",
            "1e+",
            "0x10",
            "0b101",
            "0o77",
            "1_000",
            "._",
        ] {
            assert!(is_reserved_lon_token(token), "{token}");
        }
        for token in [
            "table",
            "2026-08-02",
            "123abc",
            "Ada",
            "e",
            ".",
            "-",
            "1-2",
            "0x",
            "Infinityx",
            "1_",
        ] {
            assert!(!is_reserved_lon_token(token), "{token}");
        }
    }

    #[test]
    fn bare_eligibility() {
        for token in [
            "Ada", "don't", "O'Reilly", "users", "简体", "0", "null", "e\u{301}",
        ] {
            assert!(bare_eligible_token(token), "{token:?}");
        }
        for token in [
            "",
            "'quoted'",
            "users'",
            "a b",
            "a:b",
            "a;b",
            "a,b",
            "a(b",
            "a|b",
            "a[b",
            "a{b",
            "a\"b",
            "a\\b",
            "a\u{2028}b",
            "a\u{0}b",
            "a\u{ad}b",
        ] {
            assert!(!bare_eligible_token(token), "{token:?}");
        }
    }
}

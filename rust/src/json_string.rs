//! JSON string scanning and canonical escaping over UTF-16 code units (port
//! of `json_string.dart`).

use crate::error::LonError;
use crate::json_writer::LimitedWriter;
use crate::syntax::{requires_visibility_escape, surrogate_pair_rune};

/// A scanned JSON string: its UTF-16 code units and the byte offset just past
/// the closing quote.
pub(crate) struct ParsedString {
    pub(crate) units: Vec<u16>,
    pub(crate) next_offset: usize,
}

/// Scans the JSON string starting at `offset`, which must point at a `"`.
///
/// With `require_visible`, every unescaped code point that a LON decoder could
/// not see reliably (controls, invisible separators, directional marks) is
/// rejected, matching the stricter quoted-string rule of LON input.
///
/// Plain ASCII runs between interesting bytes (`"`, `\`, controls, non-ASCII)
/// are widened into the unit vector in bulk.
pub(crate) fn parse_json_string_at(
    source: &str,
    offset: usize,
    require_visible: bool,
) -> Result<ParsedString, LonError> {
    let bytes = source.as_bytes();
    if bytes.get(offset) != Some(&b'"') {
        return Err(LonError::format_at("Expected a JSON string", offset));
    }

    let mut units = Vec::new();
    let mut cursor = offset + 1;
    while cursor < bytes.len() {
        let byte = bytes[cursor];
        if byte == b'"' {
            return Ok(ParsedString {
                units,
                next_offset: cursor + 1,
            });
        }
        if byte == b'\\' {
            cursor += 1;
            let Some(&escape) = bytes.get(cursor) else {
                return Err(LonError::format_at(
                    "Incomplete JSON escape sequence",
                    cursor,
                ));
            };
            cursor += 1;
            match escape {
                b'"' => units.push(0x22),
                b'\\' => units.push(0x5C),
                b'/' => units.push(0x2F),
                b'b' => units.push(0x08),
                b'f' => units.push(0x0C),
                b'n' => units.push(0x0A),
                b'r' => units.push(0x0D),
                b't' => units.push(0x09),
                b'u' => {
                    if cursor + 4 > bytes.len() {
                        return Err(LonError::format_at(
                            "Incomplete Unicode escape sequence",
                            cursor,
                        ));
                    }
                    let mut value: u16 = 0;
                    for _ in 0..4 {
                        let Some(digit) = hex_value(bytes[cursor]) else {
                            return Err(LonError::format_at(
                                "Invalid Unicode escape sequence",
                                cursor,
                            ));
                        };
                        value = (value << 4) | u16::from(digit);
                        cursor += 1;
                    }
                    // A lone escape may produce an unpaired surrogate; the
                    // unit is kept verbatim so the value round-trips.
                    units.push(value);
                }
                _ => {
                    return Err(LonError::format_at(
                        "Invalid JSON escape sequence",
                        cursor - 1,
                    ));
                }
            }
            continue;
        }
        if byte < 0x20 {
            return Err(LonError::format_at(
                "Unescaped control character in JSON string",
                cursor,
            ));
        }
        if byte < 0x7F {
            // Bulk-widen the plain ASCII run: bytes in 0x20..=0x7E are never
            // quotes, escapes, or visibility-sensitive (DEL, 0x7F, still needs
            // the require_visible check below).
            let run_start = cursor;
            loop {
                cursor += 1;
                match bytes.get(cursor) {
                    Some(&b) if (0x20..0x7F).contains(&b) && b != b'"' && b != b'\\' => {}
                    _ => break,
                }
            }
            units.extend(bytes[run_start..cursor].iter().copied().map(u16::from));
            continue;
        }

        // The cursor is always on a UTF-8 boundary: it only advances over
        // ASCII bytes or by whole characters.
        let ch = source[cursor..]
            .chars()
            .next()
            .expect("cursor is on a UTF-8 boundary");
        if require_visible && requires_visibility_escape(ch as u32) {
            return Err(LonError::format_at(
                "Invisible or unpaired code point must use a Unicode escape",
                cursor,
            ));
        }
        let mut encoded = [0u16; 2];
        units.extend_from_slice(ch.encode_utf16(&mut encoded));
        cursor += ch.len_utf8();
    }
    Err(LonError::format_at("Unterminated JSON string", cursor))
}

fn hex_value(code: u8) -> Option<u8> {
    match code {
        b'0'..=b'9' => Some(code - b'0'),
        b'a'..=b'f' => Some(code - b'a' + 10),
        b'A'..=b'F' => Some(code - b'A' + 10),
        _ => None,
    }
}

/// The UTF-16 code units of `text`. Pure-ASCII text (the common case) is
/// widened in bulk instead of walking `encode_utf16`'s per-character machine.
pub(crate) fn units_from_str(text: &str) -> Vec<u16> {
    if text.is_ascii() {
        text.bytes().map(u16::from).collect()
    } else {
        text.encode_utf16().collect()
    }
}

/// The exact Rust string for `units`, or `None` when it holds lone
/// surrogates. All-ASCII units take a bulk widening fast path.
pub(crate) fn exact_string_from_units(units: &[u16]) -> Option<String> {
    if units.iter().all(|&u| u <= 0x7F) {
        Some(
            String::from_utf8(units.iter().map(|&u| u as u8).collect())
                .expect("ASCII code units are valid UTF-8"),
        )
    } else {
        String::from_utf16(units).ok()
    }
}

/// The Rust string for `units` with lone surrogates replaced by U+FFFD.
pub(crate) fn string_from_units_lossy(units: &[u16]) -> String {
    if units.iter().all(|&u| u <= 0x7F) {
        String::from_utf8(units.iter().map(|&u| u as u8).collect())
            .expect("ASCII code units are valid UTF-8")
    } else {
        String::from_utf16_lossy(units)
    }
}

/// Whether a code unit continues a run of plain ASCII characters inside a
/// quoted string: visible ASCII that needs no escape.
fn continues_plain_ascii_run(unit: u16) -> bool {
    (0x20..0x7F).contains(&unit) && unit != 0x22 && unit != 0x5C
}

/// Writes a canonical JSON string: the short escapes, `\uXXXX` for
/// visibility-sensitive code points and unpaired surrogates, and raw
/// characters everywhere else. The output is always valid UTF-8.
///
/// Runs of plain ASCII units are widened and appended in bulk.
pub(crate) fn write_json_string(out: &mut LimitedWriter, units: &[u16]) -> Result<(), LonError> {
    out.push_byte(b'"')?;
    let mut index = 0;
    while index < units.len() {
        let unit = units[index];
        match unit {
            0x22 => out.push_ascii_bytes(b"\\\"")?,
            0x5C => out.push_ascii_bytes(b"\\\\")?,
            0x08 => out.push_ascii_bytes(b"\\b")?,
            0x09 => out.push_ascii_bytes(b"\\t")?,
            0x0A => out.push_ascii_bytes(b"\\n")?,
            0x0C => out.push_ascii_bytes(b"\\f")?,
            0x0D => out.push_ascii_bytes(b"\\r")?,
            _ => {
                if requires_visibility_escape(u32::from(unit)) {
                    write_unicode_escape(out, unit)?;
                } else if (0xD800..=0xDBFF).contains(&unit) {
                    if let Some(&low) = units.get(index + 1) {
                        if (0xDC00..=0xDFFF).contains(&low) {
                            let rune = surrogate_pair_rune(unit, low);
                            if requires_visibility_escape(rune) {
                                write_unicode_escape(out, unit)?;
                                write_unicode_escape(out, low)?;
                            } else {
                                out.push(
                                    char::from_u32(rune)
                                        .expect("a surrogate pair is a scalar value"),
                                )?;
                            }
                            index += 2;
                            continue;
                        }
                    }
                    write_unicode_escape(out, unit)?;
                } else if (0xDC00..=0xDFFF).contains(&unit) {
                    write_unicode_escape(out, unit)?;
                } else if unit < 0x7F {
                    // A plain ASCII unit: widen the whole run in one go.
                    let run_start = index;
                    index += 1;
                    while index < units.len() && continues_plain_ascii_run(units[index]) {
                        index += 1;
                    }
                    out.push_ascii_units(&units[run_start..index])?;
                    continue;
                } else {
                    out.push(
                        char::from_u32(u32::from(unit))
                            .expect("a non-surrogate BMP unit is a scalar value"),
                    )?;
                }
            }
        }
        index += 1;
    }
    out.push_byte(b'"')
}

fn write_unicode_escape(out: &mut LimitedWriter, unit: u16) -> Result<(), LonError> {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    out.push_byte(b'\\')?;
    out.push_byte(b'u')?;
    let value = u32::from(unit);
    for shift in [12, 8, 4, 0] {
        out.push_byte(HEX[((value >> shift) & 0xF) as usize])?;
    }
    Ok(())
}

/// The UTF-8 byte length of a code-unit sequence, used by the table cost gate.
pub(crate) fn utf8_length(units: &[u16]) -> usize {
    let mut length = 0;
    let mut index = 0;
    while index < units.len() {
        let unit = units[index];
        if unit <= 0x7F {
            length += 1;
        } else if unit <= 0x7FF {
            length += 2;
        } else if (0xD800..=0xDBFF).contains(&unit) {
            let paired =
                matches!(units.get(index + 1), Some(&low) if (0xDC00..=0xDFFF).contains(&low));
            length += if paired { 4 } else { 3 };
            index += usize::from(paired);
        } else {
            length += 3;
        }
        index += 1;
    }
    length
}

/// The UTF-8 byte length of a code-unit sequence in quoted JSON form.
pub(crate) fn json_string_utf8_length(units: &[u16]) -> usize {
    let mut length = 2;
    let mut index = 0;
    while index < units.len() {
        let unit = units[index];
        match unit {
            0x22 | 0x5C | 0x08 | 0x09 | 0x0A | 0x0C | 0x0D => length += 2,
            _ => {
                if requires_visibility_escape(u32::from(unit)) {
                    length += 6;
                } else if (0xD800..=0xDBFF).contains(&unit) {
                    let paired = match units.get(index + 1) {
                        Some(&low) if (0xDC00..=0xDFFF).contains(&low) => {
                            let rune = surrogate_pair_rune(unit, low);
                            Some(if requires_visibility_escape(rune) {
                                12
                            } else {
                                4
                            })
                        }
                        _ => None,
                    };
                    match paired {
                        Some(add) => {
                            length += add;
                            index += 1;
                        }
                        None => length += 6,
                    }
                } else if (0xDC00..=0xDFFF).contains(&unit) {
                    length += 6;
                } else if unit <= 0x7F {
                    length += 1;
                } else if unit <= 0x7FF {
                    length += 2;
                } else {
                    length += 3;
                }
            }
        }
        index += 1;
    }
    length
}

//! Compact JSON serialization of node trees (port of `json_writer.dart`).

use crate::error::LonError;
use crate::json_string::write_json_string;
use crate::node::{Member, Node};

/// A `String` sink that counts UTF-16 code units and fails as soon as the
/// output limit would be exceeded, so no partial document is ever returned.
///
/// The buffer holds raw UTF-8 bytes because every append path feeds valid
/// UTF-8 (`&str` slices, encoded `char`s, or known-ASCII code units); the one
/// validation happens when the finished buffer converts back to a `String`.
pub(crate) struct LimitedWriter {
    buffer: Vec<u8>,
    units: usize,
    max_units: usize,
}

/// The UTF-16 code-unit count of `text`: ASCII text widens 1:1.
fn utf16_units(text: &str) -> usize {
    if text.is_ascii() {
        text.len()
    } else {
        text.chars()
            .map(|c| usize::from((c as u32) >= 0x10000) + 1)
            .sum()
    }
}

impl LimitedWriter {
    pub(crate) fn with_capacity(max_units: usize, capacity: usize) -> Self {
        // Capacity beyond the output limit can never be used.
        Self {
            buffer: Vec::with_capacity(capacity.min(max_units)),
            units: 0,
            max_units,
        }
    }

    pub(crate) fn push_str(&mut self, text: &str) -> Result<(), LonError> {
        self.reserve(utf16_units(text))?;
        self.buffer.extend_from_slice(text.as_bytes());
        Ok(())
    }

    /// Appends a byte slice that is ASCII by construction (escape sequences,
    /// widened ASCII code units), so every byte is one code unit.
    pub(crate) fn push_ascii_bytes(&mut self, bytes: &[u8]) -> Result<(), LonError> {
        self.reserve(bytes.len())?;
        self.buffer.extend_from_slice(bytes);
        Ok(())
    }

    pub(crate) fn push(&mut self, c: char) -> Result<(), LonError> {
        self.reserve(usize::from((c as u32) >= 0x10000) + 1)?;
        let mut encoded = [0u8; 4];
        self.buffer
            .extend_from_slice(c.encode_utf8(&mut encoded).as_bytes());
        Ok(())
    }

    pub(crate) fn push_byte(&mut self, byte: u8) -> Result<(), LonError> {
        self.reserve(1)?;
        self.buffer.push(byte);
        Ok(())
    }

    /// Appends a run of plain ASCII code units (all `0x20..=0x7F`), widening
    /// in bulk.
    pub(crate) fn push_ascii_units(&mut self, units: &[u16]) -> Result<(), LonError> {
        self.reserve(units.len())?;
        self.buffer.extend(units.iter().map(|&u| u as u8));
        Ok(())
    }

    pub(crate) fn into_string(self) -> String {
        String::from_utf8(self.buffer).expect("only valid UTF-8 is ever appended")
    }

    fn reserve(&mut self, count: usize) -> Result<(), LonError> {
        if count > self.max_units - self.units {
            return Err(LonError::format(format!(
                "Output code-unit limit of {} exceeded",
                self.max_units
            )));
        }
        self.units += count;
        Ok(())
    }
}

/// A cheap lower bound on any serialized form of `node` (LON or JSON), used
/// only to pre-size the output buffer and never affecting what is written.
pub(crate) fn estimate_output_bytes(node: &Node) -> usize {
    fn estimate(node: &Node) -> usize {
        match node {
            Node::Null => 4,
            Node::Bool(_) => 5,
            Node::Number(lexeme) => lexeme.len(),
            Node::String(units) => units.len() + 2,
            Node::Array(values) => values
                .iter()
                .fold(values.len().saturating_add(2), |sum, value| {
                    sum.saturating_add(estimate(value))
                }),
            Node::Object(members) => members.iter().fold(1, |sum, member| {
                sum.saturating_add(member.key.len() + 2 + estimate(&member.value))
            }),
        }
    }
    estimate(node)
}

/// Serializes a node tree as JSON, preserving member order, duplicate
/// keys, string code units, and number lexemes. Compact unless `pretty`
/// selects standard two-space indentation.
pub(crate) fn write_json_node(
    out: &mut LimitedWriter,
    node: &Node,
    depth: usize,
    pretty: bool,
) -> Result<(), LonError> {
    match node {
        Node::Null => out.push_str("null"),
        Node::Bool(true) => out.push_str("true"),
        Node::Bool(false) => out.push_str("false"),
        Node::Number(lexeme) => out.push_str(lexeme),
        Node::String(units) => write_json_string(out, units),
        Node::Array(values) => {
            out.push_byte(b'[')?;
            let multiline = pretty && !values.is_empty();
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    out.push_byte(b',')?;
                }
                if multiline {
                    write_line_break(out, depth + 1)?;
                }
                write_json_node(out, value, depth + 1, pretty)?;
            }
            if multiline {
                write_line_break(out, depth)?;
            }
            out.push_byte(b']')
        }
        Node::Object(members) => {
            out.push_byte(b'{')?;
            let multiline = pretty && !members.is_empty();
            for (index, Member { key, value }) in members.iter().enumerate() {
                if index > 0 {
                    out.push_byte(b',')?;
                }
                if multiline {
                    write_line_break(out, depth + 1)?;
                }
                write_json_string(out, key)?;
                out.push_byte(b':')?;
                if pretty {
                    out.push_byte(b' ')?;
                }
                write_json_node(out, value, depth + 1, pretty)?;
            }
            if multiline {
                write_line_break(out, depth)?;
            }
            out.push_byte(b'}')
        }
    }
}

fn write_line_break(out: &mut LimitedWriter, depth: usize) -> Result<(), LonError> {
    out.push_byte(b'\n')?;
    for _ in 0..depth {
        out.push_str("  ")?;
    }
    Ok(())
}

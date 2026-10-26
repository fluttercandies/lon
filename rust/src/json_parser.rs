//! Strict JSON text parser producing node trees (port of `json_parser.dart`).

use crate::error::LonError;
use crate::json_string::parse_json_string_at;
use crate::node::{Member, Node};

pub(crate) struct JsonTextParser<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
    max_depth: usize,
    max_expanded_elements: usize,
    expanded_elements: usize,
}

impl<'a> JsonTextParser<'a> {
    pub(crate) fn new(source: &'a str, max_depth: usize, max_expanded_elements: usize) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            max_depth,
            max_expanded_elements,
            expanded_elements: 0,
        }
    }

    pub(crate) fn parse(mut self) -> Result<Node, LonError> {
        self.skip_whitespace();
        if self.is_at_end() {
            return Err(LonError::format_at("Expected a JSON value", self.pos));
        }
        let value = self.parse_value(0)?;
        self.skip_whitespace();
        if !self.is_at_end() {
            return Err(LonError::format_at("Unexpected trailing content", self.pos));
        }
        Ok(value)
    }

    fn parse_value(&mut self, depth: usize) -> Result<Node, LonError> {
        if depth > self.max_depth {
            return Err(LonError::format(format!(
                "Maximum nesting depth of {} exceeded",
                self.max_depth
            )));
        }
        match self.bytes.get(self.pos) {
            None => Err(LonError::format_at("Expected a JSON value", self.pos)),
            Some(&b'"') => Ok(Node::String(self.parse_string()?)),
            Some(&b'[') => self.parse_array(depth),
            Some(&b'{') => self.parse_object(depth),
            Some(&b't') => self.parse_literal("true", Node::Bool(true)),
            Some(&b'f') => self.parse_literal("false", Node::Bool(false)),
            Some(&b'n') => self.parse_literal("null", Node::Null),
            Some(&b'-') | Some(&(b'0'..=b'9')) => self.parse_number(),
            _ => Err(LonError::format_at("Expected a JSON value", self.pos)),
        }
    }

    fn parse_literal(&mut self, literal: &str, value: Node) -> Result<Node, LonError> {
        if !self.source[self.pos..].starts_with(literal) {
            return Err(LonError::format_at("Invalid JSON literal", self.pos));
        }
        self.pos += literal.len();
        Ok(value)
    }

    fn parse_array(&mut self, depth: usize) -> Result<Node, LonError> {
        self.pos += 1;
        self.skip_whitespace();
        if self.consume(b']') {
            return Ok(Node::Array(Vec::new()));
        }
        let mut values = Vec::new();
        loop {
            self.reserve()?;
            values.push(self.parse_value(depth + 1)?);
            self.skip_whitespace();
            if self.consume(b']') {
                return Ok(Node::Array(values));
            }
            self.expect(b',', "Expected a comma or closing bracket")?;
            self.skip_whitespace();
        }
    }

    fn parse_object(&mut self, depth: usize) -> Result<Node, LonError> {
        self.pos += 1;
        self.skip_whitespace();
        if self.consume(b'}') {
            return Ok(Node::Object(Vec::new()));
        }
        let mut members = Vec::new();
        loop {
            if self.bytes.get(self.pos) != Some(&b'"') {
                return Err(LonError::format_at(
                    "Expected a quoted object key",
                    self.pos,
                ));
            }
            let key = self.parse_string()?;
            self.skip_whitespace();
            self.expect(b':', "Expected a colon after the object key")?;
            self.skip_whitespace();
            self.reserve()?;
            let value = self.parse_value(depth + 1)?;
            members.push(Member { key, value });
            self.skip_whitespace();
            if self.consume(b'}') {
                return Ok(Node::Object(members));
            }
            self.expect(b',', "Expected a comma or closing brace")?;
            self.skip_whitespace();
        }
    }

    fn parse_number(&mut self) -> Result<Node, LonError> {
        let start = self.pos;
        self.consume(b'-');
        match self.bytes.get(self.pos) {
            None => return Err(LonError::format_at("Incomplete JSON number", self.pos)),
            Some(&b'0') => {
                self.pos += 1;
                if matches!(self.bytes.get(self.pos), Some(&d) if d.is_ascii_digit()) {
                    return Err(LonError::format_at(
                        "Leading zeroes are not allowed in JSON numbers",
                        self.pos,
                    ));
                }
            }
            Some(&first) if (b'1'..=b'9').contains(&first) => {
                self.skip_digits();
            }
            _ => {
                return Err(LonError::format_at(
                    "Expected a digit in JSON number",
                    self.pos,
                ))
            }
        }

        if self.bytes.get(self.pos) == Some(&b'.') {
            self.pos += 1;
            if !matches!(self.bytes.get(self.pos), Some(&d) if d.is_ascii_digit()) {
                return Err(LonError::format_at(
                    "Expected a digit after the decimal point",
                    self.pos,
                ));
            }
            self.skip_digits();
        }

        if matches!(self.bytes.get(self.pos), Some(e) if *e == b'e' || *e == b'E') {
            self.pos += 1;
            if matches!(self.bytes.get(self.pos), Some(s) if *s == b'+' || *s == b'-') {
                self.pos += 1;
            }
            if !matches!(self.bytes.get(self.pos), Some(&d) if d.is_ascii_digit()) {
                return Err(LonError::format_at("Expected an exponent digit", self.pos));
            }
            self.skip_digits();
        }

        Ok(Node::Number(self.source[start..self.pos].to_string()))
    }

    fn parse_string(&mut self) -> Result<Vec<u16>, LonError> {
        let parsed = parse_json_string_at(self.source, self.pos, false)?;
        self.pos = parsed.next_offset;
        Ok(parsed.units)
    }

    fn skip_digits(&mut self) {
        while matches!(self.bytes.get(self.pos), Some(&d) if d.is_ascii_digit()) {
            self.pos += 1;
        }
    }

    fn skip_whitespace(&mut self) {
        while matches!(
            self.bytes.get(self.pos),
            Some(&c) if matches!(c, b' ' | b'\t' | b'\n' | b'\r')
        ) {
            self.pos += 1;
        }
    }

    fn consume(&mut self, code: u8) -> bool {
        if self.bytes.get(self.pos) == Some(&code) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, code: u8, message: &str) -> Result<(), LonError> {
        if !self.consume(code) {
            return Err(LonError::format_at(message, self.pos));
        }
        Ok(())
    }

    fn reserve(&mut self) -> Result<(), LonError> {
        if self.expanded_elements >= self.max_expanded_elements {
            return Err(LonError::format(format!(
                "Expanded element limit of {} exceeded",
                self.max_expanded_elements
            )));
        }
        self.expanded_elements += 1;
        Ok(())
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.bytes.len()
    }
}

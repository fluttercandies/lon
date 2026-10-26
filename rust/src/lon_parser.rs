//! Strict LON text parser producing node trees (port of `lon_parser.dart`).

use crate::error::LonError;
use crate::json_string::{parse_json_string_at, units_from_str};
use crate::node::{Member, Node};
use crate::syntax::{
    can_use_bare_lon_key, can_use_bare_lon_string, is_ascii_lon_delimiter, is_ascii_lon_whitespace,
    is_json_number_token, is_lon_whitespace,
};

pub(crate) struct LonTextParser<'a> {
    source: &'a str,
    bytes: &'a [u8],
    pos: usize,
    max_depth: usize,
    max_expanded_elements: usize,
    expanded_elements: usize,
    schema_fields: usize,
}

impl<'a> LonTextParser<'a> {
    pub(crate) fn new(source: &'a str, max_depth: usize, max_expanded_elements: usize) -> Self {
        Self {
            source,
            bytes: source.as_bytes(),
            pos: 0,
            max_depth,
            max_expanded_elements,
            expanded_elements: 0,
            schema_fields: 0,
        }
    }

    pub(crate) fn parse(mut self) -> Result<Node, LonError> {
        self.skip_whitespace();
        if self.is_at_end() {
            return Err(LonError::format_at("Expected a LON value", self.pos));
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
            return Err(LonError::format_at(
                format!("Maximum nesting depth of {} exceeded", self.max_depth),
                self.pos,
            ));
        }
        self.skip_whitespace();
        match self.bytes.get(self.pos) {
            Some(&b'"') => return Ok(Node::String(self.parse_quoted_string()?)),
            Some(&b'[') => return self.parse_array(depth),
            Some(&b'{') => return self.parse_object(depth),
            _ => {}
        }

        let token_offset = self.pos;
        let token = self.read_bare_token()?;
        match token {
            "null" => return Ok(Node::Null),
            "true" => return Ok(Node::Bool(true)),
            "false" => return Ok(Node::Bool(false)),
            _ => {}
        }
        if is_json_number_token(token) {
            return Ok(Node::Number(token.to_string()));
        }
        if !can_use_bare_lon_string(token) {
            return Err(LonError::format_at(
                "Ambiguous or invalid bare string must be quoted",
                token_offset,
            ));
        }
        Ok(Node::String(units_from_str(token)))
    }

    fn parse_array(&mut self, depth: usize) -> Result<Node, LonError> {
        let table_offset = self.pos;
        self.pos += 1;
        if self.has_schema_terminator(self.pos) {
            return self.parse_record_table(depth, table_offset);
        }

        self.skip_whitespace();
        if self.consume(b']') {
            return Ok(Node::Array(Vec::new()));
        }
        let mut values = Vec::new();
        loop {
            self.reserve(1)?;
            values.push(self.parse_value(depth + 1)?);

            let had_whitespace = self.skip_whitespace();
            if self.consume(b']') {
                return Ok(Node::Array(values));
            }
            if self.consume(b',') {
                self.skip_whitespace();
                if self.is_at_end() || self.bytes[self.pos] == b']' {
                    return Err(LonError::format_at(
                        "Expected an array value after comma",
                        self.pos,
                    ));
                }
                continue;
            }
            if !had_whitespace {
                return Err(LonError::format_at(
                    "Expected whitespace, comma, or ] after array value",
                    self.pos,
                ));
            }
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
            let key = self.parse_object_key()?;
            self.reserve(1)?;
            let value = self.parse_value(depth + 1)?;
            members.push(Member { key, value });

            let had_whitespace = self.skip_whitespace();
            if self.consume(b'}') {
                return Ok(Node::Object(members));
            }
            if !had_whitespace {
                return Err(LonError::format_at(
                    "Expected whitespace or } after object field",
                    self.pos,
                ));
            }
        }
    }

    fn parse_record_table(&mut self, depth: usize, table_offset: usize) -> Result<Node, LonError> {
        let schema = self.parse_schema_header(depth, table_offset)?;
        let mut rows = Vec::new();
        self.skip_whitespace();
        if self.consume(b']') {
            return Err(LonError::format_at(
                "A table must contain at least two rows",
                table_offset,
            ));
        }

        loop {
            self.reserve(schema.field_count.saturating_add(1))?;
            rows.push(self.parse_schema_row(&schema, depth + 1)?);

            self.skip_whitespace();
            if self.consume(b']') {
                if rows.len() < 2 {
                    return Err(LonError::format_at(
                        "A table must contain at least two rows",
                        table_offset,
                    ));
                }
                return Ok(Node::Array(rows));
            }
            self.expect(b';', "Expected a semicolon between table rows")?;
            self.skip_whitespace();
            if self.is_at_end() || self.bytes[self.pos] == b']' {
                return Err(LonError::format_at(
                    "Expected a table row after semicolon",
                    self.pos,
                ));
            }
        }
    }

    fn parse_schema_header(
        &mut self,
        depth: usize,
        table_offset: usize,
    ) -> Result<RecordSchema, LonError> {
        let schema = self.parse_schema_fields(b';', depth, 1, table_offset)?;
        if schema.fields.is_empty() || schema.value_count < 2 {
            return Err(LonError::format_at(
                "A table header must contain at least two value fields",
                table_offset,
            ));
        }
        if depth + schema.max_path_depth + 1 > self.max_depth {
            return Err(LonError::format_at(
                format!("Maximum nesting depth of {} exceeded", self.max_depth),
                table_offset,
            ));
        }
        Ok(schema)
    }

    fn parse_schema_fields(
        &mut self,
        terminator: u8,
        table_depth: usize,
        path_depth: usize,
        table_offset: usize,
    ) -> Result<RecordSchema, LonError> {
        if table_depth + path_depth > self.max_depth {
            return Err(LonError::format_at(
                format!("Maximum nesting depth of {} exceeded", self.max_depth),
                table_offset,
            ));
        }

        let mut fields = Vec::new();
        self.skip_whitespace();

        while self.bytes.get(self.pos) != Some(&terminator) && !self.is_at_end() {
            self.reserve_schema_field(table_offset)?;
            let key = self.parse_schema_key()?;
            let nested = if self.bytes.get(self.pos) == Some(&b'{') {
                self.pos += 1;
                Some(self.parse_schema_fields(b'}', table_depth, path_depth + 1, table_offset)?)
            } else {
                None
            };
            fields.push(SchemaField { key, nested });

            let had_whitespace = self.skip_whitespace();
            if self.bytes.get(self.pos) == Some(&terminator) {
                break;
            }
            if !had_whitespace {
                return Err(LonError::format_at(
                    "Schema fields must be separated by whitespace",
                    self.pos,
                ));
            }
        }

        self.expect(terminator, "Unterminated table schema")?;
        Ok(RecordSchema::new(fields))
    }

    fn parse_schema_key(&mut self) -> Result<Vec<u16>, LonError> {
        self.skip_whitespace();
        if self.is_at_end() {
            return Err(LonError::format_at("Expected a table field name", self.pos));
        }
        if self.bytes.get(self.pos) == Some(&b'"') {
            return self.parse_quoted_string();
        }

        let key_offset = self.pos;
        let key = self.read_bare_token()?;
        if !can_use_bare_lon_key(key) {
            return Err(LonError::format_at(
                "Invalid bare table field name must be quoted",
                key_offset,
            ));
        }
        Ok(units_from_str(key))
    }

    fn parse_schema_row(&mut self, schema: &RecordSchema, depth: usize) -> Result<Node, LonError> {
        let mut state = RowState { has_value: false };
        self.parse_schema_object(schema, depth, &mut state)
    }

    fn parse_schema_object(
        &mut self,
        schema: &RecordSchema,
        depth: usize,
        state: &mut RowState,
    ) -> Result<Node, LonError> {
        let mut members = Vec::with_capacity(schema.fields.len());
        for field in &schema.fields {
            let value = match &field.nested {
                None => {
                    if state.has_value {
                        if !self.skip_whitespace() {
                            return Err(LonError::format_at(
                                "Table fields must be separated by whitespace",
                                self.pos,
                            ));
                        }
                    } else {
                        self.skip_whitespace();
                    }
                    state.has_value = true;
                    self.parse_value(depth + 1)?
                }
                Some(nested) => self.parse_schema_object(nested, depth + 1, state)?,
            };
            members.push(Member {
                key: field.key.clone(),
                value,
            });
        }
        Ok(Node::Object(members))
    }

    fn parse_object_key(&mut self) -> Result<Vec<u16>, LonError> {
        self.skip_whitespace();
        if self.is_at_end() {
            return Err(LonError::format_at("Expected an object key", self.pos));
        }

        let key = if self.bytes.get(self.pos) == Some(&b'"') {
            self.parse_quoted_string()?
        } else {
            units_from_str(self.read_key_before_colon()?)
        };
        self.skip_whitespace();
        self.expect(b':', "Expected a colon after the object key")?;
        Ok(key)
    }

    fn read_key_before_colon(&mut self) -> Result<&'a str, LonError> {
        let start = self.pos;
        let bytes = self.bytes;
        let mut pos = self.pos;
        while pos < bytes.len() {
            let byte = bytes[pos];
            if byte == b':' || is_ascii_lon_whitespace(byte) {
                break;
            }
            if byte < 0x80 {
                if matches!(byte, b'(' | b')' | b',' | b';' | b'[' | b']' | b'{' | b'}') {
                    self.pos = pos;
                    return Err(LonError::format_at(
                        "Invalid character in bare object key",
                        pos,
                    ));
                }
                pos += 1;
                continue;
            }
            // Non-ASCII: only Unicode whitespace ends the key early.
            let c = self.char_at(pos);
            if is_lon_whitespace(c as u32) {
                break;
            }
            pos += c.len_utf8();
        }
        self.pos = pos;
        if pos == start {
            return Err(LonError::format_at("Expected an object key", start));
        }
        let key = &self.source[start..pos];
        if !can_use_bare_lon_key(key) {
            return Err(LonError::format_at(
                "Invalid bare object key must be quoted",
                start,
            ));
        }
        Ok(key)
    }

    fn parse_quoted_string(&mut self) -> Result<Vec<u16>, LonError> {
        let parsed = parse_json_string_at(self.source, self.pos, true)?;
        self.pos = parsed.next_offset;
        Ok(parsed.units)
    }

    fn read_bare_token(&mut self) -> Result<&'a str, LonError> {
        let start = self.pos;
        let bytes = self.bytes;
        let mut pos = self.pos;
        while pos < bytes.len() {
            let byte = bytes[pos];
            if byte < 0x80 {
                if is_ascii_lon_delimiter(byte) {
                    break;
                }
                pos += 1;
                continue;
            }
            // Non-ASCII code points are delimiters only when they are
            // Unicode whitespace.
            let c = self.char_at(pos);
            if is_lon_whitespace(c as u32) {
                break;
            }
            pos += c.len_utf8();
        }
        self.pos = pos;
        if pos == start {
            return Err(LonError::format_at("Expected a LON token", pos));
        }
        Ok(&self.source[start..pos])
    }

    /// Whether the array starting at `cursor` opens with a valid schema
    /// terminated by an unquoted top-level semicolon.
    fn has_schema_terminator(&self, mut cursor: usize) -> bool {
        let bytes = self.bytes;
        let mut nested_schema_depth = 0usize;
        while cursor < bytes.len() {
            let code = bytes[cursor];
            match code {
                b'"' => {
                    cursor += 1;
                    while cursor < bytes.len() {
                        let quoted = bytes[cursor];
                        cursor += 1;
                        if quoted == b'\\' && cursor < bytes.len() {
                            cursor += 1;
                        } else if quoted == b'"' {
                            break;
                        }
                    }
                }
                b'{' => {
                    nested_schema_depth += 1;
                    cursor += 1;
                }
                b'}' => {
                    if nested_schema_depth == 0 {
                        return false;
                    }
                    nested_schema_depth -= 1;
                    cursor += 1;
                }
                b';' if nested_schema_depth == 0 => return true,
                b'(' | b')' | b',' | b':' | b'[' | b']' | b'|' if nested_schema_depth == 0 => {
                    return false;
                }
                _ => cursor += 1,
            }
        }
        false
    }

    /// The character starting at `pos`, which must be a UTF-8 boundary holding
    /// a non-ASCII lead byte.
    fn char_at(&self, pos: usize) -> char {
        self.source[pos..]
            .chars()
            .next()
            .expect("pos is on a UTF-8 boundary")
    }

    fn skip_whitespace(&mut self) -> bool {
        let start = self.pos;
        let bytes = self.bytes;
        let mut pos = self.pos;
        while pos < bytes.len() {
            let byte = bytes[pos];
            if is_ascii_lon_whitespace(byte) {
                pos += 1;
                continue;
            }
            if byte < 0x80 {
                break;
            }
            // Non-ASCII: only Unicode whitespace (for example U+2028)
            // continues the run.
            let c = self.char_at(pos);
            if !is_lon_whitespace(c as u32) {
                break;
            }
            pos += c.len_utf8();
        }
        self.pos = pos;
        pos != start
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

    fn reserve_schema_field(&mut self, table_offset: usize) -> Result<(), LonError> {
        if self.schema_fields >= self.max_expanded_elements {
            return Err(LonError::format_at(
                format!(
                    "Schema field limit of {} exceeded",
                    self.max_expanded_elements
                ),
                table_offset,
            ));
        }
        self.schema_fields += 1;
        Ok(())
    }

    fn reserve(&mut self, count: usize) -> Result<(), LonError> {
        if count > self.max_expanded_elements - self.expanded_elements {
            return Err(LonError::format(format!(
                "Expanded element limit of {} exceeded",
                self.max_expanded_elements
            )));
        }
        self.expanded_elements += count;
        Ok(())
    }

    fn is_at_end(&self) -> bool {
        self.pos >= self.bytes.len()
    }
}

struct RowState {
    has_value: bool,
}

struct RecordSchema {
    fields: Vec<SchemaField>,
    field_count: usize,
    value_count: usize,
    max_path_depth: usize,
}

struct SchemaField {
    key: Vec<u16>,
    nested: Option<RecordSchema>,
}

impl RecordSchema {
    fn new(fields: Vec<SchemaField>) -> Self {
        let mut field_count = fields.len();
        let mut value_count = 0;
        let mut max_path_depth = usize::from(!fields.is_empty());
        for field in &fields {
            match &field.nested {
                None => value_count += 1,
                Some(nested) => {
                    field_count += nested.field_count;
                    value_count += nested.value_count;
                    max_path_depth = max_path_depth.max(nested.max_path_depth + 1);
                }
            }
        }
        Self {
            fields,
            field_count,
            value_count,
            max_path_depth,
        }
    }
}

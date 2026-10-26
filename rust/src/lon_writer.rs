//! Canonical LON serialization of node trees (port of `lon_writer.dart`).

use crate::error::LonError;
use crate::json_string::{
    exact_string_from_units, json_string_utf8_length, utf8_length, write_json_string,
};
use crate::json_writer::LimitedWriter;
use crate::node::{Member, Node};
use crate::syntax::{bare_eligible_units, is_reserved_ascii_bytes, is_reserved_lon_token};

pub(crate) struct LonWriter {
    use_tables: bool,
    pretty: bool,
    max_output_code_units: usize,
    /// Reused widening scratch for bare ASCII strings, so the hot path never
    /// allocates a `String` per value.
    scratch: Vec<u8>,
}

impl LonWriter {
    pub(crate) fn new(use_tables: bool, pretty: bool, max_output_code_units: usize) -> Self {
        Self {
            use_tables,
            pretty,
            max_output_code_units,
            scratch: Vec::new(),
        }
    }

    pub(crate) fn write(&mut self, node: &Node) -> Result<String, LonError> {
        let capacity = crate::json_writer::estimate_output_bytes(node);
        let mut out = LimitedWriter::with_capacity(self.max_output_code_units, capacity);
        self.write_value(&mut out, node, 0, false)?;
        Ok(out.into_string())
    }

    fn write_value(
        &mut self,
        out: &mut LimitedWriter,
        node: &Node,
        depth: usize,
        inline: bool,
    ) -> Result<(), LonError> {
        match node {
            Node::Null => out.push_str("null"),
            Node::Bool(true) => out.push_str("true"),
            Node::Bool(false) => out.push_str("false"),
            Node::Number(lexeme) => out.push_str(lexeme),
            Node::String(units) => self.write_string(out, units),
            Node::Array(values) => {
                if self.use_tables && self.write_record_table(out, values, depth, inline)? {
                    return Ok(());
                }
                self.write_array(out, values, depth, inline)
            }
            Node::Object(members) => self.write_object(out, members, depth, inline),
        }
    }

    /// Spaces separate array items only when every item is a safely bare string.
    fn write_array(
        &mut self,
        out: &mut LimitedWriter,
        values: &[Node],
        depth: usize,
        inline: bool,
    ) -> Result<(), LonError> {
        out.push_byte(b'[')?;
        let use_spaces = !values.is_empty()
            && values.iter().all(|value| match value {
                Node::String(units) => self.is_bare_string(units),
                _ => false,
            });
        if use_spaces {
            // The scan already validated every element, so write the bare form
            // without revalidation. Bare strings are never containers, so the
            // array stays on one line even in pretty mode.
            for (index, value) in values.iter().enumerate() {
                if index > 0 {
                    out.push_byte(b' ')?;
                }
                let Node::String(units) = value else {
                    unreachable!("checked above");
                };
                if self.fill_ascii_scratch(units) {
                    out.push_ascii_bytes(&self.scratch)?;
                } else {
                    // A bare non-ASCII string never holds lone surrogates, so
                    // its exact text always exists.
                    let text =
                        exact_string_from_units(units).expect("a bare string has scalar values");
                    out.push_str(&text)?;
                }
            }
            return out.push_byte(b']');
        }
        let multiline = self.pretty && !inline && values.iter().any(is_container);
        for (index, value) in values.iter().enumerate() {
            if multiline {
                self.write_line_break(out, depth + 1)?;
            } else if index > 0 {
                out.push_byte(b',')?;
            }
            self.write_value(out, value, depth + 1, inline)?;
            if multiline && index + 1 != values.len() {
                out.push_byte(b',')?;
            }
        }
        if multiline {
            self.write_line_break(out, depth)?;
        }
        out.push_byte(b']')
    }

    fn write_object(
        &mut self,
        out: &mut LimitedWriter,
        members: &[Member],
        depth: usize,
        inline: bool,
    ) -> Result<(), LonError> {
        out.push_byte(b'{')?;
        let multiline = self.pretty && !inline && !members.is_empty();
        for (index, member) in members.iter().enumerate() {
            if multiline {
                self.write_line_break(out, depth + 1)?;
            } else if index > 0 {
                out.push_byte(b' ')?;
            }
            self.write_key(out, &member.key)?;
            out.push_byte(b':')?;
            if multiline {
                out.push_byte(b' ')?;
            }
            self.write_value(out, &member.value, depth + 1, inline)?;
        }
        if multiline {
            self.write_line_break(out, depth)?;
        }
        out.push_byte(b'}')
    }

    fn write_record_table(
        &mut self,
        out: &mut LimitedWriter,
        values: &[Node],
        depth: usize,
        inline: bool,
    ) -> Result<bool, LonError> {
        if values.len() < 2 {
            return Ok(false);
        }
        let mut rows = Vec::with_capacity(values.len());
        for value in values {
            let Node::Object(members) = value else {
                return Ok(false);
            };
            rows.push(members.as_slice());
        }

        let Some(schema) = Schema::infer(&rows) else {
            return Ok(false);
        };
        if schema.fields.is_empty()
            || schema.value_count < 2
            || !self.should_use_table(rows.len(), &schema)
        {
            return Ok(false);
        }

        out.push_byte(b'[')?;
        if self.pretty && !inline {
            self.write_line_break(out, depth + 1)?;
            self.write_schema(out, &schema)?;
            for members in rows {
                out.push_byte(b';')?;
                self.write_line_break(out, depth + 1)?;
                self.write_row_values(out, &schema, members, depth + 1, false)?;
            }
            self.write_line_break(out, depth)?;
            out.push_byte(b']')?;
            return Ok(true);
        }

        self.write_schema(out, &schema)?;
        for members in rows {
            out.push_byte(b';')?;
            self.write_row_values(out, &schema, members, depth + 1, false)?;
        }
        out.push_byte(b']')?;
        Ok(true)
    }

    /// The deterministic table compactness gate: three or more rows or three
    /// or more field names always factor; two-row two-field schemas compare
    /// the structural UTF-8 byte cost against the ordinary form. Saturating
    /// multiplication keeps the comparison safe on 32-bit targets without
    /// changing any 64-bit decision.
    fn should_use_table(&self, row_count: usize, schema: &Schema) -> bool {
        if row_count >= 3 || schema.field_count >= 3 {
            return true;
        }
        let ordinary_bytes = row_count + 1 + row_count.saturating_mul(schema.object_structural_len);
        let table_bytes = 2 + schema.encoded_len + row_count.saturating_mul(schema.value_count);
        table_bytes <= ordinary_bytes
    }

    /// Row cells always render inline: a table row stays on one line even
    /// when a cell holds a nested table or object.
    fn write_row_values(
        &mut self,
        out: &mut LimitedWriter,
        schema: &Schema,
        members: &[Member],
        depth: usize,
        mut has_value: bool,
    ) -> Result<bool, LonError> {
        for (index, field) in schema.fields.iter().enumerate() {
            let member = &members[index];
            match &field.nested {
                None => {
                    if has_value {
                        out.push_byte(b' ')?;
                    }
                    self.write_value(out, &member.value, depth, true)?;
                    has_value = true;
                }
                Some(nested) => {
                    let Node::Object(nested_members) = &member.value else {
                        unreachable!(
                            "schema inference keeps nested fields aligned with object cells"
                        );
                    };
                    has_value =
                        self.write_row_values(out, nested, nested_members, depth, has_value)?;
                }
            }
        }
        Ok(has_value)
    }

    fn write_line_break(&self, out: &mut LimitedWriter, depth: usize) -> Result<(), LonError> {
        out.push_byte(b'\n')?;
        for _ in 0..depth {
            out.push_str("  ")?;
        }
        Ok(())
    }

    /// Whether the string value can render unquoted, without building it.
    fn is_bare_string(&mut self, units: &[u16]) -> bool {
        if !bare_eligible_units(units) {
            return false;
        }
        if self.fill_ascii_scratch(units) {
            !is_reserved_ascii_bytes(&self.scratch)
        } else {
            // Rare non-ASCII candidate: the Unicode fold needs the real text.
            match exact_string_from_units(units) {
                Some(text) => !is_reserved_lon_token(&text),
                None => false,
            }
        }
    }

    /// Widens `units` into the scratch buffer when they are all ASCII.
    /// Returns false for non-ASCII input, which needs exact decoding.
    fn fill_ascii_scratch(&mut self, units: &[u16]) -> bool {
        if !units.iter().all(|&u| u < 0x80) {
            return false;
        }
        self.scratch.clear();
        self.scratch.extend(units.iter().map(|&u| u as u8));
        true
    }

    /// Writes a string value bare when allowed, quoted JSON otherwise.
    fn write_string(&mut self, out: &mut LimitedWriter, units: &[u16]) -> Result<(), LonError> {
        if bare_eligible_units(units) {
            if self.fill_ascii_scratch(units) {
                if !is_reserved_ascii_bytes(&self.scratch) {
                    return out.push_ascii_bytes(&self.scratch);
                }
            } else if let Some(text) = exact_string_from_units(units) {
                if !is_reserved_lon_token(&text) {
                    return out.push_str(&text);
                }
            }
        }
        write_json_string(out, units)
    }

    /// Writes an object key bare when allowed, quoted JSON otherwise.
    /// Reserved spellings stay bare in the key position.
    fn write_key(&mut self, out: &mut LimitedWriter, units: &[u16]) -> Result<(), LonError> {
        if bare_eligible_units(units) {
            if self.fill_ascii_scratch(units) {
                return out.push_ascii_bytes(&self.scratch);
            }
            if let Some(text) = exact_string_from_units(units) {
                return out.push_str(&text);
            }
        }
        write_json_string(out, units)
    }

    fn write_schema(&mut self, out: &mut LimitedWriter, schema: &Schema) -> Result<(), LonError> {
        for (index, field) in schema.fields.iter().enumerate() {
            if index > 0 {
                out.push_byte(b' ')?;
            }
            self.write_key(out, &field.key)?;
            if let Some(nested) = &field.nested {
                out.push_byte(b'{')?;
                self.write_schema(out, nested)?;
                out.push_byte(b'}')?;
            }
        }
        Ok(())
    }
}

fn is_container(node: &Node) -> bool {
    matches!(node, Node::Array(_) | Node::Object(_))
}

struct Schema {
    fields: Vec<SchemaField>,
    field_count: usize,
    encoded_len: usize,
    object_structural_len: usize,
    value_count: usize,
}

struct SchemaField {
    key: Vec<u16>,
    nested: Option<Schema>,
}

impl Schema {
    fn new(fields: Vec<SchemaField>) -> Self {
        let mut field_count = fields.len();
        // The schema separates fields with single spaces; an empty schema has none.
        let mut encoded_len = fields.len().saturating_sub(1);
        let mut object_structural_len = 2;
        let mut value_count = 0;

        for (index, field) in fields.iter().enumerate() {
            let key_len = key_byte_length(&field.key);
            encoded_len += key_len;
            object_structural_len += key_len + 1 + usize::from(index != 0);
            match &field.nested {
                None => value_count += 1,
                Some(nested) => {
                    field_count += nested.field_count;
                    encoded_len += 2 + nested.encoded_len;
                    object_structural_len += nested.object_structural_len;
                    value_count += nested.value_count;
                }
            }
        }

        Self {
            fields,
            field_count,
            encoded_len,
            object_structural_len,
            value_count,
        }
    }

    /// Infers the common schema of uniform records, or `None` when the rows
    /// differ in field names, order, count, or object/non-object cell kinds.
    fn infer(rows: &[&[Member]]) -> Option<Self> {
        let first = rows.first()?;
        for row in &rows[1..] {
            if row.len() != first.len() {
                return None;
            }
            if row.iter().zip(first.iter()).any(|(a, b)| a.key != b.key) {
                return None;
            }
        }

        let mut fields = Vec::with_capacity(first.len());
        for (index, member) in first.iter().enumerate() {
            let nested = match &member.value {
                Node::Object(_) => {
                    let mut nested_rows = Vec::with_capacity(rows.len());
                    for row in rows {
                        let Node::Object(nested_members) = &row[index].value else {
                            return None;
                        };
                        nested_rows.push(nested_members.as_slice());
                    }
                    Some(Self::infer(&nested_rows)?)
                }
                _ => {
                    if rows[1..]
                        .iter()
                        .any(|row| matches!(row[index].value, Node::Object(_)))
                    {
                        return None;
                    }
                    None
                }
            };
            fields.push(SchemaField {
                key: member.key.clone(),
                nested,
            });
        }
        Some(Self::new(fields))
    }
}

fn key_byte_length(key: &[u16]) -> usize {
    if bare_eligible_units(key) {
        utf8_length(key)
    } else {
        json_string_utf8_length(key)
    }
}

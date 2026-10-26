//! Conversions between node trees and `serde_json::Value` (port of
//! `json_value_converter.dart`).

use crate::error::LonError;
use crate::json_string::{string_from_units_lossy, units_from_str};
use crate::node::{Member, Node};
use serde_json::{Map, Number, Value};

/// Converts a JSON value into a node tree, enforcing depth and element limits.
pub(crate) fn node_from_value(
    value: &Value,
    max_depth: usize,
    max_expanded_elements: usize,
) -> Result<Node, LonError> {
    ValueConverter {
        max_depth,
        max_expanded_elements,
        expanded_elements: 0,
    }
    .convert(value, 0)
}

struct ValueConverter {
    max_depth: usize,
    max_expanded_elements: usize,
    expanded_elements: usize,
}

impl ValueConverter {
    fn convert(&mut self, value: &Value, depth: usize) -> Result<Node, LonError> {
        if depth > self.max_depth {
            return Err(LonError::invalid_value(format!(
                "Maximum nesting depth of {} exceeded",
                self.max_depth
            )));
        }
        Ok(match value {
            Value::Null => Node::Null,
            Value::Bool(boolean) => Node::Bool(*boolean),
            Value::Number(number) => {
                if number.as_f64().is_some_and(|f| !f.is_finite()) {
                    return Err(LonError::invalid_value("JSON numbers must be finite"));
                }
                Node::Number(number.to_string())
            }
            Value::String(text) => Node::String(units_from_str(text)),
            Value::Array(items) => {
                let mut values = Vec::with_capacity(items.len());
                for item in items {
                    self.reserve()?;
                    values.push(self.convert(item, depth + 1)?);
                }
                Node::Array(values)
            }
            Value::Object(members) => {
                let mut entries = Vec::with_capacity(members.len());
                for (key, item) in members {
                    self.reserve()?;
                    let value = self.convert(item, depth + 1)?;
                    entries.push(Member {
                        key: units_from_str(key),
                        value,
                    });
                }
                Node::Object(entries)
            }
        })
    }

    fn reserve(&mut self) -> Result<(), LonError> {
        if self.expanded_elements >= self.max_expanded_elements {
            return Err(LonError::invalid_value(format!(
                "Expanded element limit of {} exceeded",
                self.max_expanded_elements
            )));
        }
        self.expanded_elements += 1;
        Ok(())
    }
}

/// Converts a node tree into a JSON value.
///
/// Duplicate object keys keep their first position and the last value wins,
/// matching JSON map semantics. Unpaired surrogates in strings decode to
/// U+FFFD exactly like `serde_json`'s own parser, because a Rust string cannot
/// hold them; use the text codecs when surrogate-level fidelity is required.
pub(crate) fn node_to_value(node: &Node) -> Result<Value, LonError> {
    Ok(match node {
        Node::Null => Value::Null,
        Node::Bool(boolean) => Value::Bool(*boolean),
        Node::Number(lexeme) => Value::Number(number_from_lexeme(lexeme)?),
        Node::String(units) => Value::String(string_from_units_lossy(units)),
        Node::Array(values) => {
            let mut items = Vec::with_capacity(values.len());
            for value in values {
                items.push(node_to_value(value)?);
            }
            Value::Array(items)
        }
        Node::Object(members) => {
            let mut map = Map::with_capacity(members.len());
            for Member { key, value } in members {
                map.insert(string_from_units_lossy(key), node_to_value(value)?);
            }
            Value::Object(map)
        }
    })
}

fn number_from_lexeme(lexeme: &str) -> Result<Number, LonError> {
    if !lexeme.contains('.') && !lexeme.contains(['e', 'E']) {
        if let Ok(integer) = lexeme.parse::<i64>() {
            return Ok(integer.into());
        }
        if let Ok(unsigned) = lexeme.parse::<u64>() {
            return Ok(unsigned.into());
        }
    }
    lexeme
        .parse::<f64>()
        .ok()
        .filter(|f| f.is_finite())
        .and_then(Number::from_f64)
        .ok_or_else(|| LonError::format("Number out of range"))
}

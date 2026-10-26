//! Lossless JSON codec for [Language Object Notation (LON) v1], a compact and
//! canonical text encoding of the JSON data model designed for AI-first data
//! exchange. LON keeps familiar JSON delimiters, removes unnecessary quotes
//! and commas, and factors repeated record keys into visible
//! `[SCHEMA;ROW;ROW]` tables.
//!
//! [Language Object Notation (LON) v1]: https://github.com/fluttercandies/lon/blob/main/spec/lon-v1.md
//!
//! The text-to-text functions are the lossless core: they preserve
//! object-member order, duplicate members, string code units (including
//! escaped lone surrogates), and number lexemes.
//!
//! ```
//! let lon = lon::json_to_lon(r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}"#).unwrap();
//! assert_eq!(lon, "{users:[id name;1 Ada;2 Bob]}");
//! assert_eq!(
//!     lon::lon_to_json(&lon).unwrap(),
//!     r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}"#,
//! );
//! ```
//!
//! A [`serde_json::Value`] convenience layer is also available. Duplicate keys
//! decode with the last value winning and object order is preserved via
//! serde_json's `preserve_order` feature:
//!
//! ```
//! use serde_json::json;
//!
//! let value = lon::LonCodec::new().decode("[id name;1 Ada;2 Bob]").unwrap();
//! assert_eq!(value, json!([{"id": 1, "name": "Ada"}, {"id": 2, "name": "Bob"}]));
//! assert_eq!(lon::LonCodec::new().encode(&value).unwrap(), "[id name;1 Ada;2 Bob]");
//! ```

mod error;
mod json_parser;
mod json_string;
mod json_writer;
mod lon_parser;
mod lon_writer;
mod node;
mod serde_impl;
mod syntax;
mod value;

pub use error::{ErrorKind, LonError};

use json_parser::JsonTextParser;
use json_writer::{estimate_output_bytes, write_json_node, LimitedWriter};
use lon_parser::LonTextParser;
use lon_writer::LonWriter;
use node::Node;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// The normative Language Object Notation version implemented by this crate.
pub const LON_FORMAT_VERSION: u32 = 1;

/// Default limits, identical to the Dart reference codec.
const DEFAULT_MAX_DEPTH: usize = 512;
const DEFAULT_MAX_EXPANDED_ELEMENTS: usize = 1_000_000;
const DEFAULT_MAX_CODE_UNITS: usize = 64 * 1024 * 1024;

/// Configurable LON v1 codec with the reference defaults.
///
/// Limits are `usize`, so a negative limit is unrepresentable at the type
/// level; the Dart codec's `RangeError` for negative limits has no Rust
/// counterpart to raise. Both input and output limits count UTF-16 code units
/// exactly like the Dart codec and the specification.
///
/// ```
/// use serde_json::json;
///
/// let compact = lon::LonCodec::new()
///     .use_tables(false)
///     .max_output_code_units(1024);
/// assert_eq!(
///     compact.encode(&json!({"a": 1, "b": 2})).unwrap(),
///     "{a:1 b:2}",
/// );
/// ```
#[derive(Debug, Clone, Copy)]
pub struct LonCodec {
    use_tables: bool,
    pretty: bool,
    max_depth: usize,
    max_expanded_elements: usize,
    max_input_code_units: usize,
    max_output_code_units: usize,
}

impl LonCodec {
    /// A codec with the default limits: depth 512, 1,000,000 expanded
    /// elements, 67,108,864 input and output code units, tables enabled.
    pub const fn new() -> Self {
        Self {
            use_tables: true,
            pretty: false,
            max_depth: DEFAULT_MAX_DEPTH,
            max_expanded_elements: DEFAULT_MAX_EXPANDED_ELEMENTS,
            max_input_code_units: DEFAULT_MAX_CODE_UNITS,
            max_output_code_units: DEFAULT_MAX_CODE_UNITS,
        }
    }

    /// Enables or disables self-describing record tables (enabled by default).
    pub const fn use_tables(mut self, use_tables: bool) -> Self {
        self.use_tables = use_tables;
        self
    }

    /// Renders encoded LON across lines, indented two spaces per nesting
    /// depth: every object member, table row, and container-array element
    /// occupies its own line between the container's brackets, while table
    /// rows and scalar arrays stay inline. Also selects standard
    /// two-space-indented JSON from [`LonCodec::lon_to_json`]. Both outputs
    /// decode to exactly the same values as the compact forms.
    ///
    /// ```
    /// let pretty = lon::LonCodec::new().pretty(true);
    /// assert_eq!(
    ///     pretty
    ///         .json_to_lon(r#"{"users":[{"id":1,"name":"Ada"},{"id":2,"name":"Bob"}]}"#)
    ///         .unwrap(),
    ///     "{\n  users: [\n    id name;\n    1 Ada;\n    2 Bob\n  ]\n}",
    /// );
    /// ```
    pub const fn pretty(mut self, pretty: bool) -> Self {
        self.pretty = pretty;
        self
    }

    /// Sets the maximum accepted or encoded container nesting depth.
    pub const fn max_depth(mut self, max_depth: usize) -> Self {
        self.max_depth = max_depth;
        self
    }

    /// Sets the maximum parsed or encoded container-element occurrences. A
    /// table's schema-field count is bounded independently by the same value.
    pub const fn max_expanded_elements(mut self, max_expanded_elements: usize) -> Self {
        self.max_expanded_elements = max_expanded_elements;
        self
    }

    /// Sets the maximum accepted source length in UTF-16 code units.
    pub const fn max_input_code_units(mut self, max_input_code_units: usize) -> Self {
        self.max_input_code_units = max_input_code_units;
        self
    }

    /// Sets the maximum serialized output length in UTF-16 code units.
    pub const fn max_output_code_units(mut self, max_output_code_units: usize) -> Self {
        self.max_output_code_units = max_output_code_units;
        self
    }

    /// Encodes a `serde_json` value as canonical LON.
    ///
    /// Non-finite floats and graphs exceeding the depth or element limits are
    /// rejected with [`ErrorKind::InvalidValue`] instead of producing a
    /// truncated document.
    pub fn encode(&self, value: &serde_json::Value) -> Result<String, LonError> {
        let node = value::node_from_value(value, self.max_depth, self.max_expanded_elements)?;
        self.write_lon(&node)
    }

    /// Serializes any `Serialize` value directly to LON, reusing this
    /// codec's limits, table setting, and pretty mode.
    ///
    /// `Vec<T>` and other sequences of uniform structs hit the record-table
    /// form automatically. Non-finite floats, `i128`/`u128` values outside
    /// the `i64`/`u64` ranges, and over-budget graphs are rejected.
    ///
    /// ```
    /// #[derive(serde::Serialize)]
    /// struct User { id: u32, name: String }
    ///
    /// let users = vec![
    ///     User { id: 1, name: "Ada".into() },
    ///     User { id: 2, name: "Bob".into() },
    /// ];
    /// assert_eq!(
    ///     lon::LonCodec::new().to_string(&users).unwrap(),
    ///     "[id name;1 Ada;2 Bob]",
    /// );
    /// ```
    pub fn to_string<T: ?Sized + Serialize>(&self, value: &T) -> Result<String, LonError> {
        let node = serde_impl::to_node(self, value)?;
        self.write_lon(&node)
    }

    /// Deserializes any `DeserializeOwned` type from LON, reusing this
    /// codec's input limits.
    ///
    /// Duplicate object keys keep their first position and the last value
    /// wins. Escaped lone surrogates decode as U+FFFD, matching
    /// `serde_json`. Use [`LonCodec::lon_to_json`] when duplicate-key and
    /// number-lexeme preservation is required.
    pub fn from_str<T: DeserializeOwned>(&self, source: &str) -> Result<T, LonError> {
        self.check_input(source)?;
        let node = self.parse_lon(source)?;
        serde_impl::from_node(&node)
    }

    /// Decodes LON into a `serde_json` value.
    ///
    /// Duplicate object keys keep their first position and the last value
    /// wins. Use [`LonCodec::lon_to_json`] when duplicate-key and
    /// number-lexeme preservation is required.
    pub fn decode(&self, source: &str) -> Result<serde_json::Value, LonError> {
        self.check_input(source)?;
        let node = self.parse_lon(source)?;
        value::node_to_value(&node)
    }

    /// Converts JSON text to canonical LON while preserving member order,
    /// duplicate keys, and number lexemes.
    pub fn json_to_lon(&self, source: &str) -> Result<String, LonError> {
        self.check_input(source)?;
        let node =
            JsonTextParser::new(source, self.max_depth, self.max_expanded_elements).parse()?;
        self.write_lon(&node)
    }

    /// Converts LON to JSON while preserving member order, duplicate keys,
    /// and number lexemes. Output is compact unless [`LonCodec::pretty`]
    /// selects standard two-space indentation.
    pub fn lon_to_json(&self, source: &str) -> Result<String, LonError> {
        self.check_input(source)?;
        let node = self.parse_lon(source)?;
        let capacity = estimate_output_bytes(&node);
        let mut out = LimitedWriter::with_capacity(self.max_output_code_units, capacity);
        write_json_node(&mut out, &node, 0, self.pretty)?;
        Ok(out.into_string())
    }

    fn parse_lon(&self, source: &str) -> Result<Node, LonError> {
        LonTextParser::new(source, self.max_depth, self.max_expanded_elements).parse()
    }

    fn write_lon(&self, node: &Node) -> Result<String, LonError> {
        LonWriter::new(self.use_tables, self.pretty, self.max_output_code_units).write(node)
    }

    fn check_input(&self, source: &str) -> Result<(), LonError> {
        // UTF-16 code units never exceed UTF-8 bytes, so a source within the
        // byte budget is within the unit budget without counting anything.
        if source.len() <= self.max_input_code_units {
            return Ok(());
        }
        let units = if source.is_ascii() {
            source.len()
        } else {
            source
                .chars()
                .map(|c| usize::from((c as u32) >= 0x10000) + 1)
                .sum()
        };
        if units > self.max_input_code_units {
            return Err(LonError::format(format!(
                "Input code-unit limit of {} exceeded",
                self.max_input_code_units
            )));
        }
        Ok(())
    }
}

impl Default for LonCodec {
    fn default() -> Self {
        Self::new()
    }
}

/// Converts JSON text to canonical Language Object Notation with the default
/// limits. See [`LonCodec::json_to_lon`].
///
/// ```
/// assert_eq!(lon::json_to_lon("[1,2]").unwrap(), "[1,2]");
/// ```
pub fn json_to_lon(source: &str) -> Result<String, LonError> {
    LonCodec::new().json_to_lon(source)
}

/// Converts Language Object Notation to compact JSON text with the default
/// limits. See [`LonCodec::lon_to_json`].
///
/// ```
/// assert_eq!(lon::lon_to_json("{a:1}").unwrap(), r#"{"a":1}"#);
/// ```
pub fn lon_to_json(source: &str) -> Result<String, LonError> {
    LonCodec::new().lon_to_json(source)
}

/// Serializes any `Serialize` value to canonical LON with the default
/// limits. See [`LonCodec::to_string`].
///
/// ```
/// assert_eq!(lon::to_string(&vec!["Ada", "Bob"]).unwrap(), "[Ada Bob]");
/// ```
pub fn to_string<T: ?Sized + Serialize>(value: &T) -> Result<String, LonError> {
    LonCodec::new().to_string(value)
}

/// Serializes any `Serialize` value to multi-line LON with the default
/// limits. See [`LonCodec::pretty`].
///
/// ```
/// assert_eq!(
///     lon::to_string_pretty(&vec![vec![1, 2], vec![3, 4]]).unwrap(),
///     "[\n  [1,2],\n  [3,4]\n]",
/// );
/// ```
pub fn to_string_pretty<T: ?Sized + Serialize>(value: &T) -> Result<String, LonError> {
    LonCodec::new().pretty(true).to_string(value)
}

/// Deserializes any `DeserializeOwned` type from LON with the default
/// limits. See [`LonCodec::from_str`].
///
/// ```
/// let words: Vec<String> = lon::from_str("[hello world]").unwrap();
/// assert_eq!(words, ["hello", "world"]);
/// ```
pub fn from_str<T: DeserializeOwned>(source: &str) -> Result<T, LonError> {
    LonCodec::new().from_str(source)
}

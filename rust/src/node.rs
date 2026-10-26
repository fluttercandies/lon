//! The shared JSON data-model tree (port of `json_node.dart`).
//!
//! Strings live as UTF-16 code-unit vectors so that escaped lone surrogates
//! from JSON or LON input survive every conversion unchanged, exactly like a
//! Dart string. Rust `String` cannot hold lone surrogates, so this is the
//! node-level representation that keeps the format's string domain lossless.

/// A parsed JSON value.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Node {
    Null,
    Bool(bool),
    /// The original number lexeme, preserved verbatim (for example `1e+02` or `-0`).
    Number(String),
    /// UTF-16 code units; may contain escaped lone surrogates.
    String(Vec<u16>),
    Array(Vec<Node>),
    Object(Vec<Member>),
}

/// An object member. Objects keep an ordered member list so duplicate keys
/// survive text-to-text conversion.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Member {
    pub(crate) key: Vec<u16>,
    pub(crate) value: Node,
}

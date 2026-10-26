//! The single error type for every LON encoding and decoding failure.

use std::fmt;

/// The reason a LON operation failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    /// The input text is not valid JSON or LON, or a parse, expansion, input,
    /// or output limit was exceeded while processing it. This corresponds to
    /// the Dart reference implementation's `FormatException`.
    Format,
    /// A caller-supplied value cannot be encoded as LON (non-finite number,
    /// excessive nesting, or an over-budget value graph). This corresponds to
    /// the Dart reference implementation's `ArgumentError`.
    InvalidValue,
}

/// The error returned by every [`crate::LonCodec`] operation.
///
/// The optional offset is a byte offset into the rejected input text.
#[derive(Debug, Clone)]
pub struct LonError {
    kind: ErrorKind,
    message: String,
    offset: Option<usize>,
}

impl LonError {
    pub(crate) fn format(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::Format,
            message: message.into(),
            offset: None,
        }
    }

    pub(crate) fn format_at(message: impl Into<String>, offset: usize) -> Self {
        Self {
            kind: ErrorKind::Format,
            message: message.into(),
            offset: Some(offset),
        }
    }

    pub(crate) fn invalid_value(message: impl Into<String>) -> Self {
        Self {
            kind: ErrorKind::InvalidValue,
            message: message.into(),
            offset: None,
        }
    }

    /// The category of failure.
    pub fn kind(&self) -> ErrorKind {
        self.kind
    }

    /// The byte offset into the input where the failure was detected, if any.
    pub fn offset(&self) -> Option<usize> {
        self.offset
    }

    /// The diagnostic message. It never quotes untrusted input text.
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for LonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.offset {
            Some(offset) => write!(f, "{} at offset {}", self.message, offset),
            None => f.write_str(&self.message),
        }
    }
}

impl std::error::Error for LonError {}

impl serde::ser::Error for LonError {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::invalid_value(message.to_string())
    }
}

impl serde::de::Error for LonError {
    fn custom<T: fmt::Display>(message: T) -> Self {
        Self::format(message.to_string())
    }
}

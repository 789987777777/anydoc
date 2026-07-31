//! Typed conversion errors.
//!
//! An error means meaningful conversion was impossible: the input was
//! unreadable or structurally unusable, encrypted, or crossed a fixed
//! safety/resource limit. Recoverable producer quirks never surface here -
//! they are recovered or skipped (and logged via the `log` facade) while
//! conversion continues.

use std::fmt;

/// Why a conversion could not produce a useful result.
#[derive(Debug)]
#[non_exhaustive]
pub enum ConvertError {
    /// The format is unknown or cannot be converted at all.
    Unsupported(String),
    /// The document is structurally unusable - no meaningful content could be
    /// extracted. `part` names the package part or stream when known.
    Malformed { part: Option<String>, detail: String },
    /// The document is encrypted or password-protected.
    Encrypted,
    /// A fixed safety limit was exceeded (decompression, nesting depth, node
    /// count, repeat expansion, retained asset bytes). These are hard errors
    /// in every case; see `package::limits` for the documented defaults.
    ResourceLimit { limit: &'static str, detail: String },
    /// A part required for any meaningful output is missing.
    MissingPart { part: String },
    /// The input could not be read.
    Io(std::io::Error),
}

impl fmt::Display for ConvertError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConvertError::Unsupported(what) => write!(f, "unsupported input: {what}"),
            ConvertError::Malformed { part: Some(part), detail } => {
                write!(f, "malformed document ({part}): {detail}")
            }
            ConvertError::Malformed { part: None, detail } => {
                write!(f, "malformed document: {detail}")
            }
            ConvertError::Encrypted => write!(f, "document is encrypted"),
            ConvertError::ResourceLimit { limit, detail } => {
                write!(f, "resource limit exceeded ({limit}): {detail}")
            }
            ConvertError::MissingPart { part } => write!(f, "missing required part: {part}"),
            ConvertError::Io(e) => write!(f, "io error: {e}"),
        }
    }
}

impl std::error::Error for ConvertError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ConvertError::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for ConvertError {
    fn from(e: std::io::Error) -> Self {
        ConvertError::Io(e)
    }
}

impl ConvertError {
    pub(crate) fn malformed(detail: impl Into<String>) -> Self {
        ConvertError::Malformed { part: None, detail: detail.into() }
    }

    pub(crate) fn malformed_part(part: impl Into<String>, detail: impl Into<String>) -> Self {
        ConvertError::Malformed { part: Some(part.into()), detail: detail.into() }
    }

    /// True when recovery must not swallow this error: fixed safety limits
    /// hard-fail in every context, including optional parts.
    pub(crate) fn is_fatal(&self) -> bool {
        matches!(self, ConvertError::ResourceLimit { .. })
    }
}

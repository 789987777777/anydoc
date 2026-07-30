//! anydoc converts documents to GitHub-Flavored Markdown.

pub mod ir;

mod formats;
mod markdown;
mod support;

use anyhow::{bail, Context, Result};
use std::path::Path;

pub use markdown::document_to_markdown;

/// Input format. Selects the parser; container variants that share a parser
/// (docm, xlsm, ...) map onto these via [`Format::from_extension`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    Doc,
    Docx,
    Odt,
    Rtf,
    Epub,
    Xlsx,
    Xls,
    Ods,
    Csv,
}

impl Format {
    pub fn from_extension(ext: &str) -> Option<Format> {
        Some(match ext.to_ascii_lowercase().as_str() {
            "doc" => Format::Doc,
            "docx" | "docm" => Format::Docx,
            "odt" => Format::Odt,
            "rtf" => Format::Rtf,
            "epub" => Format::Epub,
            "xlsx" | "xlsm" | "xlsb" => Format::Xlsx,
            "xls" => Format::Xls,
            "ods" => Format::Ods,
            "csv" => Format::Csv,
            _ => return None,
        })
    }

    pub fn from_path(path: &Path) -> Option<Format> {
        path.extension().and_then(|e| e.to_str()).and_then(Format::from_extension)
    }
}

/// Convert a document file to Markdown, inferring the format from its extension.
pub fn to_markdown(path: impl AsRef<Path>) -> Result<String> {
    let path = path.as_ref();
    let Some(format) = Format::from_path(path) else {
        bail!("unsupported or missing file extension: {}", path.display());
    };
    let bytes =
        std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    to_markdown_bytes(&bytes, format)
        .with_context(|| format!("failed to convert {}", path.display()))
}

/// Convert an in-memory document to Markdown.
pub fn to_markdown_bytes(bytes: &[u8], format: Format) -> Result<String> {
    Ok(markdown::document_to_markdown(&to_document(bytes, format)?))
}

/// Parse an in-memory document into the intermediate representation.
pub fn to_document(bytes: &[u8], format: Format) -> Result<ir::Document> {
    formats::parse(bytes, format)
}

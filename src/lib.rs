//! anydoc converts documents to GitHub-Flavored Markdown.
//!
//! One conversion behavior: missing optional parts are ignored, recoverable
//! producer quirks are recovered automatically, and corrupt or unsupported
//! subparts are skipped when useful output can still be produced. An error is
//! returned only when meaningful conversion is impossible, the document is
//! encrypted, or a fixed safety/resource limit is exceeded.
//!
//! Page chrome is excluded by fixed policy in every format: page headers and
//! footers, page numbers, and date/time placeholders never appear in the
//! output. Speaker notes in presentations are always included.
//!
//! Recovery and skipped-content events are reported through the [`log`]
//! facade (debug/warn level); logging never changes conversion behavior and
//! its messages are not a stable API.

pub mod model;

mod error;
mod formats;
mod package;
mod render;
mod shared;

pub use error::ConvertError;

use render::markdown::document_to_markdown;

use std::path::Path;

/// Input format. Selects the parser; container variants that share a parser
/// (docm, xlsm, ...) map onto these via [`Format::from_extension`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Format {
    Doc,
    Docx,
    Odt,
    /// Converted with [pdf-inspector], which emits Markdown directly:
    /// [`to_document`] is unsupported for PDFs. Scanned/image-only PDFs
    /// (needing OCR) error as unsupported.
    ///
    /// [pdf-inspector]: https://github.com/firecrawl/pdf-inspector
    Pdf,
    Ppt,
    Pptx,
    Rtf,
    Epub,
    Excel,
    Ods,
    Odp,
    Csv,
}

impl Format {
    pub fn from_extension(ext: &str) -> Option<Format> {
        Some(match ext.to_ascii_lowercase().as_str() {
            "doc" => Format::Doc,
            "docx" | "docm" => Format::Docx,
            "odt" => Format::Odt,
            "pdf" => Format::Pdf,
            "pptx" | "pptm" | "ppsx" | "ppsm" => Format::Pptx,
            "ppt" | "pps" | "pot" => Format::Ppt,
            "rtf" => Format::Rtf,
            "epub" => Format::Epub,
            "xlsx" | "xlsm" | "xlsb" | "xls" => Format::Excel,
            "ods" => Format::Ods,
            "odp" => Format::Odp,
            "csv" => Format::Csv,
            _ => return None,
        })
    }

    pub fn from_path(path: &Path) -> Option<Format> {
        path.extension().and_then(|e| e.to_str()).and_then(Format::from_extension)
    }
}

/// Convert a document file to Markdown, inferring the format from its extension.
pub fn to_markdown(path: impl AsRef<Path>) -> Result<String, ConvertError> {
    let path = path.as_ref();
    let Some(format) = Format::from_path(path) else {
        return Err(ConvertError::Unsupported(format!(
            "unrecognized file extension: {}",
            path.display()
        )));
    };
    let bytes = std::fs::read(path)?;
    to_markdown_bytes(&bytes, format)
}

/// Convert an in-memory document to Markdown.
pub fn to_markdown_bytes(bytes: &[u8], format: Format) -> Result<String, ConvertError> {
    // PDFs convert to Markdown directly (pdf-inspector) without passing
    // through the document model.
    if format == Format::Pdf {
        return formats::pdf::to_markdown(bytes);
    }
    Ok(document_to_markdown(&to_document(bytes, format)?))
}

/// Parse an in-memory document into the document model.
///
/// Unsupported for [`Format::Pdf`]: PDF conversion produces Markdown
/// directly and has no document-model form; use [`to_markdown_bytes`].
pub fn to_document(bytes: &[u8], format: Format) -> Result<model::Document, ConvertError> {
    formats::parse(bytes, format)
}

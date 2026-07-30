//! One frontend per input format; each parses bytes into the shared IR.

mod csv;
mod doc;
mod docx;
mod epub;
mod odf;
mod rtf;
mod sheet;

use crate::Format;
use crate::ir::Document;
use anyhow::Result;

pub fn parse(bytes: &[u8], format: Format) -> Result<Document> {
    match format {
        Format::Doc => doc::parse(bytes),
        Format::Docx => docx::parse(bytes),
        Format::Odt => odf::parse(bytes),
        Format::Rtf => rtf::parse(bytes),
        Format::Epub => epub::parse(bytes),
        Format::Ods => odf::parse(bytes),
        Format::Xlsx | Format::Xls => sheet::parse(bytes),
        Format::Csv => csv::parse(bytes),
    }
}

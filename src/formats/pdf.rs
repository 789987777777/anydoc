//! PDF via [pdf-inspector]: classification plus direct Markdown extraction.
//!
//! Unlike the other frontends, pdf-inspector emits Markdown itself, so PDFs
//! bypass the document model and the shared GFM writer. OCR is out of scope
//! here. Whole-document conversion of a PDF with scanned or image-only pages
//! errors naming them, whether that is every page or one of a hundred,
//! because output missing those pages would read as complete; per-page
//! output keeps the text pages and marks the rest instead.
//!
//! [pdf-inspector]: https://github.com/firecrawl/pdf-inspector

use crate::Page;
use crate::error::ConvertError;
use pdf_inspector::PdfError;

pub fn to_markdown(bytes: &[u8]) -> Result<String, ConvertError> {
    let result = pdf_inspector::process_pdf_mem(bytes).map_err(map_error)?;
    if !result.pages_needing_ocr.is_empty() {
        // Detection samples content streams and over-reports short or
        // image-heavy text pages; extraction knows which pages yielded none.
        let pages = pdf_inspector::extract_pages_markdown_mem(bytes, None)
            .map_err(map_error)?
            .pages_needing_ocr;
        if !pages.is_empty() {
            return Err(ConvertError::NeedsOcr { pages, page_count: result.page_count });
        }
    }
    if result.has_encoding_issues {
        log::warn!("broken font encodings detected; extracted text may be garbled");
    }
    match result.markdown {
        Some(markdown) if !markdown.trim().is_empty() => Ok(terminated(markdown)),
        _ => Err(ConvertError::Unsupported(format!(
            "PDF has no extractable text ({:?}, {} pages)",
            result.pdf_type, result.page_count
        ))),
    }
}

pub fn to_markdown_pages(bytes: &[u8]) -> Result<Vec<Page>, ConvertError> {
    let result = pdf_inspector::extract_pages_markdown_mem(bytes, None).map_err(map_error)?;
    Ok(result
        .pages
        .into_iter()
        .map(|page| Page {
            number: page.page + 1,
            markdown: if page.markdown.is_empty() {
                page.markdown
            } else {
                terminated(page.markdown)
            },
            needs_ocr: page.needs_ocr,
        })
        .collect())
}

fn terminated(mut markdown: String) -> String {
    if !markdown.ends_with('\n') {
        markdown.push('\n');
    }
    markdown
}

fn map_error(e: PdfError) -> ConvertError {
    match e {
        PdfError::Encrypted => ConvertError::Encrypted,
        PdfError::Io(e) => ConvertError::Io(e),
        PdfError::NotAPdf(detail) => ConvertError::malformed(format!("not a PDF: {detail}")),
        PdfError::InvalidStructure => ConvertError::malformed("invalid PDF structure"),
        PdfError::Parse(detail) => ConvertError::malformed(detail),
    }
}

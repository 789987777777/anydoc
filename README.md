# anydoc

Convert documents to GitHub-Flavored Markdown. A [Firecrawl](https://firecrawl.dev) project.

Every format parses into one shared document model and renders through a single GFM serializer, so escaping, tables, anchors, and footnotes behave the same no matter what you feed in.

## Supported formats

| Format | Extensions |
| --- | --- |
| Word | `.doc`, `.docx`, `.docm` |
| PowerPoint | `.ppt`, `.pps`, `.pot`, `.pptx`, `.pptm`, `.ppsx`, `.ppsm` |
| Excel | `.xls`, `.xlsx`, `.xlsm`, `.xlsb` |
| OpenDocument | `.odt`, `.ods`, `.odp` |
| Rich Text Format | `.rtf` |
| EPUB | `.epub` |
| CSV | `.csv` |
| PDF | `.pdf` |

The format is detected from the file content (container signatures and package metadata), not the extension; the extension is only the fallback for signature-less formats like CSV.

PDFs are converted with [pdf-inspector](https://github.com/firecrawl/pdf-inspector), which emits Markdown directly, so `to_document` is unsupported for them. Scanned and image-only PDFs need OCR, which is out of scope: they error as unsupported.

## Usage

```rust
// From a file path (format detected from content, extension as fallback):
let markdown = anydoc::to_markdown("report.docx")?;

// From bytes:
let format = anydoc::Format::from_bytes(&bytes).unwrap_or(anydoc::Format::Csv);
let markdown = anydoc::to_markdown_bytes(&bytes, format)?;

// Or stop at the document model, which also carries embedded assets:
let document = anydoc::to_document(&bytes, anydoc::Format::Rtf)?;
```

A small CLI for manual testing ships as an example:

```
cargo run --release --example convert -- file.docx [-o out.md] [--bench N]
```

## Conversion behavior

There is exactly one conversion behavior, with no options or modes:

- Missing optional parts are ignored: a document without a stylesheet still converts.
- Recoverable producer quirks are repaired automatically.
- Corrupt or unsupported subparts are skipped when useful output can still be produced: an unreadable sheet, chapter, or slide is dropped and the rest converts.
- An error is returned only when meaningful conversion is impossible, the document is encrypted, or a fixed safety limit is exceeded. Attack-shaped input (decompression bombs, pathological nesting, runaway expansion) always hard-fails; the caps are fixed constants, not configuration.

Page headers and footers are excluded; speaker notes are included. Internal links work end to end. Embedded images and objects are retained as bytes in `Document::assets`, with their alt text rendered in the Markdown. Recovery and skipped-content events are reported through the [`log`](https://docs.rs/log) facade and never change conversion behavior.

## Development

```
cargo test
```

A committed fixture corpus under `tests/fixtures/` is snapshot-tested, `tests/robustness.rs` mutation-tests every fixture, and `fuzz/` carries cargo-fuzz targets per format. A speed and quality benchmark against other converters lives in [`bench/`](bench/README.md).

## License

[MIT](LICENSE)

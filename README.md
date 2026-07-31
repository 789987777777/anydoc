# anydoc

Convert documents to GitHub-Flavored Markdown. A [Firecrawl](https://firecrawl.dev) project.

anydoc parses each format through an explicit resolution pipeline — package parts, then style/numbering/reference resolution, then a shared document model — and serializes with a single GFM writer, so escaping, table fallbacks, emphasis merging, anchors, and footnote numbering behave identically across every input format.

## Supported formats

| Format | Extensions |
| --- | --- |
| Word (legacy binary) | `.doc` |
| Word (OOXML) | `.docx`, `.docm` |
| OpenDocument Text | `.odt` |
| Rich Text Format | `.rtf` |
| EPUB | `.epub` |
| Excel (OOXML) | `.xlsx`, `.xlsm`, `.xlsb` |
| Excel (legacy binary) | `.xls` |
| OpenDocument Spreadsheet | `.ods` |
| PowerPoint (OOXML) | `.pptx`, `.pptm`, `.ppsx`, `.ppsm` |
| PowerPoint (legacy binary) | `.ppt`, `.pps`, `.pot` |
| OpenDocument Presentation | `.odp` |
| CSV | `.csv` |
| PDF | `.pdf` |

PDFs are converted with [pdf-inspector](https://github.com/firecrawl/pdf-inspector), which emits Markdown directly — they bypass the document model, so `to_document` is unsupported for them. Scanned and image-only PDFs need OCR, which is out of scope: they error as unsupported.

## Usage

```rust
// From a file path (format inferred from the extension):
let markdown = anydoc::to_markdown("report.docx")?;

// From bytes:
let markdown = anydoc::to_markdown_bytes(&bytes, anydoc::Format::Docx)?;

// Or stop at the document model (which also carries embedded assets):
let document = anydoc::to_document(&bytes, anydoc::Format::Rtf)?;
```

A small CLI for manual testing and benchmarking ships as an example:

```
cargo run --release --example convert -- file.docx [-o out.md] [--bench N]
```

## Conversion behavior

There is exactly one conversion behavior — no options or modes:

- **Missing optional parts are ignored.** A document without a stylesheet still converts.
- **Recoverable producer quirks are recovered automatically** (unclosed XML, unbalanced RTF groups, stray covered table cells).
- **Corrupt or unsupported subparts are skipped** when useful output can still be produced (an unreadable sheet, chapter, or slide is dropped; the rest converts).
- **`ConvertError` is returned only when meaningful conversion is impossible**, the document is encrypted, or a fixed safety limit is exceeded (`Unsupported`, `Malformed`, `Encrypted`, `ResourceLimit`, `MissingPart`, `Io`).

Recovery and skipped-content events are reported through the [`log`](https://docs.rs/log) facade at debug/warn level. Logging never changes conversion behavior and its messages are not a stable API.

### Fixed resource limits

Attack-shaped input (decompression bombs, pathological XML nesting, style-reference cycles, runaway repeat expansion, oversized embedded assets) hard-fails with `ResourceLimit`/`Malformed` — always. The caps are fixed, documented constants in the source (`package::limits`), not configuration.

### Fixed content policy

- Page headers and footers are excluded; speaker notes are included (rendered as a quote after each slide).
- Slides and shapes convert in document (z-)order.
- Spreadsheet and CSV data is never promoted to a table header; the GFM header row is left empty unless the format marks real header rows.
- Internal links work end-to-end: bookmarks and EPUB chapter fragments become heading slugs or `<a id>` anchors in the output.
- Embedded images and objects are retained as bytes in `Document::assets`; the Markdown output renders their alt text (Markdown cannot embed bytes). Charts and SmartArt render their textual content (titles, series data, diagram text).

## Layout

```
src/
  lib.rs          public API (Format, to_markdown, to_markdown_bytes, to_document)
  model/          the document model: blocks, inlines, canonical table grid, assets, anchors
  render/markdown the single model -> GFM serializer
  package/        shared package layer: limited zip access, namespace-aware XML, OPC rels/paths
  shared/         cross-format resolution: style deltas/chains, list identity, fields, HTML
  formats/        one frontend per input format (docx, odf, pptx, epub, rtf, doc, ppt, sheet, csv, pdf)
```

The `.doc` frontend is a from-scratch Word 97 binary parser implementing the published resolution algorithms (OLE2 container, FIB, piece table with property modifiers, CHPX/PAPX runs over STSH style chains, PlfLst/PlfLfo list tables, footnote/endnote subdocuments with UTF-16 CP accounting), and `.ppt` likewise (record stream, persist directory, style/master text atoms, with raw scanning only as a logged recovery path). Excel formats use Firecrawl's [calamine fork](https://github.com/firecrawl/calamine); ODS is parsed natively so cells keep their formatted display text.

## Development

```
cargo test
```

- Serializer behavior (escaping, emphasis, tables, anchors, footnotes) is covered by unit tests in `src/render/markdown/tests.rs`.
- A committed fixture corpus (`tests/fixtures/`, generated from authored sources by `tests/gen_fixtures.py` through LibreOffice/Pandoc plus handmade edge cases) is snapshot-tested by `tests/snapshots.rs`; malformed fixtures encode their expected outcome in the filename (`--recovers`, `--skips`, `--errors`).
- A local corpus of real-world documents in `samples/` (not committed) is swept by `cargo test --test snapshots -- --ignored`.
- `tests/robustness.rs` mutation-tests every fixture; `fuzz/` carries cargo-fuzz targets per format.
- A speed and quality benchmark against other converters lives in [`bench/`](bench/README.md).

## License

[MIT](LICENSE)

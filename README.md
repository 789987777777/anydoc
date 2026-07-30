# anydoc

Convert documents to GitHub-Flavored Markdown. A [Firecrawl](https://firecrawl.dev) project.

anydoc parses each format into a shared intermediate representation and serializes it with a single GFM writer, so escaping, table fallbacks, emphasis merging, and footnote numbering behave identically across every input format.

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
| CSV | `.csv` |

PDFs are out of scope; see [pdf-inspector](https://github.com/firecrawl/pdf-inspector). Presentation formats (`.ppt`, `.pptx`, `.odp`) are not supported yet.

## Usage

```rust
// From a file path (format inferred from the extension):
let markdown = anydoc::to_markdown("report.docx")?;

// From bytes:
let markdown = anydoc::to_markdown_bytes(&bytes, anydoc::Format::Docx)?;

// Or stop at the intermediate representation:
let document = anydoc::to_document(&bytes, anydoc::Format::Rtf)?;
```

A small CLI for manual testing and benchmarking ships as an example:

```
cargo run --release --example convert -- file.docx [-o out.md] [--bench N]
```

## Output

- Headings (from styles and outline levels), paragraphs, hard line breaks
- Bold, italic, strikethrough, inline code, with split runs merged and edge whitespace normalized
- Nested ordered/unordered lists, including numbering definitions in OOXML and ODF
- GFM tables, with merged cells padded, multi-paragraph cells joined with `<br>`, and single-cell layout tables unwrapped
- Hyperlinks, including `HYPERLINK` field codes in doc/docx/rtf
- Footnotes and endnotes as GFM footnotes (`[^1]` / `[^1]: ...`)
- Spreadsheets as one table per sheet with number formats applied (dates render as dates, not serials)
- Markdown syntax in source text is escaped, including line-start constructs like `1.` and `-`

## Layout

```
src/
  lib.rs        public API (Format, to_markdown, to_markdown_bytes, to_document)
  ir.rs         format-neutral document tree
  markdown.rs   the single IR -> GFM serializer
  formats/      one frontend per input format
  support/      shared infrastructure (XML DOM, HTML converter, text cleanup, field codes)
```

The `.doc` frontend is a from-scratch Word 97 binary parser (OLE2 container, FIB, piece table, CHPX/PAPX formatting runs, stylesheet, footnote/endnote subdocuments). Spreadsheets use Firecrawl's [calamine fork](https://github.com/firecrawl/calamine).

## Development

```
cargo test
```

Serializer behavior (escaping, emphasis, tables, footnotes) is covered by unit tests in `src/markdown.rs`. A local corpus of real-world documents in `samples/` (not committed) is used for regression and benchmark runs via the `convert` example.

## License

[MIT](LICENSE)

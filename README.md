# anydoc

Convert documents to GitHub-Flavored Markdown. A [Firecrawl](https://firecrawl.dev) project.

Every format parses into one shared document model and renders through a single GFM serializer, so escaping, tables, anchors, and footnotes behave the same no matter what you feed in.

## Supported formats

| Format           | Extensions                                                 |
| ---------------- | ---------------------------------------------------------- |
| Word             | `.doc`, `.docx`, `.docm`                                   |
| PowerPoint       | `.ppt`, `.pps`, `.pot`, `.pptx`, `.pptm`, `.ppsx`, `.ppsm` |
| Excel            | `.xls`, `.xlsx`, `.xlsm`, `.xlsb`                          |
| OpenDocument     | `.odt`, `.ods`, `.odp`                                     |
| Rich Text Format | `.rtf`                                                     |
| EPUB             | `.epub`                                                    |
| CSV              | `.csv`                                                     |
| PDF              | `.pdf`                                                     |

PDFs are converted with [pdf-inspector](https://github.com/firecrawl/pdf-inspector), which emits Markdown directly, so they have no document-model form. Scanned and image-only PDFs need OCR, which is out of scope: they error as unsupported.

## What's parsed

Headings and their anchors, paragraphs, bold, italic, strikethrough and inline code, links (external, relative, and internal cross-references), bulleted, ordered, nested and task lists with the numbering the source assigns them, tables with merged cells and header rows, block quotes, code blocks, footnotes and endnotes, speaker notes, and embedded images and objects.

Markdown cannot embed bytes, so an embedded image renders as its alt text while the bytes stay on the document model, tagged with a media type and the part they came from (`Document::assets` in Rust, `document.assets` in Node). Images that carry an external URL render as ordinary Markdown images.

## Node

```
npm install @firecrawl/anydoc
```

```js
import { toDocument, toMarkdown, toMarkdownBytes } from '@firecrawl/anydoc';

// From a file path:
const markdown = await toMarkdown('report.docx');

// From bytes, with the format detected from the content:
const markdown = await toMarkdownBytes(bytes);

// Or name it, which signature-less formats (CSV) need:
const markdown = await toMarkdownBytes(bytes, 'csv');

// Or stop at the document model, which also carries embedded assets:
const document = await toDocument(bytes);
```

Conversion runs on the libuv thread pool, so it never blocks the event loop. TypeScript types ship with the package.

## Rust

```
cargo add anydoc
```

```rust
// From a file path:
let markdown = anydoc::to_markdown("report.docx")?;

// From bytes, with the format detected from the content:
let markdown = anydoc::to_markdown_bytes(&bytes, None)?;

// Or name it, which signature-less formats (CSV) need:
let markdown = anydoc::to_markdown_bytes(&bytes, anydoc::Format::Csv)?;

// Or stop at the document model, which also carries embedded assets:
let document = anydoc::to_document(&bytes, None)?;
```

A CLI for manual testing ships in [`examples/`](examples/), in both languages:

```
cargo run --release --example convert -- file.docx [-f csv] [-o out.md] [--assets dir]
node examples/convert.mjs file.docx [-f csv] [-o out.md] [--assets dir]
```

## Format detection

The format is read from the file content: the signature and identity each container specification designates (PDF header, RTF open group, OLE stream names, ZIP package mimetype and content types). Signature-less formats like CSV have no such marker, so detection returns nothing for them and the extension, or an explicit format, names them instead.

```js
formatFromBytes(bytes); // 'docx', or null when nothing matches
formatFromExtension('.pptm'); // 'pptx'
formatFromPath('report.odt'); // 'odt'
```

```rust
anydoc::Format::from_bytes(&bytes); // Option<Format>
anydoc::Format::from_extension("pptm");
anydoc::Format::from_path(Path::new("report.odt"));
```

## Development

```
cargo test
cd node && npm install && npm run build && npm test
```

A committed fixture corpus under `tests/fixtures/` is snapshot-tested, `tests/robustness.rs` mutation-tests every fixture, and `fuzz/` carries cargo-fuzz targets per format. A speed and quality benchmark against other converters lives in [`bench/`](bench/README.md).

Releases are tagged `v<version>`, which publishes the crate and the npm package from [`.github/workflows/release.yml`](.github/workflows/release.yml).

## License

[MIT](LICENSE)

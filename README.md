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

## Benchmarks

Against six well-known converters on 107 real-world documents spanning fourteen formats, run with the harness in [`bench/`](bench/README.md).

| tool | formats | median ms | docs judged | score | completeness | structure | formatting | cleanliness |
| --- | --- | --- | --- | --- | --- | --- | --- | --- |
| anydoc | **14/14** | **4.7** | 94 | **80** | **88** | **78** | **77** | **79** |
| libreoffice | 12/14 | 1129.5 | 87 | 40 | 59 | 43 | 43 | 24 |
| unstructured | 8/14 | 572.9 | 58 | 65 | 76 | 62 | 52 | 67 |
| markitdown | 6/14 | 134.8 | 33 | 65 | 80 | 67 | 61 | 53 |
| pandoc | 5/14 | 102.1 | 34 | 57 | 75 | 57 | 58 | 39 |
| docling | 4/14 | 513.6 | 21 | 57 | 63 | 59 | 57 | 52 |
| mammoth | 1/14 | 52.5 | 8 | 70 | 85 | 68 | 74 | 55 |

| format | anydoc | libreoffice | unstructured | markitdown | pandoc | docling | mammoth |
| --- | --- | --- | --- | --- | --- | --- | --- |
| doc | **88** | 58 | 68 | - | - | - | - |
| docm | **82** | 49 | - | - | - | - | - |
| docx | **86** | 53 | 56 | 72 | 68 | 68 | 70 |
| epub | 74 | - | 74 | **77** | 53 | - | - |
| odp | **87** | 22 | - | - | - | - | - |
| ods | **82** | 42 | - | - | - | - | - |
| odt | **80** | 52 | 70 | - | 61 | - | - |
| ppt | **80** | 25 | - | - | - | - | - |
| pptx | **76** | 22 | - | 59 | - | 50 | - |
| rtf | **89** | 58 | 48 | - | 46 | - | - |
| xls | **77** | 40 | 68 | 64 | - | - | - |
| xlsm | **70** | 30 | - | - | - | - | - |
| xlsx | **70** | 31 | 69 | 55 | - | 51 | - |

Quality is scored by an LLM judge (Claude Sonnet 5). For each document, anydoc's output and one competitor's are shown blind against ground truth - the document's first six pages rendered by LibreOffice, attached as images - and each output is scored 1 to 5 on completeness, structure, formatting, and cleanliness. Every pair is judged twice with the two outputs swapped, so position bias shows up as disagreement. The numbers above are the mean of those scores as a percentage of the maximum, over 479 verdicts. CSV is converted but not judged: a rendered CSV is not a meaningful reference.

The judge is told the deliverable is GitHub-Flavored Markdown, so raw HTML counts against cleanliness and content carried only inside HTML does not count as present. Where the source uses something Markdown has no syntax for, the rubric takes no position on the right answer.

`score` averages a tool's per-format scores over the formats it supports, so a corpus with many documents in one format cannot skew it. It is not comparable across rows: each tool is judged only on the formats it reads, and mammoth's number covers eight Word documents where anydoc's covers ninety-four across every format. The per-format table is the like-for-like comparison.

Speed is one warm conversion per document, timed in-process for anydoc and the Python libraries and including process spawn for the CLI tools, since that is how they are used.

Preliminary: one machine (Windows 11), one run, and the corpus is not public.

## Development

```
cargo test
cd node && npm install && npm run build && npm test
```

A committed fixture corpus under `tests/fixtures/` is snapshot-tested, `tests/robustness.rs` mutation-tests every fixture, and `fuzz/` carries cargo-fuzz targets per format. A speed and quality benchmark against other converters lives in [`bench/`](bench/README.md).

Releases are tagged `v<version>`, which publishes the crate and the npm package from [`.github/workflows/release.yml`](.github/workflows/release.yml).

## License

[MIT](LICENSE)

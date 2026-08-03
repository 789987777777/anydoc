# anydoc

Convert documents to GitHub-Flavored Markdown. A [Firecrawl](https://firecrawl.dev) project.

Every format parses into one shared document model and renders through a single GFM serializer. Escaping, tables, anchors, and footnotes behave the same no matter what you feed in.

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

PDFs are converted with [pdf-inspector](https://github.com/firecrawl/pdf-inspector), which emits Markdown directly without building a document model. `to_document` rejects them. Use `to_markdown_bytes` instead. Scanned and image-only PDFs need OCR, which is out of scope: they error as unsupported.

## What's parsed

Headings and their anchors, paragraphs, bold, italic, strikethrough and inline code, links (external, relative, and internal cross-references), bulleted, ordered, nested and task lists with the numbering the source assigns them, tables with merged cells and header rows, block quotes, code blocks, footnotes and endnotes, speaker notes, and embedded images and objects.

An embedded image renders as its alt text, because Markdown cannot embed bytes. The bytes stay on the document model, tagged with a media type and the part they came from (`Document::assets` in Rust, `document.assets` in Node). Images that carry an external URL render as ordinary Markdown images.

## Node

```
npm install @firecrawl/anydoc
```

```js
import { toDocument, toMarkdown, toMarkdownBytes } from '@firecrawl/anydoc';

// From a file path:
const fromPath = await toMarkdown('report.docx');

// From bytes, with the format detected from the content:
const fromBytes = await toMarkdownBytes(bytes);

// Or name it, which signature-less formats (CSV) need:
const fromCsv = await toMarkdownBytes(bytes, 'csv');

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

The format is read from the file content, using the marker its specification designates: the PDF header, the RTF open group, OLE stream names, the ZIP package mimetype and content types. CSV has no such marker. Detection returns nothing for it, and the extension or an explicit format names it instead.

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

anydoc is measured against six other converters on 100 real-world documents spanning fourteen formats, using the harness in [`bench/`](bench/README.md).

| tool         | formats   | median ms | docs judged | score  | completeness | structure | formatting | cleanliness |
| ------------ | --------- | --------- | ----------- | ------ | ------------ | --------- | ---------- | ----------- |
| anydoc       | **14/14** | **4.7**   | 94          | **80** | **88**       | **78**    | **77**     | **79**      |
| libreoffice  | 12/14     | 1129.5    | 87          | 40     | 59           | 43        | 43         | 24          |
| unstructured | 8/14      | 572.9     | 58          | 65     | 76           | 62        | 52         | 67          |
| markitdown   | 6/14      | 134.8     | 33          | 65     | 80           | 67        | 61         | 53          |
| pandoc       | 5/14      | 102.1     | 34          | 57     | 75           | 57        | 58         | 39          |
| docling      | 4/14      | 513.6     | 21          | 57     | 63           | 59        | 57         | 52          |
| mammoth      | 1/14      | 52.5      | 8           | 70     | 85           | 68        | 74         | 55          |

| format | anydoc | libreoffice | unstructured | markitdown | pandoc | docling | mammoth |
| ------ | ------ | ----------- | ------------ | ---------- | ------ | ------- | ------- |
| doc    | **88** | 58          | 68           | -          | -      | -       | -       |
| docm   | **82** | 49          | -            | -          | -      | -       | -       |
| docx   | **86** | 53          | 56           | 72         | 68     | 68      | 70      |
| epub   | 74     | -           | 74           | **77**     | 53     | -       | -       |
| odp    | **87** | 22          | -            | -          | -      | -       | -       |
| ods    | **82** | 42          | -            | -          | -      | -       | -       |
| odt    | **80** | 52          | 70           | -          | 61     | -       | -       |
| ppt    | **80** | 25          | -            | -          | -      | -       | -       |
| pptx   | **76** | 22          | -            | 59         | -      | 50      | -       |
| rtf    | **89** | 58          | 48           | -          | 46     | -       | -       |
| xls    | **77** | 40          | 68           | 64         | -      | -       | -       |
| xlsm   | **70** | 30          | -            | -          | -      | -       | -       |
| xlsx   | **70** | 31          | 69           | 55         | -      | 51      | -       |

Quality is scored by an LLM judge (Claude Sonnet 5). For each document, anydoc's output and one competitor's are shown blind against ground truth: the document's first six pages, rendered by LibreOffice and attached as images. Each output is scored 1 to 5 on completeness, structure, formatting, and cleanliness. Every pair is judged twice with the two outputs swapped, so position bias shows up as disagreement. The numbers above are the mean of those scores as a percentage of the maximum, over 479 verdicts.

`score` is the mean of a tool's per-format scores over the formats it supports, which keeps a corpus heavy in one format from skewing it. The tradeoff is that each row averages a different set: mammoth's 70 is docx alone, while anydoc's 80 spans all fourteen. The per-format table is the like-for-like comparison.

Speed is one warm conversion per document. anydoc and the Python libraries are timed with process spawn excluded. The CLI tools include it, since that is how they are used.

Measured in one run on a Ryzen 9 9950X3D with 64 GB of DDR5-6400, on Windows 11. The corpus itself is not redistributable and is not in the repo. The harness reads whatever documents are in `samples/`.

## Development

```
cargo test
cd node && npm install && npm run build && npm test
```

A committed fixture corpus under `tests/fixtures/` is snapshot-tested, `tests/robustness.rs` mutation-tests every fixture, and `fuzz/` carries cargo-fuzz targets per format. A speed and quality benchmark against other converters lives in [`bench/`](bench/README.md).

Releases are tagged `v<version>`, which publishes the crate and the npm package from [`.github/workflows/release.yml`](.github/workflows/release.yml).

## License

[MIT](LICENSE)

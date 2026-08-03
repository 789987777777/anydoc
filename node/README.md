# @firecrawl/anydoc

Convert documents to GitHub-Flavored Markdown. Node bindings for the [anydoc](https://github.com/firecrawl/anydoc) Rust crate. A [Firecrawl](https://firecrawl.dev) project.

```
npm install @firecrawl/anydoc
```

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

## Usage

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

## Format detection

The format is read from the file content: the signature and identity each container specification designates (PDF header, RTF open group, OLE stream names, ZIP package mimetype and content types). Signature-less formats like CSV have no such marker, so detection returns `null` for them and the extension, or an explicit format, names them instead.

```js
formatFromBytes(bytes); // 'docx', or null when nothing matches
formatFromExtension('.pptm'); // 'pptx'
formatFromPath('report.odt'); // 'odt'
```

## Images and embedded objects

Markdown cannot embed bytes, so an embedded image renders as its alt text while the bytes stay on `document.assets`, tagged with a media type and the part they came from. Images that carry an external URL render as ordinary Markdown images.

The full format and behavior notes live in the [repository README](https://github.com/firecrawl/anydoc#readme).

## License

[MIT](https://github.com/firecrawl/anydoc/blob/main/LICENSE)

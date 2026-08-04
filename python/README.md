# firecrawl-anydoc

Convert documents to GitHub-Flavored Markdown. Python bindings for the [anydoc](https://github.com/firecrawl/anydoc) Rust crate. A [Firecrawl](https://firecrawl.dev) project.

```
pip install firecrawl-anydoc
```

The package installs as `firecrawl-anydoc` and imports as `anydoc`.

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

```python
import anydoc

# From a file path:
markdown = anydoc.to_markdown("report.docx")

# From bytes, with the format detected from the content:
markdown = anydoc.to_markdown_bytes(data)

# Or name it, which signature-less formats (CSV) need:
markdown = anydoc.to_markdown_bytes(data, "csv")

# Or stop at the document model, which also carries embedded assets:
document = anydoc.to_document(data)
```

Conversion releases the GIL, so other threads keep running. Type stubs ship with the package.

## Format detection

The format is read from the file content: the signature and identity each container specification designates (PDF header, RTF open group, OLE stream names, ZIP package mimetype and content types). Signature-less formats like CSV have no such marker, so detection returns `None` for them and the extension, or an explicit format, names them instead.

```python
anydoc.format_from_bytes(data)  # 'docx', or None when nothing matches
anydoc.format_from_extension(".pptm")  # 'pptx'
anydoc.format_from_path("report.odt")  # 'odt'
```

## Images and embedded objects

Markdown cannot embed bytes, so an embedded image renders as its alt text while the bytes stay on `document.assets`, tagged with a media type and the part they came from. Images that carry an external URL render as ordinary Markdown images.

The full format and behavior notes live in the [repository README](https://github.com/firecrawl/anydoc#readme).

## License

[MIT](https://github.com/firecrawl/anydoc/blob/main/LICENSE)

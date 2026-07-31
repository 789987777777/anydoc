# Changelog

## 0.2.0

Breaking architectural refactor. The public API is now exactly three
functions with a typed error:

```rust
to_markdown(path)            -> Result<String, ConvertError>
to_markdown_bytes(bytes, f)  -> Result<String, ConvertError>
to_document(bytes, f)        -> Result<Document, ConvertError>
```

### Breaking

- `anyhow::Error` replaced by the typed `ConvertError`
  (`Unsupported` / `Malformed` / `Encrypted` / `ResourceLimit` /
  `MissingPart` / `Io`).
- The public IR module `ir` is replaced by `model`: canonical table grid
  (`CellSlot::Origin`/`Covered` with spans), link targets
  (`External`/`Relative`/`Anchor`), anchors, note kinds, and a
  self-contained embedded-asset store (`Document::assets`).
- Spreadsheet/CSV first rows are no longer promoted to table headers.
- Speaker notes are now included (as quotes); relative links and internal
  bookmarks are preserved instead of dropped.

### Fixed / improved

- One unified recovery policy: optional parts ignored, producer quirks
  recovered (logged), corrupt subparts skipped when useful output remains;
  hard typed errors only when conversion is impossible.
- Fixed resource limits against decompression bombs, deep XML, reference
  cycles, and runaway repeat expansion.
- Namespace-aware XML with encoding detection (UTF-16 parts decode).
- DOCX: full style `basedOn`/`docDefaults` resolution with Word toggle
  semantics, complete numbering (overrides, `numStyleLink`, restarts,
  continuation, suppression), bookmarks as anchors, images/charts/SmartArt/
  OLE extraction, `AlternateContent` `Requires` handling.
- ODF: covered-cell double-counting fixed via the canonical grid, typed
  value fallback for text-less cells, repeats honored in all containers,
  `draw:g` recursion, tri-state style deltas (`font-weight: normal` works).
- PPTX: full slide -> layout -> master -> presentation-default text cascade,
  speaker notes, slide tables/charts/SmartArt, image assets.
- EPUB: chapter-scoped anchors (in-book navigation survives), CSS subset
  (`display:none`, weight/style/strike), `rowspan` tables, `ol`
  `start`/`reversed`/`type`/`value`.
- RTF: `\bin`-safe lexer, typed font (charset), stylesheet, and list tables
  (no more digit-guessing), nested tables, merge properties, surrogate
  pairs.
- DOC: published resolution algorithms — full STSH with `istdBase` chains
  and UPX payloads, piece `Prm`s, PlfLst/PlfLfo list numbering, UTF-16 CP
  accounting (footnote positions after astral characters are correct).
- PPT: `StyleTextPropAtom`/`TxMasterStyleAtom` styling, bullets and depths,
  speaker notes, and raw scanning demoted to a logged recovery path.
- Renderer: link-label and image-alt escaping contexts, GFM heading slugs +
  `<a id>` anchors, duplicate-footnote dedup, single-pass run merging.

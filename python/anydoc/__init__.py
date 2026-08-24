"""Convert documents to GitHub-Flavored Markdown."""

import os
from pathlib import Path
from typing import Literal

from anydoc._anydoc import (
    Asset,
    Block,
    Cell,
    CellSlot,
    ConvertError,
    Document,
    EncryptedError,
    ImageSource,
    Inline,
    LinkTarget,
    List,
    ListItem,
    MalformedError,
    MissingPartError,
    NeedsOcrError,
    Note,
    Page,
    ResourceLimitError,
    Style,
    Table,
    UnsupportedError,
    format_from_bytes,
    format_from_extension,
    format_from_path,
    to_document,
    to_markdown_pages,
)
from anydoc._anydoc import to_markdown as _to_markdown
from anydoc._anydoc import to_markdown_bytes as _to_markdown_bytes

Format = Literal[
    "doc", "docx", "odt", "pdf", "ppt", "pptx", "rtf", "epub", "xlsx", "ods", "odp", "csv"
]
"""Input format, named after the extension that identifies it. Container
variants that share a parser (`.docm`, `.xlsm`, `.ppsx`, ...) map onto these
via `format_from_bytes` or `format_from_extension`."""

Ocr = Literal["reject", "hosted"]
"""What happens to a PDF whose pages need OCR. `reject` (the default) raises
`NeedsOcrError` naming the pages. `hosted` sends the document to Firecrawl
Parse instead, through the `firecrawl` package
(`pip install 'firecrawl-anydoc[hosted]'`). Documents anydoc converts itself
never leave the machine."""


class HostedError(ConvertError):
    """`ocr="hosted"` could not get the document through Firecrawl Parse."""


def to_markdown(
    path: "str | os.PathLike[str]", *, ocr: Ocr = "reject", api_key: "str | None" = None
) -> str:
    """Convert a document file to Markdown. The format is detected from the
    file content; the extension is the fallback for signature-less formats
    (CSV) and unrecognizable containers.

    `api_key` is the Firecrawl key for `ocr="hosted"`, else
    `FIRECRAWL_API_KEY`; without either the keyless tier applies, rate-limited
    per IP."""
    try:
        return _to_markdown(path)
    except NeedsOcrError:
        if ocr != "hosted":
            raise
    path = Path(path)
    return _parse_hosted(path.read_bytes(), path.name, api_key)


def to_markdown_bytes(
    data: "bytes | bytearray",
    format: "Format | None" = None,
    *,
    ocr: Ocr = "reject",
    api_key: "str | None" = None,
) -> str:
    """Convert an in-memory document to Markdown. Without a format, it is
    detected from the content, which signature-less formats (CSV) have to
    name explicitly. `ocr` and `api_key` are as for `to_markdown`."""
    try:
        return _to_markdown_bytes(data, format)
    except NeedsOcrError:
        if ocr != "hosted":
            raise
    return _parse_hosted(bytes(data), "document.pdf", api_key)


# The document goes through the firecrawl SDK, an optional extra: key
# handling, the keyless tier and retries are its business, not ours.
def _parse_hosted(data: bytes, filename: str, api_key: "str | None") -> str:
    try:
        from firecrawl import Firecrawl
        from firecrawl.v2.types import ParseOptions, PDFParser
    except ImportError as error:
        raise HostedError('install firecrawl-anydoc[hosted] to use ocr="hosted"') from error
    keyed = bool(api_key or os.environ.get("FIRECRAWL_API_KEY"))
    client = Firecrawl(
        api_key=api_key,
        api_url=os.environ.get("FIRECRAWL_API_URL", "https://api.firecrawl.dev"),
    )
    try:
        document = client.parse(
            data,
            filename=filename,
            content_type="application/pdf",
            options=ParseOptions(parsers=[PDFParser(mode="auto")], timeout=300000),
        )
    except Exception as error:
        raise HostedError(_describe(error, keyed)) from error
    return document.markdown or ""


def _describe(error: Exception, keyed: bool) -> str:
    status = getattr(error, "status_code", None)
    if status == 401:
        return f"Firecrawl Parse rejected the API key: {error}"
    if status == 402:
        return f"Firecrawl Parse is out of credits: {error}"
    if status == 429 and keyed:
        return f"Firecrawl Parse rate limit reached: {error}"
    if status == 429:
        return f"Firecrawl Parse keyless limit reached, set FIRECRAWL_API_KEY: {error}"
    return f"Firecrawl Parse: {error}"


__all__ = [
    "Asset",
    "Block",
    "Cell",
    "CellSlot",
    "ConvertError",
    "Document",
    "EncryptedError",
    "Format",
    "HostedError",
    "ImageSource",
    "Inline",
    "LinkTarget",
    "List",
    "ListItem",
    "MalformedError",
    "MissingPartError",
    "NeedsOcrError",
    "Note",
    "Ocr",
    "Page",
    "ResourceLimitError",
    "Style",
    "Table",
    "UnsupportedError",
    "format_from_bytes",
    "format_from_extension",
    "format_from_path",
    "to_document",
    "to_markdown",
    "to_markdown_bytes",
    "to_markdown_pages",
]

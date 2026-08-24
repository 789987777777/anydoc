'use strict'

const { readFile } = require('node:fs/promises')
const { basename } = require('node:path')

const native = require('./index.js')
const { version } = require('./package.json')

/**
 * Convert a document file to Markdown, with `options.ocr` deciding what
 * happens to a PDF whose pages need OCR: `'reject'` (the default) rejects
 * with `needsOcr`, `'hosted'` sends it to Firecrawl Parse instead.
 */
async function toMarkdown(path, options) {
  try {
    return await native.toMarkdown(path)
  } catch (error) {
    if (!sendsToHosted(error, options)) throw error
    return parseHosted(await readFile(path), basename(path), options)
  }
}

/** `toMarkdown` for bytes; see it for `options`. */
async function toMarkdownBytes(bytes, format, options) {
  try {
    return await native.toMarkdownBytes(bytes, format)
  } catch (error) {
    if (!sendsToHosted(error, options)) throw error
    return parseHosted(bytes, 'document.pdf', options)
  }
}

function sendsToHosted(error, options) {
  return error.code === 'needsOcr' && options?.ocr === 'hosted'
}

// The document goes through the firecrawl SDK, an optional peer dependency:
// key handling, the keyless tier and retries are its business, not ours.
async function parseHosted(bytes, filename, options) {
  let Firecrawl
  try {
    ;({ Firecrawl } = await import('firecrawl'))
  } catch (error) {
    if (error.code !== 'ERR_MODULE_NOT_FOUND') throw error
    throw hostedError("install the firecrawl package to use ocr: 'hosted'", error)
  }
  const keyed = Boolean(options.apiKey || process.env.FIRECRAWL_API_KEY)
  const client = new Firecrawl({ apiKey: options.apiKey })
  let document
  try {
    document = await client.parse(
      { data: bytes, filename, contentType: 'application/pdf' },
      { parsers: [{ type: 'pdf', mode: 'auto' }], timeout: 300000, origin: `anydoc@${version}` },
    )
  } catch (error) {
    throw hostedError(describe(error, keyed), error)
  }
  return document.markdown ?? ''
}

function describe(error, keyed) {
  switch (error.status) {
    case 401:
      return `Firecrawl Parse rejected the API key: ${error.message}`
    case 402:
      return `Firecrawl Parse is out of credits: ${error.message}`
    case 429:
      return keyed
        ? `Firecrawl Parse rate limit reached: ${error.message}`
        : `Firecrawl Parse keyless limit reached, set FIRECRAWL_API_KEY: ${error.message}`
    default:
      return `Firecrawl Parse: ${error.message}`
  }
}

function hostedError(message, cause) {
  const error = new Error(message, { cause })
  error.code = 'hosted'
  return error
}

// Spelled out so ESM `import { ... }` sees the names.
module.exports.BlockKind = native.BlockKind
module.exports.CellSlotKind = native.CellSlotKind
module.exports.Format = native.Format
module.exports.formatFromBytes = native.formatFromBytes
module.exports.formatFromExtension = native.formatFromExtension
module.exports.formatFromPath = native.formatFromPath
module.exports.ImageSourceKind = native.ImageSourceKind
module.exports.InlineKind = native.InlineKind
module.exports.LinkTargetKind = native.LinkTargetKind
module.exports.MarkerKind = native.MarkerKind
module.exports.NoteKind = native.NoteKind
module.exports.TableKind = native.TableKind
module.exports.toDocument = native.toDocument
module.exports.toMarkdownPages = native.toMarkdownPages
module.exports.toMarkdown = toMarkdown
module.exports.toMarkdownBytes = toMarkdownBytes

# @firecrawl/anydoc-wasm

WebAssembly bindings for [anydoc](../README.md), plus the source of the demo page at [firecrawl.github.io/anydoc](https://firecrawl.github.io/anydoc/).

The API mirrors the Rust library, minus the path-based `to_markdown`: wasm has no filesystem, so conversion always starts from bytes.

```bash
npm install @firecrawl/anydoc-wasm
```

```js
import init, {
  formatFromBytes,
  toMarkdownBytes,
  toDocument,
} from '@firecrawl/anydoc-wasm';

await init();

// With the format detected from the content:
const markdown = toMarkdownBytes(bytes);

// Or name it, which signature-less formats (CSV) need:
const fromCsv = toMarkdownBytes(bytes, 'csv');

// Or stop at the document model, which also carries embedded assets:
const document = toDocument(bytes);

// Format detection on its own:
formatFromBytes(bytes); // 'docx', or undefined when nothing matches
```

The package is built with `wasm-pack --target web`: it loads with a plain `<script type="module">` and with bundlers that handle the `new URL(..., import.meta.url)` asset pattern (Vite, webpack 5, Rollup). In Node, pass the module bytes to `initSync` instead of calling `init` (see [`test.mjs`](wasm/test.mjs)).

Calls are synchronous: wasm runs single-threaded on the calling thread, so convert on a worker if the main thread must stay responsive.

## Building

```bash
wasm-pack build wasm --release --target web --scope firecrawl
node --test wasm/test.mjs
```

This produces the npm package in `wasm/pkg/`: the module, the JS glue, and TypeScript definitions. Publishing runs from [`../.github/workflows/release.yml`](../.github/workflows/release.yml) on release tags.

## Demo page

`www/` holds the static demo site, which imports the module from `www/pkg/`. Build into that directory, then serve `www/`:

```bash
wasm-pack build wasm --release --target web --no-pack --out-dir www/pkg
python -m http.server -d wasm/www
```

[`../.github/workflows/pages.yml`](../.github/workflows/pages.yml) builds and deploys the same layout to GitHub Pages on every push to main.

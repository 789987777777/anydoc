# Benchmark results (preliminary)

anydoc vs well-known document-to-markdown converters, on a corpus of 82
real-world documents (docx, doc, docm, odt, rtf, epub, xlsx, xlsm, xls, ods,
csv). Run 2026-07-30 with the harness in [`bench/`](bench/README.md).
Preliminary: single machine (Windows 11), single run, corpus not yet public.

## Speed

Median warm conversion time per document. anydoc, markitdown, docling,
unstructured, and mammoth are timed in-process; pandoc and LibreOffice include
process spawn (they are CLI tools). anydoc was the fastest tool in every
format.

| Format | anydoc | markitdown | docling | pandoc | unstructured | mammoth | LibreOffice |
| --- | --- | --- | --- | --- | --- | --- | --- |
| docx | **3.3 ms** | 103 ms | 498 ms | 106 ms | 55 ms | 21 ms | 1.08 s |
| doc | **0.7 ms** | - | - | - | 933 ms | - | 926 ms |
| odt | **4.2 ms** | - | - | 91 ms | 251 ms | - | 903 ms |
| rtf | **1.1 ms** | - | - | 78 ms | 169 ms | - | 1.04 s |
| epub | **5.9 ms** | 67 ms | - | 450 ms | 4.26 s | - | - |
| xlsx | **28 ms** | 1.44 s | 821 ms | - | 1.58 s | - | 5.57 s |
| xls | **1.6 ms** | 80 ms | - | - | 87 ms | - | 994 ms |
| ods | **9.5 ms** | - | - | - | - | - | 2.35 s |
| csv | **1.0 ms** | 2.9 ms | 13 ms | 37 ms | 56 ms | - | - |

Throughput on the 41 MB xlsx subset: anydoc 14.6 MB/s vs 0.4-0.6 MB/s for the
Python tools. anydoc converted every file in the corpus; markitdown and pandoc
each failed on one non-UTF-8 CSV.

## Quality (LLM judge)

Judge: Claude Sonnet 5 (Anthropic API). For each document and opponent, the two
markdown outputs were judged blind against ground truth (the document's first 6
pages rendered by LibreOffice as images; extracted source text for EPUB), twice
with A/B positions swapped; disagreement between the two orders counts as a
tie. CSV was excluded from judging.

**Overall: 154 wins, 63 ties, 2 losses for anydoc across 219 matchups (99%
win rate excluding ties).**

| Format | vs markitdown | vs docling | vs pandoc | vs unstructured | vs mammoth | vs LibreOffice |
| --- | --- | --- | --- | --- | --- | --- |
| docx | 3W 5T | 4W 4T | 6W 2T | 8W | 0W 8T | 7W 1T |
| doc | - | - | - | 4W 4T | - | 4W 4T |
| docm | - | - | - | - | - | 5W |
| odt | - | - | 10W 1T | 8W 3T | - | 10W 1T |
| rtf | - | - | 9W | 9W | - | 9W |
| epub | 3W 3T | - | 6W | 2W 4T | - | - |
| xlsx | 4W 7T | 8W 3T | - | 3W 6T 2L | - | 11W |
| xls | 3W 2T | - | - | 1W 4T | - | 5W |
| xlsm / ods | - | - | - | - | - | 12W 1T |

An earlier pass scored 149W/54T/16L; the losses traced to seven specific
defects (over-aggressive escaping, dropped docx text-box content, docx
character/paragraph style formatting not resolved, shape names leaking as alt
text, one .doc table bug, missing ODS number formats, EPUB image placeholder
noise). All were fixed and the affected pairs re-judged. The 2 remaining
losses are spreadsheets laid out as documents (prose typed across grid cells),
where unstructured's layout inference reads better than a faithful one-table
rendering.

## Content recall (word-trigram containment, cross-check)

Mean mutual containment between anydoc's output and each competitor's shows no
systematic content loss: docling 0.87/0.87, markitdown 0.84/0.83, unstructured
0.83/0.84. (LibreOffice and mammoth read lower mostly due to their own noise
and docx-only image/style artifacts.)

## Reproducing

See [`bench/README.md`](bench/README.md). Conversions, ground-truth renders,
deterministic metrics, and judge verdicts are all journaled and resumable;
full details in `bench/out/report.md` after a run.

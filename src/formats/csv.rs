//! CSV and friends (semicolon, tab, pipe delimited), with delimiter sniffing.

use crate::ir::{Block, Cell, Document, Inline, Table};
use crate::support::text::clean_text;
use anyhow::Result;
use csv::ReaderBuilder;
use std::borrow::Cow;

pub fn parse(bytes: &[u8]) -> Result<Document> {
    let text = decode(bytes);
    let delimiter = sniff_delimiter(&text);

    let mut reader = ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .delimiter(delimiter)
        .from_reader(text.as_bytes());

    let mut rows: Vec<Vec<Cell>> = Vec::new();
    for record in reader.records() {
        let record = record?;
        let cells: Vec<Cell> = record
            .iter()
            .map(|f| Cell::from_inlines(vec![Inline::plain(clean_text(f.trim()))]))
            .collect();
        rows.push(cells);
    }

    let mut doc = Document::default();
    if !rows.is_empty() {
        doc.blocks.push(Block::Table(Table { rows, has_header: true }));
    }
    Ok(doc)
}

fn decode(bytes: &[u8]) -> Cow<'_, str> {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => Cow::Borrowed(s),
        Err(_) => {
            let (s, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
            Cow::Owned(s.into_owned())
        }
    }
}

/// Pick the candidate with the most hits in the first lines; comma wins ties.
fn sniff_delimiter(text: &str) -> u8 {
    let count =
        |d: u8| text.lines().take(10).map(|l| l.bytes().filter(|&b| b == d).count()).sum::<usize>();
    let mut best = (b',', count(b','));
    for d in [b';', b'\t', b'|'] {
        let c = count(d);
        if c > best.1 {
            best = (d, c);
        }
    }
    best.0
}

use crate::ir::{Block, Cell, Document, Inline, Table};
use crate::support::text::clean_text;
use anyhow::Result;

pub fn parse(bytes: &[u8]) -> Result<Document> {
    let text = decode(bytes);
    let delimiter = sniff_delimiter(&text);

    let mut reader = ::csv::ReaderBuilder::new()
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

fn decode(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            let (s, _, _) = encoding_rs::WINDOWS_1252.decode(bytes);
            s.into_owned()
        }
    }
}

fn sniff_delimiter(text: &str) -> u8 {
    let sample: String = text.lines().take(10).collect::<Vec<_>>().join("\n");
    let candidates = [b',', b';', b'\t', b'|'];
    candidates
        .into_iter()
        .max_by_key(|&d| sample.bytes().filter(|&b| b == d).count())
        .unwrap_or(b',')
}

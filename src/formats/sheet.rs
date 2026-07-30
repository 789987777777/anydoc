//! Excel spreadsheets (xlsx, xlsm, xlsb, xls) via calamine.

use crate::ir::{Block, Cell, Document, Inline, Table, inlines_are_empty};
use crate::support::text::clean_text;
use anyhow::Result;
use calamine::{Data, Reader, open_workbook_auto_from_rs};
use std::io::Cursor;

pub fn parse(bytes: &[u8]) -> Result<Document> {
    let mut workbook = open_workbook_auto_from_rs(Cursor::new(bytes))?;
    let sheet_names = workbook.sheet_names().to_owned();
    let multi_sheet = sheet_names.len() > 1;

    let mut doc = Document::default();
    for name in sheet_names {
        let range = match workbook.worksheet_range(&name) {
            Ok(r) => r,
            Err(_) => continue,
        };
        if range.is_empty() {
            continue;
        }
        if multi_sheet {
            doc.blocks.push(Block::Heading { level: 2, content: vec![Inline::plain(&name)] });
        }
        let mut rows: Vec<Vec<Cell>> = range
            .rows()
            .map(|row| {
                row.iter()
                    .map(|d| Cell::from_inlines(vec![Inline::plain(format_data(d))]))
                    .collect()
            })
            .collect();
        while rows.last().is_some_and(|r| r.iter().all(cell_is_empty)) {
            rows.pop();
        }
        if rows.is_empty() {
            continue;
        }
        doc.blocks.push(Block::Table(Table { rows, has_header: true }));
    }
    Ok(doc)
}

fn cell_is_empty(cell: &Cell) -> bool {
    match cell.blocks.as_slice() {
        [Block::Paragraph(inlines)] => inlines_are_empty(inlines),
        [] => true,
        _ => false,
    }
}

fn format_data(data: &Data) -> String {
    match data {
        Data::Empty => String::new(),
        Data::String(s) => clean_text(s.trim()),
        Data::Float(f) => format_number(*f),
        Data::Int(i) => i.to_string(),
        Data::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Data::Error(e) => format!("#{e:?}"),
        Data::DateTime(dt) => match dt.as_datetime() {
            Some(d) => {
                let s = d.to_string();
                s.strip_suffix(" 00:00:00").unwrap_or(&s).to_string()
            }
            None => format_number(dt.as_f64()),
        },
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
    }
}

fn format_number(f: f64) -> String {
    if f.fract() == 0.0 && f.abs() < 1e15 {
        format!("{}", f as i64)
    } else {
        let s = format!("{f:.6}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    }
}

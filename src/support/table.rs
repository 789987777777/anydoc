//! Table and spreadsheet helpers shared by the frontends.

use crate::ir::{Block, Cell, Inline, inlines_are_empty};

pub fn block_is_blank(block: &Block) -> bool {
    match block {
        Block::Paragraph(inlines) => inlines_are_empty(inlines),
        _ => false,
    }
}

pub fn cell_is_empty(cell: &Cell) -> bool {
    cell.blocks.iter().all(block_is_blank)
}

/// Trim trailing all-empty rows and trailing empty cells from each row.
pub fn trim_trailing_empty(rows: &mut Vec<Vec<Cell>>) {
    while rows.last().is_some_and(|r| r.iter().all(cell_is_empty)) {
        rows.pop();
    }
    for row in rows {
        while row.last().is_some_and(cell_is_empty) {
            row.pop();
        }
    }
}

/// Append one worksheet's blocks: a heading when the workbook has several
/// sheets, tables marked as having a header row.
pub fn push_sheet(out: &mut Vec<Block>, multi_sheet: bool, name: &str, mut blocks: Vec<Block>) {
    if blocks.is_empty() {
        return;
    }
    for block in &mut blocks {
        if let Block::Table(t) = block {
            t.has_header = true;
        }
    }
    if multi_sheet {
        out.push(Block::Heading { level: 2, content: vec![Inline::plain(name)] });
    }
    out.append(&mut blocks);
}

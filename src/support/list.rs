//! Nested-list assembly shared by the doc, docx, and rtf frontends.

use crate::ir::{Block, List, ListItem};

/// One flat list paragraph, keyed on indent level.
pub struct ListEntry {
    pub level: usize,
    pub ordered: bool,
    pub start: u64,
    pub block: Block,
}

/// Pop the accumulated run of list paragraphs into a list block.
pub fn flush_list(blocks: &mut Vec<Block>, list_run: &mut Vec<ListEntry>) {
    if let Some(list) = build_list(std::mem::take(list_run)) {
        blocks.push(Block::List(list));
    }
}

/// Fold a flat run of entries into a nested list keyed on indent level.
fn build_list(entries: Vec<ListEntry>) -> Option<List> {
    let min_lvl = entries.iter().map(|e| e.level).min()?;
    let first = entries.iter().find(|e| e.level == min_lvl).unwrap();
    let mut list = List { ordered: first.ordered, start: first.start, items: Vec::new() };
    let mut iter = entries.into_iter().peekable();
    while let Some(level) = iter.peek().map(|e| e.level) {
        if level <= min_lvl {
            let entry = iter.next().unwrap();
            list.items.push(ListItem { blocks: vec![entry.block], checked: None });
        } else {
            let mut sub = Vec::new();
            while iter.peek().is_some_and(|e| e.level > min_lvl) {
                sub.push(iter.next().unwrap());
            }
            if let Some(sublist) = build_list(sub) {
                if list.items.is_empty() {
                    list.items.push(ListItem { blocks: Vec::new(), checked: None });
                }
                list.items.last_mut().unwrap().blocks.push(Block::List(sublist));
            }
        }
    }
    Some(list)
}

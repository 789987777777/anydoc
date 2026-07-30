//! Nested-list assembly shared by the doc, docx, and rtf frontends.

use crate::ir::{Block, List, ListItem};

/// One flat list paragraph: (indent level, ordered, start number, content).
pub type ListEntry = (usize, bool, u64, Block);

/// Pop the accumulated run of list paragraphs into a list block.
pub fn flush_list(blocks: &mut Vec<Block>, list_run: &mut Vec<ListEntry>) {
    if list_run.is_empty() {
        return;
    }
    let items = std::mem::take(list_run);
    if let Some(list) = build_list(&items) {
        blocks.push(Block::List(list));
    }
}

/// Fold a flat run of entries into a nested list keyed on indent level.
pub fn build_list(items: &[ListEntry]) -> Option<List> {
    if items.is_empty() {
        return None;
    }
    let min_lvl = items.iter().map(|(l, ..)| *l).min().unwrap();
    let (_, ordered, start, _) = items.iter().find(|(l, ..)| *l == min_lvl).unwrap();
    let mut list = List { ordered: *ordered, start: *start, items: Vec::new() };
    let mut i = 0;
    while i < items.len() {
        let (lvl, _, _, block) = &items[i];
        if *lvl <= min_lvl {
            list.items.push(ListItem { blocks: vec![block.clone()], checked: None });
            i += 1;
        } else {
            let child_start = i;
            while i < items.len() && items[i].0 > min_lvl {
                i += 1;
            }
            if let Some(sub) = build_list(&items[child_start..i]) {
                if list.items.is_empty() {
                    list.items.push(ListItem { blocks: Vec::new(), checked: None });
                }
                list.items.last_mut().unwrap().blocks.push(Block::List(sub));
            }
        }
    }
    Some(list)
}

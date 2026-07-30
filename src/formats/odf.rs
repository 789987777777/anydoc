//! OpenDocument Text (.odt) and Spreadsheet (.ods).
//! Spreadsheet cells keep their display text, so number formats are as rendered.

use crate::ir::*;
use crate::support::text::{clean_text, collapse_ws};
use crate::support::xml::{Element, Node, parse_xml};
use crate::support::zip::read_zip_string;
use anyhow::{Context, Result};
use std::cell::RefCell;
use std::collections::HashMap;
use std::io::Cursor;

#[derive(Default)]
struct Ctx {
    text_styles: HashMap<String, Style>,
    list_styles: HashMap<String, Vec<bool>>,
    notes: RefCell<Vec<Note>>,
}

pub fn parse(bytes: &[u8]) -> Result<Document> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;

    let mut ctx = Ctx::default();
    if let Ok(s) = read_zip_string(&mut zip, "styles.xml")
        && let Ok(tree) = parse_xml(&s)
    {
        collect_styles(&tree, &mut ctx);
    }
    let content = read_zip_string(&mut zip, "content.xml")?;
    let tree = parse_xml(&content)?;
    collect_styles(&tree, &mut ctx);

    let body = tree
        .find("document-content")
        .and_then(|d| d.find("body"))
        .context("content.xml has no office:body")?;

    let blocks = if let Some(text) = body.find("text") {
        parse_container(text, &ctx)
    } else if let Some(sheet) = body.find("spreadsheet") {
        parse_spreadsheet(sheet, &ctx)
    } else {
        anyhow::bail!("content.xml has no office:text or office:spreadsheet body");
    };
    Ok(Document { blocks, notes: ctx.notes.into_inner() })
}

fn parse_spreadsheet(sheet: &Element, ctx: &Ctx) -> Vec<Block> {
    let tables: Vec<&Element> = sheet.child_elems().filter(|e| e.name == "table").collect();
    let multi_sheet = tables.len() > 1;
    let mut blocks = Vec::new();
    for table in tables {
        let mut parsed = parse_table(table, ctx);
        if parsed.is_empty() {
            continue;
        }
        for block in &mut parsed {
            if let Block::Table(t) = block {
                t.has_header = true;
            }
        }
        if multi_sheet && let Some(name) = table.attr("name") {
            blocks.push(Block::Heading { level: 2, content: vec![Inline::plain(name)] });
        }
        blocks.append(&mut parsed);
    }
    blocks
}

fn collect_styles(tree: &Element, ctx: &mut Ctx) {
    let mut style_elems: Vec<&Element> = Vec::new();
    for root in tree.child_elems() {
        for section in root.child_elems() {
            if matches!(section.name.as_str(), "automatic-styles" | "styles") {
                style_elems.extend(section.child_elems());
            }
        }
    }
    let mut raw: HashMap<&str, (&Element, Option<&str>)> = HashMap::new();
    for style in &style_elems {
        if style.name == "style"
            && let Some(name) = style.attr("name")
        {
            raw.insert(name, (style, style.attr("parent-style-name")));
        }
    }
    for name in raw.keys() {
        let style = resolve_text_style(name, &raw, 0);
        ctx.text_styles.insert((*name).to_string(), style);
    }
    for style in &style_elems {
        if style.name == "list-style"
            && let Some(name) = style.attr("name")
        {
            let mut levels = vec![false; 10];
            for lvl in style.child_elems() {
                let ordered = lvl.name == "list-level-style-number";
                if let Some(n) = lvl.attr("level").and_then(|v| v.parse::<usize>().ok())
                    && n >= 1
                    && n <= levels.len()
                {
                    levels[n - 1] = ordered;
                }
            }
            ctx.list_styles.insert(name.to_string(), levels);
        }
    }
}

fn resolve_text_style(
    name: &str,
    raw: &HashMap<&str, (&Element, Option<&str>)>,
    depth: usize,
) -> Style {
    if depth > 8 {
        return Style::PLAIN;
    }
    let Some((elem, parent)) = raw.get(name) else {
        return Style::PLAIN;
    };
    let mut style = match parent {
        Some(p) => resolve_text_style(p, raw, depth + 1),
        None => Style::PLAIN,
    };
    if let Some(props) = elem.find("text-properties") {
        if let Some(w) = props.attr("font-weight") {
            style.bold = w == "bold" || w.parse::<u32>().is_ok_and(|n| n >= 600);
        }
        if let Some(s) = props.attr("font-style") {
            style.italic = s == "italic" || s == "oblique";
        }
        if let Some(lt) = props.attr("text-line-through-style") {
            style.strike = lt != "none";
        }
    }
    style
}

fn parse_container(parent: &Element, ctx: &Ctx) -> Vec<Block> {
    let mut blocks = Vec::new();
    for child in parent.child_elems() {
        parse_block_elem(child, ctx, &mut blocks);
    }
    blocks
}

fn parse_block_elem(elem: &Element, ctx: &Ctx, blocks: &mut Vec<Block>) {
    match elem.name.as_str() {
        "h" => {
            let level = elem.attr("outline-level").and_then(|v| v.parse::<u8>().ok()).unwrap_or(1);
            let inlines = parse_inlines(elem, ctx, Style::PLAIN);
            if !inlines_are_empty(&inlines) {
                blocks.push(Block::Heading { level, content: inlines });
            }
        }
        "p" => blocks.push(Block::Paragraph(parse_inlines(elem, ctx, Style::PLAIN))),
        "list" => {
            if let Some(list) = parse_list(elem, ctx, 0, None) {
                blocks.push(Block::List(list));
            }
        }
        "table" => blocks.extend(parse_table(elem, ctx)),
        "section" => blocks.extend(parse_container(elem, ctx)),
        "table-of-content" | "alphabetical-index" | "bibliography" | "illustration-index" => {}
        _ => {}
    }
}

fn parse_list(
    elem: &Element,
    ctx: &Ctx,
    depth: usize,
    inherited_style: Option<&str>,
) -> Option<List> {
    let style_name = elem.attr("style-name").or(inherited_style);
    let ordered = style_name
        .and_then(|n| ctx.list_styles.get(n))
        .and_then(|levels| levels.get(depth).copied())
        .unwrap_or(false);
    let mut list = List { ordered, start: 1, items: Vec::new() };
    for item in elem.child_elems() {
        if !matches!(item.name.as_str(), "list-item" | "list-header") {
            continue;
        }
        let mut item_blocks = Vec::new();
        for child in item.child_elems() {
            if child.name == "list" {
                if let Some(sub) = parse_list(child, ctx, depth + 1, style_name) {
                    item_blocks.push(Block::List(sub));
                }
            } else {
                parse_block_elem(child, ctx, &mut item_blocks);
            }
        }
        list.items.push(ListItem { blocks: item_blocks, checked: None });
    }
    if list.items.is_empty() { None } else { Some(list) }
}

fn parse_table(elem: &Element, ctx: &Ctx) -> Vec<Block> {
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let mut has_header = false;
    for child in elem.child_elems() {
        match child.name.as_str() {
            "table-header-rows" => {
                has_header = true;
                for row in child.find_all("table-row") {
                    rows.push(parse_row(row, ctx));
                }
            }
            "table-row" => {
                let repeat: usize = child
                    .attr("number-rows-repeated")
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1)
                    .clamp(1, 100);
                let row = parse_row(child, ctx);
                for _ in 0..repeat {
                    rows.push(row.clone());
                }
            }
            "table-rows" | "table-row-group" => {
                for row in child.find_all("table-row") {
                    rows.push(parse_row(row, ctx));
                }
            }
            _ => {}
        }
    }
    while rows.last().is_some_and(|r| r.iter().all(|c| c.blocks.iter().all(block_is_empty))) {
        rows.pop();
    }
    if rows.is_empty() {
        return Vec::new();
    }
    vec![Block::Table(Table { rows, has_header })]
}

fn block_is_empty(block: &Block) -> bool {
    match block {
        Block::Paragraph(inlines) => inlines_are_empty(inlines),
        _ => false,
    }
}

fn parse_row(row: &Element, ctx: &Ctx) -> Vec<Cell> {
    let mut cells = Vec::new();
    for cell in row.child_elems() {
        let repeat: usize = cell
            .attr("number-columns-repeated")
            .and_then(|v| v.parse().ok())
            .unwrap_or(1)
            .clamp(1, 100);
        match cell.name.as_str() {
            "table-cell" => {
                let span: usize =
                    cell.attr("number-columns-spanned").and_then(|v| v.parse().ok()).unwrap_or(1);
                let blocks = parse_container(cell, ctx);
                for _ in 0..repeat {
                    cells.push(Cell { blocks: blocks.clone() });
                    for _ in 1..span.max(1) {
                        cells.push(Cell::default());
                    }
                }
            }
            "covered-table-cell" => {
                for _ in 0..repeat {
                    cells.push(Cell::default());
                }
            }
            _ => {}
        }
    }
    while cells.last().is_some_and(|c| c.blocks.iter().all(block_is_empty)) {
        cells.pop();
    }
    cells
}

fn parse_inlines(elem: &Element, ctx: &Ctx, style: Style) -> Vec<Inline> {
    let mut out = Vec::new();
    walk_inlines(elem, ctx, style, &mut out);
    out
}

fn walk_inlines(elem: &Element, ctx: &Ctx, style: Style, out: &mut Vec<Inline>) {
    for node in &elem.children {
        match node {
            Node::Text(t) => {
                let text = collapse_ws(&clean_text(t));
                if !text.is_empty() {
                    out.push(Inline::Text { text, style });
                }
            }
            Node::Elem(child) => match child.name.as_str() {
                "span" => {
                    let child_style = child
                        .attr("style-name")
                        .and_then(|n| ctx.text_styles.get(n).copied())
                        .unwrap_or(Style::PLAIN);
                    let merged = Style {
                        bold: style.bold || child_style.bold,
                        italic: style.italic || child_style.italic,
                        strike: style.strike || child_style.strike,
                        code: false,
                    };
                    walk_inlines(child, ctx, merged, out);
                }
                "a" => {
                    let url = child.attr("href").unwrap_or("").to_string();
                    let content = parse_inlines(child, ctx, style);
                    if !content.is_empty() || !url.is_empty() {
                        out.push(Inline::Link { content, url });
                    }
                }
                "s" => {
                    let n = child.attr("c").and_then(|v| v.parse::<usize>().ok()).unwrap_or(1);
                    out.push(Inline::Text { text: " ".repeat(n.min(20)), style: Style::PLAIN });
                }
                "tab" => out.push(Inline::Text { text: " ".into(), style: Style::PLAIN }),
                "line-break" => out.push(Inline::LineBreak),
                "note" => {
                    let idx = ctx.notes.borrow().len();
                    let id =
                        child.attr("id").map(String::from).unwrap_or_else(|| format!("odt{idx}"));
                    let blocks = child
                        .find("note-body")
                        .map(|b| parse_container(b, ctx))
                        .unwrap_or_default();
                    ctx.notes.borrow_mut().push(Note { id: id.clone(), blocks });
                    out.push(Inline::NoteRef(id));
                }
                "annotation" | "tracked-changes" | "soft-page-break" => {}
                "frame" => {
                    let alt = child
                        .descendants("title")
                        .first()
                        .map(|t| t.text())
                        .or_else(|| child.descendants("desc").first().map(|d| d.text()))
                        .unwrap_or_default();
                    let url = child
                        .descendants("image")
                        .first()
                        .and_then(|i| i.attr("href"))
                        .map(|s| s.to_string());
                    if url.is_some() || !alt.trim().is_empty() {
                        out.push(Inline::Image { alt: clean_text(alt.trim()), url });
                    }
                }
                _ => walk_inlines(child, ctx, style, out),
            },
        }
    }
}

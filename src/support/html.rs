//! (X)HTML element tree -> IR blocks. Used by the EPUB frontend.

use super::text::{clean_text, collapse_ws};
use super::xml::{Element, Node};
use crate::ir::*;

pub fn to_blocks(body: &Element) -> Vec<Block> {
    let mut builder = Builder::default();
    builder.walk_children(body, Style::PLAIN);
    builder.finish()
}

#[derive(Default)]
struct Builder {
    blocks: Vec<Block>,
    inlines: Vec<Inline>,
}

impl Builder {
    fn flush_paragraph(&mut self) {
        if !self.inlines.is_empty() {
            let inlines = std::mem::take(&mut self.inlines);
            if !inlines_are_empty(&inlines) {
                self.blocks.push(Block::Paragraph(inlines));
            }
        }
    }

    fn finish(mut self) -> Vec<Block> {
        self.flush_paragraph();
        self.blocks
    }

    fn walk_children(&mut self, elem: &Element, style: Style) {
        for node in &elem.children {
            match node {
                Node::Text(t) => self.push_text(t, style),
                Node::Elem(e) => self.walk_elem(e, style),
            }
        }
    }

    fn push_text(&mut self, text: &str, style: Style) {
        let collapsed = collapse_ws(&clean_text(text));
        if collapsed.is_empty() {
            return;
        }
        if collapsed == " " && self.inlines.is_empty() {
            return;
        }
        self.inlines.push(Inline::Text { text: collapsed, style });
    }

    fn walk_elem(&mut self, elem: &Element, style: Style) {
        match elem.name.as_str() {
            "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => {
                self.flush_paragraph();
                let level = elem.name[1..].parse::<u8>().unwrap_or(1);
                let content = inline_children(elem, Style::PLAIN);
                if !inlines_are_empty(&content) {
                    self.blocks.push(Block::Heading { level, content });
                }
            }
            "p" => {
                self.flush_paragraph();
                let content = inline_children(elem, style);
                if !inlines_are_empty(&content) {
                    self.blocks.push(Block::Paragraph(content));
                }
            }
            "ul" | "ol" => {
                self.flush_paragraph();
                if let Some(list) = parse_list(elem) {
                    self.blocks.push(Block::List(list));
                }
            }
            "table" => {
                self.flush_paragraph();
                if let Some(t) = parse_table(elem) {
                    self.blocks.push(t);
                }
            }
            "blockquote" => {
                self.flush_paragraph();
                let inner = to_blocks(elem);
                if !inner.is_empty() {
                    self.blocks.push(Block::BlockQuote(inner));
                }
            }
            "pre" => {
                self.flush_paragraph();
                let text = elem.text();
                if !text.trim().is_empty() {
                    self.blocks.push(Block::CodeBlock { lang: None, text });
                }
            }
            "hr" => {
                self.flush_paragraph();
                self.blocks.push(Block::Rule);
            }
            name if is_container_tag(name) => {
                if has_block_children(elem) {
                    self.flush_paragraph();
                    self.walk_children(elem, style);
                    self.flush_paragraph();
                } else {
                    self.walk_children(elem, style);
                }
            }
            "script" | "style" | "head" | "template" | "noscript" => {}
            _ => self.walk_inline(elem, style),
        }
    }

    fn walk_inline(&mut self, elem: &Element, style: Style) {
        let style = merge_inline_style(elem, style);
        match elem.name.as_str() {
            "br" => self.inlines.push(Inline::LineBreak),
            "img" | "image" => {
                let alt = elem.attr("alt").unwrap_or("").to_string();
                let url = elem.attr("src").or_else(|| elem.attr("href")).map(String::from);
                self.inlines.push(Inline::Image { alt: clean_text(&alt), url });
            }
            "a" => {
                let url = elem.attr("href").unwrap_or("").to_string();
                let url = if url.starts_with('#') { String::new() } else { url };
                let content = inline_children(elem, style);
                if !content.is_empty() {
                    if url.is_empty() {
                        self.inlines.extend(content);
                    } else {
                        self.inlines.push(Inline::Link { content, url });
                    }
                }
            }
            _ => self.walk_children(elem, style),
        }
    }
}

fn merge_inline_style(elem: &Element, mut style: Style) -> Style {
    match elem.name.as_str() {
        "b" | "strong" => style.bold = true,
        "i" | "em" | "cite" | "dfn" | "var" => style.italic = true,
        "s" | "del" | "strike" => style.strike = true,
        "code" | "kbd" | "samp" | "tt" => style.code = true,
        _ => {}
    }
    style
}

fn inline_children(elem: &Element, style: Style) -> Vec<Inline> {
    let mut b = Builder::default();
    b.walk_children(elem, style);
    let mut blocks = b.finish();
    if blocks.len() == 1
        && let Block::Paragraph(inlines) = blocks.remove(0)
    {
        return inlines;
    }
    let mut out = Vec::new();
    for (i, block) in blocks.iter().enumerate() {
        if i > 0 {
            out.push(Inline::LineBreak);
        }
        match block {
            Block::Paragraph(inlines) | Block::Heading { content: inlines, .. } => {
                out.extend(inlines.iter().cloned())
            }
            other => out.push(Inline::plain(collapse_ws(&block_text(other)))),
        }
    }
    out
}

fn block_text(block: &Block) -> String {
    match block {
        Block::Paragraph(i) | Block::Heading { content: i, .. } => inlines_to_plain_text(i),
        Block::List(list) => list
            .items
            .iter()
            .flat_map(|it| it.blocks.iter().map(block_text))
            .collect::<Vec<_>>()
            .join(" "),
        Block::BlockQuote(blocks) => blocks.iter().map(block_text).collect::<Vec<_>>().join(" "),
        Block::CodeBlock { text, .. } => text.clone(),
        Block::Table(_) | Block::Rule => String::new(),
    }
}

/// Elements that only group other content, walked through transparently.
fn is_container_tag(name: &str) -> bool {
    matches!(
        name,
        "div"
            | "section"
            | "article"
            | "aside"
            | "main"
            | "nav"
            | "header"
            | "footer"
            | "figure"
            | "figcaption"
            | "center"
            | "details"
            | "summary"
            | "li"
            | "dl"
            | "dt"
            | "dd"
            | "body"
    )
}

fn is_block_tag(name: &str) -> bool {
    is_container_tag(name)
        || matches!(
            name,
            "p" | "ul"
                | "ol"
                | "table"
                | "blockquote"
                | "pre"
                | "hr"
                | "h1"
                | "h2"
                | "h3"
                | "h4"
                | "h5"
                | "h6"
        )
}

fn has_block_children(elem: &Element) -> bool {
    elem.child_elems().any(|e| is_block_tag(&e.name))
}

fn parse_list(elem: &Element) -> Option<List> {
    let ordered = elem.name == "ol";
    let start = elem.attr("start").and_then(|v| v.parse().ok()).unwrap_or(1);
    let mut items = Vec::new();
    for li in elem.find_all("li") {
        let blocks = to_blocks(li);
        items.push(ListItem { blocks, checked: None });
    }
    if items.is_empty() { None } else { Some(List { ordered, start, items }) }
}

fn parse_table(elem: &Element) -> Option<Block> {
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let mut has_header = false;
    let mut row_elems: Vec<(&Element, bool)> = Vec::new();
    for child in elem.child_elems() {
        match child.name.as_str() {
            "thead" => {
                for tr in child.find_all("tr") {
                    row_elems.push((tr, true));
                }
            }
            "tbody" | "tfoot" => {
                for tr in child.find_all("tr") {
                    row_elems.push((tr, false));
                }
            }
            "tr" => row_elems.push((child, false)),
            _ => {}
        }
    }
    for (i, (tr, in_head)) in row_elems.iter().enumerate() {
        let mut cells = Vec::new();
        let mut all_th = true;
        for cell in tr.child_elems() {
            if !matches!(cell.name.as_str(), "td" | "th") {
                continue;
            }
            if cell.name != "th" {
                all_th = false;
            }
            let span: usize =
                cell.attr("colspan").and_then(|v| v.parse().ok()).unwrap_or(1).clamp(1, 100);
            cells.push(Cell { blocks: to_blocks(cell) });
            for _ in 1..span {
                cells.push(Cell::default());
            }
        }
        if i == 0 && (*in_head || (all_th && !cells.is_empty())) {
            has_header = true;
        }
        rows.push(cells);
    }
    if rows.is_empty() {
        return None;
    }
    Some(Block::Table(Table { rows, has_header }))
}

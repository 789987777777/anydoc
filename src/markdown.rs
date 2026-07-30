//! The single IR -> GitHub-Flavored Markdown serializer.

use crate::ir::*;
use std::collections::{HashMap, HashSet};

/// Footnote id -> rendered number, shared by all render functions.
type NoteNumbers = HashMap<String, usize>;

pub fn document_to_markdown(doc: &Document) -> String {
    let nums = number_notes(doc);
    let mut parts: Vec<String> = Vec::new();
    for block in &doc.blocks {
        if let Some(s) = render_block(block, &nums) {
            parts.push(s);
        }
    }
    let mut ordered: Vec<(&Note, usize)> =
        doc.notes.iter().filter_map(|n| nums.get(&n.id).map(|&num| (n, num))).collect();
    ordered.sort_by_key(|(_, num)| *num);
    for (note, num) in ordered {
        let body = render_blocks(&note.blocks, &nums);
        if body.is_empty() {
            continue;
        }
        let mut lines = body.lines();
        let first = lines.next().unwrap_or("");
        let mut s = format!("[^{num}]: {first}");
        for line in lines {
            s.push('\n');
            if !line.is_empty() {
                s.push_str("    ");
                s.push_str(line);
            }
        }
        parts.push(s);
    }
    let mut out = parts.join("\n\n");
    if !out.is_empty() {
        out.push('\n');
    }
    out
}

/// Number notes in first-reference order; unreferenced notes follow at the end.
fn number_notes(doc: &Document) -> NoteNumbers {
    let valid: HashMap<&str, &Note> = doc
        .notes
        .iter()
        .filter(|n| !n.blocks.iter().all(block_is_blank))
        .map(|n| (n.id.as_str(), n))
        .collect();
    let mut order: Vec<String> = Vec::new();
    let mut seen = HashSet::new();
    collect_note_refs(&doc.blocks, &valid, &mut order, &mut seen);
    for note in &doc.notes {
        if valid.contains_key(note.id.as_str()) && seen.insert(note.id.clone()) {
            order.push(note.id.clone());
        }
    }
    order.into_iter().enumerate().map(|(i, id)| (id, i + 1)).collect()
}

fn block_is_blank(block: &Block) -> bool {
    match block {
        Block::Paragraph(inlines) => inlines_are_empty(inlines),
        _ => false,
    }
}

fn collect_note_refs(
    blocks: &[Block],
    valid: &HashMap<&str, &Note>,
    order: &mut Vec<String>,
    seen: &mut HashSet<String>,
) {
    fn walk_inlines(
        inlines: &[Inline],
        valid: &HashMap<&str, &Note>,
        order: &mut Vec<String>,
        seen: &mut HashSet<String>,
    ) {
        for inline in inlines {
            match inline {
                Inline::NoteRef(id) => {
                    if valid.contains_key(id.as_str()) && seen.insert(id.clone()) {
                        order.push(id.clone());
                        if let Some(note) = valid.get(id.as_str()) {
                            collect_note_refs(&note.blocks, valid, order, seen);
                        }
                    }
                }
                Inline::Link { content, .. } => walk_inlines(content, valid, order, seen),
                _ => {}
            }
        }
    }
    for block in blocks {
        match block {
            Block::Paragraph(i) | Block::Heading { content: i, .. } => {
                walk_inlines(i, valid, order, seen)
            }
            Block::List(list) => {
                for item in &list.items {
                    collect_note_refs(&item.blocks, valid, order, seen);
                }
            }
            Block::Table(table) => {
                for row in &table.rows {
                    for cell in row {
                        collect_note_refs(&cell.blocks, valid, order, seen);
                    }
                }
            }
            Block::BlockQuote(blocks) => collect_note_refs(blocks, valid, order, seen),
            Block::CodeBlock { .. } | Block::Rule => {}
        }
    }
}

fn render_blocks(blocks: &[Block], nums: &NoteNumbers) -> String {
    let parts: Vec<String> = blocks.iter().filter_map(|b| render_block(b, nums)).collect();
    parts.join("\n\n")
}

fn render_block(block: &Block, nums: &NoteNumbers) -> Option<String> {
    match block {
        Block::Heading { level, content } => {
            let text = render_inlines(content, InlineContext::Heading, nums);
            let text = text.trim();
            if text.is_empty() {
                return None;
            }
            let level = (*level).clamp(1, 6) as usize;
            Some(format!("{} {}", "#".repeat(level), text))
        }
        Block::Paragraph(inlines) => {
            let text = render_inlines(inlines, InlineContext::Block, nums);
            let trimmed = trim_paragraph(&text);
            if trimmed.is_empty() { None } else { Some(trimmed) }
        }
        Block::List(list) => render_list(list, nums),
        Block::Table(table) => render_table(table, nums),
        Block::BlockQuote(blocks) => {
            let inner = render_blocks(blocks, nums);
            if inner.is_empty() {
                return None;
            }
            let quoted: Vec<String> = inner
                .lines()
                .map(|l| if l.is_empty() { ">".to_string() } else { format!("> {l}") })
                .collect();
            Some(quoted.join("\n"))
        }
        Block::CodeBlock { lang, text } => {
            let longest_run = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
            let fence = "`".repeat((longest_run + 1).max(3));
            let lang = lang.as_deref().unwrap_or("");
            let body = text.trim_end_matches('\n');
            Some(format!("{fence}{lang}\n{body}\n{fence}"))
        }
        Block::Rule => Some("---".to_string()),
    }
}

/// Trim paragraph lines, keeping hard-break backslashes intact.
fn trim_paragraph(text: &str) -> String {
    let lines: Vec<&str> = text
        .lines()
        .map(|l| {
            let t = l.trim_start();
            let t = if ends_with_hard_break(t) { t } else { t.trim_end() };
            if t.trim_end_matches('\\').trim().is_empty() { "" } else { t }
        })
        .collect();
    let start = lines.iter().position(|l| !l.is_empty());
    let end = lines.iter().rposition(|l| !l.is_empty());
    match (start, end) {
        (Some(s), Some(e)) => {
            let mut out = lines[s..=e].join("\n");
            if ends_with_hard_break(&out) {
                out.pop();
                out.truncate(out.trim_end().len());
            }
            out
        }
        _ => String::new(),
    }
}

fn ends_with_hard_break(line: &str) -> bool {
    line.chars().rev().take_while(|&c| c == '\\').count() % 2 == 1
}

// ---------------------------------------------------------------------------
// Lists

fn render_list(list: &List, nums: &NoteNumbers) -> Option<String> {
    if list.items.is_empty() {
        return None;
    }
    let mut rendered_items: Vec<String> = Vec::new();
    let mut loose = false;
    for (i, item) in list.items.iter().enumerate() {
        let marker = if list.ordered {
            format!("{}. ", list.start.saturating_add(i as u64))
        } else {
            "- ".to_string()
        };
        let checkbox = match item.checked {
            Some(true) => "[x] ",
            Some(false) => "[ ] ",
            None => "",
        };
        let body = render_blocks(&item.blocks, nums);
        if item.blocks.len() > 1 {
            loose = true;
        }
        let indent = " ".repeat(marker.len());
        let mut lines = body.lines();
        let first = lines.next().unwrap_or("");
        let mut s = format!("{marker}{checkbox}{first}");
        for line in lines {
            s.push('\n');
            if line.is_empty() {
                loose = true;
            } else {
                s.push_str(&indent);
                s.push_str(line);
            }
        }
        rendered_items.push(s);
    }
    let sep = if loose { "\n\n" } else { "\n" };
    Some(rendered_items.join(sep))
}

// ---------------------------------------------------------------------------
// Tables

fn render_table(table: &Table, nums: &NoteNumbers) -> Option<String> {
    let rows: Vec<&Vec<Cell>> = table.rows.iter().filter(|r| !r.is_empty()).collect();
    if rows.is_empty() {
        return None;
    }
    let width = rows.iter().map(|r| r.len()).max().unwrap_or(0);
    if width == 0 {
        return None;
    }

    let mut rendered: Vec<Vec<String>> = rows
        .iter()
        .map(|row| {
            let mut cells: Vec<String> = row.iter().map(|c| render_cell(c, nums)).collect();
            cells.resize(width, String::new());
            cells
        })
        .collect();

    let mut out = String::new();
    let header: Vec<String> = if table.has_header && !rendered.is_empty() {
        rendered.remove(0)
    } else {
        vec![String::new(); width]
    };
    out.push_str(&format_row(&header));
    out.push('\n');
    out.push_str(&format_row(&vec!["---".to_string(); width]));
    for row in &rendered {
        out.push('\n');
        out.push_str(&format_row(row));
    }
    Some(out)
}

fn format_row(cells: &[String]) -> String {
    let mut s = String::from("|");
    for cell in cells {
        s.push(' ');
        s.push_str(cell);
        s.push_str(" |");
    }
    s
}

/// Flatten arbitrary block content into a single table-cell line.
fn render_cell(cell: &Cell, nums: &NoteNumbers) -> String {
    let mut parts: Vec<String> = Vec::new();
    for block in &cell.blocks {
        cell_block_text(block, nums, &mut parts);
    }
    parts
        .join("<br>")
        .lines()
        .map(|l| l.trim().trim_end_matches('\\').trim_end())
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join("<br>")
}

fn cell_block_text(block: &Block, nums: &NoteNumbers, parts: &mut Vec<String>) {
    match block {
        Block::Heading { content, .. } => {
            let t = render_inlines(content, InlineContext::TableCell, nums);
            if !t.trim().is_empty() {
                parts.push(format!("**{}**", t.trim()));
            }
        }
        Block::Paragraph(inlines) => {
            let t = render_inlines(inlines, InlineContext::TableCell, nums);
            if !t.trim().is_empty() {
                parts.push(t.trim().to_string());
            }
        }
        Block::List(list) => {
            for (i, item) in list.items.iter().enumerate() {
                let mut inner: Vec<String> = Vec::new();
                for b in &item.blocks {
                    cell_block_text(b, nums, &mut inner);
                }
                let marker = if list.ordered {
                    format!("{}. ", list.start.saturating_add(i as u64))
                } else {
                    "• ".to_string()
                };
                if !inner.is_empty() {
                    parts.push(format!("{marker}{}", inner.join(" ")));
                }
            }
        }
        Block::Table(table) => {
            for row in &table.rows {
                let cells: Vec<String> =
                    row.iter().map(|c| render_cell(c, nums)).filter(|c| !c.is_empty()).collect();
                if !cells.is_empty() {
                    parts.push(cells.join(" / "));
                }
            }
        }
        Block::BlockQuote(blocks) => {
            for b in blocks {
                cell_block_text(b, nums, parts);
            }
        }
        Block::CodeBlock { text, .. } => {
            let t = text.trim();
            if !t.is_empty() {
                parts.push(format!("`{}`", t.replace('`', "'").replace('\n', " ")));
            }
        }
        Block::Rule => {}
    }
}

// ---------------------------------------------------------------------------
// Inlines

#[derive(Clone, Copy, PartialEq)]
enum InlineContext {
    Block,
    Heading,
    TableCell,
}

fn render_inlines(inlines: &[Inline], ctx: InlineContext, nums: &NoteNumbers) -> String {
    let runs = normalize(inlines);
    let mut out = String::new();
    for run in &runs {
        match run {
            Norm::Text { text, style } => render_text_run(text, *style, ctx, &mut out),
            Norm::NoteRef(id) => {
                if let Some(num) = nums.get(id) {
                    out.push_str(&format!("[^{num}]"));
                }
            }
            Norm::Link { content, url } => {
                let text = render_inlines(content, ctx, nums);
                let text = text.trim();
                if url.is_empty() {
                    out.push_str(text);
                } else if text.is_empty() {
                    out.push_str(&format!("[{}]({})", escape_url_as_text(url), format_url(url)));
                } else {
                    out.push_str(&format!("[{}]({})", text, format_url(url)));
                }
            }
            Norm::Image { alt, url } => match url {
                Some(u) if !u.is_empty() => {
                    let alt = alt.replace(['[', ']'], " ");
                    out.push_str(&format!("![{}]({})", alt.trim(), format_url(u)));
                }
                _ => {
                    if !alt.trim().is_empty() {
                        out.push_str(&escape_text(alt.trim(), ctx, false));
                    }
                }
            },
            Norm::LineBreak => match ctx {
                InlineContext::Block => out.push_str("\\\n"),
                InlineContext::Heading => out.push(' '),
                InlineContext::TableCell => out.push('\n'),
            },
        }
    }
    out
}

enum Norm {
    Text { text: String, style: Style },
    Link { content: Vec<Inline>, url: String },
    Image { alt: String, url: Option<String> },
    NoteRef(String),
    LineBreak,
}

fn normalize(inlines: &[Inline]) -> Vec<Norm> {
    let mut out = normalize_pass(inlines);
    // Re-join styled runs split only by whitespace: **a** **b** -> **a b**.
    let mut i = 0;
    while i + 2 < out.len() {
        let merge = matches!(
            (&out[i], &out[i + 1], &out[i + 2]),
            (
                Norm::Text { style: a, .. },
                Norm::Text { text: ws, style: Style::PLAIN },
                Norm::Text { style: b, .. },
            ) if a == b && *a != Style::PLAIN && !a.code && ws.trim().is_empty()
        );
        if merge {
            let Norm::Text { text: ws, .. } = out.remove(i + 1) else { unreachable!() };
            let Norm::Text { text: tail, .. } = out.remove(i + 1) else { unreachable!() };
            let Norm::Text { text, .. } = &mut out[i] else { unreachable!() };
            text.push_str(&ws);
            text.push_str(&tail);
        } else {
            i += 1;
        }
    }
    out
}

fn normalize_pass(inlines: &[Inline]) -> Vec<Norm> {
    let mut out: Vec<Norm> = Vec::new();
    for inline in inlines {
        match inline {
            Inline::Text { text, style } => {
                if text.is_empty() {
                    continue;
                }
                let style = if text.trim().is_empty() { Style::PLAIN } else { *style };
                if let Some(Norm::Text { text: prev, style: prev_style }) = out.last_mut()
                    && *prev_style == style
                {
                    prev.push_str(text);
                    continue;
                }
                out.push(Norm::Text { text: text.clone(), style });
            }
            Inline::Link { content, url } => {
                if inlines_are_empty(content) && url.is_empty() {
                    continue;
                }
                out.push(Norm::Link { content: content.clone(), url: url.clone() });
            }
            Inline::Image { alt, url } => {
                out.push(Norm::Image { alt: alt.clone(), url: url.clone() })
            }
            Inline::NoteRef(id) => out.push(Norm::NoteRef(id.clone())),
            Inline::LineBreak => out.push(Norm::LineBreak),
        }
    }
    out
}

/// Emit a styled run, moving edge whitespace outside the delimiters.
fn render_text_run(text: &str, style: Style, ctx: InlineContext, out: &mut String) {
    if style == Style::PLAIN {
        let at_line_start = out.is_empty() || out.ends_with('\n');
        out.push_str(&escape_text(text, ctx, at_line_start));
        return;
    }
    let core_start = text.len() - text.trim_start().len();
    let core_end = text.trim_end().len();
    let (lead, core, trail) = (&text[..core_start], &text[core_start..core_end], &text[core_end..]);
    if !lead.is_empty() {
        out.push_str(lead);
    }
    if !core.is_empty() {
        if style.code {
            push_code_span(core, out);
        } else {
            let mut open = String::new();
            if style.strike {
                open.push_str("~~");
            }
            if style.bold {
                open.push_str("**");
            }
            if style.italic {
                open.push('*');
            }
            let close: String = open.chars().rev().collect();
            out.push_str(&open);
            out.push_str(&escape_text(core, ctx, false));
            out.push_str(&close);
        }
    }
    if !trail.is_empty() {
        out.push_str(trail);
    }
}

fn push_code_span(text: &str, out: &mut String) {
    let text = text.replace('\n', " ");
    let longest_run = text.split(|c| c != '`').map(str::len).max().unwrap_or(0);
    let fence = "`".repeat((longest_run + 1).max(1));
    let pad = if text.starts_with('`') || text.ends_with('`') { " " } else { "" };
    out.push_str(&format!("{fence}{pad}{text}{pad}{fence}"));
}

// ---------------------------------------------------------------------------
// Escaping

/// Escape Markdown syntax in document text.
fn escape_text(text: &str, ctx: InlineContext, at_line_start: bool) -> String {
    let mut out = String::with_capacity(text.len() + 8);
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut line_has_content = !at_line_start;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            out.push('\n');
            line_has_content = false;
            i += 1;
            continue;
        }
        let start_of_line = !line_has_content;
        if !c.is_whitespace() {
            line_has_content = true;
        }
        match c {
            '\\' | '`' | '*' | '_' | '[' | ']' | '<' | '~' => {
                out.push('\\');
                out.push(c);
            }
            '|' if ctx == InlineContext::TableCell => out.push_str("\\|"),
            '&' if entity_ahead(&chars[i..]) => out.push_str("&amp;"),
            '#' | '-' | '+' | '>' | '=' if start_of_line => {
                out.push('\\');
                out.push(c);
            }
            '0'..='9' if start_of_line => {
                let mut j = i;
                while j < chars.len() && chars[j].is_ascii_digit() {
                    j += 1;
                }
                if j < chars.len() && (chars[j] == '.' || chars[j] == ')') {
                    out.extend(&chars[i..j]);
                    out.push('\\');
                    out.push(chars[j]);
                    i = j + 1;
                    continue;
                }
                out.push(c);
            }
            _ => out.push(c),
        }
        i += 1;
    }
    out
}

fn entity_ahead(chars: &[char]) -> bool {
    let mut i = 1;
    if i < chars.len() && chars[i] == '#' {
        return true;
    }
    let mut seen = 0;
    while i < chars.len() && chars[i].is_ascii_alphanumeric() {
        i += 1;
        seen += 1;
    }
    seen > 0 && i < chars.len() && chars[i] == ';'
}

/// Format a link destination, angle-bracketing when needed.
fn format_url(url: &str) -> String {
    if url.chars().any(|c| c.is_whitespace() || c == '(' || c == ')' || c == '<' || c == '>') {
        let escaped: String = url
            .chars()
            .map(|c| match c {
                '<' => "%3C".to_string(),
                '>' => "%3E".to_string(),
                '\n' | '\r' => String::new(),
                c => c.to_string(),
            })
            .collect();
        format!("<{escaped}>")
    } else {
        url.to_string()
    }
}

fn escape_url_as_text(url: &str) -> String {
    url.replace('\\', "\\\\").replace('[', "\\[").replace(']', "\\]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{Block, Cell, Document, Inline, List, ListItem, Style, Table};

    fn doc(blocks: Vec<Block>) -> String {
        document_to_markdown(&Document { blocks, notes: Vec::new() })
    }

    fn styled(text: &str, style: Style) -> Inline {
        Inline::Text { text: text.into(), style }
    }

    const BOLD: Style = Style { bold: true, italic: false, strike: false, code: false };
    const ITALIC: Style = Style { bold: false, italic: true, strike: false, code: false };

    #[test]
    fn heading_and_paragraph() {
        let md = doc(vec![
            Block::Heading { level: 2, content: vec![Inline::plain("Title")] },
            Block::Paragraph(vec![Inline::plain("Hello world.")]),
        ]);
        assert_eq!(md, "## Title\n\nHello world.\n");
    }

    #[test]
    fn escapes_syntax_chars() {
        let md = doc(vec![Block::Paragraph(vec![Inline::plain("2 * 3 = 6 [really] #tag")])]);
        assert_eq!(md, "2 \\* 3 = 6 \\[really\\] #tag\n");
    }

    #[test]
    fn escapes_line_start_only_chars() {
        let md = doc(vec![Block::Paragraph(vec![Inline::plain("- not a list")])]);
        assert_eq!(md, "\\- not a list\n");
        let md = doc(vec![Block::Paragraph(vec![Inline::plain("1. not a list")])]);
        assert_eq!(md, "1\\. not a list\n");
        let md = doc(vec![Block::Paragraph(vec![Inline::plain("take 2. then rest")])]);
        assert_eq!(md, "take 2. then rest\n");
    }

    #[test]
    fn bold_trailing_space_moved_out() {
        let md = doc(vec![Block::Paragraph(vec![styled("bold ", BOLD), Inline::plain("plain")])]);
        assert_eq!(md, "**bold** plain\n");
    }

    #[test]
    fn adjacent_same_style_runs_merged() {
        let md = doc(vec![Block::Paragraph(vec![styled("bo", BOLD), styled("ld", BOLD)])]);
        assert_eq!(md, "**bold**\n");
    }

    #[test]
    fn bold_italic_combo() {
        let md = doc(vec![Block::Paragraph(vec![styled(
            "both",
            Style { bold: true, italic: true, strike: false, code: false },
        )])]);
        assert_eq!(md, "***both***\n");
    }

    #[test]
    fn whitespace_only_run_loses_styling() {
        let md = doc(vec![Block::Paragraph(vec![
            styled("a", BOLD),
            styled(" ", ITALIC),
            styled("b", BOLD),
        ])]);
        assert_eq!(md, "**a b**\n");
    }

    #[test]
    fn link_render() {
        let md = doc(vec![Block::Paragraph(vec![Inline::Link {
            content: vec![Inline::plain("site")],
            url: "https://example.com/a(b)".into(),
        }])]);
        assert_eq!(md, "[site](<https://example.com/a(b)>)\n");
    }

    #[test]
    fn table_basic() {
        let md = doc(vec![Block::Table(Table {
            rows: vec![
                vec![
                    Cell::from_inlines(vec![Inline::plain("Name")]),
                    Cell::from_inlines(vec![Inline::plain("Age")]),
                ],
                vec![
                    Cell::from_inlines(vec![Inline::plain("Ann | Bob")]),
                    Cell::from_inlines(vec![Inline::plain("30")]),
                ],
            ],
            has_header: true,
        })]);
        assert_eq!(md, "| Name | Age |\n| --- | --- |\n| Ann \\| Bob | 30 |\n");
    }

    #[test]
    fn table_headerless_and_ragged() {
        let md = doc(vec![Block::Table(Table {
            rows: vec![
                vec![Cell::from_inlines(vec![Inline::plain("a")])],
                vec![
                    Cell::from_inlines(vec![Inline::plain("b")]),
                    Cell::from_inlines(vec![Inline::plain("c")]),
                ],
            ],
            has_header: false,
        })]);
        assert_eq!(md, "|  |  |\n| --- | --- |\n| a |  |\n| b | c |\n");
    }

    #[test]
    fn table_multiparagraph_cell() {
        let cell = Cell {
            blocks: vec![
                Block::Paragraph(vec![Inline::plain("one")]),
                Block::Paragraph(vec![Inline::plain("two")]),
            ],
        };
        let md = doc(vec![Block::Table(Table { rows: vec![vec![cell]], has_header: false })]);
        assert_eq!(md, "|  |\n| --- |\n| one<br>two |\n");
    }

    #[test]
    fn nested_list() {
        let md = doc(vec![Block::List(List {
            ordered: false,
            start: 1,
            items: vec![ListItem {
                blocks: vec![
                    Block::Paragraph(vec![Inline::plain("outer")]),
                    Block::List(List {
                        ordered: true,
                        start: 3,
                        items: vec![ListItem {
                            blocks: vec![Block::Paragraph(vec![Inline::plain("inner")])],
                            checked: None,
                        }],
                    }),
                ],
                checked: None,
            }],
        })]);
        assert_eq!(md, "- outer\n\n  3. inner\n");
    }

    #[test]
    fn code_span_with_backticks() {
        let md =
            doc(vec![Block::Paragraph(vec![styled("a`b", Style { code: true, ..Style::PLAIN })])]);
        assert_eq!(md, "``a`b``\n");
    }

    #[test]
    fn hard_break() {
        let md = doc(vec![Block::Paragraph(vec![
            Inline::plain("line one"),
            Inline::LineBreak,
            Inline::plain("line two"),
        ])]);
        assert_eq!(md, "line one\\\nline two\n");
    }

    #[test]
    fn line_start_escape_after_hard_break() {
        let md = doc(vec![Block::Paragraph(vec![
            Inline::plain("intro"),
            Inline::LineBreak,
            Inline::plain("- dash"),
        ])]);
        assert_eq!(md, "intro\\\n\\- dash\n");
    }

    #[test]
    fn blockquote() {
        let md =
            doc(vec![Block::BlockQuote(vec![Block::Paragraph(vec![Inline::plain("quoted")])])]);
        assert_eq!(md, "> quoted\n");
    }

    #[test]
    fn empty_paragraphs_dropped() {
        let md = doc(vec![
            Block::Paragraph(vec![Inline::plain("  ")]),
            Block::Paragraph(vec![]),
            Block::Paragraph(vec![Inline::plain("real")]),
        ]);
        assert_eq!(md, "real\n");
    }

    #[test]
    fn entity_escaped_but_plain_ampersand_kept() {
        let md = doc(vec![Block::Paragraph(vec![Inline::plain("A & B &amp; C")])]);
        assert_eq!(md, "A & B &amp;amp; C\n");
    }

    #[test]
    fn footnotes() {
        let md = document_to_markdown(&Document {
            blocks: vec![Block::Paragraph(vec![
                Inline::plain("Claim."),
                Inline::NoteRef("b".into()),
                Inline::plain(" More."),
                Inline::NoteRef("a".into()),
            ])],
            notes: vec![
                Note {
                    id: "a".into(),
                    blocks: vec![Block::Paragraph(vec![Inline::plain("Second note.")])],
                },
                Note {
                    id: "b".into(),
                    blocks: vec![
                        Block::Paragraph(vec![Inline::plain("First note.")]),
                        Block::Paragraph(vec![Inline::plain("With a second paragraph.")]),
                    ],
                },
            ],
        });
        assert_eq!(
            md,
            "Claim.[^1] More.[^2]\n\n[^1]: First note.\n\n    With a second paragraph.\n\n[^2]: Second note.\n"
        );
    }

    #[test]
    fn empty_and_unreferenced_notes() {
        let md = document_to_markdown(&Document {
            blocks: vec![Block::Paragraph(vec![
                Inline::plain("Text"),
                Inline::NoteRef("empty".into()),
            ])],
            notes: vec![
                Note { id: "empty".into(), blocks: vec![Block::Paragraph(vec![])] },
                Note {
                    id: "orphan".into(),
                    blocks: vec![Block::Paragraph(vec![Inline::plain("Kept.")])],
                },
            ],
        });
        assert_eq!(md, "Text\n\n[^1]: Kept.\n");
    }

    #[test]
    fn task_list() {
        let md = doc(vec![Block::List(List {
            ordered: false,
            start: 1,
            items: vec![
                ListItem {
                    blocks: vec![Block::Paragraph(vec![Inline::plain("done")])],
                    checked: Some(true),
                },
                ListItem {
                    blocks: vec![Block::Paragraph(vec![Inline::plain("todo")])],
                    checked: Some(false),
                },
            ],
        })]);
        assert_eq!(md, "- [x] done\n- [ ] todo\n");
    }
}

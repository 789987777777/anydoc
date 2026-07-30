//! OOXML WordprocessingML (.docx / .docm).

use crate::ir::*;
use crate::support::fields::hyperlink_from_instr;
use crate::support::list::{ListEntry, flush_list};
use crate::support::text::clean_text;
use crate::support::xml::{Element, parse_xml};
use crate::support::zip::read_zip_string;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::Cursor;

#[derive(Default)]
struct Ctx {
    rels: HashMap<String, String>,
    numbering: HashMap<String, Vec<LevelDef>>,
    heading_styles: HashMap<String, u8>,
}

#[derive(Clone, Copy)]
struct LevelDef {
    ordered: bool,
    start: u64,
}

pub fn parse(bytes: &[u8]) -> Result<Document> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;

    let mut numbering = HashMap::new();
    let mut heading_styles = HashMap::new();
    if let Ok(s) = read_zip_string(&mut zip, "word/numbering.xml")
        && let Ok(tree) = parse_xml(&s)
    {
        numbering = parse_numbering(&tree);
    }
    if let Ok(s) = read_zip_string(&mut zip, "word/styles.xml")
        && let Ok(tree) = parse_xml(&s)
    {
        heading_styles = parse_styles(&tree);
    }
    let part_ctx = |zip: &mut zip::ZipArchive<Cursor<&[u8]>>, rels_name: &str| Ctx {
        rels: read_zip_string(zip, rels_name)
            .ok()
            .and_then(|s| parse_xml(&s).ok())
            .map(|t| parse_rels(&t))
            .unwrap_or_default(),
        numbering: numbering.clone(),
        heading_styles: heading_styles.clone(),
    };

    let ctx = part_ctx(&mut zip, "word/_rels/document.xml.rels");
    let xml = read_zip_string(&mut zip, "word/document.xml")?;
    let tree = parse_xml(&xml)?;
    let body =
        tree.find("document").and_then(|d| d.find("body")).context("document.xml has no body")?;
    let blocks = parse_blocks(body, &ctx);

    let mut notes = Vec::new();
    for (part, root_name, elem_name, prefix) in [
        ("word/footnotes.xml", "footnotes", "footnote", "fn"),
        ("word/endnotes.xml", "endnotes", "endnote", "en"),
    ] {
        let Ok(s) = read_zip_string(&mut zip, part) else {
            continue;
        };
        let Ok(tree) = parse_xml(&s) else { continue };
        let Some(root) = tree.find(root_name) else {
            continue;
        };
        let rels_name = format!("word/_rels/{}.rels", part.trim_start_matches("word/"));
        let note_ctx = part_ctx(&mut zip, &rels_name);
        for note in root.find_all(elem_name) {
            if matches!(
                note.attr("type"),
                Some("separator") | Some("continuationSeparator") | Some("continuationNotice")
            ) {
                continue;
            }
            let Some(id) = note.attr("id") else { continue };
            notes.push(Note { id: format!("{prefix}{id}"), blocks: parse_blocks(note, &note_ctx) });
        }
    }

    Ok(Document { blocks, notes })
}

fn parse_rels(tree: &Element) -> HashMap<String, String> {
    let mut rels = HashMap::new();
    for rel in tree.descendants("Relationship") {
        if let (Some(id), Some(target)) = (rel.attr("Id"), rel.attr("Target")) {
            rels.insert(id.to_string(), target.to_string());
        }
    }
    rels
}

fn parse_numbering(tree: &Element) -> HashMap<String, Vec<LevelDef>> {
    let root = match tree.find("numbering") {
        Some(r) => r,
        None => return HashMap::new(),
    };
    let mut abstracts: HashMap<&str, Vec<LevelDef>> = HashMap::new();
    for abs in root.find_all("abstractNum") {
        let Some(id) = abs.attr("abstractNumId") else {
            continue;
        };
        let mut levels = vec![LevelDef { ordered: false, start: 1 }; 9];
        for lvl in abs.find_all("lvl") {
            let ilvl: usize = lvl.attr("ilvl").and_then(|v| v.parse().ok()).unwrap_or(0);
            if ilvl >= levels.len() {
                continue;
            }
            let fmt = lvl.find("numFmt").and_then(|e| e.attr("val")).unwrap_or("bullet");
            let start = lvl
                .find("start")
                .and_then(|e| e.attr("val"))
                .and_then(|v| v.parse().ok())
                .unwrap_or(1);
            levels[ilvl] = LevelDef { ordered: !matches!(fmt, "bullet" | "none"), start };
        }
        abstracts.insert(id, levels);
    }
    let mut out = HashMap::new();
    for num in root.find_all("num") {
        let Some(num_id) = num.attr("numId") else {
            continue;
        };
        let Some(abs_id) = num.find("abstractNumId").and_then(|e| e.attr("val")) else {
            continue;
        };
        if let Some(levels) = abstracts.get(abs_id) {
            out.insert(num_id.to_string(), levels.clone());
        }
    }
    out
}

fn parse_styles(tree: &Element) -> HashMap<String, u8> {
    let mut map = HashMap::new();
    let Some(root) = tree.find("styles") else {
        return map;
    };
    for style in root.find_all("style") {
        let Some(id) = style.attr("styleId") else {
            continue;
        };
        let name =
            style.find("name").and_then(|e| e.attr("val")).unwrap_or("").to_ascii_lowercase();
        let level = if let Some(rest) = name.strip_prefix("heading ") {
            rest.trim().parse::<u8>().ok()
        } else if name == "title" {
            Some(1)
        } else {
            style
                .find("pPr")
                .and_then(|p| p.find("outlineLvl"))
                .and_then(|e| e.attr("val"))
                .and_then(|v| v.parse::<u8>().ok())
                .filter(|&l| l < 9)
                .map(|l| l + 1)
        };
        if let Some(level) = level {
            map.insert(id.to_string(), level);
        }
    }
    map
}

enum ParaKind {
    Heading(u8),
    ListItem { ilvl: usize, ordered: bool, start: u64 },
    Plain,
}

fn parse_blocks(parent: &Element, ctx: &Ctx) -> Vec<Block> {
    let mut blocks: Vec<Block> = Vec::new();
    let mut list_run: Vec<ListEntry> = Vec::new();
    collect_blocks(parent, ctx, &mut blocks, &mut list_run);
    flush_list(&mut blocks, &mut list_run);
    blocks
}

fn collect_blocks(
    parent: &Element,
    ctx: &Ctx,
    blocks: &mut Vec<Block>,
    list_run: &mut Vec<ListEntry>,
) {
    for child in parent.child_elems() {
        match child.name.as_str() {
            "p" => {
                let (kind, inlines) = parse_paragraph(child, ctx);
                match kind {
                    ParaKind::ListItem { ilvl, ordered, start } => {
                        list_run.push((ilvl, ordered, start, Block::Paragraph(inlines)));
                    }
                    ParaKind::Heading(level) => {
                        flush_list(blocks, list_run);
                        if !inlines_are_empty(&inlines) {
                            blocks.push(Block::Heading { level, content: inlines });
                        }
                    }
                    ParaKind::Plain => {
                        flush_list(blocks, list_run);
                        blocks.push(Block::Paragraph(inlines));
                    }
                }
            }
            "tbl" => {
                flush_list(blocks, list_run);
                blocks.extend(parse_table(child, ctx));
            }
            "sdt" => {
                if let Some(content) = child.find("sdtContent") {
                    collect_blocks(content, ctx, blocks, list_run);
                }
            }
            _ => {}
        }
    }
}

fn parse_paragraph(p: &Element, ctx: &Ctx) -> (ParaKind, Vec<Inline>) {
    let mut kind = ParaKind::Plain;
    if let Some(ppr) = p.find("pPr") {
        let style_level = ppr
            .find("pStyle")
            .and_then(|e| e.attr("val"))
            .and_then(|id| ctx.heading_styles.get(id).copied());
        let outline_level = ppr
            .find("outlineLvl")
            .and_then(|e| e.attr("val"))
            .and_then(|v| v.parse::<u8>().ok())
            .filter(|&l| l < 9)
            .map(|l| l + 1);
        if let Some(level) = style_level.or(outline_level) {
            kind = ParaKind::Heading(level);
        } else if let Some(numpr) = ppr.find("numPr") {
            let num_id = numpr.find("numId").and_then(|e| e.attr("val")).unwrap_or("");
            let ilvl: usize = numpr
                .find("ilvl")
                .and_then(|e| e.attr("val"))
                .and_then(|v| v.parse().ok())
                .unwrap_or(0);
            if num_id != "0" {
                let def = ctx
                    .numbering
                    .get(num_id)
                    .and_then(|levels| levels.get(ilvl).copied())
                    .unwrap_or(LevelDef { ordered: false, start: 1 });
                kind = ParaKind::ListItem { ilvl, ordered: def.ordered, start: def.start };
            }
        }
    }
    let mut walker = InlineWalker { ctx, out: Vec::new(), fields: Vec::new() };
    walker.walk(p);
    let inlines = walker.finish();
    (kind, inlines)
}

struct FieldFrame {
    instr: String,
    in_result: bool,
    inlines: Vec<Inline>,
}

struct InlineWalker<'a> {
    ctx: &'a Ctx,
    out: Vec<Inline>,
    fields: Vec<FieldFrame>,
}

impl<'a> InlineWalker<'a> {
    fn push(&mut self, inline: Inline) {
        match self.fields.last_mut() {
            Some(f) if f.in_result => f.inlines.push(inline),
            Some(_) => {}
            None => self.out.push(inline),
        }
    }

    fn walk(&mut self, elem: &Element) {
        for child in elem.child_elems() {
            match child.name.as_str() {
                "pPr" => {}
                "r" => self.walk_run(child),
                "hyperlink" => {
                    let url = child
                        .attr("id")
                        .and_then(|id| self.ctx.rels.get(id).cloned())
                        .or_else(|| child.attr("anchor").map(|a| format!("#{a}")))
                        .unwrap_or_default();
                    let mut inner =
                        InlineWalker { ctx: self.ctx, out: Vec::new(), fields: Vec::new() };
                    inner.walk(child);
                    let content = inner.finish();
                    if !content.is_empty() {
                        self.push(Inline::Link { content, url });
                    }
                }
                "fldSimple" => {
                    let instr = child.attr("instr").unwrap_or("").to_string();
                    let mut inner =
                        InlineWalker { ctx: self.ctx, out: Vec::new(), fields: Vec::new() };
                    inner.walk(child);
                    let content = inner.finish();
                    self.push_field_result(&instr, content);
                }
                "sdt" => {
                    if let Some(content) = child.find("sdtContent") {
                        self.walk(content);
                    }
                }
                "smartTag" | "ins" | "bdo" | "dir" => self.walk(child),
                _ => {}
            }
        }
    }

    fn walk_run(&mut self, run: &Element) {
        let style = run.find("rPr").map(run_style).unwrap_or_default();
        for child in run.child_elems() {
            match child.name.as_str() {
                "t" => {
                    let text = clean_text(&child.text());
                    if !text.is_empty() {
                        self.push(Inline::Text { text, style });
                    }
                }
                "tab" | "ptab" => self.push(Inline::Text { text: " ".into(), style: Style::PLAIN }),
                "br" => {
                    if child.attr("type") != Some("page") {
                        self.push(Inline::LineBreak);
                    }
                }
                "cr" => self.push(Inline::LineBreak),
                "footnoteReference" => {
                    if let Some(id) = child.attr("id") {
                        self.push(Inline::NoteRef(format!("fn{id}")));
                    }
                }
                "endnoteReference" => {
                    if let Some(id) = child.attr("id") {
                        self.push(Inline::NoteRef(format!("en{id}")));
                    }
                }
                "drawing" => {
                    let doc_pr = child.descendants("docPr").into_iter().next();
                    let descr = doc_pr.and_then(|d| d.attr("descr")).unwrap_or("");
                    let name = doc_pr.and_then(|d| d.attr("name")).unwrap_or("");
                    let alt = if !descr.trim().is_empty() {
                        descr
                    } else if !is_generic_image_name(name) {
                        name
                    } else {
                        ""
                    };
                    if !alt.trim().is_empty() {
                        self.push(Inline::Image { alt: clean_text(alt), url: None });
                    }
                }
                "fldChar" => match child.attr("fldCharType") {
                    Some("begin") => self.fields.push(FieldFrame {
                        instr: String::new(),
                        in_result: false,
                        inlines: Vec::new(),
                    }),
                    Some("separate") => {
                        if let Some(f) = self.fields.last_mut() {
                            f.in_result = true;
                        }
                    }
                    Some("end") => {
                        if let Some(frame) = self.fields.pop() {
                            let instr = frame.instr.clone();
                            self.push_field_result(&instr, frame.inlines);
                        }
                    }
                    _ => {}
                },
                "instrText" => {
                    if let Some(f) = self.fields.last_mut() {
                        f.instr.push_str(&child.text());
                    }
                }
                _ => {}
            }
        }
    }

    fn push_field_result(&mut self, instr: &str, content: Vec<Inline>) {
        let url = hyperlink_from_instr(instr);
        match url {
            Some(url) if !inlines_are_empty(&content) => self.push(Inline::Link { content, url }),
            _ => {
                for inline in content {
                    self.push(inline);
                }
            }
        }
    }

    fn finish(mut self) -> Vec<Inline> {
        while let Some(frame) = self.fields.pop() {
            for inline in frame.inlines {
                match self.fields.last_mut() {
                    Some(f) if f.in_result => f.inlines.push(inline),
                    Some(_) => {}
                    None => self.out.push(inline),
                }
            }
        }
        self.out
    }
}

/// Auto-generated names like "Picture 3" or "Graphic 1" carry no information.
fn is_generic_image_name(name: &str) -> bool {
    let mut words = name.split_whitespace();
    let Some(first) = words.next() else {
        return true;
    };
    let generic = matches!(
        first.to_ascii_lowercase().as_str(),
        "picture"
            | "image"
            | "graphic"
            | "grafik"
            | "bild"
            | "imagen"
            | "immagine"
            | "obraz"
            | "chart"
            | "diagram"
            | "shape"
            | "textbox"
            | "object"
    );
    generic && words.all(|w| w.chars().all(|c| c.is_ascii_digit()))
}

fn run_style(rpr: &Element) -> Style {
    Style {
        bold: on_off(rpr, "b"),
        italic: on_off(rpr, "i"),
        strike: on_off(rpr, "strike") || on_off(rpr, "dstrike"),
        code: false,
    }
}

fn on_off(parent: &Element, name: &str) -> bool {
    match parent.find(name) {
        Some(e) => !matches!(e.attr("val"), Some("false") | Some("0") | Some("none")),
        None => false,
    }
}

fn parse_table(tbl: &Element, ctx: &Ctx) -> Vec<Block> {
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    let mut has_header = false;
    for (i, tr) in tbl.find_all("tr").enumerate() {
        if i == 0 {
            has_header = tr.find("trPr").and_then(|p| p.find("tblHeader")).is_some();
        }
        let mut cells: Vec<Cell> = Vec::new();
        collect_row_cells(tr, ctx, &mut cells);
        rows.push(cells);
    }
    if rows.is_empty() {
        return Vec::new();
    }
    if rows.len() == 1 && rows[0].len() == 1 {
        return std::mem::take(&mut rows[0][0].blocks);
    }
    vec![Block::Table(Table { rows, has_header })]
}

fn collect_row_cells(parent: &Element, ctx: &Ctx, cells: &mut Vec<Cell>) {
    for child in parent.child_elems() {
        match child.name.as_str() {
            "tc" => {
                let tcpr = child.find("tcPr");
                let vmerge_cont = tcpr
                    .and_then(|p| p.find("vMerge"))
                    .is_some_and(|v| !matches!(v.attr("val"), Some("restart")));
                let span: usize = tcpr
                    .and_then(|p| p.find("gridSpan"))
                    .and_then(|e| e.attr("val"))
                    .and_then(|v| v.parse().ok())
                    .unwrap_or(1);
                if vmerge_cont {
                    cells.push(Cell::default());
                } else {
                    cells.push(Cell { blocks: parse_blocks(child, ctx) });
                }
                for _ in 1..span.max(1) {
                    cells.push(Cell::default());
                }
            }
            "sdt" => {
                if let Some(content) = child.find("sdtContent") {
                    collect_row_cells(content, ctx, cells);
                }
            }
            _ => {}
        }
    }
}

//! OOXML PresentationML (.pptx / .pptm / .ppsx): slides in sldIdLst order,
//! title placeholders as headings.

use crate::ir::*;
use crate::support::list::{ListEntry, flush_list};
use crate::support::text::clean_text;
use crate::support::xml::{Element, parse_xml};
use crate::support::zip::{read_rels, read_zip_string, resolve_path};
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::Cursor;

type Rels = HashMap<String, String>;

pub fn parse(bytes: &[u8]) -> Result<Document> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;
    let pres = parse_xml(&read_zip_string(&mut zip, "ppt/presentation.xml")?)?;
    let rels = read_rels(&mut zip, "ppt/_rels/presentation.xml.rels");

    let slide_paths: Vec<String> = pres
        .first_descendant("sldIdLst")
        .context("presentation.xml has no slide list")?
        .find_all("sldId")
        .filter_map(|s| s.attr("r:id"))
        .filter_map(|rid| rels.get(rid))
        .map(|target| resolve_path("ppt/", target))
        .collect();

    let mut blocks = Vec::new();
    for path in slide_paths {
        let Ok(xml) = read_zip_string(&mut zip, &path) else {
            continue;
        };
        let Ok(tree) = parse_xml(&xml) else { continue };
        let Some(sp_tree) =
            tree.find("sld").and_then(|s| s.find("cSld")).and_then(|c| c.find("spTree"))
        else {
            continue;
        };
        let rels_name = match path.rsplit_once('/') {
            Some((dir, file)) => format!("{dir}/_rels/{file}.rels"),
            None => continue,
        };
        let slide_rels = read_rels(&mut zip, &rels_name);
        // The title placeholder is the slide's heading no matter where it
        // sits in the shape tree; emit it first.
        let mut title = Vec::new();
        let mut body = Vec::new();
        parse_shapes(sp_tree, &slide_rels, &mut title, &mut body);
        blocks.append(&mut title);
        blocks.append(&mut body);
    }
    Ok(Document { blocks, notes: Vec::new() })
}

fn parse_shapes(parent: &Element, rels: &Rels, title: &mut Vec<Block>, blocks: &mut Vec<Block>) {
    for child in parent.child_elems() {
        match child.name.as_str() {
            "sp" => parse_shape(child, rels, title, blocks),
            "grpSp" => parse_shapes(child, rels, title, blocks),
            "graphicFrame" => {
                if let Some(tbl) = child.first_descendant("tbl") {
                    parse_table(tbl, rels, blocks);
                }
            }
            "pic" => {
                let descr =
                    child.first_descendant("cNvPr").and_then(|p| p.attr("descr")).unwrap_or("");
                if !descr.trim().is_empty() {
                    blocks.push(Block::Paragraph(vec![Inline::Image {
                        alt: clean_text(descr),
                        url: None,
                    }]));
                }
            }
            _ => {}
        }
    }
}

fn parse_shape(sp: &Element, rels: &Rels, title: &mut Vec<Block>, blocks: &mut Vec<Block>) {
    let ph_type = sp
        .find("nvSpPr")
        .and_then(|nv| nv.find("nvPr"))
        .and_then(|nv| nv.find("ph"))
        .and_then(|ph| ph.attr("type"))
        .unwrap_or("");
    if matches!(ph_type, "sldNum" | "dt" | "ftr") {
        return;
    }
    let Some(tx) = sp.find("txBody") else {
        return;
    };
    if matches!(ph_type, "title" | "ctrTitle") {
        push_title_heading(tx, rels, title);
    } else {
        parse_text_body(tx, rels, blocks);
    }
}

/// Collapse a title placeholder's paragraphs into one slide heading.
fn push_title_heading(tx: &Element, rels: &Rels, blocks: &mut Vec<Block>) {
    let mut inlines: Vec<Inline> = Vec::new();
    for p in tx.find_all("p") {
        let para = parse_para_inlines(p, rels);
        if inlines_are_empty(&para) {
            continue;
        }
        if !inlines.is_empty() {
            inlines.push(Inline::LineBreak);
        }
        inlines.extend(para);
    }
    if !inlines_are_empty(&inlines) {
        blocks.push(Block::Heading { level: 2, content: inlines });
    }
}

fn parse_text_body(tx: &Element, rels: &Rels, blocks: &mut Vec<Block>) {
    let mut list_run: Vec<ListEntry> = Vec::new();
    for p in tx.find_all("p") {
        let inlines = parse_para_inlines(p, rels);
        if inlines_are_empty(&inlines) {
            flush_list(blocks, &mut list_run);
            continue;
        }
        let ppr = p.find("pPr");
        let level: usize =
            ppr.and_then(|pr| pr.attr("lvl")).and_then(|v| v.parse().ok()).unwrap_or(0);
        let bullet = ppr.and_then(|pr| pr.find("buChar")).is_some();
        let autonum = ppr.and_then(|pr| pr.find("buAutoNum"));
        if bullet || autonum.is_some() {
            let start =
                autonum.and_then(|b| b.attr("startAt")).and_then(|v| v.parse().ok()).unwrap_or(1);
            list_run.push(ListEntry {
                level,
                ordered: autonum.is_some(),
                start,
                block: Block::Paragraph(inlines),
            });
        } else {
            flush_list(blocks, &mut list_run);
            blocks.push(Block::Paragraph(inlines));
        }
    }
    flush_list(blocks, &mut list_run);
}

fn parse_para_inlines(p: &Element, rels: &Rels) -> Vec<Inline> {
    let mut out: Vec<Inline> = Vec::new();
    for child in p.child_elems() {
        match child.name.as_str() {
            "r" => {
                let rpr = child.find("rPr");
                let text = clean_text(&child.find("t").map(|t| t.text()).unwrap_or_default());
                if text.is_empty() {
                    continue;
                }
                let inline = Inline::Text { text, style: run_style(rpr) };
                let url = rpr
                    .and_then(|r| r.find("hlinkClick"))
                    .and_then(|h| h.attr("r:id"))
                    .and_then(|rid| rels.get(rid));
                match url {
                    Some(url) => out.push(Inline::Link { content: vec![inline], url: url.clone() }),
                    None => out.push(inline),
                }
            }
            "br" => out.push(Inline::LineBreak),
            _ => {}
        }
    }
    out
}

fn run_style(rpr: Option<&Element>) -> Style {
    let mut style = Style::PLAIN;
    let Some(rpr) = rpr else {
        return style;
    };
    style.bold = matches!(rpr.attr("b"), Some("1") | Some("true"));
    style.italic = matches!(rpr.attr("i"), Some("1") | Some("true"));
    style.strike = matches!(rpr.attr("strike"), Some("sngStrike") | Some("dblStrike"));
    style
}

fn parse_table(tbl: &Element, rels: &Rels, blocks: &mut Vec<Block>) {
    let has_header =
        tbl.find("tblPr").is_some_and(|p| matches!(p.attr("firstRow"), Some("1") | Some("true")));
    let mut rows: Vec<Vec<Cell>> = Vec::new();
    for tr in tbl.find_all("tr") {
        let mut cells: Vec<Cell> = Vec::new();
        for tc in tr.find_all("tc") {
            let merged = matches!(tc.attr("hMerge"), Some("1") | Some("true"))
                || matches!(tc.attr("vMerge"), Some("1") | Some("true"));
            if merged {
                cells.push(Cell::default());
                continue;
            }
            let span: usize =
                tc.attr("gridSpan").and_then(|v| v.parse().ok()).unwrap_or(1).clamp(1, 100);
            let mut cell_blocks = Vec::new();
            if let Some(tx) = tc.find("txBody") {
                parse_text_body(tx, rels, &mut cell_blocks);
            }
            cells.push(Cell { blocks: cell_blocks });
            for _ in 1..span {
                cells.push(Cell::default());
            }
        }
        rows.push(cells);
    }
    if !rows.is_empty() {
        blocks.push(Block::Table(Table { rows, has_header }));
    }
}

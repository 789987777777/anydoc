//! EPUB: XHTML chapters in spine order.

use crate::ir::{Block, Document, Inline};
use crate::support::html;
use crate::support::xml::parse_xml;
use crate::support::zip::read_zip_string;
use anyhow::{Context, Result};
use std::collections::HashMap;
use std::io::Cursor;

pub fn parse(bytes: &[u8]) -> Result<Document> {
    let mut zip = zip::ZipArchive::new(Cursor::new(bytes))?;

    let container = read_zip_string(&mut zip, "META-INF/container.xml")?;
    let container = parse_xml(&container)?;
    let opf_path = container
        .descendants("rootfile")
        .first()
        .and_then(|r| r.attr("full-path"))
        .context("container.xml has no rootfile")?
        .to_string();
    let opf_dir = match opf_path.rsplit_once('/') {
        Some((dir, _)) => format!("{dir}/"),
        None => String::new(),
    };

    let opf = parse_xml(&read_zip_string(&mut zip, &opf_path)?)?;
    let mut doc = Document::default();

    if let Some(title) = opf.descendants("title").first().map(|t| t.text()) {
        let title = title.trim().to_string();
        if !title.is_empty() {
            doc.blocks.push(Block::Heading { level: 1, content: vec![Inline::plain(title)] });
        }
    }

    let mut manifest: HashMap<String, (String, String)> = HashMap::new();
    for item in opf.descendants("item") {
        if let (Some(id), Some(href)) = (item.attr("id"), item.attr("href")) {
            let media = item.attr("media-type").unwrap_or("").to_string();
            manifest.insert(id.to_string(), (href.to_string(), media));
        }
    }

    for itemref in opf.descendants("itemref") {
        let Some(idref) = itemref.attr("idref") else {
            continue;
        };
        let Some((href, media)) = manifest.get(idref) else {
            continue;
        };
        if !media.contains("xhtml") && !media.contains("html") {
            continue;
        }
        let full = resolve_path(&opf_dir, &percent_decode(href));
        let Ok(xml) = read_zip_string(&mut zip, &full) else {
            continue;
        };
        let Ok(tree) = parse_xml(&xml) else { continue };
        if let Some(body) = tree.find("html").and_then(|h| h.find("body")) {
            doc.blocks.extend(html::to_blocks(body));
        }
    }
    Ok(doc)
}

fn resolve_path(base_dir: &str, href: &str) -> String {
    let href = href.split('#').next().unwrap_or(href);
    let mut parts: Vec<&str> = base_dir.split('/').filter(|s| !s.is_empty()).collect();
    for seg in href.split('/') {
        match seg {
            "." | "" => {}
            ".." => {
                parts.pop();
            }
            s => parts.push(s),
        }
    }
    parts.join("/")
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && let Some(hex) = s.get(i + 1..i + 3)
            && let Ok(b) = u8::from_str_radix(hex, 16)
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

//! Zip archive helpers.

use crate::support::xml::parse_xml;
use anyhow::Result;
use std::collections::HashMap;
use std::io::{Read, Seek};

/// Read a file out of a zip archive as a UTF-8 string (BOM stripped).
pub fn read_zip_string<R: Read + Seek>(zip: &mut zip::ZipArchive<R>, name: &str) -> Result<String> {
    let mut file = zip.by_name(name)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let mut s = match String::from_utf8(bytes) {
        Ok(s) => s,
        Err(e) => String::from_utf8_lossy(e.as_bytes()).into_owned(),
    };
    if s.starts_with('\u{feff}') {
        s.drain(..'\u{feff}'.len_utf8());
    }
    Ok(s)
}

/// Read an OPC relationships part into an Id -> Target map; empty if absent.
pub fn read_rels<R: Read + Seek>(
    zip: &mut zip::ZipArchive<R>,
    name: &str,
) -> HashMap<String, String> {
    let mut rels = HashMap::new();
    if let Ok(s) = read_zip_string(zip, name)
        && let Ok(tree) = parse_xml(&s)
    {
        for rel in tree.descendants("Relationship") {
            if let (Some(id), Some(target)) = (rel.attr("Id"), rel.attr("Target")) {
                rels.insert(id.to_string(), target.to_string());
            }
        }
    }
    rels
}

/// Join a relative href onto a base directory inside the archive.
pub fn resolve_path(base_dir: &str, href: &str) -> String {
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

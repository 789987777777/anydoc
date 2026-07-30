//! Zip archive helpers.

use anyhow::Result;
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

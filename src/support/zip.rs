//! Zip archive helpers.

use anyhow::Result;
use std::io::{Read, Seek};

/// Read a file out of a zip archive as a UTF-8 string (BOM stripped).
pub fn read_zip_string<R: Read + Seek>(zip: &mut zip::ZipArchive<R>, name: &str) -> Result<String> {
    let mut file = zip.by_name(name)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let s = String::from_utf8_lossy(&bytes);
    Ok(s.strip_prefix('\u{feff}').unwrap_or(&s).to_string())
}

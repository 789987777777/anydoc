//! Zip archive helpers.

use anyhow::Result;

/// Read a file out of a zip archive as a UTF-8 string (BOM stripped).
pub fn read_zip_string<R: std::io::Read + std::io::Seek>(
    zip: &mut ::zip::ZipArchive<R>,
    name: &str,
) -> Result<String> {
    use std::io::Read;
    let mut file = zip.by_name(name)?;
    let mut bytes = Vec::new();
    file.read_to_end(&mut bytes)?;
    let s = String::from_utf8_lossy(&bytes);
    Ok(s.strip_prefix('\u{feff}').unwrap_or(&s).to_string())
}

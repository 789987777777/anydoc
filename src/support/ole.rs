//! OLE2 compound file helpers.

use anyhow::Result;
use std::io::{Read, Seek};

/// Read a named stream out of a compound file.
pub fn read_stream<R: Read + Seek>(ole: &mut cfb::CompoundFile<R>, name: &str) -> Result<Vec<u8>> {
    let mut stream = ole.open_stream(format!("/{name}"))?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    Ok(buf)
}

//! Legacy PowerPoint 97-2003 binary (.ppt): OLE2 container, record stream.
//! Slides resolve through the persist directory; text comes from
//! TextHeaderAtom + TextCharsAtom/TextBytesAtom records.

use crate::ir::*;
use crate::support::ole::read_stream;
use crate::support::text::clean_text;
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::io::Cursor;

pub fn parse(bytes: &[u8]) -> Result<Document> {
    let cursor = Cursor::new(bytes);
    let mut ole = cfb::CompoundFile::open(cursor).context("not an OLE2 compound file")?;
    let data =
        read_stream(&mut ole, "PowerPoint Document").context("no PowerPoint Document stream")?;
    let current_user = read_stream(&mut ole, "Current User").unwrap_or_default();
    if get_u32(&current_user, 12) == Some(0xF3D1_C4DF) {
        bail!("document is encrypted");
    }

    let mut ex = Extractor { blocks: Vec::new(), title: false, encrypted: false };
    if ex.parse_slides(&data, &current_user).is_none() {
        // No usable persist directory: take text in raw stream order.
        ex.blocks.clear();
        ex.walk(&data);
    }
    if ex.encrypted {
        bail!("document is encrypted");
    }
    Ok(Document { blocks: ex.blocks, notes: Vec::new() })
}

fn get_u32(b: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(off..off + 4)?.try_into().ok()?))
}

/// (verAndInstance, recType, body) of the record at `off`.
fn record_at(data: &[u8], off: usize) -> Option<(u16, u16, &[u8])> {
    let hdr = data.get(off..off + 8)?;
    let ver_inst = u16::from_le_bytes([hdr[0], hdr[1]]);
    let rec_type = u16::from_le_bytes([hdr[2], hdr[3]]);
    let len = u32::from_le_bytes([hdr[4], hdr[5], hdr[6], hdr[7]]) as usize;
    let body = data.get(off + 8..(off + 8).checked_add(len)?)?;
    Some((ver_inst, rec_type, body))
}

struct Extractor {
    blocks: Vec<Block>,
    title: bool,
    encrypted: bool,
}

impl Extractor {
    /// Walk slides in presentation order: the UserEditAtom chain yields the
    /// persist directory, the DocumentContainer's SlideListWithText yields
    /// slide order and outline text, each slide container its own textboxes.
    fn parse_slides(&mut self, data: &[u8], current_user: &[u8]) -> Option<()> {
        let mut persist: HashMap<u32, usize> = HashMap::new();
        let mut doc_persist: Option<u32> = None;
        let mut edit_off = get_u32(current_user, 16)? as usize;
        for _ in 0..100 {
            if edit_off == 0 {
                break;
            }
            let (_, rec_type, body) = record_at(data, edit_off)?;
            if rec_type != 0x0FF5 {
                return None;
            }
            if doc_persist.is_none() {
                doc_persist = get_u32(body, 16);
            }
            let dir_off = get_u32(body, 12)? as usize;
            if let Some((_, 0x1772, dir)) = record_at(data, dir_off) {
                let mut pos = 0;
                while pos + 4 <= dir.len() {
                    let head = get_u32(dir, pos)?;
                    let id = head & 0xF_FFFF;
                    let count = (head >> 20) as usize;
                    pos += 4;
                    for k in 0..count {
                        // Newer edits win: keep the first offset seen.
                        persist.entry(id + k as u32).or_insert(get_u32(dir, pos)? as usize);
                        pos += 4;
                    }
                }
            }
            let prev = get_u32(body, 8)? as usize;
            if prev == edit_off {
                break;
            }
            edit_off = prev;
        }

        let doc_off = *persist.get(&doc_persist?)?;
        let Some((_, 0x03E8, doc)) = record_at(data, doc_off) else {
            return None;
        };
        let (.., slide_list) = children(doc)
            .find(|&(ver_inst, rec_type, _)| rec_type == 0x0FF0 && ver_inst >> 4 == 0)?;

        let mut pending_slide: Option<u32> = None;
        for (ver_inst, rec_type, body) in children(slide_list) {
            match rec_type {
                // SlidePersistAtom: the next slide begins.
                0x03F3 => {
                    self.finish_slide(pending_slide.take(), &persist, data);
                    pending_slide = get_u32(body, 0);
                    self.title = false;
                }
                _ => self.record(ver_inst, rec_type, body),
            }
        }
        self.finish_slide(pending_slide, &persist, data);
        Some(())
    }

    /// Emit a slide's own textboxes after its outline text.
    fn finish_slide(&mut self, slide: Option<u32>, persist: &HashMap<u32, usize>, data: &[u8]) {
        let Some(off) = slide.and_then(|id| persist.get(&id)) else {
            return;
        };
        if let Some((_, 0x03EE, body)) = record_at(data, *off) {
            self.title = false;
            self.walk(body);
        }
    }

    fn walk(&mut self, data: &[u8]) {
        for (ver_inst, rec_type, body) in children(data) {
            self.record(ver_inst, rec_type, body);
        }
    }

    fn record(&mut self, ver_inst: u16, rec_type: u16, body: &[u8]) {
        if ver_inst & 0xF == 0xF {
            match rec_type {
                // CryptSession10Container: the stream is encrypted.
                0x2F14 => self.encrypted = true,
                // Notes, master, and handout content is not slide text.
                0x03F0 | 0x03F8 | 0x0FC9 => {}
                // Only instance 0 of SlideListWithText holds slide text.
                0x0FF0 if ver_inst >> 4 != 0 => {}
                _ => self.walk(body),
            }
        } else {
            self.atom(rec_type, body);
        }
    }

    fn atom(&mut self, rec_type: u16, body: &[u8]) {
        match rec_type {
            // TextHeaderAtom: text type 0 is title, 6 is centered title.
            0x0F9F => self.title = matches!(body.first(), Some(0) | Some(6)),
            // TextCharsAtom: UTF-16LE.
            0x0FA0 => {
                let units = body.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]]));
                let text: String =
                    char::decode_utf16(units).map(|r| r.unwrap_or('\u{fffd}')).collect();
                self.push_text(&text);
            }
            // TextBytesAtom: low bytes of UTF-16 code units.
            0x0FA8 => {
                let text: String = body.iter().map(|&b| b as char).collect();
                self.push_text(&text);
            }
            _ => {}
        }
    }

    /// One text atom holds a whole shape's text: paragraphs split on CR,
    /// soft line breaks on VT.
    fn push_text(&mut self, text: &str) {
        for para in text.split('\r') {
            let mut inlines: Vec<Inline> = Vec::new();
            for (i, seg) in para.split('\u{b}').enumerate() {
                if i > 0 {
                    inlines.push(Inline::LineBreak);
                }
                let seg = clean_text(seg);
                if !seg.trim().is_empty() {
                    inlines.push(Inline::plain(seg));
                }
            }
            if inlines_are_empty(&inlines) {
                continue;
            }
            if self.title {
                self.blocks.push(Block::Heading { level: 2, content: inlines });
            } else {
                self.blocks.push(Block::Paragraph(inlines));
            }
        }
    }
}

/// Iterate the records laid out back to back in `data`.
fn children(data: &[u8]) -> impl Iterator<Item = (u16, u16, &[u8])> {
    let mut pos = 0;
    std::iter::from_fn(move || {
        let (ver_inst, rec_type, body) = record_at(data, pos)?;
        pos += 8 + body.len();
        Some((ver_inst, rec_type, body))
    })
}

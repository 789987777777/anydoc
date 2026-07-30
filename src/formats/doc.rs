//! Legacy Word 97-2003 binary (.doc): OLE2 container, FIB, piece table,
//! CHPX/PAPX formatting runs, STSH style sheet.

use crate::ir::*;
use crate::support::fields::{FieldFrame, field_result};
use crate::support::list::{ListEntry, flush_list};
use anyhow::{Context, Result, bail};
use std::collections::HashMap;
use std::io::{Cursor, Read, Seek};

pub fn parse(bytes: &[u8]) -> Result<Document> {
    if bytes.starts_with(b"{\\rtf") {
        return super::rtf::parse(bytes);
    }
    let cursor = Cursor::new(bytes);
    let mut ole = cfb::CompoundFile::open(cursor).context("not an OLE2 compound file")?;

    let word_doc = read_stream(&mut ole, "WordDocument")?;
    if get_u16(&word_doc, 0) != Some(0xA5EC) {
        bail!("WordDocument stream has invalid FIB magic");
    }
    let flags = get_u16(&word_doc, 0x0A).unwrap_or(0);
    if flags & 0x0100 != 0 {
        bail!("document is encrypted");
    }
    let table_name = if flags & 0x0200 != 0 { "1Table" } else { "0Table" };
    let table = read_stream(&mut ole, table_name)
        .or_else(|_| read_stream(&mut ole, "0Table"))
        .or_else(|_| read_stream(&mut ole, "1Table"))
        .unwrap_or_default();

    let ccp_text = get_u32(&word_doc, 0x4C).unwrap_or(0) as usize;
    let ccp_ftn = get_u32(&word_doc, 0x50).unwrap_or(0) as usize;
    let ccp_hdd = get_u32(&word_doc, 0x54).unwrap_or(0) as usize;
    let ccp_mcr = get_u32(&word_doc, 0x58).unwrap_or(0) as usize;
    let ccp_atn = get_u32(&word_doc, 0x5C).unwrap_or(0) as usize;
    let ccp_edn = get_u32(&word_doc, 0x60).unwrap_or(0) as usize;
    let fc_clx = get_u32(&word_doc, 0x1A2).unwrap_or(0) as usize;
    let lcb_clx = get_u32(&word_doc, 0x1A6).unwrap_or(0) as usize;

    let pieces = if lcb_clx > 0 {
        parse_clx(&table, fc_clx, lcb_clx)?
    } else {
        legacy_single_piece(&word_doc)
    };
    let total_cp = ccp_text + ccp_ftn + ccp_hdd + ccp_mcr + ccp_atn + ccp_edn;
    let (chars, fcs) = extract_text(&word_doc, &pieces, total_cp);

    let data = read_stream(&mut ole, "Data").unwrap_or_default();
    let chpx_runs = parse_fkps(&word_doc, &table, &data, 0xFA, FkpKind::Chpx);
    let papx_runs = parse_fkps(&word_doc, &table, &data, 0x102, FkpKind::Papx);
    let heading_styles = parse_stsh(&word_doc, &table);

    let mut note_refs: HashMap<usize, String> = HashMap::new();
    let mut note_ranges: Vec<(usize, usize, String)> = Vec::new();
    let ftn_base = ccp_text;
    let edn_base = ccp_text + ccp_ftn + ccp_hdd + ccp_mcr + ccp_atn;
    for (ref_off, txt_off, base, prefix) in
        [(0xAA, 0xB2, ftn_base, "fn"), (0x20A, 0x212, edn_base, "en")]
    {
        let (ref_cps, n_refs) = parse_plc(&word_doc, &table, ref_off, 2);
        let (txt_cps, _) = parse_plc(&word_doc, &table, txt_off, 0);
        for i in 0..n_refs {
            note_refs.insert(ref_cps[i] as usize, format!("{prefix}{i}"));
            if i + 1 < txt_cps.len() {
                note_ranges.push((
                    base + txt_cps[i] as usize,
                    base + txt_cps[i + 1] as usize,
                    format!("{prefix}{i}"),
                ));
            }
        }
    }

    let assembler = Assembler {
        chars,
        fcs,
        chpx: Runs::new(chpx_runs),
        papx: Runs::new(papx_runs),
        heading_styles,
        note_refs,
        note_ranges,
        ccp_text,
    };
    Ok(assembler.build())
}

/// Read a PLC's CP array; n is the number of data elements.
fn parse_plc(word_doc: &[u8], table: &[u8], fib_off: usize, data_size: usize) -> (Vec<u32>, usize) {
    let fc = get_u32(word_doc, fib_off).unwrap_or(0) as usize;
    let lcb = get_u32(word_doc, fib_off + 4).unwrap_or(0) as usize;
    let Some(plc) = table.get(fc..fc.saturating_add(lcb)) else {
        return (Vec::new(), 0);
    };
    if lcb < 8 {
        return (Vec::new(), 0);
    }
    let n = if data_size == 0 { lcb / 4 - 1 } else { (lcb - 4) / (4 + data_size) };
    let mut cps = Vec::with_capacity(n + 1);
    for i in 0..=n {
        cps.push(get_u32(plc, i * 4).unwrap_or(0));
    }
    (cps, n)
}

fn read_stream<R: Read + Seek>(ole: &mut cfb::CompoundFile<R>, name: &str) -> Result<Vec<u8>> {
    let mut stream = ole.open_stream(format!("/{name}"))?;
    let mut buf = Vec::new();
    stream.read_to_end(&mut buf)?;
    Ok(buf)
}

fn get_u16(b: &[u8], off: usize) -> Option<u16> {
    Some(u16::from_le_bytes(b.get(off..off + 2)?.try_into().ok()?))
}

fn get_u32(b: &[u8], off: usize) -> Option<u32> {
    Some(u32::from_le_bytes(b.get(off..off + 4)?.try_into().ok()?))
}

struct Piece {
    cp_start: usize,
    cp_end: usize,
    fc: usize,
    compressed: bool,
}

fn parse_clx(table: &[u8], fc: usize, lcb: usize) -> Result<Vec<Piece>> {
    let clx = table.get(fc..fc + lcb).context("Clx out of bounds")?;
    let mut pos = 0;
    loop {
        match clx.get(pos) {
            Some(1) => {
                let cb = get_u16(clx, pos + 1).context("bad Prc")? as usize;
                pos += 3 + cb;
            }
            Some(2) => {
                let lcb_plc = get_u32(clx, pos + 1).context("bad Pcdt")? as usize;
                let plc = clx.get(pos + 5..pos + 5 + lcb_plc).context("PlcPcd out of bounds")?;
                return parse_plc_pcd(plc);
            }
            _ => bail!("malformed Clx"),
        }
    }
}

fn parse_plc_pcd(plc: &[u8]) -> Result<Vec<Piece>> {
    if plc.len() < 4 + 12 {
        bail!("empty piece table");
    }
    let n = (plc.len() - 4) / 12;
    let mut pieces = Vec::with_capacity(n);
    for i in 0..n {
        let cp_start = get_u32(plc, i * 4).context("bad cp")? as usize;
        let cp_end = get_u32(plc, (i + 1) * 4).context("bad cp")? as usize;
        let pcd_off = (n + 1) * 4 + i * 8;
        let fc_raw = get_u32(plc, pcd_off + 2).context("bad pcd")?;
        let compressed = fc_raw & 0x4000_0000 != 0;
        let fc = (fc_raw & 0x3FFF_FFFF) as usize;
        let fc = if compressed { fc / 2 } else { fc };
        pieces.push(Piece { cp_start, cp_end, fc, compressed });
    }
    Ok(pieces)
}

fn legacy_single_piece(word_doc: &[u8]) -> Vec<Piece> {
    let fc_min = get_u32(word_doc, 0x18).unwrap_or(0) as usize;
    let fc_mac = get_u32(word_doc, 0x1C).unwrap_or(0) as usize;
    if fc_mac <= fc_min {
        return Vec::new();
    }
    vec![Piece { cp_start: 0, cp_end: fc_mac - fc_min, fc: fc_min, compressed: true }]
}

fn extract_text(word_doc: &[u8], pieces: &[Piece], ccp_text: usize) -> (Vec<char>, Vec<u32>) {
    let mut chars = Vec::new();
    let mut fcs = Vec::new();
    for piece in pieces {
        if chars.len() >= ccp_text {
            break;
        }
        let len = piece.cp_end.saturating_sub(piece.cp_start).min(ccp_text - chars.len());
        if piece.compressed {
            let Some(bytes) = word_doc.get(piece.fc..piece.fc + len) else {
                continue;
            };
            let (s, _) = encoding_rs::WINDOWS_1252.decode_without_bom_handling(bytes);
            for (i, c) in s.chars().enumerate() {
                chars.push(c);
                fcs.push((piece.fc + i) as u32);
            }
        } else {
            let Some(bytes) = word_doc.get(piece.fc..piece.fc + len * 2) else {
                continue;
            };
            let units: Vec<u16> =
                bytes.chunks_exact(2).map(|c| u16::from_le_bytes([c[0], c[1]])).collect();
            let mut unit_idx = 0usize;
            for r in char::decode_utf16(units.iter().copied()) {
                let c = r.unwrap_or('\u{fffd}');
                chars.push(c);
                fcs.push((piece.fc + unit_idx * 2) as u32);
                unit_idx += c.len_utf16();
            }
        }
    }
    (chars, fcs)
}

// ---------------------------------------------------------------------------
// Formatting runs (CHPX / PAPX out of FKP pages)

#[derive(Clone, Copy, PartialEq)]
enum FkpKind {
    Chpx,
    Papx,
}

#[derive(Clone, Default)]
struct RunProps {
    style: Style,
    istd: u16,
    in_table: bool,
    ttp: bool,
    outline: Option<u8>,
    ilfo: u16,
    ilvl: u8,
}

struct Run {
    fc_start: u32,
    fc_end: u32,
    props: RunProps,
}

struct Runs {
    runs: Vec<Run>,
}

impl Runs {
    fn new(mut runs: Vec<Run>) -> Self {
        runs.sort_by_key(|r| r.fc_start);
        Runs { runs }
    }

    fn lookup(&self, fc: u32) -> Option<&RunProps> {
        let idx = self.runs.partition_point(|r| r.fc_start <= fc);
        if idx == 0 {
            return None;
        }
        let run = &self.runs[idx - 1];
        (fc < run.fc_end).then_some(&run.props)
    }
}

fn parse_fkps(
    word_doc: &[u8],
    table: &[u8],
    data: &[u8],
    fib_off: usize,
    kind: FkpKind,
) -> Vec<Run> {
    let mut runs = Vec::new();
    let fc = get_u32(word_doc, fib_off).unwrap_or(0) as usize;
    let lcb = get_u32(word_doc, fib_off + 4).unwrap_or(0) as usize;
    let Some(plc) = table.get(fc..fc + lcb) else {
        return runs;
    };
    if plc.len() < 8 {
        return runs;
    }
    let n = (plc.len() - 4) / 8;
    for i in 0..n {
        let Some(pn_raw) = get_u32(plc, (n + 1) * 4 + i * 4) else {
            continue;
        };
        let pn = (pn_raw & 0x3F_FFFF) as usize;
        let Some(page) = word_doc.get(pn * 512..pn * 512 + 512) else {
            continue;
        };
        parse_fkp_page(page, data, kind, &mut runs);
    }
    runs
}

fn parse_fkp_page(page: &[u8], data: &[u8], kind: FkpKind, runs: &mut Vec<Run>) {
    let count = page[511] as usize;
    if count == 0 {
        return;
    }
    let entry_size = if kind == FkpKind::Papx { 13 } else { 1 };
    for k in 0..count {
        let Some(fc_start) = get_u32(page, k * 4) else {
            continue;
        };
        let Some(fc_end) = get_u32(page, (k + 1) * 4) else {
            continue;
        };
        let b_offset_pos = (count + 1) * 4 + k * entry_size;
        let Some(&b_offset) = page.get(b_offset_pos) else {
            continue;
        };
        let mut props = RunProps::default();
        if b_offset != 0 {
            let off = b_offset as usize * 2;
            match kind {
                FkpKind::Chpx => {
                    if let Some(&cb) = page.get(off)
                        && let Some(grpprl) = page.get(off + 1..off + 1 + cb as usize)
                    {
                        apply_chp_sprms(grpprl, &mut props);
                    }
                }
                FkpKind::Papx => {
                    if let Some(&cb) = page.get(off) {
                        let (start, len) = if cb == 0 {
                            let cb2 = page.get(off + 1).copied().unwrap_or(0) as usize;
                            (off + 2, cb2 * 2)
                        } else {
                            (off + 1, cb as usize * 2 - 1)
                        };
                        if let Some(grpprl) = page.get(start..start + len)
                            && grpprl.len() >= 2
                        {
                            props.istd = u16::from_le_bytes([grpprl[0], grpprl[1]]);
                            apply_pap_sprms(&grpprl[2..], data, &mut props);
                        }
                    }
                }
            }
        }
        runs.push(Run { fc_start, fc_end, props });
    }
}

fn sprm_operand_len(sprm: u16, operand: &[u8]) -> usize {
    match sprm >> 13 {
        0 | 1 => 1,
        2 | 4 | 5 => 2,
        3 => 4,
        7 => 3,
        _ => {
            if sprm == 0xD608 {
                operand.get(..2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize + 1).unwrap_or(0)
            } else {
                operand.first().map(|&b| b as usize + 1).unwrap_or(0)
            }
        }
    }
}

fn walk_sprms(grpprl: &[u8], mut f: impl FnMut(u16, &[u8])) {
    let mut pos = 0;
    while pos + 2 <= grpprl.len() {
        let sprm = u16::from_le_bytes([grpprl[pos], grpprl[pos + 1]]);
        pos += 2;
        let len = sprm_operand_len(sprm, &grpprl[pos..]);
        let Some(operand) = grpprl.get(pos..pos + len) else {
            break;
        };
        f(sprm, operand);
        pos += len;
    }
}

fn toggle_on(operand: &[u8]) -> Option<bool> {
    match operand.first() {
        Some(0) => Some(false),
        Some(1) | Some(0x81) => Some(true),
        _ => None,
    }
}

fn apply_chp_sprms(grpprl: &[u8], props: &mut RunProps) {
    walk_sprms(grpprl, |sprm, operand| match sprm {
        0x0835 => {
            if let Some(v) = toggle_on(operand) {
                props.style.bold = v;
            }
        }
        0x0836 => {
            if let Some(v) = toggle_on(operand) {
                props.style.italic = v;
            }
        }
        0x0837 => {
            if let Some(v) = toggle_on(operand) {
                props.style.strike = v;
            }
        }
        _ => {}
    });
}

fn apply_pap_sprms(grpprl: &[u8], data: &[u8], props: &mut RunProps) {
    walk_sprms(grpprl, |sprm, operand| match sprm {
        0x2416 => props.in_table = operand.first().is_some_and(|&v| v != 0),
        0x2417 => props.ttp = operand.first().is_some_and(|&v| v != 0),
        // sprmPHugePapx: the real grpprl lives length-prefixed in the Data stream.
        0x6646 => {
            if let Some(off) = get_u32(operand, 0).map(|v| v as usize)
                && let Some(cb) = get_u16(data, off).map(|v| v as usize)
                && let Some(huge) = data.get(off + 2..off + 2 + cb)
            {
                apply_pap_sprms(huge, &[], props);
            }
        }
        0x2640 => {
            if let Some(&v) = operand.first()
                && v < 9
            {
                props.outline = Some(v + 1);
            }
        }
        0x260A => props.ilvl = operand.first().copied().unwrap_or(0),
        0x460B => props.ilfo = get_u16(operand, 0).unwrap_or(0),
        _ => {}
    });
}

fn parse_stsh(word_doc: &[u8], table: &[u8]) -> HashMap<u16, u8> {
    let mut map = HashMap::new();
    let fc = get_u32(word_doc, 0xA2).unwrap_or(0) as usize;
    let lcb = get_u32(word_doc, 0xA6).unwrap_or(0) as usize;
    let Some(stsh) = table.get(fc..fc + lcb) else {
        return map;
    };
    let Some(cb_stshi) = get_u16(stsh, 0) else {
        return map;
    };
    let Some(cstd) = get_u16(stsh, 2) else {
        return map;
    };
    let mut pos = 2 + cb_stshi as usize;
    for istd in 0..cstd {
        let Some(cb_std) = get_u16(stsh, pos) else {
            break;
        };
        pos += 2;
        if cb_std == 0 {
            continue;
        }
        if let Some(first) = get_u16(stsh, pos) {
            let sti = first & 0x0FFF;
            if (1..=9).contains(&sti) {
                map.insert(istd, sti as u8);
            }
        }
        pos += cb_std as usize;
    }
    map
}

// ---------------------------------------------------------------------------
// Assembly: text stream + formatting runs -> IR

struct Assembler {
    chars: Vec<char>,
    fcs: Vec<u32>,
    chpx: Runs,
    papx: Runs,
    heading_styles: HashMap<u16, u8>,
    note_refs: HashMap<usize, String>,
    note_ranges: Vec<(usize, usize, String)>,
    ccp_text: usize,
}

struct ParaBuilder {
    inlines: Vec<Inline>,
    fields: Vec<FieldFrame>,
    text: String,
    style: Style,
}

impl ParaBuilder {
    fn new() -> Self {
        ParaBuilder {
            inlines: Vec::new(),
            fields: Vec::new(),
            text: String::new(),
            style: Style::PLAIN,
        }
    }

    fn flush_text(&mut self) {
        if self.text.is_empty() {
            return;
        }
        let text = std::mem::take(&mut self.text);
        let inline = Inline::Text { text, style: self.style };
        match self.fields.last_mut() {
            Some(f) if !f.in_result => f.instr.push_str(&inlines_to_plain_text(&[inline])),
            Some(f) => f.inlines.push(inline),
            None => self.inlines.push(inline),
        }
    }

    fn push_char(&mut self, c: char, style: Style) {
        if style != self.style {
            self.flush_text();
            self.style = style;
        }
        self.text.push(c);
    }

    fn push_inline(&mut self, inline: Inline) {
        self.flush_text();
        match self.fields.last_mut() {
            Some(f) if !f.in_result => {}
            Some(f) => f.inlines.push(inline),
            None => self.inlines.push(inline),
        }
    }

    fn field_begin(&mut self) {
        self.flush_text();
        self.fields.push(FieldFrame::default());
    }

    fn field_separate(&mut self) {
        self.flush_text();
        if let Some(f) = self.fields.last_mut() {
            f.in_result = true;
        }
    }

    fn field_end(&mut self) {
        self.flush_text();
        let Some(frame) = self.fields.pop() else {
            return;
        };
        for inline in field_result(&frame.instr, frame.inlines) {
            match self.fields.last_mut() {
                Some(f) if !f.in_result => {}
                Some(f) => f.inlines.push(inline),
                None => self.inlines.push(inline),
            }
        }
    }

    fn finish(mut self) -> Vec<Inline> {
        self.flush_text();
        while !self.fields.is_empty() {
            self.field_end();
        }
        self.inlines
    }
}

impl Assembler {
    fn build(&self) -> Document {
        let blocks = self.build_blocks(0, self.ccp_text.min(self.chars.len()));
        let mut notes = Vec::new();
        for (lo, hi, id) in &self.note_ranges {
            let lo = (*lo).min(self.chars.len());
            let hi = (*hi).min(self.chars.len());
            if lo >= hi {
                continue;
            }
            notes.push(Note { id: id.clone(), blocks: self.build_blocks(lo, hi) });
        }
        Document { blocks, notes }
    }

    fn build_blocks(&self, lo: usize, hi: usize) -> Vec<Block> {
        let mut blocks: Vec<Block> = Vec::new();
        let mut list_run: Vec<ListEntry> = Vec::new();
        let mut cell_blocks: Vec<Block> = Vec::new();
        let mut row: Vec<Cell> = Vec::new();
        let mut table_rows: Vec<Vec<Cell>> = Vec::new();
        let mut para = ParaBuilder::new();

        let mut i = lo;
        while i < hi {
            let c = self.chars[i];
            let fc = self.fcs[i];
            if let Some(id) = self.note_refs.get(&i) {
                para.push_inline(Inline::NoteRef(id.clone()));
                i += 1;
                continue;
            }
            match c {
                '\r' | '\u{7}' | '\u{c}' | '\u{e}' => {
                    let props = self.papx.lookup(fc).cloned().unwrap_or_default();
                    let inlines = std::mem::replace(&mut para, ParaBuilder::new()).finish();
                    let is_cell_mark = c == '\u{7}';
                    if props.in_table || is_cell_mark {
                        if is_cell_mark && props.ttp {
                            if !row.is_empty() {
                                table_rows.push(std::mem::take(&mut row));
                            }
                        } else if is_cell_mark {
                            if !inlines_are_empty(&inlines) {
                                cell_blocks.push(Block::Paragraph(inlines));
                            }
                            row.push(Cell { blocks: std::mem::take(&mut cell_blocks) });
                        } else if !inlines_are_empty(&inlines) {
                            cell_blocks.push(Block::Paragraph(inlines));
                        }
                    } else {
                        Self::flush_table(&mut blocks, &mut table_rows, &mut row, &mut cell_blocks);
                        self.emit_paragraph(&props, inlines, &mut blocks, &mut list_run);
                    }
                }
                '\u{b}' => para.push_inline(Inline::LineBreak),
                '\u{13}' => para.field_begin(),
                '\u{14}' => para.field_separate(),
                '\u{15}' => para.field_end(),
                '\t' => {
                    let style = self.char_style(fc);
                    para.push_char(' ', style);
                }
                '\u{1e}' => para.push_char('-', self.char_style(fc)),
                '\u{1}' | '\u{2}' | '\u{5}' | '\u{8}' | '\u{1f}' => {}
                c if ('\u{f000}'..='\u{f0ff}').contains(&c) => {
                    para.push_char('\u{2022}', Style::PLAIN);
                }
                c if c.is_control() => {}
                c => {
                    let style = self.char_style(fc);
                    para.push_char(c, style);
                }
            }
            i += 1;
        }
        let inlines = para.finish();
        if !inlines_are_empty(&inlines) {
            blocks.push(Block::Paragraph(inlines));
        }
        Self::flush_table(&mut blocks, &mut table_rows, &mut row, &mut cell_blocks);
        flush_list(&mut blocks, &mut list_run);
        blocks
    }

    fn char_style(&self, fc: u32) -> Style {
        self.chpx.lookup(fc).map(|p| p.style).unwrap_or(Style::PLAIN)
    }

    fn emit_paragraph(
        &self,
        props: &RunProps,
        inlines: Vec<Inline>,
        blocks: &mut Vec<Block>,
        list_run: &mut Vec<ListEntry>,
    ) {
        if inlines_are_empty(&inlines) {
            flush_list(blocks, list_run);
            return;
        }
        let heading = self.heading_styles.get(&props.istd).copied().or(props.outline);
        if let Some(level) = heading {
            flush_list(blocks, list_run);
            blocks.push(Block::Heading { level, content: inlines });
            return;
        }
        if props.ilfo != 0 && props.ilfo != 0xF801 {
            list_run.push((props.ilvl as usize, false, 1, Block::Paragraph(inlines)));
            return;
        }
        flush_list(blocks, list_run);
        blocks.push(Block::Paragraph(inlines));
    }

    fn flush_table(
        blocks: &mut Vec<Block>,
        table_rows: &mut Vec<Vec<Cell>>,
        row: &mut Vec<Cell>,
        cell_blocks: &mut Vec<Block>,
    ) {
        if !cell_blocks.is_empty() {
            row.push(Cell { blocks: std::mem::take(cell_blocks) });
        }
        if !row.is_empty() {
            table_rows.push(std::mem::take(row));
        }
        if table_rows.is_empty() {
            return;
        }
        let rows = std::mem::take(table_rows);
        blocks.push(Block::Table(Table { rows, has_header: false }));
    }
}

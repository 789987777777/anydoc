//! RTF frontend: tokenizer + state machine over control words.

use crate::ir::*;
use crate::support::fields::hyperlink_from_instr;
use crate::support::list::{ListEntry, flush_list};
use crate::support::text::clean_text;
use anyhow::{Result, bail};

pub fn parse(bytes: &[u8]) -> Result<Document> {
    if !bytes.starts_with(b"{\\rtf") {
        bail!("not an RTF file");
    }
    let mut parser = Parser::new(bytes);
    parser.run();
    Ok(parser.finish())
}

#[derive(Clone, Copy, PartialEq)]
enum Capture {
    None,
    ListText,
    FieldInstr,
}

#[derive(Clone, Copy)]
struct CharState {
    style: Style,
    uc_skip: u32,
    in_table: bool,
    ilvl: usize,
    list_item: bool,
    list_ordered: bool,
    outline: Option<u8>,
    suppress: bool,
    capture: Capture,
    in_note: bool,
}

impl Default for CharState {
    fn default() -> Self {
        CharState {
            style: Style::PLAIN,
            uc_skip: 1,
            in_table: false,
            ilvl: 0,
            list_item: false,
            list_ordered: false,
            outline: None,
            suppress: false,
            capture: Capture::None,
            in_note: false,
        }
    }
}

struct FieldFrame {
    depth: usize,
    instr: String,
    start: usize,
}

struct NoteFrame {
    depth: usize,
    start: usize,
}

struct Parser<'a> {
    bytes: &'a [u8],
    pos: usize,
    stack: Vec<CharState>,
    state: CharState,
    codepage: &'static encoding_rs::Encoding,
    pending_bytes: Vec<u8>,
    skip_chars: u32,

    inlines: Vec<Inline>,
    blocks: Vec<Block>,
    list_run: Vec<ListEntry>,
    cell_blocks: Vec<Block>,
    row: Vec<Cell>,
    table_rows: Vec<Vec<Cell>>,
    listtext: Option<String>,
    fields: Vec<FieldFrame>,
    note_frames: Vec<NoteFrame>,
    notes: Vec<Note>,
}

impl<'a> Parser<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Parser {
            bytes,
            pos: 0,
            stack: Vec::new(),
            state: CharState::default(),
            codepage: encoding_rs::WINDOWS_1252,
            pending_bytes: Vec::new(),
            skip_chars: 0,
            inlines: Vec::new(),
            blocks: Vec::new(),
            list_run: Vec::new(),
            cell_blocks: Vec::new(),
            row: Vec::new(),
            table_rows: Vec::new(),
            listtext: None,
            fields: Vec::new(),
            note_frames: Vec::new(),
            notes: Vec::new(),
        }
    }

    fn run(&mut self) {
        while self.pos < self.bytes.len() {
            let b = self.bytes[self.pos];
            match b {
                b'{' => {
                    self.flush_pending();
                    self.pos += 1;
                    self.stack.push(self.state);
                }
                b'}' => {
                    self.flush_pending();
                    self.pos += 1;
                    if let Some(prev) = self.stack.pop() {
                        self.state = prev;
                    }
                    self.close_fields();
                    self.close_notes();
                }
                b'\\' => {
                    self.pos += 1;
                    self.control();
                }
                b'\r' | b'\n' => self.pos += 1,
                _ => {
                    self.pos += 1;
                    if self.accepts_text() {
                        if self.skip_chars > 0 {
                            self.skip_chars -= 1;
                        } else {
                            self.pending_bytes.push(b);
                        }
                    }
                }
            }
        }
        self.flush_pending();
        self.end_paragraph();
    }

    fn accepts_text(&self) -> bool {
        self.state.capture != Capture::None || !self.state.suppress
    }

    fn close_notes(&mut self) {
        while self.note_frames.last().is_some_and(|f| f.depth > self.stack.len()) {
            let frame = self.note_frames.pop().unwrap();
            let start = frame.start.min(self.inlines.len());
            let content: Vec<Inline> = self.inlines.drain(start..).collect();
            if !inlines_are_empty(&content) {
                let id = format!("rtf{}", self.notes.len());
                self.notes.push(Note { id: id.clone(), blocks: vec![Block::Paragraph(content)] });
                self.inlines.push(Inline::NoteRef(id));
            }
        }
    }

    fn close_fields(&mut self) {
        while self.fields.last().is_some_and(|f| f.depth > self.stack.len()) {
            let frame = self.fields.pop().unwrap();
            let start = frame.start.min(self.inlines.len());
            let content: Vec<Inline> = self.inlines.drain(start..).collect();
            match hyperlink_from_instr(&frame.instr) {
                Some(url) if !inlines_are_empty(&content) => {
                    self.inlines.push(Inline::Link { content, url });
                }
                _ => self.inlines.extend(content),
            }
        }
    }

    fn control(&mut self) {
        let Some(&b) = self.bytes.get(self.pos) else {
            return;
        };
        if !b.is_ascii_alphabetic() {
            self.pos += 1;
            match b {
                b'\'' => {
                    let hex = self.read_hex_pair();
                    if self.accepts_text() {
                        if self.skip_chars > 0 {
                            self.skip_chars -= 1;
                        } else if let Some(v) = hex {
                            self.pending_bytes.push(v);
                        }
                    }
                }
                b'~' => self.push_char('\u{a0}'),
                b'-' => {}
                b'_' => self.push_char('-'),
                b'*' => self.state.suppress = true,
                b'\\' | b'{' | b'}' => self.push_char(b as char),
                b'\r' | b'\n' => self.control_word("par", None),
                _ => {}
            }
            return;
        }
        let start = self.pos;
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_alphabetic() {
            self.pos += 1;
        }
        let word_end = self.pos;
        let mut param: Option<i32> = None;
        let mut negative = false;
        if self.bytes.get(self.pos) == Some(&b'-') {
            negative = true;
            self.pos += 1;
        }
        let num_start = self.pos;
        while self.pos < self.bytes.len() && self.bytes[self.pos].is_ascii_digit() {
            self.pos += 1;
        }
        if self.pos > num_start {
            if let Ok(n) =
                std::str::from_utf8(&self.bytes[num_start..self.pos]).unwrap_or("0").parse::<i32>()
            {
                param = Some(if negative { -n } else { n });
            }
        } else if negative {
            self.pos -= 1;
        }
        if self.bytes.get(self.pos) == Some(&b' ') {
            self.pos += 1;
        }
        let word = std::str::from_utf8(&self.bytes[start..word_end]).unwrap_or("");
        self.control_word(word, param);
    }

    fn read_hex_pair(&mut self) -> Option<u8> {
        let hi = (*self.bytes.get(self.pos)? as char).to_digit(16)?;
        let lo = (*self.bytes.get(self.pos + 1)? as char).to_digit(16)?;
        self.pos += 2;
        Some((hi * 16 + lo) as u8)
    }

    fn control_word(&mut self, word: &str, param: Option<i32>) {
        let on = param != Some(0);
        match word {
            "ansicpg" => {
                if let Some(cp) = param {
                    self.codepage = codepage_encoding(cp as u32);
                }
            }
            "u" => {
                if self.accepts_text() {
                    self.flush_pending();
                    if let Some(n) = param {
                        let code = if n < 0 { (n + 65536) as u32 } else { n as u32 };
                        if let Some(c) = char::from_u32(code) {
                            self.push_char(c);
                        }
                        self.skip_chars = self.state.uc_skip;
                    }
                }
            }
            "uc" => self.state.uc_skip = param.unwrap_or(1).max(0) as u32,
            "b" => self.set_style(|s| s.bold = on),
            "i" => self.set_style(|s| s.italic = on),
            "strike" | "striked" => self.set_style(|s| s.strike = on),
            "plain" => self.set_style(|s| *s = Style::PLAIN),
            "par" | "sect" => {
                self.flush_pending();
                if self.state.in_note {
                    self.inlines.push(Inline::LineBreak);
                } else if !self.state.suppress {
                    self.end_paragraph();
                }
            }
            "pard" => {
                self.flush_pending();
                self.state.in_table = false;
                self.state.list_item = false;
                self.state.ilvl = 0;
                self.state.outline = None;
            }
            "line" | "lbr" => {
                self.flush_pending();
                if !self.state.suppress {
                    self.inlines.push(Inline::LineBreak);
                }
            }
            "tab" => self.push_char(' '),
            "emdash" => self.push_char('\u{2014}'),
            "endash" => self.push_char('\u{2013}'),
            "lquote" => self.push_char('\u{2018}'),
            "rquote" => self.push_char('\u{2019}'),
            "ldblquote" => self.push_char('\u{201c}'),
            "rdblquote" => self.push_char('\u{201d}'),
            "bullet" => self.push_char('\u{2022}'),
            "enspace" | "emspace" | "qmspace" => self.push_char(' '),
            "cell" | "nestcell" => {
                self.flush_pending();
                if !self.state.suppress && !self.state.in_note {
                    self.end_cell();
                }
            }
            "row" | "nestrow" => {
                self.flush_pending();
                if !self.state.suppress && !self.state.in_note {
                    self.end_row();
                }
            }
            "intbl" => self.state.in_table = true,
            "outlinelevel" => {
                if let Some(n) = param
                    && (0..9).contains(&n)
                {
                    self.state.outline = Some((n + 1) as u8);
                }
            }
            "ilvl" => self.state.ilvl = param.unwrap_or(0).clamp(0, 8) as usize,
            "ls" => self.state.list_item = true,
            "listtext" | "pntext" => {
                self.flush_pending();
                self.state.capture = Capture::ListText;
                if self.listtext.is_none() {
                    self.listtext = Some(String::new());
                }
            }
            "pnlvlblt" => {
                self.state.list_item = true;
                self.state.list_ordered = false;
            }
            "pnlvlbody" | "pndec" => {
                self.state.list_item = true;
                self.state.list_ordered = true;
            }
            "field" => {
                self.flush_pending();
                if !self.state.suppress {
                    self.fields.push(FieldFrame {
                        depth: self.stack.len(),
                        instr: String::new(),
                        start: self.inlines.len(),
                    });
                }
            }
            "fldinst" => {
                self.flush_pending();
                if !self.fields.is_empty() {
                    self.state.capture = Capture::FieldInstr;
                }
                self.state.suppress = true;
            }
            "fldrslt" => {
                self.flush_pending();
                self.state.capture = Capture::None;
            }
            "footnote" => {
                self.flush_pending();
                self.state.in_note = true;
                self.state.suppress = false;
                self.state.capture = Capture::None;
                self.note_frames
                    .push(NoteFrame { depth: self.stack.len(), start: self.inlines.len() });
            }
            "chftn" | "ftnalt" => {}
            "fonttbl" | "colortbl" | "stylesheet" | "info" | "pict" | "object" | "header"
            | "footer" | "headerl" | "headerr" | "headerf" | "footerl" | "footerr" | "footerf"
            | "ftnsep" | "ftnsepc" | "aftnsep" | "aftnsepc" | "xmlnstbl" | "themedata"
            | "colorschememapping" | "datastore" | "latentstyles" | "listtable"
            | "listoverridetable" | "rsidtbl" | "generator" | "filetbl" | "revtbl"
            | "datafield" | "bkmkstart" | "bkmkend" | "annotation" | "atnid" | "atnauthor"
            | "template" | "defchp" | "defpap" | "panose" | "falt" | "objdata" | "blipuid"
            | "nonshppict" | "wgrffmtfilter" | "pgdsctbl" | "docvar" | "sp" | "sn" | "sv"
            | "shpinst" | "background" | "userprops" | "operator" | "author" | "title"
            | "subject" | "keywords" | "doccomm" | "creatim" | "revtim" | "printim" => {
                self.flush_pending();
                self.state.suppress = true;
                self.state.capture = Capture::None;
            }
            _ => {}
        }
    }

    fn set_style(&mut self, f: impl FnOnce(&mut Style)) {
        self.flush_pending();
        f(&mut self.state.style);
    }

    fn push_char(&mut self, c: char) {
        if !self.accepts_text() {
            return;
        }
        if self.skip_chars > 0 {
            self.skip_chars -= 1;
            return;
        }
        self.flush_pending();
        self.push_text(c.to_string());
    }

    fn flush_pending(&mut self) {
        if self.pending_bytes.is_empty() {
            return;
        }
        let bytes = std::mem::take(&mut self.pending_bytes);
        let (text, _, _) = self.codepage.decode(&bytes);
        self.push_text(text.into_owned());
    }

    fn push_text(&mut self, text: String) {
        let text = clean_text(&text);
        if text.is_empty() {
            return;
        }
        match self.state.capture {
            Capture::ListText => {
                if let Some(lt) = &mut self.listtext {
                    lt.push_str(&text);
                }
            }
            Capture::FieldInstr => {
                if let Some(f) = self.fields.last_mut() {
                    f.instr.push_str(&text);
                }
            }
            Capture::None => {
                if !self.state.suppress {
                    self.inlines.push(Inline::Text { text, style: self.state.style });
                }
            }
        }
    }

    fn end_paragraph(&mut self) {
        let inlines = std::mem::take(&mut self.inlines);
        let listtext = self.listtext.take();

        if self.state.in_table {
            if !inlines_are_empty(&inlines) {
                self.cell_blocks.push(Block::Paragraph(inlines));
            }
            return;
        }
        self.flush_table();

        if inlines_are_empty(&inlines) {
            self.flush_list();
            return;
        }
        if let Some(level) = self.state.outline {
            self.flush_list();
            self.blocks.push(Block::Heading { level, content: inlines });
            return;
        }
        if self.state.list_item || listtext.as_deref().is_some_and(|t| !t.trim().is_empty()) {
            let ordered = match &listtext {
                Some(lt) if !lt.trim().is_empty() => {
                    lt.trim_start().chars().next().is_some_and(|c| c.is_ascii_digit())
                }
                _ => self.state.list_ordered,
            };
            self.list_run.push((self.state.ilvl, ordered, 1, Block::Paragraph(inlines)));
            return;
        }
        self.flush_list();
        self.blocks.push(Block::Paragraph(inlines));
    }

    fn end_cell(&mut self) {
        let inlines = std::mem::take(&mut self.inlines);
        self.listtext = None;
        if !inlines_are_empty(&inlines) {
            self.cell_blocks.push(Block::Paragraph(inlines));
        }
        let blocks = std::mem::take(&mut self.cell_blocks);
        self.row.push(Cell { blocks });
    }

    fn end_row(&mut self) {
        if !self.cell_blocks.is_empty() || !inlines_are_empty(&self.inlines) {
            self.end_cell();
        }
        let row = std::mem::take(&mut self.row);
        if !row.is_empty() {
            self.table_rows.push(row);
        }
    }

    fn flush_table(&mut self) {
        if self.table_rows.is_empty() {
            return;
        }
        self.flush_list();
        let mut rows = std::mem::take(&mut self.table_rows);
        if rows.len() == 1 && rows[0].len() == 1 {
            let cell = rows.remove(0).into_iter().next().unwrap();
            self.blocks.extend(cell.blocks);
        } else {
            self.blocks.push(Block::Table(Table { rows, has_header: false }));
        }
    }

    fn flush_list(&mut self) {
        flush_list(&mut self.blocks, &mut self.list_run);
    }

    fn finish(mut self) -> Document {
        if !self.row.is_empty() || !self.cell_blocks.is_empty() {
            self.end_row();
        }
        self.state.in_table = false;
        self.flush_table();
        self.flush_list();
        Document { blocks: self.blocks, notes: self.notes }
    }
}

fn codepage_encoding(cp: u32) -> &'static encoding_rs::Encoding {
    match cp {
        932 => encoding_rs::SHIFT_JIS,
        936 => encoding_rs::GBK,
        949 => encoding_rs::EUC_KR,
        950 => encoding_rs::BIG5,
        1250 => encoding_rs::WINDOWS_1250,
        1251 => encoding_rs::WINDOWS_1251,
        1253 => encoding_rs::WINDOWS_1253,
        1254 => encoding_rs::WINDOWS_1254,
        1255 => encoding_rs::WINDOWS_1255,
        1256 => encoding_rs::WINDOWS_1256,
        1257 => encoding_rs::WINDOWS_1257,
        1258 => encoding_rs::WINDOWS_1258,
        874 => encoding_rs::WINDOWS_874,
        _ => encoding_rs::WINDOWS_1252,
    }
}

//! Format-neutral document tree

#[derive(Debug, Clone, Default)]
pub struct Document {
    pub blocks: Vec<Block>,
    pub notes: Vec<Note>,
}

/// Footnote or endnote body, referenced from text by `Inline::NoteRef`.
#[derive(Debug, Clone)]
pub struct Note {
    pub id: String,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone)]
pub enum Block {
    Heading { level: u8, content: Vec<Inline> },
    Paragraph(Vec<Inline>),
    List(List),
    Table(Table),
    BlockQuote(Vec<Block>),
    CodeBlock { lang: Option<String>, text: String },
    Rule,
}

#[derive(Debug, Clone)]
pub struct List {
    pub ordered: bool,
    pub start: u64,
    pub items: Vec<ListItem>,
}

#[derive(Debug, Clone)]
pub struct ListItem {
    pub blocks: Vec<Block>,
    pub checked: Option<bool>,
}

#[derive(Debug, Clone, Default)]
pub struct Table {
    pub rows: Vec<Vec<Cell>>,
    pub has_header: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Cell {
    pub blocks: Vec<Block>,
}

impl Cell {
    pub fn from_inlines(inlines: Vec<Inline>) -> Self {
        Cell { blocks: vec![Block::Paragraph(inlines)] }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Style {
    pub bold: bool,
    pub italic: bool,
    pub strike: bool,
    pub code: bool,
}

impl Style {
    pub const PLAIN: Style = Style { bold: false, italic: false, strike: false, code: false };
}

#[derive(Debug, Clone)]
pub enum Inline {
    Text { text: String, style: Style },
    Link { content: Vec<Inline>, url: String },
    Image { alt: String, url: Option<String> },
    NoteRef(String),
    LineBreak,
}

impl Inline {
    pub fn plain(text: impl Into<String>) -> Self {
        Inline::Text { text: text.into(), style: Style::PLAIN }
    }
}

pub fn inlines_to_plain_text(inlines: &[Inline]) -> String {
    let mut out = String::new();
    for inline in inlines {
        match inline {
            Inline::Text { text, .. } => out.push_str(text),
            Inline::Link { content, .. } => out.push_str(&inlines_to_plain_text(content)),
            Inline::Image { alt, .. } => out.push_str(alt),
            Inline::NoteRef(_) => {}
            Inline::LineBreak => out.push('\n'),
        }
    }
    out
}

pub fn inlines_are_empty(inlines: &[Inline]) -> bool {
    inlines.iter().all(|i| match i {
        Inline::Text { text, .. } => text.trim().is_empty(),
        Inline::Link { content, url } => url.is_empty() && inlines_are_empty(content),
        Inline::Image { .. } => false,
        Inline::NoteRef(_) => false,
        Inline::LineBreak => true,
    })
}

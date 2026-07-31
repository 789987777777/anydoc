use crate::model::{AnchorId, Inline, List, Table};

#[derive(Debug, Clone)]
pub enum Block {
    Heading {
        level: u8,
        /// Stable anchor id when the source document targets this heading
        /// (bookmark, chapter fragment, ...). Renderers map it to the
        /// heading's own anchor rather than emitting a separate one.
        anchor: Option<AnchorId>,
        content: Vec<Inline>,
    },
    Paragraph(Vec<Inline>),
    List(List),
    Table(Table),
    BlockQuote(Vec<Block>),
    CodeBlock {
        lang: Option<String>,
        text: String,
    },
    Rule,
}

impl Block {
    pub fn heading(level: u8, content: Vec<Inline>) -> Block {
        Block::Heading { level, anchor: None, content }
    }
}

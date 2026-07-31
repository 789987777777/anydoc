use crate::model::{AnchorId, ImageSource, LinkTarget, Style};

#[derive(Debug, Clone)]
pub enum Inline {
    Text {
        text: String,
        style: Style,
    },
    Link {
        content: Vec<Inline>,
        target: LinkTarget,
    },
    Image {
        alt: String,
        source: ImageSource,
    },
    /// Zero-width anchor marking an internal link target at this position
    /// (bookmarks on paragraphs, spans, list items, table cells, ...).
    Anchor(AnchorId),
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
    collect_plain_text(inlines, &mut out);
    out
}

fn collect_plain_text(inlines: &[Inline], out: &mut String) {
    for inline in inlines {
        match inline {
            Inline::Text { text, .. } => out.push_str(text),
            Inline::Link { content, .. } => collect_plain_text(content, out),
            Inline::Image { alt, .. } => out.push_str(alt),
            Inline::Anchor(_) | Inline::NoteRef(_) => {}
            Inline::LineBreak => out.push('\n'),
        }
    }
}

pub fn inlines_are_empty(inlines: &[Inline]) -> bool {
    inlines.iter().all(|i| match i {
        Inline::Text { text, .. } => text.trim().is_empty(),
        Inline::Link { content, target } => target.is_empty() && inlines_are_empty(content),
        Inline::Image { .. } | Inline::NoteRef(_) => false,
        Inline::Anchor(_) | Inline::LineBreak => true,
    })
}

//! Information-preserving document model.
//!
//! Only fully resolved content lives here: format frontends resolve style
//! cascades, numbering, and references before constructing these types.
//! A [`Document`] is self-contained - embedded assets carry their bytes, so it
//! stays usable after the source archive is gone.

mod asset;
mod block;
mod inline;
mod link;
mod list;
mod style;
mod table;

pub use asset::{Asset, AssetId};
pub use block::Block;
pub use inline::{Inline, inlines_are_empty, inlines_to_plain_text};
pub use link::{AnchorId, ImageSource, LinkTarget};
pub use list::{List, ListItem, MarkerKind};
pub use style::Style;
pub use table::{Cell, CellSlot, GridBuilder, Table, TableKind};

#[derive(Debug, Clone, Default)]
pub struct Document {
    pub blocks: Vec<Block>,
    pub notes: Vec<Note>,
    pub assets: Vec<Asset>,
}

/// Footnote or endnote body, referenced from text by `Inline::NoteRef`.
#[derive(Debug, Clone)]
pub struct Note {
    pub id: String,
    pub kind: NoteKind,
    pub blocks: Vec<Block>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteKind {
    Footnote,
    Endnote,
}

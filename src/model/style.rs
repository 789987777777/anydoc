/// Fully resolved character style. Tri-state deltas exist only during
/// frontend resolution (`shared::delta`); by the time content reaches the
/// model every toggle has a definite value.
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

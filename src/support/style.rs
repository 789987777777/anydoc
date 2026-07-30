//! Style inheritance resolution shared by the docx and odf frontends.

use crate::ir::Style;
use crate::support::xml::Element;
use std::collections::HashMap;

/// Style id -> (definition element, parent style id).
pub type RawStyles<'a> = HashMap<&'a str, (&'a Element, Option<&'a str>)>;

/// Resolve a style by walking its parent chain, then applying its own
/// properties. `base` supplies styles defined outside `raw`.
pub fn resolve_chain(
    id: &str,
    raw: &RawStyles,
    base: &impl Fn(&str) -> Style,
    apply: &impl Fn(&Element, &mut Style),
) -> Style {
    resolve(id, raw, base, apply, 0)
}

fn resolve(
    id: &str,
    raw: &RawStyles,
    base: &impl Fn(&str) -> Style,
    apply: &impl Fn(&Element, &mut Style),
    depth: usize,
) -> Style {
    if depth > 8 {
        return Style::PLAIN;
    }
    let Some((elem, parent)) = raw.get(id) else {
        return base(id);
    };
    let mut style = match parent {
        Some(p) => resolve(p, raw, base, apply, depth + 1),
        None => Style::PLAIN,
    };
    apply(elem, &mut style);
    style
}

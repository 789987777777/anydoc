//! Shared text normalization.

/// Drop control characters and layout-only invisibles, convert NBSP to a
/// regular space, strip soft hyphens. U+200C (ZWNJ) and U+200D (ZWJ) are
/// **preserved**: they carry meaning in Arabic/Indic shaping and emoji
/// sequences.
pub fn clean_text(text: &str) -> String {
    text.chars()
        .filter_map(|c| match c {
            '\u{a0}' => Some(' '),
            '\u{ad}' | '\u{200b}' | '\u{feff}' => None,
            '\t' => Some('\t'),
            '\n' | '\r' => Some(' '),
            c if c.is_control() => None,
            c => Some(c),
        })
        .collect()
}

/// Collapse whitespace runs to single spaces.
pub fn collapse_ws(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev_space = false;
    for c in text.chars() {
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
            }
            prev_space = true;
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::clean_text;

    #[test]
    fn join_controls_preserved() {
        assert_eq!(clean_text("می\u{200c}خواهم"), "می\u{200c}خواهم");
        assert_eq!(clean_text("👨\u{200d}👩\u{200d}👧"), "👨\u{200d}👩\u{200d}👧");
    }

    #[test]
    fn layout_invisibles_stripped() {
        assert_eq!(clean_text("a\u{ad}b\u{200b}c\u{feff}d\u{a0}e"), "abcd e");
    }
}

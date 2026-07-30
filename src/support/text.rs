//! Shared text normalization.

/// Drop control characters and zero-width marks, convert NBSP to a regular
/// space, strip soft hyphens.
pub fn clean_text(text: &str) -> String {
    text.chars()
        .filter_map(|c| match c {
            '\u{a0}' => Some(' '),
            '\u{ad}' | '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{feff}' => None,
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

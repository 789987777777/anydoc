//! Word field instruction parsing (shared by doc, docx, rtf).

/// Extract a link target from a HYPERLINK field instruction. The target may
/// be quoted or bare, preceded by switches like \l (anchor).
pub fn hyperlink_from_instr(instr: &str) -> Option<String> {
    let rest = instr.trim().strip_prefix("HYPERLINK")?;
    let chars: Vec<char> = rest.chars().collect();
    let mut anchor = false;
    let mut url: Option<String> = None;
    let mut i = 0;
    while i < chars.len() && url.is_none() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c == '"' {
            let start = i + 1;
            let mut j = start;
            while j < chars.len() && chars[j] != '"' {
                j += 1;
            }
            let tok: String = chars[start..j].iter().collect();
            if !tok.trim().is_empty() {
                url = Some(tok.trim().to_string());
            }
            i = j + 1;
        } else {
            let start = i;
            let mut j = i;
            while j < chars.len() && !chars[j].is_whitespace() {
                j += 1;
            }
            let tok: String = chars[start..j].iter().collect();
            if tok == "\\l" {
                anchor = true;
            } else if !tok.starts_with('\\') {
                url = Some(tok);
            }
            i = j;
        }
    }
    let url = url?;
    Some(if anchor { format!("#{url}") } else { url })
}

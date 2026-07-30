//! Minimal DOM built on quick-xml. Element names are stored without their
//! namespace prefix; attributes keep it, and lookup matches the qualified
//! name or its local part.

use anyhow::Result;
use quick_xml::Reader;
use quick_xml::events::{BytesStart, Event};

#[derive(Debug)]
pub enum Node {
    Elem(Element),
    Text(String),
}

#[derive(Debug)]
pub struct Element {
    pub name: String,
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Node>,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(k, _)| k == name || k.rsplit_once(':').is_some_and(|(_, local)| local == name))
            .map(|(_, v)| v.as_str())
    }

    pub fn child_elems(&self) -> impl Iterator<Item = &Element> {
        self.children.iter().filter_map(|n| match n {
            Node::Elem(e) => Some(e),
            Node::Text(_) => None,
        })
    }

    pub fn find(&self, name: &str) -> Option<&Element> {
        self.child_elems().find(|e| e.name == name)
    }

    pub fn find_all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> {
        self.child_elems().filter(move |e| e.name == name)
    }

    /// Depth-first search for the first descendant element with the given name.
    pub fn first_descendant(&self, name: &str) -> Option<&Element> {
        for child in self.child_elems() {
            if child.name == name {
                return Some(child);
            }
            if let Some(found) = child.first_descendant(name) {
                return Some(found);
            }
        }
        None
    }

    /// Depth-first search for all descendant elements with the given name.
    pub fn descendants<'a>(&'a self, name: &'a str) -> Vec<&'a Element> {
        let mut out = Vec::new();
        self.collect_descendants(name, &mut out);
        out
    }

    fn collect_descendants<'a>(&'a self, name: &str, out: &mut Vec<&'a Element>) {
        for child in self.child_elems() {
            if child.name == name {
                out.push(child);
            }
            child.collect_descendants(name, out);
        }
    }

    /// Concatenated text of all descendant text nodes.
    pub fn text(&self) -> String {
        let mut s = String::new();
        self.collect_text(&mut s);
        s
    }

    fn collect_text(&self, out: &mut String) {
        for child in &self.children {
            match child {
                Node::Text(t) => out.push_str(t),
                Node::Elem(e) => e.collect_text(out),
            }
        }
    }
}

fn local_name(qname: &[u8]) -> String {
    let start = qname.iter().rposition(|&b| b == b':').map_or(0, |i| i + 1);
    String::from_utf8_lossy(&qname[start..]).into_owned()
}

fn start_to_element(e: &BytesStart, reader: &Reader<&[u8]>) -> Element {
    let mut attrs = Vec::new();
    for attr in e.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
        let value = attr
            .decoded_and_normalized_value(quick_xml::XmlVersion::Implicit1_0, reader.decoder())
            .map(|v| v.into_owned())
            .unwrap_or_else(|_| String::from_utf8_lossy(&attr.value).into_owned());
        attrs.push((key, value));
    }
    Element { name: local_name(e.name().as_ref()), attrs, children: Vec::new() }
}

/// Resolve entity references quick-xml surfaces as GeneralRef events.
fn resolve_entity(name: &str) -> Option<String> {
    if let Some(num) = name.strip_prefix('#') {
        let code = if let Some(hex) = num.strip_prefix('x').or_else(|| num.strip_prefix('X')) {
            u32::from_str_radix(hex, 16).ok()?
        } else {
            num.parse().ok()?
        };
        return char::from_u32(code).map(String::from);
    }
    let ch = match name {
        "amp" => '&',
        "lt" => '<',
        "gt" => '>',
        "apos" => '\'',
        "quot" => '"',
        "nbsp" => '\u{a0}',
        "shy" => '\u{ad}',
        "mdash" => '\u{2014}',
        "ndash" => '\u{2013}',
        "lsquo" => '\u{2018}',
        "rsquo" => '\u{2019}',
        "ldquo" => '\u{201c}',
        "rdquo" => '\u{201d}',
        "hellip" => '\u{2026}',
        "copy" => '\u{a9}',
        "reg" => '\u{ae}',
        "trade" => '\u{2122}',
        "deg" => '\u{b0}',
        "middot" => '\u{b7}',
        "bull" => '\u{2022}',
        "sect" => '\u{a7}',
        "para" => '\u{b6}',
        "laquo" => '\u{ab}',
        "raquo" => '\u{bb}',
        "times" => '\u{d7}',
        "divide" => '\u{f7}',
        "plusmn" => '\u{b1}',
        "frac12" => '\u{bd}',
        "frac14" => '\u{bc}',
        "eacute" => '\u{e9}',
        "egrave" => '\u{e8}',
        "agrave" => '\u{e0}',
        "ccedil" => '\u{e7}',
        "uuml" => '\u{fc}',
        "ouml" => '\u{f6}',
        "auml" => '\u{e4}',
        "szlig" => '\u{df}',
        "aring" => '\u{e5}',
        "oslash" => '\u{f8}',
        "aelig" => '\u{e6}',
        "euro" => '\u{20ac}',
        "pound" => '\u{a3}',
        "yen" => '\u{a5}',
        "cent" => '\u{a2}',
        _ => return None,
    };
    Some(ch.to_string())
}

/// Parse an XML document into a synthetic root element containing the
/// top-level nodes. Namespace prefixes are stripped.
pub fn parse_xml(xml: &str) -> Result<Element> {
    let mut reader = Reader::from_str(xml);
    reader.config_mut().check_end_names = false;

    let mut root = Element { name: String::new(), attrs: Vec::new(), children: Vec::new() };
    let mut stack: Vec<Element> = Vec::new();

    loop {
        match reader.read_event()? {
            Event::Start(e) => {
                stack.push(start_to_element(&e, &reader));
            }
            Event::Empty(e) => {
                let elem = start_to_element(&e, &reader);
                match stack.last_mut() {
                    Some(parent) => parent.children.push(Node::Elem(elem)),
                    None => root.children.push(Node::Elem(elem)),
                }
            }
            Event::End(_) => {
                if let Some(elem) = stack.pop() {
                    match stack.last_mut() {
                        Some(parent) => parent.children.push(Node::Elem(elem)),
                        None => root.children.push(Node::Elem(elem)),
                    }
                }
            }
            Event::Text(e) => {
                let text = match e.decode() {
                    Ok(t) => t.into_owned(),
                    Err(_) => String::from_utf8_lossy(e.as_ref()).into_owned(),
                };
                if !text.is_empty() {
                    push_text(&mut stack, &mut root, text);
                }
            }
            Event::GeneralRef(e) => {
                let name = String::from_utf8_lossy(e.as_ref()).into_owned();
                let resolved = resolve_entity(&name).unwrap_or_else(|| format!("&{name};"));
                push_text(&mut stack, &mut root, resolved);
            }
            Event::CData(e) => {
                let text = String::from_utf8_lossy(e.as_ref()).into_owned();
                push_text(&mut stack, &mut root, text);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    // Recover elements left open by malformed input.
    while let Some(elem) = stack.pop() {
        match stack.last_mut() {
            Some(parent) => parent.children.push(Node::Elem(elem)),
            None => root.children.push(Node::Elem(elem)),
        }
    }
    Ok(root)
}

fn push_text(stack: &mut [Element], root: &mut Element, text: String) {
    let target = match stack.last_mut() {
        Some(parent) => &mut parent.children,
        None => &mut root.children,
    };
    if let Some(Node::Text(prev)) = target.last_mut() {
        prev.push_str(&text);
    } else {
        target.push(Node::Text(text));
    }
}

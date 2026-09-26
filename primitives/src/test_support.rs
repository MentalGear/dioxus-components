//! Order-independent parsing of rendered SSR HTML, for tests only
//! (backlog row 108).
//!
//! `merge_attributes` (row 93) sorts an element's merged attribute list by
//! name, so a test that locates rendered content by slicing the SSR HTML
//! string at the position of a specific `attribute="value"` match -- or
//! that assumes one attribute appears before another -- is fragile:
//! nothing in Dioxus's own attribute model guarantees a stable order (see
//! row 93's root-cause account), and any future change to how an
//! element's attributes are constructed can reorder them again, exactly
//! as `merge_attributes`'s own sort did the first time. Two tests broke
//! this way in one round (`calendar.rs`'s unit test, chart's
//! `tooltip.rs::tooltip_fragment()`), which per this repo's own
//! `CLAUDE.md` rule makes it a class, not a coincidence.
//!
//! This module is the by-construction fix: a small, real (if minimal)
//! start-tag parser that turns a rendered tag into a name -> value map,
//! so call sites do `attrs.get("data-slot") == Some(&"...".to_string())`
//! -style lookups instead of string-position slicing. Those lookups are
//! order-independent by construction and can never break this way again,
//! regardless of what future attribute-ordering change lands.
//!
//! Deliberately not a general HTML parser: it only understands what
//! `dioxus_ssr::render`'s own flat output actually produces (well-formed
//! start tags, no unescaped `<`/`>` inside attribute values, no raw-text
//! elements like `<script>`/`<style>` in these component fixtures), which
//! is all these tests ever need.

use std::collections::HashMap;

/// One parsed start tag: its raw text, its attributes (name -> value,
/// looked up order-independently), and its position in the source `html`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    /// The tag's own name, e.g. `"div"` or `"path"`.
    pub name: String,
    /// The raw text of the opening tag itself, from its leading `<` up to
    /// and including its closing `>` (and, for a self-closing tag, the
    /// `/` immediately before it).
    pub tag: String,
    /// This tag's attributes, name -> value. A boolean attribute (no `=`
    /// at all, e.g. `inert`) is stored with an empty string value -- its
    /// presence is what `attrs.contains_key(...)` tests for. An
    /// entity-escaped quote (e.g. `&quot;`) inside a value is left
    /// exactly as written, never decoded.
    pub attrs: HashMap<String, String>,
    /// Index of this tag's leading `<` in the source `html`.
    pub start: usize,
    /// Index one past this tag's closing `>` in the source `html`, i.e.
    /// `&html[start..end] == tag`.
    pub end: usize,
    self_closing: bool,
}

impl Element {
    /// This element's own inner HTML: everything between its opening
    /// tag's `>` and its matching `</name>`, tracking nested same-name
    /// tags so an inner element sharing this one's tag name doesn't end
    /// the search early. A self-closing tag (or one with no matching
    /// close tag in `html`) has no inner HTML and returns `""`.
    pub fn inner_html<'h>(&self, html: &'h str) -> &'h str {
        if self.self_closing {
            return "";
        }
        let open = format!("<{}", self.name);
        let close = format!("</{}>", self.name);
        let mut depth: usize = 1;
        let mut pos = self.end;
        loop {
            let next_open = html[pos..].find(&open).map(|i| pos + i);
            let next_close = html[pos..].find(&close).map(|i| pos + i);
            match (next_open, next_close) {
                (Some(o), Some(c)) if o < c => {
                    // Only a real nested open tag, not e.g. `<divider>`
                    // matching the `<div` needle: the character right
                    // after the name must end the tag name (whitespace,
                    // `/`, or `>`).
                    let after = o + open.len();
                    let ends_name = html[after..]
                        .chars()
                        .next()
                        .is_none_or(|c| c.is_whitespace() || c == '/' || c == '>');
                    if ends_name {
                        depth += 1;
                    }
                    pos = after;
                }
                (_, Some(c)) => {
                    depth -= 1;
                    if depth == 0 {
                        return &html[self.end..c];
                    }
                    pos = c + close.len();
                }
                (_, None) => return &html[self.end..],
            }
        }
    }
}

/// Parses every start tag in `html` (skipping closing tags, comments and
/// doctypes) and returns the ones whose attribute map satisfies
/// `predicate`, in document order.
pub fn find_elements(
    html: &str,
    predicate: impl Fn(&HashMap<String, String>) -> bool,
) -> Vec<Element> {
    parse_all(html)
        .into_iter()
        .filter(|el| predicate(&el.attrs))
        .collect()
}

/// The first start tag in `html` whose attribute map satisfies
/// `predicate`, if any.
pub fn find_element(
    html: &str,
    predicate: impl Fn(&HashMap<String, String>) -> bool,
) -> Option<Element> {
    parse_all(html).into_iter().find(|el| predicate(&el.attrs))
}

/// Parses every start tag in `html`, in document order.
fn parse_all(html: &str) -> Vec<Element> {
    let bytes = html.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    let len = bytes.len();

    while let Some(rel) = html[i..].find('<') {
        let lt = i + rel;
        let after = lt + 1;
        if after >= len {
            break;
        }
        let next = bytes[after] as char;

        if next == '!' {
            // Comment or doctype: skip to the next '>' (good enough for
            // the well-formed fixtures these tests render; a `-->`
            // containing a stray '>' inside a comment never occurs in
            // dioxus_ssr's own output).
            i = html[after..]
                .find('>')
                .map(|p| after + p + 1)
                .unwrap_or(len);
            continue;
        }
        if next == '/' || !next.is_ascii_alphabetic() {
            // Closing tag, or a '<' that isn't a tag at all (e.g. inside
            // text content) -- move past just this '<' and keep scanning.
            i = after;
            continue;
        }

        // Opening tag: parse the name.
        let name_start = after;
        let mut p = name_start;
        while p < len && ((bytes[p] as char).is_ascii_alphanumeric() || bytes[p] == b'-') {
            p += 1;
        }
        let name = html[name_start..p].to_string();

        // Parse attributes up to the tag's own closing '>'.
        let mut attrs = HashMap::new();
        let mut self_closing = false;
        loop {
            p = skip_ws(bytes, p);
            if p >= len {
                break;
            }
            match bytes[p] {
                b'>' => {
                    p += 1;
                    break;
                }
                b'/' => {
                    // Only a self-close marker when immediately followed
                    // by '>' -- anything else would be malformed markup
                    // dioxus_ssr never emits.
                    if p + 1 < len && bytes[p + 1] == b'>' {
                        self_closing = true;
                        p += 2;
                        break;
                    }
                    p += 1;
                }
                _ => {
                    let name_begin = p;
                    while p < len {
                        let c = bytes[p] as char;
                        if c.is_whitespace() || c == '=' || c == '/' || c == '>' {
                            break;
                        }
                        p += 1;
                    }
                    if p == name_begin {
                        // Shouldn't happen for well-formed input; avoid
                        // an infinite loop just in case.
                        p += 1;
                        continue;
                    }
                    let attr_name = html[name_begin..p].to_string();
                    let after_name = skip_ws(bytes, p);
                    if after_name < len && bytes[after_name] == b'=' {
                        let value_start = skip_ws(bytes, after_name + 1);
                        let (value, next_p) = parse_attr_value(html, bytes, value_start);
                        attrs.insert(attr_name, value);
                        p = next_p;
                    } else {
                        // Boolean attribute: present, no value.
                        attrs.insert(attr_name, String::new());
                        p = after_name;
                    }
                }
            }
        }

        let end = p;
        out.push(Element {
            name,
            tag: html[lt..end].to_string(),
            attrs,
            start: lt,
            end,
            self_closing,
        });
        i = end;
    }

    out
}

fn skip_ws(bytes: &[u8], mut p: usize) -> usize {
    while p < bytes.len() && (bytes[p] as char).is_whitespace() {
        p += 1;
    }
    p
}

/// Parses one attribute value starting at `start` (already past any
/// whitespace following the `=`): double- or single-quoted (the value is
/// everything up to the matching quote, taken literally -- an
/// entity-escaped quote like `&quot;` contains no literal quote character
/// so it never ends the value early, and is left undecoded), or, with no
/// quote at all, unquoted (up to the next whitespace, `/`, or `>`).
/// Returns the value and the index just past it.
fn parse_attr_value(html: &str, bytes: &[u8], start: usize) -> (String, usize) {
    let len = bytes.len();
    if start >= len {
        return (String::new(), start);
    }
    let quote = bytes[start];
    if quote == b'"' || quote == b'\'' {
        let value_start = start + 1;
        let end = html[value_start..]
            .find(quote as char)
            .map(|p| value_start + p)
            .unwrap_or(len);
        let value = html[value_start..end].to_string();
        (value, (end + 1).min(len))
    } else {
        let mut p = start;
        while p < len {
            let c = bytes[p] as char;
            if c.is_whitespace() || c == '>' || c == '/' {
                break;
            }
            p += 1;
        }
        (html[start..p].to_string(), p)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_double_single_and_unquoted_values() {
        let html = r#"<div id="a" data-x='b' data-y=c>text</div>"#;
        let el = find_element(html, |_| true).expect("one element");
        assert_eq!(el.name, "div");
        assert_eq!(el.attrs.get("id").map(String::as_str), Some("a"));
        assert_eq!(el.attrs.get("data-x").map(String::as_str), Some("b"));
        assert_eq!(el.attrs.get("data-y").map(String::as_str), Some("c"));
    }

    #[test]
    fn parses_boolean_attributes() {
        let html = r#"<button disabled inert data-slot="x">Go</button>"#;
        let el = find_element(html, |_| true).unwrap();
        assert_eq!(el.attrs.get("disabled").map(String::as_str), Some(""));
        assert_eq!(el.attrs.get("inert").map(String::as_str), Some(""));
        assert_eq!(el.attrs.get("data-slot").map(String::as_str), Some("x"));
    }

    #[test]
    fn leaves_entity_escaped_quotes_undecoded() {
        let html = r#"<div title="a &quot;quoted&quot; word">x</div>"#;
        let el = find_element(html, |_| true).unwrap();
        assert_eq!(
            el.attrs.get("title").map(String::as_str),
            Some(r#"a &quot;quoted&quot; word"#)
        );
    }

    #[test]
    fn parses_self_closing_tags_and_gives_empty_inner_html() {
        let html = r#"<svg><path d="M0,0" data-slot="chart-arc"/></svg>"#;
        let el = find_element(html, |a| {
            a.get("data-slot").map(String::as_str) == Some("chart-arc")
        })
        .unwrap();
        assert_eq!(el.name, "path");
        assert!(el.tag.ends_with("/>"));
        assert_eq!(el.inner_html(html), "");
    }

    #[test]
    fn reordered_attributes_give_identical_results() {
        let a = r#"<div data-slot="chart-swatch" data-indicator="line" aria-hidden="true">A</div>"#;
        let b = r#"<div aria-hidden="true" data-indicator="line" data-slot="chart-swatch">A</div>"#;
        let el_a = find_element(a, |_| true).unwrap();
        let el_b = find_element(b, |_| true).unwrap();
        assert_eq!(el_a.attrs, el_b.attrs);
        assert_eq!(el_a.inner_html(a), el_b.inner_html(b));
    }

    #[test]
    fn inner_html_skips_over_nested_same_name_tags() {
        let html = r#"<div id="outer"><div id="inner">deep</div>tail</div>next"#;
        let el = find_element(html, |a| a.get("id").map(String::as_str) == Some("outer")).unwrap();
        assert_eq!(el.inner_html(html), r#"<div id="inner">deep</div>tail"#);
    }

    #[test]
    fn find_elements_returns_every_match_in_document_order() {
        let html = r#"<span data-slot="chart-swatch">1</span><span data-slot="chart-icon">2</span><span data-slot="chart-swatch">3</span>"#;
        let matches = find_elements(html, |a| {
            a.get("data-slot").map(String::as_str) == Some("chart-swatch")
        });
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].inner_html(html), "1");
        assert_eq!(matches[1].inner_html(html), "3");
    }

    #[test]
    fn skips_comments_and_doctypes() {
        let html = r#"<!DOCTYPE html><!-- a comment --><div id="x">y</div>"#;
        let el = find_element(html, |_| true).unwrap();
        assert_eq!(el.name, "div");
        assert_eq!(el.attrs.get("id").map(String::as_str), Some("x"));
    }
}

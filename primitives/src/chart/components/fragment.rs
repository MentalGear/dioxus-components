//! Fragment-safe ids: the one place a chart's (caller-suppliable, via
//! `ChartContainer`'s `id` prop) id is turned into something that may appear
//! in an SVG `id="..."`, an `href="#..."` or a `url(#...)` reference.
//!
//! Every id a chart emits for such a reference (the area gradient, the radial
//! arc-label text paths, any future `clipPath`/`mask`) MUST be built through
//! [`safe_fragment_id`] / [`fragment_url`] rather than formatted from the raw
//! chart id: a `%` starts a percent-escape in a URL fragment, and a space or
//! `)` terminates an unquoted `url(...)`, so a raw id containing one breaks the
//! reference silently (the paint falls back to nothing). Routing every site
//! through this helper makes that class of bug unable to recur per site.

/// `id` made safe to appear in a `url(#...)`/`href="#..."` fragment: every
/// character outside `[A-Za-z0-9-]` becomes `_` + its code point in hex +
/// `_`. `_` itself is escaped too and every escape is terminated, so the
/// mapping is injective: two different ids can never collapse to the same
/// fragment (`a%` -> `a_25_`, but a literal `a_25_` -> `a_5f_25_5f_`).
pub(crate) fn safe_fragment_id(id: &str) -> String {
    let mut out = String::with_capacity(id.len());
    for c in id.chars() {
        if c.is_ascii_alphanumeric() || c == '-' {
            out.push(c);
        } else {
            out.push_str(&format!("_{:x}_", c as u32));
        }
    }
    out
}

/// `url(#<safe id>)`, the paint-server reference form (`fill`, `stroke`,
/// `clip-path`, `mask`). The argument is the raw id; it is escaped here.
pub(crate) fn fragment_url(id: &str) -> String {
    format!("url(#{})", safe_fragment_id(id))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safe_fragment_id_escapes_fragment_breaking_characters() {
        assert_eq!(safe_fragment_id("abc-DEF09"), "abc-DEF09");
        assert_eq!(safe_fragment_id("a%b"), "a_25_b");
        assert_eq!(safe_fragment_id("a b)"), "a_20_b_29_");
        for c in safe_fragment_id("x%y#z (1)").chars() {
            assert!(c.is_ascii_alphanumeric() || c == '-' || c == '_', "{c}");
        }
    }

    #[test]
    fn safe_fragment_id_is_injective() {
        // `_` is escaped and every escape is terminated, so neither a literal
        // lookalike nor a following hex-digit character can collide.
        let ids = ["a%", "a_", "a_25_", "a%5", "a\u{255}", "a_25", "a%25", ""];
        for (i, a) in ids.iter().enumerate() {
            for b in &ids[i + 1..] {
                assert_ne!(safe_fragment_id(a), safe_fragment_id(b), "{a:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn fragment_url_never_contains_a_url_breaking_character() {
        assert_eq!(fragment_url("chart-1"), "url(#chart-1)");
        let u = fragment_url("my chart%1)");
        let inner = &u["url(#".len()..u.len() - 1];
        assert!(
            inner
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
            "{u}"
        );
        assert_eq!(u, "url(#my_20_chart_25_1_29_)");
    }
}

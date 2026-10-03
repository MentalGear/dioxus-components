//! The variant source a user sees and copies, as it reads *in their app*.
//!
//! A variant's `mod.rs` lives in this repo at
//! `components/<name>/variants/<variant>/mod.rs`, so it imports its component
//! with `use super::super::component::*;`. `dx components add <name>` installs
//! the component (but not `variants/`) as `src/components/<name>/` with a
//! `mod.rs` of `mod component; pub use component::*;`, so in the user's
//! crate the same names live at `crate::components::<name>`, and the
//! repo-relative path does not compile once the variant is pasted into an app.
//!
//! [`installed`] is the one place that rewrites it. It works on the already
//! highlighted source (the highlighting is embedded at compile time, and the
//! wasm bundle carries no Rust grammar to redo it): the text is rewritten and
//! every highlight span is remapped, so the result is colored exactly like the
//! original. Both the code tabs of the component pages and the charts
//! gallery's Copy / View Code go through it, so none of them can drift.

use crate::HighlightedCode;
use dioxus_code::advanced::{HighlightSpan, HighlightedSource};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

/// How a variant file reaches its component inside this repo.
const REPO_PATH: [&str; 3] = ["super", "super", "component"];

/// `crate::components::<owner>` as path segments: the same number of
/// segments as [`REPO_PATH`], which is what lets the highlighting carry over
/// segment by segment.
fn installed_path(owner: &str) -> [String; 3] {
    ["crate".into(), "components".into(), owner.into()]
}

/// A run of the path (a segment or a `::`) before and after the rewrite.
struct Piece {
    old_start: usize,
    old_len: usize,
    new_start: usize,
    new_len: usize,
}

fn pieces(owner: &str) -> Vec<Piece> {
    let new = installed_path(owner);
    let (mut old_at, mut new_at) = (0, 0);
    let mut out = Vec::new();
    for (index, (old, new)) in REPO_PATH.iter().zip(&new).enumerate() {
        if index > 0 {
            out.push(Piece {
                old_start: old_at,
                old_len: 2,
                new_start: new_at,
                new_len: 2,
            });
            old_at += 2;
            new_at += 2;
        }
        out.push(Piece {
            old_start: old_at,
            old_len: old.len(),
            new_start: new_at,
            new_len: new.len(),
        });
        old_at += old.len();
        new_at += new.len();
    }
    out
}

/// Rewrites every `super::super::component` path of `source` to
/// `crate::components::<owner>` and moves `spans` along with the text.
/// Returns `None` when the text has no such path.
fn rewrite(
    source: &str,
    spans: &[HighlightSpan],
    owner: &str,
) -> Option<(String, Vec<HighlightSpan>)> {
    let old_path = REPO_PATH.join("::");
    let new_path = installed_path(owner).join("::");
    let is_ident = |c: char| c.is_alphanumeric() || c == '_';
    let found: Vec<usize> = source
        .match_indices(&old_path)
        .filter(|(at, _)| {
            // A whole path only: not the tail of `x::super::...` nor the
            // head of `...::components`.
            let before = source[..*at].chars().next_back();
            let after = source[at + old_path.len()..].chars().next();
            !before.is_some_and(|c| is_ident(c) || c == ':') && !after.is_some_and(is_ident)
        })
        .map(|(at, _)| at)
        .collect();
    if found.is_empty() {
        return None;
    }

    let mut text = String::with_capacity(source.len() + found.len() * 16);
    let mut last = 0;
    for &old_start in &found {
        text.push_str(&source[last..old_start]);
        text.push_str(&new_path);
        last = old_start + old_path.len();
    }
    text.push_str(&source[last..]);

    let pieces = pieces(owner);
    let delta = new_path.len() as i64 - old_path.len() as i64;
    let map = |position: usize| -> u32 {
        let mut shift = 0i64;
        for &old_start in &found {
            let end = old_start + old_path.len();
            if position <= old_start {
                break;
            }
            if position >= end {
                shift += delta;
                continue;
            }
            // Inside a rewritten path: into the matching piece (a position on
            // a segment's end is the start of the following `::`, so it lands
            // on the new segment's end).
            let relative = position - old_start;
            let piece = pieces
                .iter()
                .find(|p| p.old_start <= relative && relative < p.old_start + p.old_len)
                .expect("the pieces cover the whole path");
            let inner = (relative - piece.old_start).min(piece.new_len);
            return (old_start as i64 + shift + piece.new_start as i64 + inner as i64) as u32;
        }
        (position as i64 + shift) as u32
    };

    let spans = spans
        .iter()
        .filter_map(|span| {
            let (start, end) = (map(span.start() as usize), map(span.end() as usize));
            // Only a span that collapsed to nothing is dropped; one that was
            // already empty (the highlighter emits a few) stays as it was.
            let was_empty = span.start() == span.end();
            (was_empty || start < end).then(|| HighlightSpan::from_offsets(start, end, span.tag()))
        })
        .collect();
    Some((text, spans))
}

type Cache = Mutex<HashMap<(&'static str, &'static str), HighlightedSource>>;

/// `code` as the user gets it from `dx components add <owner>`: the repo's
/// `super::super::component` import replaced by `crate::components::<owner>`
/// (see the module docs). `variant` only keys the cache: the rewritten text
/// has to be `'static` for [`HighlightedSource`], so it is built, and leaked,
/// once per (owner, variant) rather than once per render.
pub fn installed(
    owner: &'static str,
    variant: &'static str,
    code: &HighlightedCode,
) -> HighlightedCode {
    static CACHE: OnceLock<Cache> = OnceLock::new();
    let cache = CACHE.get_or_init(Default::default);
    let mut cache = cache
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    if let Some(cached) = cache.get(&(owner, variant)) {
        return HighlightedCode {
            source: cached.clone(),
        };
    }
    let source = &code.source;
    let rewritten = match rewrite(source.source(), source.spans(), owner) {
        Some((text, spans)) => HighlightedSource::from_static_parts(
            Box::leak(text.into_boxed_str()),
            source.language(),
            Box::leak(spans.into_boxed_slice()),
        ),
        None => source.clone(),
    };
    cache.insert((owner, variant), rewritten.clone());
    HighlightedCode { source: rewritten }
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus_code::Language;

    fn span(range: std::ops::Range<u32>, tag: &'static str) -> HighlightSpan {
        HighlightSpan::new(range, tag)
    }

    #[test]
    fn rewrites_the_import_and_keeps_every_token_colored() {
        let source = "use super::super::component::*;\nfn x() {}\n";
        //            0         1         2         3
        //            0123456789012345678901234567890
        let spans = [
            span(0..3, "k"),   // use
            span(4..9, "k"),   // super
            span(9..11, "p"),  // ::
            span(11..16, "k"), // super
            span(16..18, "p"), // ::
            span(18..27, "m"), // component
            span(27..29, "p"), // ::
            span(29..30, "o"), // *
            span(30..31, "p"), // ;
            span(32..34, "k"), // fn
        ];
        let (text, spans) = rewrite(source, &spans, "bar_chart").unwrap();
        assert_eq!(text, "use crate::components::bar_chart::*;\nfn x() {}\n");
        let tokens: Vec<_> = spans
            .iter()
            .map(|s| (&text[s.start() as usize..s.end() as usize], s.tag()))
            .collect();
        assert_eq!(
            tokens,
            [
                ("use", "k"),
                ("crate", "k"),
                ("::", "p"),
                ("components", "k"),
                ("::", "p"),
                ("bar_chart", "m"),
                ("::", "p"),
                ("*", "o"),
                (";", "p"),
                ("fn", "k"),
            ]
        );
    }

    #[test]
    fn rewrites_braced_imports_and_leaves_other_paths_alone() {
        let source = "use super::super::component::{A, B};\nuse super::super::component_x::*;\nuse a::super::super::component::*;\n";
        let (text, _) = rewrite(source, &[], "card").unwrap();
        assert_eq!(
            text,
            "use crate::components::card::{A, B};\nuse super::super::component_x::*;\nuse a::super::super::component::*;\n"
        );
        assert!(rewrite("use dioxus::prelude::*;\n", &[], "card").is_none());
    }

    #[test]
    fn installed_is_cached_and_a_noop_without_the_import() {
        let plain = HighlightedCode {
            source: HighlightedSource::from_static_parts("fn x() {}", Language::Rust, &[]),
        };
        assert_eq!(installed("unit", "plain", &plain).source, plain.source);

        let repo = HighlightedCode {
            source: HighlightedSource::from_static_parts(
                "use super::super::component::*;",
                Language::Rust,
                &[],
            ),
        };
        let first = installed("unit", "repo", &repo);
        assert_eq!(first.source.source(), "use crate::components::unit::*;");
        // Same allocation the second time: nothing is leaked per call.
        let second = installed("unit", "repo", &repo);
        assert_eq!(
            first.source.source().as_ptr(),
            second.source.source().as_ptr()
        );
    }

    /// For every real demo variant: no repo-relative import survives, the
    /// text differs from the original only in the rewritten path, every span
    /// stays inside the text on char boundaries, and every token outside the
    /// rewritten path keeps its text and color.
    #[test]
    fn every_demo_variant_installs_cleanly() {
        let mut rewritten = 0;
        for demo in crate::components::DEMOS {
            for variant in demo.variants {
                let original = &variant.rs_highlighted.source;
                let code = installed(demo.name, variant.name, &variant.rs_highlighted);
                let text = code.source.source();
                let old_path = REPO_PATH.join("::");
                let new_path = installed_path(demo.name).join("::");
                assert!(
                    !text.contains(&format!("use {old_path}")),
                    "{}/{}: repo-relative import left",
                    demo.name,
                    variant.name
                );
                assert_eq!(
                    text.replace(&new_path, &old_path),
                    original.source().replace(&new_path, &old_path),
                    "{}/{}: only the import path may change",
                    demo.name,
                    variant.name
                );
                rewritten += usize::from(text != original.source());
                let new_spans = code.source.spans();
                for span in new_spans {
                    assert!(
                        text.get(span.start() as usize..span.end() as usize)
                            .is_some(),
                        "{}/{}: span out of bounds",
                        demo.name,
                        variant.name
                    );
                }
                // Every token outside the rewritten paths keeps its text and
                // color, in order.
                let delta = new_path.len() as i64 - old_path.len() as i64;
                let old_regions: Vec<_> = original
                    .source()
                    .match_indices(&old_path)
                    .map(|(at, _)| (at, at + old_path.len()))
                    .collect();
                let new_regions: Vec<_> = old_regions
                    .iter()
                    .enumerate()
                    .map(|(index, (start, _))| {
                        let start = (*start as i64 + index as i64 * delta) as usize;
                        (start, start + new_path.len())
                    })
                    .collect();
                let outside =
                    |source: &str, spans: &[HighlightSpan], regions: &[(usize, usize)]| {
                        spans
                            .iter()
                            .filter(|s| {
                                !regions.iter().any(|(start, end)| {
                                    (s.start() as usize) < *end && (s.end() as usize) > *start
                                })
                            })
                            .map(|s| {
                                (
                                    source[s.start() as usize..s.end() as usize].to_string(),
                                    s.tag(),
                                )
                            })
                            .collect::<Vec<_>>()
                    };
                assert_eq!(
                    outside(text, new_spans, &new_regions),
                    outside(original.source(), original.spans(), &old_regions),
                    "{}/{}: highlighting outside the import drifted",
                    demo.name,
                    variant.name
                );
            }
        }
        assert!(rewritten > 100, "expected most variants to be rewritten");
    }
}

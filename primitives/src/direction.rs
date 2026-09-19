//! Text/layout direction (LTR/RTL) — matches Radix's `@radix-ui/react-direction`
//! (`packages/react/direction/src/direction.tsx`, `radix-ui/primitives` commit
//! `f7ecd5ab16f5e1e820eb5786a1419a98a2d594ae`) and, per
//! `docs/recommended-implementations.md` §9, takes
//! `dignifiedquire/dx-components`'s own Rust port of it
//! (`primitives/src/direction.rs`, MIT OR Apache-2.0, commit
//! `5af3cc292559a0e8d73c7b9a827c4ca08ef34d99`) as-is for the context/provider
//! shape, renaming its `DirectionProvider` field from `dir` to `direction`
//! (this crate's own choice, to avoid a name clash with the per-component
//! `dir: Option<Direction>` *prop* every RTL-aware root also exposes).
//!
//! # What this module is, and isn't
//!
//! [`Direction`] + [`DirectionProvider`] + [`use_direction`] let an app
//! declare "everything under here is RTL" once, at any ancestor, the same
//! way Radix's own `DirectionProvider` does; every RTL-aware component in
//! this crate also accepts a local `dir: Option<Direction>` prop that wins
//! over the ambient context, matching upstream's `useDirection(localDir?)`
//! exactly.
//!
//! [`Direction::resolve_horizontal`] is this crate's construction, not
//! upstream's: Radix has one shared `RovingFocusGroup` primitive whose own
//! `getDirectionAwareKey`/`MAP_KEY_TO_FOCUS_INTENT`
//! (`packages/react/roving-focus/src/roving-focus-group.tsx`, same pinned
//! commit, lines 348–365) every direction-aware Radix component (Tabs,
//! RadioGroup, ToggleGroup, Toolbar, Menubar) delegates to. This crate has
//! no equivalent single roving-focus component (each of those six files
//! hand-rolls its own `onkeydown` over `crate::collection`'s index-based
//! engine) — `resolve_horizontal` is the one shared piece that exists here
//! instead, so every one of those handlers (plus the menu family's
//! submenu-open/close keys, plus `slider.rs`'s keyboard mapping) calls it
//! rather than each hand-writing its own `Key::ArrowLeft`/`Key::ArrowRight`
//! swap. See `$S/batch3/rtl-rust/reference.md` (this lane's own report) for
//! the full per-component table this was derived from.
//!
//! Deliberately **not reactive**: like upstream's own
//! `use_context_provider(|| props.dir)`, [`DirectionProvider`] captures its
//! `direction` prop once, at mount — a later render with a *different*
//! `direction` value does not update already-mounted descendants' context
//! read. This matches every fixture this lane's own oracle needs (a fixed
//! direction for that page's lifetime); a "toggle direction live" demo
//! would need a `Signal`-based variant instead, not built here (see the
//! reference doc's §1 for the full tradeoff).

use dioxus::prelude::*;

/// Layout direction for keyboard navigation, content flow, and the native
/// `dir` HTML attribute.
///
/// Matches Radix's `Direction = 'ltr' | 'rtl'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Direction {
    /// Left-to-right (the default).
    #[default]
    Ltr,
    /// Right-to-left.
    Rtl,
}

impl Direction {
    /// Returns `"ltr"` or `"rtl"` — the exact string the `dir` HTML
    /// attribute (and this crate's `data-direction` styling hook) expects.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ltr => "ltr",
            Self::Rtl => "rtl",
        }
    }

    /// Resolve a horizontal arrow key (`ArrowLeft`/`ArrowRight`) to the
    /// navigation intent it expresses in this direction, or `None` for
    /// every other key (including `ArrowUp`/`ArrowDown`, which are never
    /// direction-dependent — see the module doc).
    ///
    /// Matches Radix's `getDirectionAwareKey` composed with
    /// `MAP_KEY_TO_FOCUS_INTENT`, restricted to the two horizontal keys:
    /// `ArrowLeft` is [`HorizontalNav::Prev`] in LTR and
    /// [`HorizontalNav::Next`] in RTL; `ArrowRight` is the mirror image.
    /// Every consumer already gates this on its own `horizontal`/
    /// orientation flag the same way it did before this call existed —
    /// this function does not know or care about orientation.
    ///
    /// ```
    /// use dioxus_primitives::direction::{Direction, HorizontalNav};
    /// use dioxus::prelude::Key;
    ///
    /// assert_eq!(Direction::Ltr.resolve_horizontal(&Key::ArrowLeft), Some(HorizontalNav::Prev));
    /// assert_eq!(Direction::Ltr.resolve_horizontal(&Key::ArrowRight), Some(HorizontalNav::Next));
    /// assert_eq!(Direction::Rtl.resolve_horizontal(&Key::ArrowLeft), Some(HorizontalNav::Next));
    /// assert_eq!(Direction::Rtl.resolve_horizontal(&Key::ArrowRight), Some(HorizontalNav::Prev));
    /// assert_eq!(Direction::Ltr.resolve_horizontal(&Key::ArrowUp), None);
    /// ```
    pub fn resolve_horizontal(self, key: &Key) -> Option<HorizontalNav> {
        match (self, key) {
            (Self::Ltr, Key::ArrowLeft) | (Self::Rtl, Key::ArrowRight) => Some(HorizontalNav::Prev),
            (Self::Ltr, Key::ArrowRight) | (Self::Rtl, Key::ArrowLeft) => Some(HorizontalNav::Next),
            _ => None,
        }
    }
}

/// The navigation intent a horizontal arrow key expresses, after resolving
/// [`Direction`]. See [`Direction::resolve_horizontal`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HorizontalNav {
    /// Move to the previous item (LTR `ArrowLeft` / RTL `ArrowRight`).
    Prev,
    /// Move to the next item (LTR `ArrowRight` / RTL `ArrowLeft`).
    Next,
}

/// Props for [`DirectionProvider`].
#[derive(Props, Clone, PartialEq)]
pub struct DirectionProviderProps {
    /// The text direction every descendant resolves to, unless it has its
    /// own local `dir` override.
    pub direction: Direction,
    /// Children that inherit this direction.
    pub children: Element,
}

/// Provides an ambient [`Direction`] to every descendant, matching Radix's
/// `DirectionProvider`.
///
/// Every RTL-aware component in this crate resolves its effective direction
/// via [`use_direction`], which checks this context only after its own
/// local `dir` prop. See the module doc for why this provider is not
/// reactive to a later change of its `direction` prop.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::direction::{Direction, DirectionProvider};
/// use dioxus_primitives::tabs::{TabContent, TabList, TabTrigger, Tabs};
///
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         DirectionProvider { direction: Direction::Rtl,
///             Tabs { default_value: "a".to_string(),
///                 TabList {
///                     TabTrigger { value: "a".to_string(), index: 0usize, "A" }
///                 }
///                 TabContent { index: 0usize, value: "a".to_string(), "A content" }
///             }
///         }
///     }
/// }
/// ```
#[component]
pub fn DirectionProvider(props: DirectionProviderProps) -> Element {
    use_context_provider(|| props.direction);
    props.children
}

/// Resolve the effective [`Direction`]: `local` if given, else the nearest
/// [`DirectionProvider`] ancestor's, else [`Direction::Ltr`].
///
/// Matches Radix's `useDirection(localDir?)` exactly.
pub fn use_direction(local: Option<Direction>) -> Direction {
    if let Some(direction) = local {
        return direction;
    }
    try_consume_context::<Direction>().unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn resolve_horizontal_ltr_unchanged() {
        assert_eq!(
            Direction::Ltr.resolve_horizontal(&Key::ArrowLeft),
            Some(HorizontalNav::Prev)
        );
        assert_eq!(
            Direction::Ltr.resolve_horizontal(&Key::ArrowRight),
            Some(HorizontalNav::Next)
        );
    }

    #[test]
    fn resolve_horizontal_rtl_swaps_left_and_right() {
        assert_eq!(
            Direction::Rtl.resolve_horizontal(&Key::ArrowLeft),
            Some(HorizontalNav::Next)
        );
        assert_eq!(
            Direction::Rtl.resolve_horizontal(&Key::ArrowRight),
            Some(HorizontalNav::Prev)
        );
    }

    #[test]
    fn resolve_horizontal_ignores_every_other_key_both_directions() {
        for direction in [Direction::Ltr, Direction::Rtl] {
            assert_eq!(direction.resolve_horizontal(&Key::ArrowUp), None);
            assert_eq!(direction.resolve_horizontal(&Key::ArrowDown), None);
            assert_eq!(direction.resolve_horizontal(&Key::Home), None);
            assert_eq!(direction.resolve_horizontal(&Key::End), None);
            assert_eq!(direction.resolve_horizontal(&Key::Enter), None);
            assert_eq!(direction.resolve_horizontal(&Key::Escape), None);
            assert_eq!(
                direction.resolve_horizontal(&Key::Character(" ".to_string())),
                None
            );
        }
    }

    #[test]
    fn as_str_matches_the_dir_attribute_tokens() {
        assert_eq!(Direction::Ltr.as_str(), "ltr");
        assert_eq!(Direction::Rtl.as_str(), "rtl");
    }

    #[test]
    fn default_is_ltr() {
        assert_eq!(Direction::default(), Direction::Ltr);
    }

    #[test]
    fn use_direction_prefers_local_override_over_context() {
        // No provider in scope at all: falls back to `Ltr`.
        assert_eq!(with_runtime(|| use_direction(None)), Direction::Ltr);

        // A local override always wins, provider or not.
        assert_eq!(
            with_runtime(|| use_direction(Some(Direction::Rtl))),
            Direction::Rtl
        );
    }

    /// Run a closure inside a Dioxus runtime context so `try_consume_context`
    /// is callable -- mirrors `slider.rs`'s own `with_runtime` test harness.
    fn with_runtime(f: impl Fn() -> Direction + 'static) -> Direction {
        let result = Rc::new(Cell::new(Direction::Ltr));
        let result2 = result.clone();
        let test_fn = Rc::new(f);
        let mut dom = VirtualDom::new_with_props(
            move |props: TestHarnessProps| {
                result2.set((props.test_fn)());
                rsx! { div {} }
            },
            TestHarnessProps { test_fn },
        );
        dom.rebuild_in_place();
        result.get()
    }

    #[derive(Clone, Props)]
    struct TestHarnessProps {
        test_fn: Rc<dyn Fn() -> Direction>,
    }

    impl PartialEq for TestHarnessProps {
        fn eq(&self, _: &Self) -> bool {
            true
        }
    }
}

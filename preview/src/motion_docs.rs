//! The `/docs` page's "Motion" section: how animation works in the components, in general, and
//! what each component does while nobody can see it.
//!
//! The same content lives in `dev-docs/motion.md`. Prose is written twice (once for a maintainer,
//! once for an app author), but every TABLE is shown from the constants below and
//! `motion_tables_match_motion_md` fails when a row stops appearing verbatim in the markdown, and
//! the code examples are read from the markdown itself, so the two cannot drift on the facts that
//! change most (a component's gating, a token's value).

use crate::DocsPre;
use dioxus::prelude::*;

/// Renders `text` with every `` `backtick span` `` as `<code>`. The tables are written as markdown
/// cells (backticks for code) so the same string is the markdown row and the page cell.
fn inline_code(text: &str) -> Element {
    let parts: Vec<(usize, String)> = text
        .split('`')
        .enumerate()
        .map(|(i, part)| (i, part.to_string()))
        .collect();
    rsx! {
        for (i , part) in parts {
            if i % 2 == 1 {
                code { key: "{i}", "{part}" }
            } else {
                Fragment { key: "{i}", "{part}" }
            }
        }
    }
}

/// A prose table (`.dx-docs-table-scroll` is the scroll container, so a wide table scrolls inside
/// its own box at phone widths instead of overflowing the page).
fn docs_table(head: &[&str], rows: &[&[&str]]) -> Element {
    let head: Vec<String> = head.iter().map(|h| h.to_string()).collect();
    let rows: Vec<Vec<String>> = rows
        .iter()
        .map(|r| r.iter().map(|c| c.to_string()).collect())
        .collect();
    rsx! {
        div { class: "dx-docs-table-scroll",
            table {
                thead {
                    tr {
                        for h in head {
                            th { key: "{h}", "{h}" }
                        }
                    }
                }
                tbody {
                    for (ri , row) in rows.into_iter().enumerate() {
                        tr { key: "{ri}",
                            for (ci , cell) in row.into_iter().enumerate() {
                                td { key: "{ci}", {inline_code(&cell)} }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The token table of `dev-docs/motion.md`, one row per token.
pub(crate) const MOTION_TOKENS: &[[&str; 3]] = &[
    [
        r#"`--dx-motion-duration-fast`"#,
        r#"`100ms`"#,
        r#"hover and highlight colour changes, hover-card fade, menu item backgrounds"#,
    ],
    [
        r#"`--dx-motion-duration-base`"#,
        r#"`150ms`"#,
        r#"menu and picker fades, chevron rotation, tab colours, drawer drag settle"#,
    ],
    [
        r#"`--dx-motion-duration-slow`"#,
        r#"`200ms`"#,
        r#"tooltip, toast, navigation slides, carousel arrow fade, the overlay default below"#,
    ],
    [
        r#"`--dx-motion-duration-slower`"#,
        r#"`300ms`"#,
        r#"accordion open/close, sheet close"#,
    ],
    [
        r#"`--dx-motion-duration-reduced`"#,
        r#"`0.01ms`"#,
        r#"the `prefers-reduced-motion` replacement duration (not `0ms`: a zero-duration transition can still misfire `transitionend` listeners in some engines)"#,
    ],
    [r#"`--dx-motion-ease`"#, r#"`ease`"#, r#"the default curve"#],
    [r#"`--dx-motion-ease-in`"#, r#"`ease-in`"#, r#"exits"#],
    [r#"`--dx-motion-ease-out`"#, r#"`ease-out`"#, r#"entrances"#],
    [
        r#"`--dx-motion-ease-standard`"#,
        r#"`cubic-bezier(0.4, 0, 0.2, 1)`"#,
        r#"icon rotation (chevrons)"#,
    ],
];

/// The hook table of `dev-docs/motion.md`.
pub(crate) const MOTION_HOOKS: &[[&str; 3]] = &[
    [
        r#"`use_motion()`"#,
        r#"a CSS loop on an element the component renders"#,
        r#"Returns a `Motion`. Spread `motion.attributes()` on the element. While it is outside the viewport plus `ROOT_MARGIN` (`200px` pre-roll, so it is already moving when it scrolls in), inside a skipped `content-visibility` subtree, or in a hidden tab, the element carries `data-dx-motion="paused"`."#,
    ],
    [
        r#"`use_motion_when(enabled)`"#,
        r#"a loop that only sometimes exists (indeterminate progress, a still-loading avatar, a shimmering marker)"#,
        r#"Same, but subscribes nothing while `enabled()` is false."#,
    ],
    [
        r#"`use_motion_active_for(element_id)`"#,
        r#"a timer beside an element the component does not own (the carousel's autoplay watches its scroller)"#,
        r#"Returns a `ReadSignal<bool>` for the element whose DOM id is given."#,
    ],
    [
        r#"`use_interval_while(active, period, on_tick)`"#,
        r#"a repeating timer"#,
        r#"Drops its task while `active` is false (no wakeups at all); when it turns true the loop restarts with a fresh full `period`, no catch-up tick. In `primitives/src/interval.rs`."#,
    ],
    [
        r#"`use_timeout_while(active, delay, on_fire)`"#,
        r#"a one-shot countdown"#,
        r#"If `active` goes false before it fires, the countdown is dropped and restarts in full on return. Never fires twice."#,
    ],
    [
        r#"`use_entered_view_when(enabled)`"#,
        r#"one-shot entrances (chart load animations)"#,
        r#"Returns an `EnterView`; `entered()` turns true once, when at least `ENTER_RATIO` (40%) of the element is on screen (or 40% of the viewport height for a taller element) with the tab visible. No pre-roll: an entrance that starts before the element is on screen is one nobody saw. The observer lets go after it fires."#,
    ],
    [
        r#"`use_document_visible()`"#,
        r#"timers that carry meaning (a toast's auto-dismiss)"#,
        r#"Follows the tab only and ignores scroll position."#,
    ],
];

/// The per-component table of `dev-docs/motion.md`.
pub(crate) const MOTION_TABLE: &[[&str; 4]] = &[
    [
        r#"Spinner"#,
        r#"the ring rotates, `transform`, 1 s linear, forever"#,
        r#"pause (`use_motion`)"#,
        r#"override `animation-duration` or set `animation: none` on `.dx-spinner svg`; under reduced motion the ring is a still arc (the theme's `[data-dx-motion-key]` rule)"#,
    ],
    [
        r#"Skeleton"#,
        r#"opacity pulse, 2 s, forever"#,
        r#"pause (`use_motion`)"#,
        r#"override `animation` on `.dx-skeleton`; under reduced motion it is a still block (the theme's `[data-dx-motion-key]` rule)"#,
    ],
    [
        r#"Progress"#,
        r#"indeterminate bar sweeps, `translateX` 1 s linear forever; the determinate fill animates `width` for 250 ms"#,
        r#"pause while indeterminate (`use_motion_when`); a determinate bar subscribes to nothing"#,
        r#"`animation` on `.dx-progress-indicator`; the width transition is a literal in `progress/style.css`; under reduced motion the sweep stops and the bar shows diagonal stripes across the whole track (a still half-width bar would read as 50%)"#,
    ],
    [
        r#"Avatar (loading)"#,
        r#"placeholder opacity pulse, 1.5 s, only while the image is loading"#,
        r#"pause while loading (`use_motion_when`); nothing is subscribed once loaded"#,
        r#"`animation` on `.dx-avatar[data-state="loading"]`; under reduced motion the placeholder is still (the theme's `[data-dx-motion-key]` rule)"#,
    ],
    [
        r#"Carousel autoplay"#,
        r#"the timer advancing a slide every `delay_ms`"#,
        r#"stop (`use_motion_active_for`; fresh full delay on return); it also stops on hover, focus, interaction and the user's pause"#,
        r#"`CarouselAutoplay { delay_ms, default_playing, stop_on_mouse_enter, stop_on_interaction }`; leave `CarouselAutoplay` out for none; reduced motion keeps it off at mount"#,
    ],
    [
        r#"Carousel arrows and dots"#,
        r#"the arrow at either end fades; the active dot widens"#,
        r#"exempt"#,
        r#"`nav_disabled_opacity`, `--dx-carousel-nav-disabled-opacity`, `--dx-carousel-nav-disabled-visibility`; `--dx-motion-duration-slow`"#,
    ],
    [
        r#"Chart entrance"#,
        r#"bars grow, lines and areas are revealed, pies and radials sweep, once (500 to 1500 ms)"#,
        r#"waits for view (`use_entered_view_when`: 40% visible, no pre-roll, tab visible)"#,
        r#"`animate: false`; off under reduced motion; the durations are literals in `chart/style.css`; the hover transitions use `--dx-motion-duration-base` and `--dx-motion-duration-slow`"#,
    ],
    [
        r#"Toast"#,
        r#"slide-in and restack transitions; the auto-dismiss countdown"#,
        r#"stop the countdown while the tab is hidden, full duration on return (`use_document_visible` + `use_timeout_while`); scrolling never changes it"#,
        r#"`duration` and `permanent` on the toast; `--dx-motion-duration-slow` for the slide"#,
    ],
    [
        r#"Dialog, AlertDialog, CommandDialog"#,
        r#"scrim fade and panel in/out; the dialog stays open until the exit settles, then `close()`"#,
        r#"exempt"#,
        r#"`--dx-overlay-duration`, `--dx-overlay-ease`, `--dx-overlay-scrim`, `--dx-overlay-blur`; `overlay: false` drops the scrim"#,
    ],
    [
        r#"Sheet"#,
        r#"slides from its edge, 500 ms in and `--dx-motion-duration-slower` out"#,
        r#"exempt"#,
        r#"the `dx-slide-*` durations in `sheet/style.css`; the scrim through `--dx-overlay-*`"#,
    ],
    [
        r#"Drawer"#,
        r#"slides from its edge (`--dx-motion-duration-slow` in, `--dx-motion-duration-base` out); the panel follows the pointer while dragged"#,
        r#"exempt"#,
        r#"`--dx-motion-duration-*`, `--dx-overlay-*`; the drag offset is the `translate` property"#,
    ],
    [
        r#"Popover"#,
        r#"fade in and out, `--dx-overlay-duration`"#,
        r#"exempt"#,
        r#"`--dx-overlay-duration`, `--dx-overlay-ease`"#,
    ],
    [
        r#"Dropdown menu, context menu, menubar"#,
        r#"the menu panel fades and scales in (`--dx-motion-duration-base`), item highlights (`--dx-motion-duration-fast`)"#,
        r#"exempt"#,
        r#"`--dx-motion-duration-fast`, `--dx-motion-duration-base`, `--dx-motion-ease-in`, `--dx-motion-ease-out`"#,
    ],
    [
        r#"Select, Combobox"#,
        r#"the list fades in (`--dx-motion-duration-base`) and out (`--dx-motion-duration-fast`)"#,
        r#"exempt"#,
        r#"the same tokens; the `dx-picker-in` and `dx-picker-out` keyframes"#,
    ],
    [
        r#"Tooltip, Hover card"#,
        r#"fade in and out (tooltip `--dx-motion-duration-slow`, hover card `--dx-motion-duration-fast`)"#,
        r#"exempt"#,
        r#"the same tokens; reduced motion swaps in `--dx-motion-duration-reduced`"#,
    ],
    [
        r#"Navigation menu, Navbar"#,
        r#"the open content slides in (`--dx-motion-duration-slow`) and out (`--dx-motion-duration-base`); the chevron rotates"#,
        r#"exempt"#,
        r#"`--dx-motion-duration-slow`, `--dx-motion-duration-base`, `--dx-motion-ease-standard`"#,
    ],
    [
        r#"Accordion, Collapsible"#,
        r#"accordion panel height opens and closes (`--dx-motion-duration-slower`) and the chevron rotates; Collapsible just shows and hides"#,
        r#"exempt"#,
        r#"`--dx-motion-duration-slower`; a find-in-page reveal skips the animation by design"#,
    ],
    [
        r#"Tabs"#,
        r#"trigger colours and the ghost underline fade (`--dx-motion-duration-base`)"#,
        r#"exempt"#,
        r#"`--dx-motion-duration-base`, `--dx-motion-ease-standard`"#,
    ],
    [
        r#"Drag and drop list"#,
        r#"the dragged item stays as a ghost; the items after the slot part by a `transform` transition"#,
        r#"exempt (only runs during a drag)"#,
        r#"`ghost_opacity` and `drop_gap`, or `--dx-dnd-ghost-opacity` and `--dx-dnd-drop-gap`; instant under reduced motion"#,
    ],
    [
        r#"Date picker"#,
        r#"popover fade; after a selection it waits `close_delay` so the click visibly registered, then plays the normal exit"#,
        r#"exempt"#,
        r#"`close_delay` (default `300ms`, `Duration::ZERO` closes at once), `close_on_select: false`"#,
    ],
    [
        r#"Message scroller"#,
        r#"the jump button slides, scales and fades; the transcript is revealed once its scroll position is applied"#,
        r#"exempt"#,
        r#"`--dx-motion-duration-slow` for the show; the hide is a literal `0.4s` in `message_scroller/style.css`"#,
    ],
    [
        r#"Input OTP"#,
        r#"the caret blinks, `opacity`, 1 s step, forever, on the one focused slot only"#,
        r#"exempt (visible by definition; in `check-motion-gating.sh`'s `ALLOW`)"#,
        r#"`animation` on `.dx-input-otp-slot[data-active="true"]::after`; under reduced motion the caret is steady (`animation: none`)"#,
    ],
    [
        r#"Shimmer effect"#,
        r#"`.dx-shimmer` text highlight sweep (`background-position`, paint-only; the one `ALLOW` entry of the compositor-only gate)"#,
        r#"pause when the host applies `use_motion_when` (Marker content, Attachment title) or an ancestor carries `data-dx-motion="paused"`; a bare `.dx-shimmer` elsewhere needs `use_motion()` on its host"#,
        r#"`--dx-shimmer-duration`, `--dx-shimmer-color`, `--dx-shimmer-spread`, `--dx-shimmer-angle`; `dx-shimmer-once`, `dx-shimmer-reverse`, `dx-shimmer-none`; off under reduced motion"#,
    ],
];

/// `dev-docs/motion.md`, the source of the code examples below. The examples are taken from it at
/// run time (instead of being copied into this file) so they cannot drift, and because they show an
/// app author's imports, which `scripts/check-preview-composition.sh` rightly refuses in preview
/// markup.
const MOTION_MD: &str = include_str!("../../dev-docs/motion.md");

/// The `n`th fenced code block of `dev-docs/motion.md` (0-based), without its fences.
fn md_code_block(n: usize) -> String {
    let mut blocks = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    for line in MOTION_MD.lines() {
        if line.trim_start().starts_with("```") {
            match current.take() {
                Some(done) => blocks.push(done.join("\n")),
                None => current = Some(Vec::new()),
            }
        } else if let Some(lines) = current.as_mut() {
            lines.push(line);
        }
    }
    blocks.get(n).cloned().unwrap_or_default()
}

/// The token override example (the first block).
fn tokens_css_example() -> String {
    md_code_block(0)
}

/// The "your own animated component" Rust example (the second block).
fn own_component_rust_example() -> String {
    md_code_block(1)
}

/// Its stylesheet (the third block).
fn own_component_css_example() -> String {
    md_code_block(2)
}

/// The Motion section's body (the `h2` is rendered by `Docs()`, so it can be a `DOCS_SECTIONS` entry
/// with an anchor and a sidebar sub-item like the others).
#[component]
pub(crate) fn MotionDocs() -> Element {
    let tokens: Vec<&[&str]> = MOTION_TOKENS.iter().map(|r| r.as_slice()).collect();
    let hooks: Vec<&[&str]> = MOTION_HOOKS.iter().map(|r| r.as_slice()).collect();
    let components: Vec<&[&str]> = MOTION_TABLE.iter().map(|r| r.as_slice()).collect();
    let tokens_css = tokens_css_example();
    let own_rust = own_component_rust_example();
    let own_css = own_component_css_example();
    rsx! {
        p {
            {inline_code("Animation here is a few tokens, one reduced-motion rule set and one answer to \"can anyone see this?\", so a page full of live demos stays cheap. The same notes are in the repository as `dev-docs/motion.md`.")}
        }

        h3 { id: "motion-tokens", "Motion tokens" }
        p {
            {inline_code("Speeds and curves are custom properties on `:root` in `dx-components-theme.css`. Components read them and carry no fallback literals, so changing a token retunes every component that uses it.")}
        }
        {docs_table(&["Token", "Default", "Used for"], &tokens)}
        p {
            {inline_code("Modal surfaces share one more set, so a scrim and its panel cannot disagree: `--dx-overlay-duration` (defaults to `var(--dx-motion-duration-slow)`), `--dx-overlay-ease`, `--dx-overlay-scrim` and `--dx-overlay-blur`. To change them, override the token after the theme stylesheet loads, on `:root` or on any ancestor:")}
        }
        DocsPre {
            code { "{tokens_css}" }
        }
        p {
            {inline_code("A few durations are still literals in a component's own sheet (the chart's load animations, the sheet's 500 ms entrance, the progress bar's width transition); the last column of the table below says which.")}
        }

        h3 { id: "motion-reduced", "Reduced motion" }
        p {
            {inline_code("`prefers-reduced-motion` is answered by the stylesheets. The theme sets `animation-duration` and `transition-duration` to `--dx-motion-duration-reduced` (`0.01ms`, not `0ms`: a zero duration can still misfire `transitionend` listeners) on every interactive root under `@media (prefers-reduced-motion: reduce)`. Animated content that is neither an interactive root nor a loop (dialogs, accordion panels) carries its own reduced-motion block. Chart load animations are wrapped in `no-preference`, carousel autoplay stays off at mount, and `.dx-shimmer` drops its gradient.")}
        }
        p {
            {inline_code("Loops are covered by construction: every looping component calls `use_motion`, which marks its element with `data-dx-motion-key`, and one theme rule sets `animation: none !important` on every such host and everything under it under `prefers-reduced-motion: reduce`. The spinner, skeleton, indeterminate progress bar and loading avatar stop, and a looping component you add is covered by calling `use_motion`. They stop rather than shorten (an infinite 0.01 ms loop flickers), and they keep their shape, so \"loading\" stays visible: a still ring, a still block, and for the progress bar diagonal stripes across the whole track, because a still half-width bar would read as 50%. `scripts/check-infinite-animations.sh` fails an infinite animation with no reduced-motion answer. The visibility gate below is a different question and does not look at the preference.")}
        }

        h3 { id: "motion-compositor", "Compositor-only infinite animations" }
        p {
            {inline_code("An animation that runs forever is main-thread work on every frame unless the browser can hand it to the compositor, and only `transform`, `translate`, `scale`, `rotate` and `opacity` qualify. `scripts/check-infinite-animations.sh` fails the build when an `infinite` animation's `@keyframes` touches anything else, unless the pair is on its allowlist with a reason. The one exception is the shimmer: `dx-shimmer-sweep` moves `background-position`, because its highlight is painted inside the glyphs with `background-clip: text` and a translated pseudo-element would move the glyphs with it. It is paint-only, opt-in, off under reduced motion, and paused when unseen like every other loop.")}
        }

        h3 { id: "motion-visibility", "Motion nobody can see is not run" }
        p {
            {inline_code("A loop or a timer costs work whether or not anyone is looking, and fixing that one call site at a time is the mistake the next animation repeats. `primitives/src/activity.rs` answers \"is this visible\" once for the page (one shared `IntersectionObserver` per kind, the tab, and `content-visibility: auto` subtrees the browser is skipping) and components subscribe to the answer. Before the observer reports, and on the server, a motion is active, so there is no hydration mismatch.")}
        }
        {docs_table(&["Hook", "Use it for", "Behaviour"], &hooks)}
        p {
            {inline_code("Pause versus stop. A CSS loop is paused, not removed: one rule in the theme turns `data-dx-motion=\"paused\"` into `animation-play-state: paused` for the element and everything under it, so the spinner resumes at the angle it stopped at and a paused animation costs nothing. A timer is stopped, because there is no phase to keep: the task is dropped and the countdown restarts in full on return. A timer the user is waiting on follows the tab only. A one-shot entrance waits to be seen, plays once and is done. Transitions the user triggered (hover, open, close, a drag) are exempt: they run because the user is looking.")}
        }
        p {
            {inline_code("`scripts/check-motion-gating.sh` keeps the ungated shapes out: a bare `use_interval(` or a hand-rolled `loop` that `sleep`s must be the gated form or carry `// motion-ok: <reason>` (a timer that carries meaning, or one that lives as long as the page), and every component stylesheet with an `infinite` animation needs a component that applies `use_motion` or an allowlist entry with a reason.")}
        }

        h3 { id: "motion-anchored", "Anchored keyframes" }
        p {
            {inline_code("Anchored overlays (tooltip, hover card, popover, menus, select, combobox, date picker) position themselves with `transform: translateX(-50%)`. A `@keyframes` that sets `transform` replaces it while it runs, so the panel plays its fade off-centre and snaps into place on the last frame. Animate `translate`, `scale` or `rotate` instead; they compose with `transform`. `scripts/check-anchored-keyframes.sh` bans `transform` in `@keyframes`, with a short allowlist for motion that can never sit on an anchor.")}
        }

        h3 { id: "motion-modal-exits", "Modal exits" }
        p {
            {inline_code("A native `<dialog>` leaves the top layer in one frame when `close()` runs, and outside it a `position: fixed` panel is resolved against the nearest contained ancestor and clipped, which made sheets resize and drawers glitch mid-exit. So the dialog stays open until its exit animation and its backdrop's have settled (at most 1500 ms), and only then calls `close()`. Write a normal exit animation; the close waits for it. Keep exit keyframes to `transform`, `translate` and `opacity`.")}
        }

        h3 { id: "motion-theme-switch", "Theme switches" }
        p {
            {inline_code("Switching light and dark makes every component's colour transition run at once, a short wash. next-themes suppresses it; we keep it on purpose (backlog row 158 part (b)), and would add the suppression only inside the single mode writer.")}
        }

        h3 { id: "motion-own-component", "Your own animated component" }
        p {
            {inline_code("Use the hooks instead of writing a pause rule or a `setInterval`. Link `dx-components-theme.css`: it carries the one `[data-dx-motion=\"paused\"]` rule.")}
        }
        DocsPre {
            code { "{own_rust}" }
        }
        DocsPre {
            code { "{own_css}" }
        }
        p {
            {inline_code("For a loop that only sometimes exists use `use_motion_when(move || loading())`; for a one-shot entrance use `use_entered_view_when` and start the animation when `entered()` is true; for a countdown that must outlive scrolling use `use_timeout_while(use_document_visible(), duration, on_fire)`.")}
        }

        h3 { id: "motion-components", "Motion in each component" }
        p {
            {inline_code("The last two columns say what happens while nobody can see the component and how to tune or disable it. pause: the CSS loop is paused and keeps its phase. stop: the timer is dropped and restarts in full on return. waits for view: a one-shot entrance that has not played yet waits to be seen. exempt: a short transition the user triggered, or a motion that is visible by definition.")}
        }
        {docs_table(&["Component", "What moves", "While unseen", "Tune or disable"], &components)}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn markdown() -> String {
        std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../dev-docs/motion.md"),
        )
        .expect("dev-docs/motion.md")
    }

    fn row(cells: &[&str]) -> String {
        format!("| {} |", cells.join(" | "))
    }

    /// The page and `dev-docs/motion.md` show the same facts: every row of every table is in the
    /// markdown verbatim, and the code examples are read from it.
    #[test]
    fn motion_tables_match_motion_md() {
        let md = markdown();
        for r in MOTION_TOKENS
            .iter()
            .map(|r| row(r))
            .chain(MOTION_HOOKS.iter().map(|r| row(r)))
            .chain(MOTION_TABLE.iter().map(|r| row(r)))
        {
            assert!(
                md.lines().any(|l| l == r),
                "dev-docs/motion.md has no table row exactly equal to:\n{r}"
            );
        }
        // The three code examples shown on /docs are read from the markdown; they must exist.
        assert!(tokens_css_example().contains("--dx-motion-duration-slow"));
        assert!(own_component_rust_example().contains("use_interval_while"));
        assert!(own_component_css_example().contains("prefers-reduced-motion"));
    }

    /// The per-component table has one row per animated component a reader would look for.
    #[test]
    fn the_component_table_covers_the_animated_components() {
        let names: Vec<&str> = MOTION_TABLE.iter().map(|r| r[0]).collect();
        for want in [
            "Spinner",
            "Skeleton",
            "Progress",
            "Avatar (loading)",
            "Carousel autoplay",
            "Chart entrance",
            "Toast",
            "Shimmer effect",
        ] {
            assert!(
                names.contains(&want),
                "the motion table has no `{want}` row"
            );
        }
    }
}

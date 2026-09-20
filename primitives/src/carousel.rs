//! Defines the [`Carousel`] component and its sub-components.
//!
//! # Scope (v1)
//!
//! Built per `dev-docs/research/carousel-2026-09-19.md` and the approved
//! decisions (2026-09-19): the APG "basic" (prev/next, no picker) style
//! only (`carousel-1-prev-next` in the vendored reference). Deliberately
//! **not** in v1, each filed as a fast-follow rather than a silent gap:
//! infinite `loop`ing (§8.2), autoplay / a rotation control and the
//! `aria-live` region that only matters once one exists (§8.3 -- the APG
//! basic style has no rotation requirement at all, so a manually-paged
//! carousel is fully conformant without it, §1.2), and the tablist picker
//! variant (§8.5, composed on [`crate::tabs`] once this variant is
//! proven out).
//!
//! # Engine
//!
//! CSS scroll-snap, not a ported physics engine and not a JS dependency
//! (research §8.1/§2.2): [`CarouselContent`] is a native
//! `overflow-{x,y}: auto; scroll-snap-type` track; [`CarouselItem`]s are
//! `scroll-snap-align` children. Paging (buttons, keyboard) calls the
//! target item's own `Element.scrollIntoView()` -- the browser computes
//! the offset, so there is no manual pixel/offset math to get wrong, and
//! (per research §2.2's RTL paragraph) no sign-flip step either: DOM
//! order never changes, so "the next slide" is always "the next
//! sibling," and `scrollIntoView` already resolves the correct physical
//! position under `dir="rtl"`. A second, independent JS bridge
//! (`use_carousel_scroll_tracking`, a private helper) keeps `CarouselContext`'s own `selected`
//! correct after a *native* drag/wheel/trackpad scroll the caller's own
//! buttons/keyboard never drove -- both are the only two `document::eval`
//! call sites this module has, and both no-op harmlessly off a real
//! document (native/Blitz, or a plain `cargo test`), the same as every
//! other un-gated `document::eval` helper in `crate::lib` (`use_outside_dismiss`,
//! `use_form_reset_listener`, ...): this module's rendered markup never
//! branches on `feature = "web"` at all, so unlike the native-`<dialog>`
//! overlays there is nothing here for a `feature = "web"` gate to protect.
//!
//! # Accessibility
//!
//! Follows `dev-docs/research/carousel-2026-09-19.md` §1.2/§5.2's account
//! of the APG Carousel pattern's basic style: [`Carousel`] is
//! `role="region"` + `aria-roledescription="carousel"` with a **required**
//! accessible name (a `tracing::warn!` when neither `aria-label` nor
//! `aria-labelledby` is supplied -- shadcn's own `carousel.tsx` leaves this
//! entirely to the caller, undocumented; see the research's §1.4 gap
//! table), each [`CarouselItem`] is `role="group"` +
//! `aria-roledescription="slide"` with a default `"{n} of {m}"` label
//! (APG's own sanctioned exception to "don't put position/size in an
//! accessible name," since `group` supports neither `aria-setsize` nor
//! `aria-posinset`), and [`CarouselPrevious`]/[`CarouselNext`] are native
//! `<button>`s, genuinely `disabled` at the ends (v1 has no `loop`, so the
//! ends are real, reachable boundaries, not merely visual ones). A root-level
//! `ArrowLeft`/`ArrowRight` (or `ArrowUp`/`ArrowDown` for
//! [`CarouselOrientation::Vertical`]) key is a Radix/shadcn-style
//! addition, not an APG requirement (research §1.2/§1.5) -- RTL-swapped
//! via [`crate::direction`], Radix's convention rather than shadcn's own
//! (shadcn's `handleKeyDown` never swaps; it relies on embla's engine
//! silently compensating underneath it, which this crate does not have).

use crate::{
    direction::{use_direction, Direction, HorizontalNav},
    fold_style_attributes, has_own_accessible_name, use_controlled, use_id_or, use_unique_id,
};
use dioxus::prelude::*;

/// The axis a [`Carousel`] pages along.
///
/// Matches [`crate::resizable::ResizableDirection`]'s own
/// horizontal/vertical shape and `data-orientation` convention. Vertical
/// support fell out of the horizontal construction for free (the same
/// component, an axis-swapped CSS declaration and `scrollIntoView`
/// option, and `ArrowUp`/`ArrowDown` in place of the RTL-aware
/// `ArrowLeft`/`ArrowRight` pair), so it ships in v1 rather than being
/// deferred -- see the module doc's "Scope" section for what *is*
/// deferred.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CarouselOrientation {
    /// Slides are arranged left-to-right (or right-to-left under RTL) and
    /// paged with `ArrowLeft`/`ArrowRight`.
    #[default]
    Horizontal,
    /// Slides are arranged top-to-bottom and paged with
    /// `ArrowUp`/`ArrowDown` -- never RTL-aware (see the private
    /// `carousel_key_intent`, this module's own key-resolution function).
    Vertical,
}

impl CarouselOrientation {
    /// Returns `"horizontal"` or `"vertical"` -- the exact string this
    /// module's `data-orientation` styling hook uses.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }
}

/// Clamp `index` into the valid range for a carousel of `count` slides
/// (`0` when `count` is `0`, otherwise `[0, count - 1]`). The one
/// definition every path that could produce an out-of-range index
/// (a controlled `value` prop, a stale `selected` before the first
/// [`CarouselItem`] has registered, a JS-reported scroll position) is
/// funneled through, so a caller passing e.g. `value: Some(99)` on a
/// 3-slide carousel can never desync [`CarouselApi::can_scroll_next`]
/// from what is actually rendered.
fn clamp_selected(index: usize, count: usize) -> usize {
    if count == 0 {
        0
    } else {
        index.min(count - 1)
    }
}

/// Whether a "previous" action is currently meaningful. v1 has no `loop`
/// (research §8.2), so this is a real, reachable boundary -- `index == 0`
/// -- not a cosmetic one.
fn can_scroll_prev(selected: usize) -> bool {
    selected > 0
}

/// Whether a "next" action is currently meaningful -- the mirror of
/// [`can_scroll_prev`]. `count == 0` (no slides registered yet, e.g. the
/// very first render before any [`CarouselItem`] has mounted) is never
/// scrollable either way.
fn can_scroll_next(selected: usize, count: usize) -> bool {
    count > 0 && selected + 1 < count
}

/// The index one step forward from `selected`, saturating at the last
/// slide (never wrapping -- see [`can_scroll_next`]'s doc).
fn next_selected(selected: usize, count: usize) -> usize {
    if count == 0 {
        0
    } else {
        (selected + 1).min(count - 1)
    }
}

/// The index one step back from `selected`, saturating at `0` (never
/// wrapping -- see [`can_scroll_prev`]'s doc).
fn prev_selected(selected: usize) -> usize {
    selected.saturating_sub(1)
}

/// The default accessible name for a slide with no name of its own:
/// `"{one-based index} of {count}"`. APG's own sanctioned exception to
/// "don't encode position/size in an accessible name" -- `role="group"`
/// supports neither `aria-setsize` nor `aria-posinset`
/// (dev-docs/research/carousel-2026-09-19.md §1.2).
fn slide_label(index: usize, count: usize) -> String {
    format!("{} of {}", index + 1, count)
}

/// Resolve a keydown on (or bubbled up to) the [`Carousel`] root to a
/// paging intent, or `None` for every other key.
///
/// [`HorizontalNav`] is [`crate::direction`]'s own "which arrow key means
/// which paging intent" result type; reused here as a plain `Prev`/`Next`
/// result rather than duplicating an identical two-variant enum under a
/// new name -- nothing about its own definition or `resolve_horizontal`'s
/// doc restricts it to a *horizontal* caller; only its name does, and
/// this function's own doc is where that reuse is recorded. For
/// [`CarouselOrientation::Horizontal`] this defers entirely to
/// [`Direction::resolve_horizontal`], so it swaps under `dir="rtl"`
/// exactly the way `tabs.rs`'s roving focus does. For
/// [`CarouselOrientation::Vertical`], `ArrowUp`/`ArrowDown` are used
/// unconditionally -- direction never affects a vertical axis (see
/// `Direction::resolve_horizontal`'s own doc).
fn carousel_key_intent(
    key: &Key,
    orientation: CarouselOrientation,
    direction: Direction,
) -> Option<HorizontalNav> {
    match orientation {
        CarouselOrientation::Horizontal => direction.resolve_horizontal(key),
        CarouselOrientation::Vertical => match key {
            Key::ArrowUp => Some(HorizontalNav::Prev),
            Key::ArrowDown => Some(HorizontalNav::Next),
            _ => None,
        },
    }
}

/// Fire-and-forget: scroll the given slide element into view along the
/// carousel's own axis only. `inline`/`block` are chosen so a horizontal
/// carousel's `scrollIntoView` can never also nudge the page's *vertical*
/// scroll position, and vice versa for a vertical one -- the standard
/// technique for using `scrollIntoView` on one axis of a
/// multi-directionally-scrollable page. `instant` forces `behavior:
/// 'auto'` on the very first call (mount), so a non-zero `default_value`
/// does not visibly animate in on page load; every later call additionally
/// respects `prefers-reduced-motion` the same way. No response is read
/// back -- [`CAROUSEL_SCROLL_TRACKING_JS`] is what keeps `selected` in
/// sync with wherever the scroll position actually ends up.
const CAROUSEL_SCROLL_INTO_VIEW_JS: &str = "\
    const [id, orientation, instant] = await dioxus.recv();
    const behavior = (instant || window.matchMedia('(prefers-reduced-motion: reduce)').matches)
        ? 'auto' : 'smooth';
    const el = document.getElementById(id);
    if (el) {
        el.scrollIntoView({
            behavior,
            inline: orientation === 'horizontal' ? 'start' : 'nearest',
            block: orientation === 'horizontal' ? 'nearest' : 'start',
        });
    }";

/// Long-lived (mount-to-unmount): translate a [`CarouselContent`]
/// element's *actual* scroll position into a slide index, so `selected`
/// stays correct after a native drag/wheel/trackpad scroll -- one this
/// crate never drove via [`CAROUSEL_SCROLL_INTO_VIEW_JS`] itself.
///
/// Deliberately not an `IntersectionObserver` (the shape
/// `dev-docs/research/carousel-2026-09-19.md` §2.2 sketches as the
/// primary mechanism): that needs live per-item intersection-ratio
/// bookkeeping and threshold tuning to avoid mid-animation flicker
/// between two partially-visible neighbors. Settling once, on
/// `scrollend` (falling back to a debounced plain `scroll` listener where
/// `scrollend` is absent -- Safari only since v26.2 per the research's
/// own caniuse check, ~90% global today) and then comparing every
/// slide's `getBoundingClientRect()` against the container's own, is
/// simpler, fires exactly once per gesture (no flicker to begin with),
/// and is still zero-JS-dependency, feature-detected, degrade-not-break,
/// matching this repo's own platform-features-first posture
/// (`dev-docs/recommended-implementations.md`'s `<dialog>` finding).
/// Comparing viewport (`getBoundingClientRect`) positions rather than
/// `scrollLeft`/`offsetLeft` sidesteps the well-known cross-browser
/// disagreement over what `scrollLeft` itself means under `dir="rtl"` --
/// a `getBoundingClientRect` is always a physical, direction-agnostic
/// viewport rectangle regardless of `dir`.
///
/// One read (`getBoundingClientRect`, of the container and every slide)
/// followed by nothing but a `dioxus.send` -- no DOM write at all in this
/// callback -- so there is no read-then-write-layout main-thread hazard
/// (`dev-docs/recommended-implementations.md` §11 Rule 1) to begin with.
/// Both listeners are `passive: true` (§11 Rule 2).
const CAROUSEL_SCROLL_TRACKING_JS: &str = "\
    const [id, orientation] = await dioxus.recv();
    const container = document.getElementById(id);
    if (!container) {
        await dioxus.recv();
        return;
    }
    let lastSent = null;
    const settle = () => {
        const children = Array.from(container.children);
        if (children.length === 0) {
            return;
        }
        const containerRect = container.getBoundingClientRect();
        const containerStart = orientation === 'horizontal' ? containerRect.left : containerRect.top;
        let nearest = 0;
        let nearestDist = Infinity;
        children.forEach((child, i) => {
            const rect = child.getBoundingClientRect();
            const childStart = orientation === 'horizontal' ? rect.left : rect.top;
            const dist = Math.abs(childStart - containerStart);
            if (dist < nearestDist) {
                nearestDist = dist;
                nearest = i;
            }
        });
        if (nearest !== lastSent) {
            lastSent = nearest;
            dioxus.send(nearest);
        }
    };
    const supportsScrollEnd = 'onscrollend' in window;
    let debounceTimer = null;
    const onScroll = () => {
        if (supportsScrollEnd) {
            return;
        }
        clearTimeout(debounceTimer);
        debounceTimer = setTimeout(settle, 120);
    };
    container.addEventListener('scroll', onScroll, { passive: true });
    if (supportsScrollEnd) {
        container.addEventListener('scrollend', settle, { passive: true });
    }
    await dioxus.recv();
    container.removeEventListener('scroll', onScroll);
    if (supportsScrollEnd) {
        container.removeEventListener('scrollend', settle);
    }
    clearTimeout(debounceTimer);";

/// Attach [`CAROUSEL_SCROLL_TRACKING_JS`] to the element with the given
/// `id` for as long as the calling component stays mounted, forwarding
/// every reported index to `set_selected`. Mirrors `crate::lib`'s
/// `use_outside_dismiss`/`use_form_reset_listener` shape exactly: an
/// initial `eval.send(..)` of the (id, orientation) the script's own
/// `await dioxus.recv()` unpacks, a spawned task looping on
/// `eval.recv()` for as long as the script keeps sending, and a cleanup
/// closure that sends a teardown value so the script's own trailing
/// `await dioxus.recv()` can resolve and remove its listeners before the
/// element is gone.
fn use_carousel_scroll_tracking(
    id: impl Readable<Target = String> + Copy + 'static,
    orientation: ReadSignal<CarouselOrientation>,
    set_selected: Callback<usize>,
) {
    crate::use_effect_with_cleanup(move || {
        let id = id.cloned();
        let orientation_str = orientation().as_str().to_string();
        let mut eval = document::eval(CAROUSEL_SCROLL_TRACKING_JS);
        let _ = eval.send((id, orientation_str));
        spawn(async move {
            while let Ok(index) = eval.recv::<usize>().await {
                set_selected.call(index);
            }
        });
        move || {
            let _ = eval.send(true);
        }
    });
}

/// Shared state and actions, provided by [`Carousel`] and consumed by
/// every sub-component plus [`use_carousel`].
#[derive(Clone, Copy)]
struct CarouselContext {
    orientation: ReadSignal<CarouselOrientation>,
    /// Always in `[0, count)` (or `0` if `count == 0`) -- see
    /// [`clamp_selected`]. The single source of truth every sub-component
    /// reads instead of re-deriving its own notion of "which slide is
    /// current."
    selected: Memo<usize>,
    set_selected: Callback<usize>,
    /// Number of currently-registered [`CarouselItem`]s. Empty
    /// (`count == 0`) until the first render's effects have run --
    /// deliberately: see [`CarouselItem`]'s doc, "SSR stability."
    count: Memo<usize>,
    /// `index -> DOM id`, grown as each [`CarouselItem`] registers (the
    /// same grow-only-`Vec`-by-index idiom `tabs.rs`'s own
    /// `tab_content_ids` uses for the identical reason: SSR and a
    /// pre-effect client render must agree, and both start from an empty
    /// `Vec`).
    item_ids: Signal<Vec<String>>,
    /// [`CarouselContent`]'s own rendered id, published by itself so
    /// [`CarouselPrevious`]/[`CarouselNext`] can point `aria-controls` at
    /// it (mirrors the `trigger_id`-publishing construction
    /// `dev-docs/conformance-harness.md`'s tier-2 Rule 14 describes for
    /// `SelectTrigger`/`DropdownMenuTrigger`/etc.).
    content_id: Signal<String>,
}

/// The props for the [`Carousel`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CarouselProps {
    /// The axis the carousel pages along. Defaults to
    /// [`CarouselOrientation::Horizontal`].
    #[props(default)]
    pub orientation: ReadSignal<CarouselOrientation>,

    /// The controlled selected slide index (0-based). `None` for
    /// uncontrolled use (see [`Self::default_value`]).
    pub value: ReadSignal<Option<usize>>,

    /// The initial selected slide index when uncontrolled.
    #[props(default)]
    pub default_value: usize,

    /// Called whenever the selected slide settles on a new index, from
    /// any source: [`CarouselPrevious`]/[`CarouselNext`], the root
    /// keyboard handler, [`use_carousel`]'s `scroll_to`, or a user's own
    /// drag/wheel/trackpad scroll.
    #[props(default)]
    pub on_value_change: Callback<usize>,

    /// The text direction for the root-level `ArrowLeft`/`ArrowRight`
    /// paging keys. Defaults to the nearest
    /// [`crate::direction::DirectionProvider`], or LTR if there is none.
    /// See [`crate::direction::use_direction`].
    pub dir: Option<Direction>,

    /// Additional attributes to apply to the carousel's root element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the carousel -- typically [`CarouselPrevious`] and
    /// [`CarouselNext`] followed by [`CarouselContent`] (see
    /// [`Carousel`]'s own doc for why that order matters).
    pub children: Element,
}

/// # Carousel
///
/// A slideshow of [`CarouselItem`]s the user pages through with
/// [`CarouselPrevious`]/[`CarouselNext`] buttons, `ArrowLeft`/`ArrowRight`
/// (RTL-aware) at the root, or by dragging/scrolling the track natively.
/// Implements the APG Carousel pattern's "basic" (prev/next, no picker)
/// style -- see the module doc for exactly what v1 does and defers.
///
/// `Carousel` itself renders `role="region"` +
/// `aria-roledescription="carousel"` and requires an accessible name (an
/// `aria-label`/`aria-labelledby` in `attributes`) -- a
/// `tracing::warn!` fires when neither is supplied, since the pattern
/// requires one and shadcn's own reference component leaves this gap
/// entirely undocumented (`dev-docs/research/carousel-2026-09-19.md`
/// §1.4).
///
/// ## Tab order
///
/// The reference implementation keeps its prev/next buttons **before**
/// the slide content in the page's `Tab` sequence (its own keyboard
/// table: "Rotation control, previous slide, and next slide buttons
/// precede the slide content"). Because `Carousel` renders `children` in
/// whatever order the caller writes them, and [`CarouselPrevious`]/
/// [`CarouselNext`] are meant to be positioned visually with CSS (not
/// DOM order -- absolute positioning does not care which sibling comes
/// first), the recommended composition puts them **before**
/// [`CarouselContent`], matching the reference and giving any interactive
/// content inside a slide (e.g. a card's own link) its natural place
/// after the controls, not before:
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::carousel::{Carousel, CarouselContent, CarouselItem, CarouselNext, CarouselPrevious};
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         Carousel { aria_label: "Featured photos",
///             CarouselPrevious { "Previous" }
///             CarouselNext { "Next" }
///             CarouselContent {
///                 CarouselItem { index: 0usize, "Slide 1" }
///                 CarouselItem { index: 1usize, "Slide 2" }
///                 CarouselItem { index: 2usize, "Slide 3" }
///             }
///         }
///     }
/// }
/// ```
///
/// ## Keyboard
///
/// `ArrowLeft`/`ArrowRight` (or `ArrowUp`/`ArrowDown` under
/// [`CarouselOrientation::Vertical`]) page the carousel whenever focus is
/// anywhere inside it -- a Radix/shadcn-style addition, not an APG
/// requirement (the reference never binds arrow keys at the carousel-root
/// level at all), RTL-swapped like every other roving/orientable
/// primitive in this crate once focus is inside a
/// [`CarouselOrientation::Horizontal`] carousel. As with every native
/// `<button>`, activating [`CarouselPrevious`]/[`CarouselNext`] with
/// `Enter`/`Space` never moves focus.
///
/// ## Styling
///
/// The [`Carousel`] component defines the following data attributes you
/// can use to control styling:
/// - `data-orientation`: `horizontal` or `vertical`, matching
///   [`CarouselProps::orientation`].
/// - `data-direction`: the resolved text direction, `ltr` or `rtl`.
#[component]
pub fn Carousel(props: CarouselProps) -> Element {
    let (raw_selected, set_raw_selected) =
        use_controlled(props.value, props.default_value, props.on_value_change);
    let direction = use_direction(props.dir);
    let orientation = props.orientation;

    let item_ids: Signal<Vec<String>> = use_signal(Vec::new);
    let count = use_memo(move || item_ids.len());
    let selected = use_memo(move || clamp_selected(raw_selected(), count()));
    let set_selected = use_callback(move |index: usize| {
        set_raw_selected.call(clamp_selected(index, *count.peek()));
    });
    let content_id: Signal<String> = use_signal(String::new);

    use_context_provider(|| CarouselContext {
        orientation,
        selected,
        set_selected,
        count,
        item_ids,
        content_id,
    });

    // Scroll the selected slide into view whenever it changes, regardless
    // of source (buttons, keyboard, or `use_carousel_scroll_tracking`
    // reporting a native scroll). In the last case the target slide is
    // already at/near its snap point, so this is a harmless no-op --
    // `scrollIntoView` never moves an element that is already fully in
    // view. `is_first` (peeked, then set -- never tracked-read, so this
    // effect never subscribes to its own write, matching
    // `scripts/check-self-subscribing-effects.sh`'s construction) forces
    // an instant jump on mount so a non-zero `default_value` never
    // visibly animates in on page load.
    let mut is_first = use_signal(|| true);
    use_effect(move || {
        let index = selected();
        let id = item_ids.peek().get(index).cloned();
        let Some(id) = id else {
            return;
        };
        let orientation_str = orientation().as_str().to_string();
        let first = *is_first.peek();
        is_first.set(false);
        let eval = document::eval(CAROUSEL_SCROLL_INTO_VIEW_JS);
        let _ = eval.send((id, orientation_str, first));
    });

    let onkeydown = move |event: Event<KeyboardData>| {
        let key = event.key();
        let Some(intent) = carousel_key_intent(&key, orientation(), direction) else {
            return;
        };
        match intent {
            HorizontalNav::Prev => set_selected.call(prev_selected(selected())),
            HorizontalNav::Next => set_selected.call(next_selected(selected(), count())),
        }
        event.prevent_default();
    };

    if !has_own_accessible_name(&props.attributes) {
        tracing::warn!(
            "Carousel: no aria-label or aria-labelledby was supplied. The APG Carousel \
             pattern requires the carousel region to have an accessible name that does not \
             itself contain the word \"carousel\" \
             (dev-docs/research/carousel-2026-09-19.md \u{a7}1.2)."
        );
    }

    rsx! {
        div {
            role: "region",
            aria_roledescription: "carousel",
            dir: direction.as_str(),
            "data-orientation": orientation().as_str(),
            "data-direction": direction.as_str(),

            onkeydown,
            ..props.attributes,

            {props.children}
        }
    }
}

/// The props for the [`CarouselContent`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CarouselContentProps {
    /// The ID of the carousel content element.
    pub id: ReadSignal<Option<String>>,

    /// Additional attributes to apply to the carousel content element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The [`CarouselItem`] children of the carousel content.
    pub children: Element,
}

/// # CarouselContent
///
/// The scroll-snap track: a native `overflow-{x,y}: auto;
/// scroll-snap-type` element containing every [`CarouselItem`]. This
/// element itself already clips its own overflowing children (that is
/// what `overflow: auto` means), so -- unlike an embla-style engine,
/// which translates a strip via `transform` and therefore needs a
/// *separate* `overflow: hidden` clipping wrapper -- no extra wrapper
/// element is needed here at all.
///
/// This must be used inside a [`Carousel`] component.
///
/// Carries `tabindex="0"`, unconditionally: like [`crate::scroll_area::ScrollArea`]'s
/// own doc puts it, "if you don't have any focusable content within the
/// scroll area, you should make the scroll area focusable" -- a
/// [`CarouselItem`] is not itself focusable unless its own content
/// happens to be, and axe's `scrollable-region-focusable` rule (WCAG
/// 2.1.1/2.1.3) confirms this by execution: without it, a keyboard user
/// who has no pointer at all has no way to reach this element's own
/// native scroll (arrow keys/Page Up/Page Down once focused) at all --
/// [`CarouselPrevious`]/[`CarouselNext`] page one slide at a time, but
/// axe is still right that the scrollable region itself should be
/// independently reachable, e.g. for finer scroll control than a full
/// slide-step in the `multiple`-per-view layout.
///
/// ## Styling
///
/// The [`CarouselContent`] component defines the following data
/// attribute you can use to control styling:
/// - `data-orientation`: `horizontal` or `vertical`, matching the parent
///   [`Carousel`]'s own.
#[component]
pub fn CarouselContent(props: CarouselContentProps) -> Element {
    let mut ctx: CarouselContext = use_context();
    let uuid = use_unique_id();
    let id = use_id_or(uuid, props.id);

    // Publish this element's own rendered id into the shared context so
    // `CarouselPrevious`/`CarouselNext` can point `aria-controls` at it --
    // the same construction `SelectTrigger`/`DropdownMenuTrigger` already
    // use for `trigger_id` (`dev-docs/conformance-harness.md` tier-2 Rule
    // 14), one layer in reverse (content publishing to its controls,
    // rather than a trigger publishing to its content).
    use_effect(move || {
        ctx.content_id.set(id());
    });

    use_carousel_scroll_tracking(id, ctx.orientation, ctx.set_selected);

    let orientation = (ctx.orientation)();
    let (caller_style, rest_attrs) = fold_style_attributes(props.attributes);
    let axis_style = match orientation {
        CarouselOrientation::Horizontal => {
            "display:flex;flex-direction:row;overflow-x:auto;overflow-y:hidden;\
             scroll-snap-type:x mandatory;"
        }
        CarouselOrientation::Vertical => {
            "display:flex;flex-direction:column;overflow-y:auto;overflow-x:hidden;\
             scroll-snap-type:y mandatory;"
        }
    };
    let style = format!(
        "{axis_style}{}",
        caller_style.map(|s| format!(" {s}")).unwrap_or_default()
    );

    rsx! {
        div {
            id,
            style,
            tabindex: "0",
            "data-orientation": orientation.as_str(),
            ..rest_attrs,

            {props.children}
        }
    }
}

/// The props for the [`CarouselItem`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CarouselItemProps {
    /// The index of this slide (0-based). Determines both its default
    /// `"{n} of {m}"` accessible name and where it counts toward
    /// [`CarouselApi::count`] -- siblings must use contiguous indices
    /// starting at `0`, the same convention `tabs.rs`'s `TabTrigger`/
    /// `TabContent` already require of their own `index` prop.
    pub index: ReadSignal<usize>,

    /// The ID of the carousel item element.
    pub id: ReadSignal<Option<String>>,

    /// Additional attributes to apply to the carousel item element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the carousel item -- the slide's own content.
    pub children: Element,
}

/// # CarouselItem
///
/// A single slide: `role="group"` + `aria-roledescription="slide"`, with
/// a default `aria-label` of `"{n} of {m}"` unless the caller supplies
/// its own `aria-label`/`aria-labelledby`.
///
/// This must be used inside a [`CarouselContent`] component.
///
/// ## SSR stability
///
/// The `"{n} of {m}"` count is only known once every sibling
/// [`CarouselItem`] has registered itself, which -- like `tabs.rs`'s own
/// `tab_content_ids` -- happens via each item's own effect, and effects
/// do not run during SSR. So an SSR (or a client's own pre-hydration
/// first) render always sees `count == 0` and renders **no** `aria-label`
/// at all, filling it in once mounted. Server and the client's own first
/// render therefore always agree (both start from the same empty
/// registry) -- nothing here changes what SSR produced out from under
/// hydration; the label simply arrives, correctly, a moment later, the
/// same already-shipped tradeoff `tabs.rs`'s `aria-controls` makes.
///
/// ## Styling
///
/// The [`CarouselItem`] component defines the following data attribute
/// you can use to control styling:
/// - `data-selected`: `true` when this is the currently selected slide,
///   `false` otherwise.
#[component]
pub fn CarouselItem(props: CarouselItemProps) -> Element {
    let mut ctx: CarouselContext = use_context();
    let uuid = use_unique_id();
    let id = use_id_or(uuid, props.id);
    let index = props.index;

    // Grow-only registration by index, mirroring `tabs.rs`'s
    // `tab_content_ids` exactly (see this component's "SSR stability"
    // doc above for why that shape is what keeps this SSR-stable).
    use_effect(move || {
        let idx = index();
        let mut ids = ctx.item_ids.write();
        while ids.len() <= idx {
            ids.push(String::new());
        }
        ids[idx] = id();
    });

    let selected = (ctx.selected)();
    let count = (ctx.count)();
    let is_selected = selected == index();
    let has_name = has_own_accessible_name(&props.attributes);
    let default_label = (!has_name && count > 0).then(|| slide_label(index(), count));

    let (caller_style, rest_attrs) = fold_style_attributes(props.attributes);
    let style = format!(
        "flex:0 0 100%;scroll-snap-align:start;min-width:0;min-height:0;{}",
        caller_style.map(|s| format!(" {s}")).unwrap_or_default()
    );

    rsx! {
        div {
            id,
            role: "group",
            aria_roledescription: "slide",
            aria_label: default_label,
            style,
            "data-selected": is_selected,
            ..rest_attrs,

            {props.children}
        }
    }
}

/// The props shared by [`CarouselPrevious`] and [`CarouselNext`].
#[derive(Props, Clone, PartialEq)]
pub struct CarouselPreviousProps {
    /// Additional attributes to apply to the button element.
    #[props(extends = GlobalAttributes)]
    #[props(extends = button)]
    pub attributes: Vec<Attribute>,

    /// The children of the button -- its visible content. The accessible
    /// name comes from `aria-label` (defaulted to `"Previous slide"`)
    /// regardless of what is rendered here, so this can be purely
    /// decorative (an icon, an arrow glyph).
    pub children: Element,
}

/// # CarouselPrevious
///
/// A native `<button type="button">` that moves to the previous slide,
/// genuinely `disabled` (not merely `aria-disabled`) when already at the
/// first slide -- v1 has no `loop`, so this is a real boundary. Default
/// accessible name `"Previous slide"`, overridable with your own
/// `aria-label`/`aria-labelledby`. `aria-controls` points at the sibling
/// [`CarouselContent`]'s own id.
///
/// This must be used inside a [`Carousel`] component.
#[component]
pub fn CarouselPrevious(props: CarouselPreviousProps) -> Element {
    let ctx: CarouselContext = use_context();
    let selected = (ctx.selected)();
    let disabled = !can_scroll_prev(selected);
    let default_label = (!has_own_accessible_name(&props.attributes)).then_some("Previous slide");
    let content_id = (ctx.content_id)();
    let aria_controls = (!content_id.is_empty()).then_some(content_id);

    rsx! {
        button {
            r#type: "button",
            aria_label: default_label,
            aria_controls,
            disabled,

            onclick: move |_| ctx.set_selected.call(prev_selected((ctx.selected)())),
            ..props.attributes,

            {props.children}
        }
    }
}

/// # CarouselNext
///
/// The mirror of [`CarouselPrevious`]: moves to the next slide, `disabled`
/// at the last slide, default accessible name `"Next slide"`.
///
/// This must be used inside a [`Carousel`] component.
#[component]
pub fn CarouselNext(props: CarouselPreviousProps) -> Element {
    let ctx: CarouselContext = use_context();
    let selected = (ctx.selected)();
    let count = (ctx.count)();
    let disabled = !can_scroll_next(selected, count);
    let default_label = (!has_own_accessible_name(&props.attributes)).then_some("Next slide");
    let content_id = (ctx.content_id)();
    let aria_controls = (!content_id.is_empty()).then_some(content_id);

    rsx! {
        button {
            r#type: "button",
            aria_label: default_label,
            aria_controls,
            disabled,

            onclick: move |_| ctx.set_selected.call(next_selected((ctx.selected)(), (ctx.count)())),
            ..props.attributes,

            {props.children}
        }
    }
}

/// A read-only snapshot of a [`Carousel`]'s state, plus the one action
/// ([`Self::scroll_to`]) that does not already have a dedicated button
/// component -- returned by [`use_carousel`]. Mirrors shadcn's own
/// `CarouselApi` shape (`selected`/`count`/`can_scroll_prev`/
/// `can_scroll_next`) closely enough that a caller who has used shadcn's
/// Carousel (or `dignifiedquire/dx-components`' own `CarouselApi`,
/// `use_carousel`) will recognize it immediately.
#[derive(Clone, Copy, PartialEq)]
pub struct CarouselApi {
    /// The currently selected slide's index (0-based).
    pub selected: usize,
    /// The number of currently-registered [`CarouselItem`]s.
    pub count: usize,
    /// Whether [`Self::scroll_to`] to `selected - 1` would move anywhere.
    pub can_scroll_prev: bool,
    /// Whether [`Self::scroll_to`] to `selected + 1` would move anywhere.
    pub can_scroll_next: bool,
    scroll_to: Callback<usize>,
}

impl CarouselApi {
    /// Move directly to the given slide index (clamped into range the
    /// same way every other path in this module is). For building a
    /// custom picker (e.g. a row of dot indicators) alongside
    /// [`CarouselPrevious`]/[`CarouselNext`], which only ever step by one.
    pub fn scroll_to(&self, index: usize) {
        self.scroll_to.call(index);
    }
}

/// Read the nearest ancestor [`Carousel`]'s current state and reach its
/// `scroll_to` action, for building custom controls (e.g. dot indicators)
/// alongside or instead of [`CarouselPrevious`]/[`CarouselNext`].
///
/// Panics (via [`use_context`]) if called outside a [`Carousel`].
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::carousel::{Carousel, CarouselContent, CarouselItem, use_carousel};
/// #[component]
/// fn Indicators() -> Element {
///     let api = use_carousel();
///     rsx! {
///         for i in 0..api.count {
///             button {
///                 "aria-label": "Go to slide {i + 1}",
///                 "data-active": i == api.selected,
///                 onclick: move |_| api.scroll_to(i),
///             }
///         }
///     }
/// }
/// ```
pub fn use_carousel() -> CarouselApi {
    let ctx: CarouselContext = use_context();
    let selected = (ctx.selected)();
    let count = (ctx.count)();
    CarouselApi {
        selected,
        count,
        can_scroll_prev: can_scroll_prev(selected),
        can_scroll_next: can_scroll_next(selected, count),
        scroll_to: ctx.set_selected,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clamp_selected_within_bounds_is_unchanged() {
        assert_eq!(clamp_selected(1, 3), 1);
    }

    #[test]
    fn clamp_selected_clamps_to_last_index() {
        assert_eq!(clamp_selected(99, 3), 2);
    }

    #[test]
    fn clamp_selected_with_zero_count_is_zero() {
        assert_eq!(clamp_selected(0, 0), 0);
        assert_eq!(clamp_selected(5, 0), 0);
    }

    #[test]
    fn can_scroll_prev_false_at_first_slide() {
        assert!(!can_scroll_prev(0));
    }

    #[test]
    fn can_scroll_prev_true_after_first_slide() {
        assert!(can_scroll_prev(1));
    }

    #[test]
    fn can_scroll_next_true_before_last_slide() {
        assert!(can_scroll_next(0, 3));
        assert!(can_scroll_next(1, 3));
    }

    #[test]
    fn can_scroll_next_false_at_last_slide() {
        assert!(!can_scroll_next(2, 3));
    }

    #[test]
    fn can_scroll_next_false_with_zero_slides() {
        assert!(!can_scroll_next(0, 0));
    }

    #[test]
    fn next_selected_advances_by_one() {
        assert_eq!(next_selected(0, 3), 1);
        assert_eq!(next_selected(1, 3), 2);
    }

    #[test]
    fn next_selected_stops_at_last_slide_without_wrapping() {
        assert_eq!(next_selected(2, 3), 2);
    }

    #[test]
    fn next_selected_with_zero_slides_is_zero() {
        assert_eq!(next_selected(0, 0), 0);
    }

    #[test]
    fn prev_selected_retreats_by_one() {
        assert_eq!(prev_selected(2), 1);
        assert_eq!(prev_selected(1), 0);
    }

    #[test]
    fn prev_selected_stops_at_first_slide_without_wrapping() {
        assert_eq!(prev_selected(0), 0);
    }

    #[test]
    fn slide_label_is_one_based_n_of_m() {
        assert_eq!(slide_label(0, 6), "1 of 6");
        assert_eq!(slide_label(5, 6), "6 of 6");
    }

    #[test]
    fn carousel_key_intent_horizontal_ltr_matches_physical_keys() {
        assert_eq!(
            carousel_key_intent(
                &Key::ArrowLeft,
                CarouselOrientation::Horizontal,
                Direction::Ltr
            ),
            Some(HorizontalNav::Prev)
        );
        assert_eq!(
            carousel_key_intent(
                &Key::ArrowRight,
                CarouselOrientation::Horizontal,
                Direction::Ltr
            ),
            Some(HorizontalNav::Next)
        );
    }

    #[test]
    fn carousel_key_intent_horizontal_rtl_swaps_physical_keys() {
        assert_eq!(
            carousel_key_intent(
                &Key::ArrowLeft,
                CarouselOrientation::Horizontal,
                Direction::Rtl
            ),
            Some(HorizontalNav::Next)
        );
        assert_eq!(
            carousel_key_intent(
                &Key::ArrowRight,
                CarouselOrientation::Horizontal,
                Direction::Rtl
            ),
            Some(HorizontalNav::Prev)
        );
    }

    #[test]
    fn carousel_key_intent_vertical_ignores_direction() {
        for direction in [Direction::Ltr, Direction::Rtl] {
            assert_eq!(
                carousel_key_intent(&Key::ArrowUp, CarouselOrientation::Vertical, direction),
                Some(HorizontalNav::Prev)
            );
            assert_eq!(
                carousel_key_intent(&Key::ArrowDown, CarouselOrientation::Vertical, direction),
                Some(HorizontalNav::Next)
            );
        }
    }

    #[test]
    fn carousel_key_intent_vertical_ignores_horizontal_arrow_keys() {
        assert_eq!(
            carousel_key_intent(
                &Key::ArrowLeft,
                CarouselOrientation::Vertical,
                Direction::Ltr
            ),
            None
        );
        assert_eq!(
            carousel_key_intent(
                &Key::ArrowRight,
                CarouselOrientation::Vertical,
                Direction::Ltr
            ),
            None
        );
    }

    #[test]
    fn carousel_key_intent_ignores_unrelated_keys() {
        for orientation in [
            CarouselOrientation::Horizontal,
            CarouselOrientation::Vertical,
        ] {
            assert_eq!(
                carousel_key_intent(&Key::Home, orientation, Direction::Ltr),
                None
            );
            assert_eq!(
                carousel_key_intent(&Key::Enter, orientation, Direction::Ltr),
                None
            );
        }
    }

    #[test]
    fn orientation_as_str_matches_the_data_orientation_tokens() {
        assert_eq!(CarouselOrientation::Horizontal.as_str(), "horizontal");
        assert_eq!(CarouselOrientation::Vertical.as_str(), "vertical");
    }

    #[test]
    fn orientation_default_is_horizontal() {
        assert_eq!(
            CarouselOrientation::default(),
            CarouselOrientation::Horizontal
        );
    }
}

#[cfg(test)]
mod ssr_tests {
    use super::*;

    #[component]
    fn ThreeSlideCarousel() -> Element {
        rsx! {
            Carousel { aria_label: "Featured photos",
                CarouselPrevious { "Previous" }
                CarouselNext { "Next" }
                CarouselContent {
                    CarouselItem { index: 0usize, "One" }
                    CarouselItem { index: 1usize, "Two" }
                    CarouselItem { index: 2usize, "Three" }
                }
            }
        }
    }

    #[component]
    fn CarouselWithoutAccessibleName() -> Element {
        rsx! {
            Carousel {
                CarouselContent {
                    CarouselItem { index: 0usize, "One" }
                }
            }
        }
    }

    #[component]
    fn CarouselItemWithOwnLabel() -> Element {
        rsx! {
            Carousel { aria_label: "Featured photos",
                CarouselContent {
                    CarouselItem { index: 0usize, aria_label: "The Eiffel Tower at dusk", "One" }
                }
            }
        }
    }

    fn render(component: fn() -> Element) -> String {
        let mut dom = VirtualDom::new(component);
        dom.rebuild_in_place();
        dioxus_ssr::render(&dom)
    }

    /// The opening tag of the `occurrence`-th (0-based) `<button>` in
    /// `html`, in document order. Deliberately not "find the text
    /// 'Previous'/'Next' and walk backward to the nearest `<button`" --
    /// that text also appears inside each button's own `aria-label`
    /// attribute value (`aria-label="Previous slide"`), which a naive
    /// `str::find` matches *before* the button's visible children text,
    /// truncating the "opening tag" slice before it ever reaches
    /// `disabled`. Locating by structural position (`<button` occurrence
    /// count, matching `Carousel`'s own documented DOM order --
    /// `CarouselPrevious` then `CarouselNext`, both before
    /// `CarouselContent`) has no such ambiguity.
    fn button_tag(html: &str, occurrence: usize) -> &str {
        let tag_start = html
            .match_indices("<button")
            .nth(occurrence)
            .map(|(i, _)| i)
            .unwrap();
        let tag_end = html[tag_start..].find('>').unwrap() + tag_start;
        &html[tag_start..=tag_end]
    }

    #[test]
    fn root_has_region_role_and_roledescription() {
        let html = render(ThreeSlideCarousel);
        assert!(html.contains(r#"role="region""#));
        assert!(html.contains(r#"aria-roledescription="carousel""#));
        assert!(html.contains(r#"aria-label="Featured photos""#));
    }

    #[test]
    fn root_defaults_to_horizontal_ltr() {
        let html = render(ThreeSlideCarousel);
        assert!(html.contains(r#"data-orientation="horizontal""#));
        assert!(html.contains(r#"data-direction="ltr""#));
        assert!(html.contains(r#"dir="ltr""#));
    }

    #[test]
    fn renders_without_an_accessible_name_supplied() {
        // Only a `tracing::warn!`, never a panic or missing markup --
        // this just proves the warning path doesn't also break rendering.
        let html = render(CarouselWithoutAccessibleName);
        assert!(html.contains(r#"role="region""#));
    }

    #[test]
    fn item_has_group_role_and_slide_roledescription() {
        let html = render(ThreeSlideCarousel);
        assert!(html.contains(r#"role="group""#));
        assert!(html.contains(r#"aria-roledescription="slide""#));
    }

    #[test]
    fn item_has_no_default_label_on_first_render_before_registration_effects_run() {
        // See `CarouselItem`'s own "SSR stability" doc: `count` is only
        // known once every sibling's registration effect has run, and
        // `rebuild_in_place` alone never runs effects at all -- so this
        // is also exactly what a real SSR render (and a client's own
        // pre-hydration first paint) produces. A `"1 of 0"`-shaped label
        // leaking through here would be the regression this guards.
        let html = render(ThreeSlideCarousel);
        assert!(!html.contains("of 0"));
        assert!(!html.contains("1 of 3"));
    }

    #[test]
    fn item_with_own_aria_label_is_not_overridden() {
        let html = render(CarouselItemWithOwnLabel);
        assert!(html.contains(r#"aria-label="The Eiffel Tower at dusk""#));
        assert!(!html.contains("1 of"));
    }

    #[test]
    fn previous_button_is_disabled_at_the_first_slide() {
        let html = render(ThreeSlideCarousel);
        // `CarouselPrevious` is the first `<button>` in this fixture's own
        // documented DOM order (`CarouselPrevious`, `CarouselNext`, then
        // `CarouselContent`'s slides).
        let opening_tag = button_tag(&html, 0);
        assert!(opening_tag.contains("disabled"));
        assert!(opening_tag.contains(r#"aria-label="Previous slide""#));
    }

    #[test]
    fn next_button_is_disabled_before_registration_effects_run() {
        // Unlike `CarouselPrevious` (whose `disabled` never depends on
        // `count`), `CarouselNext`'s does -- and on this first,
        // pre-effect render `count == 0` (see `CarouselItem`'s "SSR
        // stability" doc: registration is effect-driven, and
        // `rebuild_in_place` alone never runs effects). So this button is
        // conservatively disabled too here, consistent with every other
        // count-derived default in this module ("N of M" included) --
        // not a bug. `next_button_becomes_enabled_once_items_have_registered`
        // below is the same fixture after effects run.
        let html = render(ThreeSlideCarousel);
        let opening_tag = button_tag(&html, 1);
        assert!(opening_tag.contains("disabled"));
        assert!(opening_tag.contains(r#"aria-label="Next slide""#));
    }

    #[test]
    fn next_button_becomes_enabled_once_items_have_registered() {
        // `rebuild_in_place` alone never runs an effect (`use_animated_open`'s
        // own doc: "tasks will not be polled") -- `render_immediate`
        // additionally drains the effect queue and re-diffs every
        // now-dirty scope synchronously, which is what actually runs
        // each `CarouselItem`'s registration effect and, once `count`
        // is no longer `0`, re-renders `CarouselNext` with the correct
        // `disabled` state. No async executor is needed here (unlike
        // `tooltip.rs`'s own `render_immediate` test): every effect this
        // triggers (`CarouselItem`'s registration, `Carousel`'s own
        // scroll-into-view effect) is a plain synchronous `use_effect`
        // body, not one that must await a spawned task's own result to
        // finish applying its state.
        let mut dom = VirtualDom::new(ThreeSlideCarousel);
        dom.rebuild_in_place();
        // One `render_immediate` runs each `CarouselItem`'s registration
        // effect (which updates `item_ids`); a *second* is what actually
        // re-diffs `CarouselNext` (and every "N of M" label) against the
        // resulting, now-nonzero `count` -- a downstream `Memo` read
        // becoming dirty from an effect's write is its own scheduling
        // wave, not folded into the same pass that ran the effect
        // (confirmed by execution: a single call here still showed
        // `CarouselNext` disabled). Looping a few times rather than
        // hardcoding "exactly two" keeps this test from being brittle to
        // a future Dioxus scheduling change.
        for _ in 0..4 {
            dom.render_immediate(&mut dioxus::core::NoOpMutations);
        }
        let html = dioxus_ssr::render(&dom);

        let opening_tag = button_tag(&html, 1);
        assert!(!opening_tag.contains("disabled"));
        assert!(html.contains("1 of 3"));
    }

    #[component]
    fn RtlCarousel() -> Element {
        rsx! {
            Carousel {
                aria_label: "Featured photos",
                dir: Direction::Rtl,
                CarouselContent {
                    CarouselItem { index: 0usize, "One" }
                }
            }
        }
    }

    #[test]
    fn dir_prop_renders_rtl_on_the_root() {
        let html = render(RtlCarousel);
        assert!(html.contains(r#"dir="rtl""#));
        assert!(html.contains(r#"data-direction="rtl""#));
    }
}

//! Defines the [`ResizablePanelGroup`], [`ResizablePanel`], and [`ResizableHandle`] components:
//! a set of panes divided by keyboard-operable, pointer-draggable separators, implementing the
//! W3C ARIA Authoring Practices Guide (APG) "Window Splitter" pattern
//! (<https://www.w3.org/WAI/ARIA/apg/patterns/windowsplitter/>).
//!
//! That pattern has no vendored executable example page in this repo (unlike e.g. the menu-button
//! pattern) -- only prose. `playwright/oracle/tier1-apg/window-splitter.spec.ts`'s header names
//! the exact pinned-commit source its quotes were read from; the doc comments below cite the same
//! pattern's "Keyboard Interaction" and "WAI-ARIA Roles, States, and Properties" sections by name.

use crate::direction::{use_direction, Direction};
use crate::merge_attributes;
use crate::move_interaction::{use_move_interaction, MoveEvent, MoveInteraction};
use crate::{use_controlled, use_unique_id};
use dioxus::prelude::*;
use dioxus_attributes::attributes;

/// The layout axis of a [`ResizablePanelGroup`]: which way its panels are arranged, and, in
/// turn, which arrow keys its [`ResizableHandle`]s respond to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ResizableDirection {
    /// Panels are arranged side by side (a row). Handles move with the Left/Right arrows.
    #[default]
    Horizontal,
    /// Panels are stacked (a column). Handles move with the Up/Down arrows.
    Vertical,
}

impl ResizableDirection {
    fn as_str(self) -> &'static str {
        match self {
            Self::Horizontal => "horizontal",
            Self::Vertical => "vertical",
        }
    }

    /// The `aria-orientation` value a [`ResizableHandle`] on a group of this direction must
    /// report -- the INVERSE of [`Self::as_str`].
    ///
    /// The APG Window Splitter pattern's "Keyboard Interaction" section reads: "Left Arrow:
    /// Moves a **vertical** splitter to the left" / "Right Arrow: Moves a **vertical** splitter
    /// to the right" / "Up Arrow: Moves a **horizontal** splitter up" / "Down Arrow: Moves a
    /// **horizontal** splitter down." A splitter moved by Left/Right -- this crate's
    /// `Horizontal`-direction group, whose panels sit side by side -- is therefore the pattern's
    /// own "vertical splitter" (a vertical bar), and a splitter moved by Up/Down (`Vertical`
    /// direction, stacked panels) is its "horizontal splitter" (a horizontal bar). That is
    /// "bar orientation," not "layout axis" -- the same reading this crate's own
    /// [`crate::separator::Separator`] already gives `aria-orientation`: its `horizontal: true`
    /// renders a horizontal *line*, the kind placed between vertically-stacked blocks of
    /// content, not between side-by-side ones. See
    /// `playwright/oracle/tier1-apg/window-splitter.spec.ts`'s header for the full derivation
    /// (the pattern's own "Roles, States, and Properties" section does not list
    /// `aria-orientation` at all, so this value is derived from the keyboard section's prose
    /// rather than quoted directly).
    fn handle_aria_orientation(self) -> &'static str {
        match self {
            Self::Horizontal => "vertical",
            Self::Vertical => "horizontal",
        }
    }
}

/// One panel's resize constraints, as registered with its group by [`ResizablePanel`].
#[derive(Debug, Clone, Copy, PartialEq)]
struct PanelConstraints {
    min_size: f64,
    max_size: f64,
    default_size: Option<f64>,
    collapsible: bool,
    collapsed_size: f64,
}

impl Default for PanelConstraints {
    fn default() -> Self {
        Self {
            min_size: 10.0,
            max_size: 100.0,
            default_size: None,
            collapsible: false,
            collapsed_size: 0.0,
        }
    }
}

/// Move the boundary between `sizes[boundary]` and `sizes[boundary + 1]` by `delta_pct`
/// (percentage points), honoring each panel's own `(min, max)` bound from `constraints` and
/// preserving the pair's combined size exactly. Every other panel in `sizes` passes through
/// unchanged -- a handle only ever adjusts its own two neighbors (the APG pattern's own "a value
/// that represents the size of ... the primary pane," singular, moved against its secondary).
///
/// `constraints[i]` is `(min, max)`. A caller may pass a `min` below a panel's own registered
/// `min_size` -- [`ResizableHandle`]'s Home/Enter-collapse handling does exactly this to let the
/// boundary travel down to a `collapsed_size` below the panel's normal floor.
///
/// A no-op (returns `sizes` unchanged) if `boundary` doesn't name a valid adjacent pair.
pub(crate) fn resize_pair(
    sizes: &[f64],
    boundary: usize,
    delta_pct: f64,
    constraints: &[(f64, f64)],
) -> Vec<f64> {
    let mut out = sizes.to_vec();
    if sizes.len() < 2
        || boundary + 1 >= sizes.len()
        || boundary >= constraints.len()
        || boundary + 1 >= constraints.len()
    {
        return out;
    }

    let (min_p, max_p) = constraints[boundary];
    let (min_s, max_s) = constraints[boundary + 1];
    let sum = sizes[boundary] + sizes[boundary + 1];

    let mut primary = (sizes[boundary] + delta_pct).clamp(min_p, max_p);
    let mut secondary = sum - primary;
    if secondary < min_s {
        secondary = min_s;
        primary = sum - secondary;
    } else if secondary > max_s {
        secondary = max_s;
        primary = sum - secondary;
    }
    // Defensive re-clamp: only changes anything when the pair's [min,max] ranges can't both be
    // satisfied at this `sum` (infeasible constraints), which would otherwise let the
    // secondary-side clamp above push `primary` back out of its own bounds.
    primary = primary.clamp(min_p, max_p);
    secondary = sum - primary;

    out[boundary] = primary;
    out[boundary + 1] = secondary;
    out
}

/// Equal-split fallback for panels that were never given a `default_size`: after honoring every
/// panel that DOES specify one, whatever percentage is left over is split evenly among the rest.
/// Two panels with no default become 50/50; three become ~33.3 each; one panel pinned at
/// `default_size: 25.0` among three total leaves the other two 37.5 each.
pub(crate) fn initial_sizes(defaults: &[Option<f64>]) -> Vec<f64> {
    let known_sum: f64 = defaults.iter().filter_map(|d| *d).sum();
    let unknown_count = defaults.iter().filter(|d| d.is_none()).count();
    let leftover = (100.0 - known_sum).max(0.0);
    let share = if unknown_count > 0 {
        leftover / unknown_count as f64
    } else {
        0.0
    };
    defaults.iter().map(|d| d.unwrap_or(share)).collect()
}

/// APG Window Splitter "Keyboard Interaction": "Home (Optional): Moves splitter to the position
/// that gives the primary pane its smallest allowed size. This may completely collapse the
/// primary pane." / "End (Optional): Moves splitter to the position that gives the primary pane
/// its largest allowed size." For a collapsible primary pane, "smallest allowed size" is its
/// `collapsed_size`, not its ordinary `min_size` -- see [`ResizableHandle`]'s keydown handler,
/// which widens the constraint passed to [`resize_pair`] to let Home reach it.
fn home_end_target(key: &Key, panel: &PanelConstraints) -> Option<f64> {
    match key {
        Key::Home => Some(if panel.collapsible {
            panel.collapsed_size
        } else {
            panel.min_size
        }),
        Key::End => Some(panel.max_size),
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct ResizableGroupContext {
    direction: ReadSignal<ResizableDirection>,
    /// Text direction (not to be confused with `direction` above, this
    /// group's *layout axis*) -- only ever consulted when `direction` is
    /// `Horizontal`; a vertical group's handle never flips, matching every
    /// other RTL-aware component in this crate that has an orientation
    /// (Slider's `SliderVertical` never consults direction either). See
    /// `ResizableHandle`'s own `onkeydown` and this lane's
    /// `$S/batch3/rtl-rust/reference.md`'s Resizable row for the
    /// extrapolation this is based on (no Radix/shadcn original exists to
    /// cite directly).
    text_direction: Direction,
    disabled: ReadSignal<bool>,
    group_id: Signal<String>,
    panels: Signal<Vec<PanelConstraints>>,
    committed: Memo<Vec<f64>>,
    set_committed: Callback<Vec<f64>>,
    /// Per-panel size immediately before Enter last collapsed it, so a later Enter can restore
    /// it -- APG: "If the pane is collapsed, restores the splitter to its **previous position**."
    pre_collapse: Signal<Vec<Option<f64>>>,
    /// Rect-tracking half of a [`MoveInteraction`] mounted on the group's own container div;
    /// only `.rect()`/`.set_mounted()`/`.refresh_rect()` are used here, never its pointer-drag
    /// half (each [`ResizableHandle`] owns its own separate `MoveInteraction` for that).
    group_rect: MoveInteraction,
}

impl ResizableGroupContext {
    fn panel_id(&self, index: usize) -> String {
        format!("{}-panel-{index}", (self.group_id)())
    }

    fn constraints_vec(&self) -> Vec<(f64, f64)> {
        (self.panels)()
            .iter()
            .map(|p| (p.min_size, p.max_size))
            .collect()
    }

    fn panel_at(&self, index: usize) -> PanelConstraints {
        (self.panels)().get(index).copied().unwrap_or_default()
    }

    /// The sizes actually shown: the committed (controlled-or-uncontrolled) vector where it
    /// covers a panel, [`initial_sizes`]'s equal-split-of-the-leftover for any panel it doesn't
    /// (yet) cover. Registration is asynchronous (each [`ResizablePanel`] registers itself in
    /// its own mount effect), so the committed vector can start shorter than the panel count;
    /// once any interaction commits a full-length vector this fallback is never consulted again.
    fn effective_sizes(&self) -> Vec<f64> {
        let panels = (self.panels)();
        let committed = (self.committed)();
        if panels.is_empty() {
            return Vec::new();
        }
        if committed.len() >= panels.len() {
            return committed[..panels.len()].to_vec();
        }
        let defaults: Vec<Option<f64>> = panels.iter().map(|p| p.default_size).collect();
        let computed = initial_sizes(&defaults);
        (0..panels.len())
            .map(|i| {
                committed
                    .get(i)
                    .copied()
                    .unwrap_or_else(|| computed.get(i).copied().unwrap_or(0.0))
            })
            .collect()
    }

    fn commit(&self, sizes: Vec<f64>) {
        self.set_committed.call(sizes);
    }

    fn register_panel(&mut self, index: usize, constraints: PanelConstraints) {
        let mut panels = self.panels.write();
        if panels.len() <= index {
            panels.resize(index + 1, PanelConstraints::default());
        }
        panels[index] = constraints;
    }

    fn pre_collapse_for(&self, index: usize) -> Option<f64> {
        (self.pre_collapse)().get(index).copied().flatten()
    }

    fn set_pre_collapse(&mut self, index: usize, size: f64) {
        let mut store = self.pre_collapse.write();
        if store.len() <= index {
            store.resize(index + 1, None);
        }
        store[index] = Some(size);
    }

    fn clear_pre_collapse(&mut self, index: usize) {
        let mut store = self.pre_collapse.write();
        if let Some(slot) = store.get_mut(index) {
            *slot = None;
        }
    }
}

/// The props for the [`ResizablePanelGroup`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ResizablePanelGroupProps {
    /// The layout axis: side-by-side panels (`Horizontal`, the default) or stacked panels
    /// (`Vertical`).
    #[props(default = ReadSignal::new(Signal::new(ResizableDirection::Horizontal)))]
    pub direction: ReadSignal<ResizableDirection>,

    /// The controlled panel sizes, as percentages that should sum to `100.0`, one per
    /// registered [`ResizablePanel`] in `index` order.
    pub sizes: ReadSignal<Option<Vec<f64>>>,

    /// The default sizes when uncontrolled. Any panel this doesn't cover (including when this
    /// is `None` entirely) falls back to an equal share of whatever percentage is left over
    /// after every panel with its own [`ResizablePanelProps::default_size`] is honored.
    #[props(default)]
    pub default_sizes: Option<Vec<f64>>,

    /// Called with the full sizes vector whenever a drag or keyboard interaction changes it,
    /// whether or not `sizes` is controlled.
    #[props(default)]
    pub on_sizes_change: Callback<Vec<f64>>,

    /// Whether every handle in this group ignores pointer and keyboard resize input.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// The text direction. Only affects a `Horizontal` group: its panels
    /// render in reverse visual order (`flex-direction: row-reverse`) and
    /// its handles' `ArrowLeft`/`ArrowRight` swap roles, so the physically-
    /// left arrow key always shrinks whichever panel is visually on the
    /// left. A `Vertical` group ignores this entirely. Defaults to the
    /// nearest [`crate::direction::DirectionProvider`], or LTR if there is
    /// none.
    #[props(default)]
    pub dir: Option<Direction>,

    /// Additional attributes to apply to the group's container element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the group, which should be alternating [`ResizablePanel`]s and
    /// [`ResizableHandle`]s.
    pub children: Element,
}

/// # ResizablePanelGroup
///
/// A container for a row (or column) of [`ResizablePanel`]s divided by draggable, keyboard-
/// operable [`ResizableHandle`]s, implementing the APG Window Splitter pattern. Renders a flex
/// container along `direction`; each panel's width (or height) is driven by its share of the
/// group's sizes as a `flex-basis` percentage.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::resizable::{ResizableHandle, ResizablePanel, ResizablePanelGroup};
///
/// #[component]
/// fn Demo() -> Element {
///     rsx! {
///         ResizablePanelGroup {
///             ResizablePanel { index: 0usize, default_size: 50.0, "Left" }
///             ResizableHandle { index: 0usize, aria_label: "Left" }
///             ResizablePanel { index: 1usize, default_size: 50.0, "Right" }
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`ResizablePanelGroup`] component defines the following data attributes you can use to
/// control styling:
/// - `data-orientation`: `horizontal` or `vertical`, matching `direction`.
/// - `data-disabled`: Indicates if the group is disabled. Values are `true` or `false`.
/// - `data-direction`: The resolved text direction. Values are `ltr` or `rtl`.
#[component]
pub fn ResizablePanelGroup(props: ResizablePanelGroupProps) -> Element {
    let (committed, set_committed) = use_controlled(
        props.sizes,
        props.default_sizes.clone().unwrap_or_default(),
        props.on_sizes_change,
    );

    let group_id = use_unique_id();
    let panels = use_signal(Vec::new);
    let pre_collapse = use_signal(Vec::new);
    let group_dragging_unused = use_signal(|| false);
    let mut group_movement = use_move_interaction(group_dragging_unused);
    let text_direction = use_direction(props.dir);

    let ctx = use_context_provider(|| ResizableGroupContext {
        direction: props.direction,
        text_direction,
        disabled: props.disabled,
        group_id,
        panels,
        committed,
        set_committed,
        pre_collapse,
        group_rect: group_movement,
    });

    let orientation = use_memo(move || (ctx.direction)().as_str());
    // Deliberately always plain "row", never "row-reverse": CSS Flexbox's
    // `flex-direction: row` is *already* direction-relative by spec (main-
    // start is inline-start, which the `dir` attribute below moves from
    // the left edge to the right edge under RTL) -- so the first DOM
    // child/panel already renders on the physically-*right* edge under
    // `dir="rtl"` with no CSS change needed at all. Adding `row-reverse`
    // on top would cancel that automatic mirroring and put the DOM order
    // back to looking LTR. `ResizableHandle`'s own keyboard delta flip
    // (below) is derived assuming exactly this unmodified `row` mirroring.
    let flex_direction = use_memo(move || match (ctx.direction)() {
        ResizableDirection::Horizontal => "row",
        ResizableDirection::Vertical => "column",
    });

    // This element's own layout CSS is one literal `style` string (structural,
    // like SliderThumb/SliderRange's own inline percent styles -- it must work
    // with zero theme CSS applied). A caller may also pass CSS the *shorthand*
    // way (e.g. a themed wrapper's `height`/`border`), which becomes its own
    // separate namespace="style" attributes in `props.attributes`; combining
    // them with `fold_style_attributes` first avoids dioxus-ssr rendering two
    // separate `style="..."` attributes on the same tag (WHATWG's duplicate-
    // attribute parse error) -- see that helper's own doc in `lib.rs` and
    // `context_menu.rs`'s identical fix for the live-site incident it names.
    let (caller_style, attributes) = crate::fold_style_attributes(props.attributes);
    let style = use_memo(move || {
        let base = format!("display: flex; flex-direction: {};", flex_direction());
        match caller_style.as_deref() {
            Some(extra) => format!("{base} {extra}"),
            None => base,
        }
    });

    // `data-*`/`style` are owned; `attributes` has already had any caller
    // `style` folded out above, so merging just puts this literal `style`
    // back in with nothing left to collide with.
    let owned = attributes!(div {
        "data-orientation": orientation,
        "data-disabled": ctx.disabled,
        "data-direction": text_direction.as_str(),
        style,
    });
    let merged = merge_attributes(vec![attributes, owned]);

    rsx! {
        div {
            dir: text_direction.as_str(),
            onmounted: move |evt| async move {
                group_movement.set_mounted(evt.data()).await;
            },
            onresize: move |_| async move {
                group_movement.refresh_rect().await;
            },
            ..merged,
            {props.children}
        }
    }
}

/// The props for the [`ResizablePanel`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ResizablePanelProps {
    /// This panel's position among its group's panels (`0`-based), matching the boundary
    /// indices its neighboring [`ResizableHandle`]s use.
    pub index: usize,

    /// This panel's initial share of the group, as a percentage. Ignored once the group has a
    /// committed size for this index (controlled, or after any interaction).
    #[props(default)]
    pub default_size: Option<f64>,

    /// The smallest percentage this panel can be resized to (outside of a keyboard/Enter
    /// collapse -- see `collapsible`).
    #[props(default = 10.0)]
    pub min_size: f64,

    /// The largest percentage this panel can be resized to.
    #[props(default = 100.0)]
    pub max_size: f64,

    /// Whether Home/Enter on this panel's preceding [`ResizableHandle`] can collapse it below
    /// `min_size`, down to `collapsed_size`.
    #[props(default)]
    pub collapsible: bool,

    /// The size this panel collapses to when `collapsible` and collapsed. Defaults to `0.0`.
    #[props(default)]
    pub collapsed_size: f64,

    /// Additional attributes to apply to the panel element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children rendered inside the panel.
    pub children: Element,
}

/// # ResizablePanel
///
/// One resizable pane inside a [`ResizablePanelGroup`]. Renders a `div` sized by `flex-basis`
/// to its current share of the group (a fixed `flex-grow`/`flex-shrink` of `0` so only explicit
/// resizing -- never flex's own content-driven growth -- changes its size) with `overflow:
/// hidden` so content never forces the panel wider/taller than its share.
///
/// This must be used inside a [`ResizablePanelGroup`].
///
/// ## Styling
///
/// The [`ResizablePanel`] component defines the following data attribute you can use to control
/// styling:
/// - `data-collapsed`: Indicates if a `collapsible` panel is currently at or below its
///   `collapsed_size`. Values are `true` or `false`.
#[component]
pub fn ResizablePanel(props: ResizablePanelProps) -> Element {
    let ctx = use_context::<ResizableGroupContext>();
    let index = props.index;
    let min_size = props.min_size;
    let max_size = props.max_size;
    let default_size = props.default_size;
    let collapsible = props.collapsible;
    let collapsed_size = props.collapsed_size;

    use_effect(move || {
        let mut ctx = ctx;
        ctx.register_panel(
            index,
            PanelConstraints {
                min_size,
                max_size,
                default_size,
                collapsible,
                collapsed_size,
            },
        );
    });

    let id = use_memo(move || ctx.panel_id(index));
    let size = use_memo(move || ctx.effective_sizes().get(index).copied().unwrap_or(0.0));
    let collapsed = use_memo(move || {
        let panel = ctx.panel_at(index);
        panel.collapsible && size() <= panel.collapsed_size + 0.001
    });
    // See `ResizablePanelGroup`'s identical comment: fold any caller-supplied
    // shorthand style attributes into this element's own literal `flex-basis`
    // style so dioxus-ssr never renders two separate `style="..."` attributes.
    let (caller_style, attributes) = crate::fold_style_attributes(props.attributes);
    let style = use_memo(move || {
        let base = format!(
            "flex-basis: {}%; flex-grow: 0; flex-shrink: 0; overflow: hidden;",
            size()
        );
        match caller_style.as_deref() {
            Some(extra) => format!("{base} {extra}"),
            None => base,
        }
    });

    // `id` is owned: `ResizableHandle` below reads it back via
    // `ctx.panel_id`/`aria_controls`, so a caller override would strand that
    // wiring on a dead id (backlog row 93 names this site explicitly).
    // `data-*`/`style` are owned too; `attributes` has already had any
    // caller `style` folded out above (see the comment there).
    let owned = attributes!(div {
        id: id(),
        "data-panel": "true",
        "data-collapsed": collapsed,
        style,
    });
    let merged = merge_attributes(vec![attributes, owned]);

    rsx! {
        div { ..merged, {props.children} }
    }
}

/// The props for the [`ResizableHandle`] component.
#[derive(Props, Clone, PartialEq)]
pub struct ResizableHandleProps {
    /// The boundary this handle sits on, between the [`ResizablePanel`] at `index` (the
    /// "primary pane") and the one at `index + 1`.
    pub index: usize,

    /// Whether this handle ignores pointer and keyboard input.
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Accessible name for the handle. APG: "the separator ... has an accessible name that
    /// matches the name of the primary pane" -- set this (or `aria_labelledby`) to that name.
    #[props(default)]
    pub aria_label: Option<String>,

    /// Accessible name for the handle, by reference to another element's text (e.g. the primary
    /// pane's own visible heading), per the same APG requirement as `aria_label`.
    #[props(default)]
    pub aria_labelledby: Option<String>,

    /// Additional attributes to apply to the handle element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the handle -- typically a small visible grip glyph.
    pub children: Element,
}

/// # ResizableHandle
///
/// The draggable, keyboard-operable separator between two adjacent [`ResizablePanel`]s,
/// implementing the APG Window Splitter pattern's `separator` widget. Dragging with the pointer,
/// or arrow keys along the group's own axis, move the boundary between this handle's two
/// neighbors, redistributing their combined size; Home/End jump the primary (preceding) pane to
/// its smallest/largest allowed size; Enter toggles collapse when the primary pane is
/// `collapsible`.
///
/// This must be used inside a [`ResizablePanelGroup`], with `index` naming the boundary it sits
/// on (so a group with `N` panels has `N - 1` handles, `index`ed `0..N-1`).
///
/// ## Example
///
/// See [`ResizablePanelGroup`]'s own example.
///
/// ## Styling
///
/// The [`ResizableHandle`] component defines the following data attributes you can use to
/// control styling:
/// - `data-orientation`: `horizontal` or `vertical`, matching the group's `direction` (note:
///   this is the group's *layout axis*, the natural attribute for a `cursor: col-resize` /
///   `row-resize` CSS rule to key off of -- NOT the same value as this element's own
///   `aria-orientation`, which is deliberately the inverse; see [`ResizableDirection`]'s own
///   doc).
/// - `data-disabled`: Indicates if the handle is disabled. Values are `true` or `false`.
/// - `data-state`: `dragging` while a pointer drag is in progress, `idle` otherwise.
#[component]
pub fn ResizableHandle(props: ResizableHandleProps) -> Element {
    let ctx = use_context::<ResizableGroupContext>();
    let index = props.index;
    let handle_disabled = props.disabled;
    let aria_label = props.aria_label.clone();
    let aria_labelledby = props.aria_labelledby.clone();

    let dragging = use_signal(|| false);
    let mut movement = use_move_interaction(dragging);
    let mut origin = use_hook(|| CopyValue::new(Vec::<f64>::new()));
    let mut raw_delta = use_hook(|| CopyValue::new(0.0_f64));

    let direction = ctx.direction;

    let size_now = use_memo(move || ctx.effective_sizes().get(index).copied().unwrap_or(0.0));
    let primary = use_memo(move || ctx.panel_at(index));

    use_effect(move || {
        if !dragging() {
            return;
        }
        let Some(rect) = ctx.group_rect.rect() else {
            return;
        };
        let size = match (direction)() {
            ResizableDirection::Horizontal => rect.width(),
            ResizableDirection::Vertical => rect.height(),
        };
        if size <= 0.0 {
            return;
        }
        let Some(move_event) = movement.pointer_move() else {
            return;
        };
        let delta_axis = match (direction)() {
            ResizableDirection::Horizontal => move_event.delta_x,
            ResizableDirection::Vertical => move_event.delta_y,
        };
        let delta_pct = delta_axis / size * 100.0;

        let mut d = raw_delta.cloned();
        d += delta_pct;
        raw_delta.set(d);

        let base = origin.cloned();
        let constraints = ctx.constraints_vec();
        let new_sizes = resize_pair(&base, index, d, &constraints);
        ctx.commit(new_sizes);
    });

    let aria_orientation = use_memo(move || (direction)().handle_aria_orientation());
    let data_orientation = use_memo(move || (direction)().as_str());
    let controls = use_memo(move || ctx.panel_id(index));
    let value_min = use_memo(move || primary().min_size);
    let value_max = use_memo(move || primary().max_size);
    let data_state = use_memo(move || if dragging() { "dragging" } else { "idle" });
    let tabindex = use_memo(move || if (handle_disabled)() { "-1" } else { "0" });

    // All owned: role/tabindex/aria-orientation define this separator's
    // widget semantics, the rest is its own functional state (the resize
    // position, drag state, and the id-reference to the panel it controls).
    let owned = attributes!(div {
        role: "separator",
        tabindex,
        aria_orientation,
        "data-orientation": data_orientation,
        "data-disabled": handle_disabled,
        "data-state": data_state,
        aria_valuenow: size_now,
        aria_valuemin: value_min,
        aria_valuemax: value_max,
        aria_controls: controls,
    });
    let merged = merge_attributes(vec![props.attributes.clone(), owned]);

    rsx! {
        div {
            aria_label,
            aria_labelledby,

            onmousedown: move |evt| {
                evt.prevent_default();
            },
            ontouchstart: move |evt| {
                evt.prevent_default();
            },

            onpointerdown: move |evt| {
                if (ctx.disabled)() || (handle_disabled)() {
                    return;
                }
                if !movement.start_pointer(&evt) {
                    return;
                }
                origin.set(ctx.effective_sizes());
                raw_delta.set(0.0);
                let mut dragging = dragging;
                dragging.set(true);
            },

            onkeydown: move |evt| {
                if (ctx.disabled)() || (handle_disabled)() {
                    return;
                }
                let dir = (direction)();
                let axis_key = match dir {
                    ResizableDirection::Horizontal => {
                        matches!(evt.key(), Key::ArrowLeft | Key::ArrowRight)
                    }
                    ResizableDirection::Vertical => {
                        matches!(evt.key(), Key::ArrowUp | Key::ArrowDown)
                    }
                };
                if axis_key {
                    if let Some(move_event) = MoveEvent::from_keyboard(&evt, 1.0) {
                        evt.prevent_default();
                        let full = ctx.effective_sizes();
                        let constraints = ctx.constraints_vec();
                        // RTL flips a horizontal handle's ArrowLeft/ArrowRight
                        // role so a physical arrow key always moves the
                        // divider in that same physical direction. Under
                        // `dir="rtl"`, `ResizablePanelGroup`'s own unmodified
                        // `flex-direction: row` already mirrors panel order
                        // per the CSS Flexbox spec (main-start becomes
                        // inline-start = the right edge) -- this panel
                        // (`index`, `resize_pair`'s "primary") is now
                        // visually on the *right*, its `index + 1` neighbor
                        // on the left. `resize_pair` always grows `index`
                        // for a positive delta regardless of visual side, so
                        // moving the divider physically rightward now means
                        // *shrinking* `index` (and growing `index + 1`) --
                        // the negation below. A vertical handle never flips
                        // -- see `ResizableGroupContext::text_direction`'s
                        // doc.
                        let delta = match dir {
                            ResizableDirection::Horizontal
                                if ctx.text_direction == Direction::Rtl =>
                            {
                                -move_event.delta_x
                            }
                            ResizableDirection::Horizontal => move_event.delta_x,
                            ResizableDirection::Vertical => move_event.delta_y,
                        };
                        let new_sizes = resize_pair(&full, index, delta, &constraints);
                        ctx.commit(new_sizes);
                    }
                    return;
                }

                let panel = primary();
                match evt.key() {
                    Key::Home | Key::End => {
                        evt.prevent_default();
                        let full = ctx.effective_sizes();
                        let mut constraints = ctx.constraints_vec();
                        let Some(target) = home_end_target(&evt.key(), &panel) else {
                            return;
                        };
                        if let Some(bound) = constraints.get_mut(index) {
                            bound.0 = bound.0.min(target);
                            bound.1 = bound.1.max(target);
                        }
                        let delta = target - full.get(index).copied().unwrap_or(0.0);
                        let new_sizes = resize_pair(&full, index, delta, &constraints);
                        ctx.commit(new_sizes);
                    }
                    Key::Enter => {
                        if !panel.collapsible {
                            return;
                        }
                        evt.prevent_default();
                        let mut ctx = ctx;
                        let full = ctx.effective_sizes();
                        let current = full.get(index).copied().unwrap_or(0.0);
                        let mut constraints = ctx.constraints_vec();
                        let is_collapsed = current <= panel.collapsed_size + 0.001;
                        let target = if is_collapsed {
                            let restore = ctx
                                .pre_collapse_for(index)
                                .or(panel.default_size)
                                .unwrap_or(panel.min_size);
                            ctx.clear_pre_collapse(index);
                            restore
                        } else {
                            ctx.set_pre_collapse(index, current);
                            panel.collapsed_size
                        };
                        if let Some(bound) = constraints.get_mut(index) {
                            bound.0 = bound.0.min(target);
                            bound.1 = bound.1.max(target);
                        }
                        let delta = target - current;
                        let new_sizes = resize_pair(&full, index, delta, &constraints);
                        ctx.commit(new_sizes);
                    }
                    _ => {}
                }
            },

            ..merged,
            {props.children}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resize_pair_moves_boundary_within_bounds() {
        let sizes = vec![50.0, 50.0];
        let constraints = vec![(10.0, 100.0), (10.0, 100.0)];
        let out = resize_pair(&sizes, 0, 10.0, &constraints);
        assert_eq!(out, vec![60.0, 40.0]);
    }

    #[test]
    fn resize_pair_clamps_primary_to_its_own_min() {
        let sizes = vec![50.0, 50.0];
        let constraints = vec![(20.0, 100.0), (10.0, 100.0)];
        let out = resize_pair(&sizes, 0, -100.0, &constraints);
        assert_eq!(out[0], 20.0, "primary must not go below its own min");
        assert_eq!(out[0] + out[1], 100.0, "pair's sum must be preserved");
    }

    #[test]
    fn resize_pair_clamps_secondary_to_its_own_max() {
        // Secondary's max is reached before primary's own min -- the boundary must stop there
        // instead of overshooting the secondary past its ceiling.
        let sizes = vec![50.0, 50.0];
        let constraints = vec![(5.0, 100.0), (10.0, 60.0)];
        let out = resize_pair(&sizes, 0, -100.0, &constraints);
        assert_eq!(out[1], 60.0);
        assert_eq!(out[0], 40.0);
    }

    #[test]
    fn resize_pair_preserves_sum_across_a_sweep() {
        let sizes = vec![30.0, 70.0];
        let constraints = vec![(0.0, 100.0), (0.0, 100.0)];
        for delta in [-40.0, -10.0, 0.0, 5.0, 25.0, 100.0] {
            let out = resize_pair(&sizes, 0, delta, &constraints);
            assert!(
                (out[0] + out[1] - 100.0).abs() < 1e-9,
                "sum drifted for delta {delta}: {out:?}"
            );
        }
    }

    #[test]
    fn resize_pair_only_touches_the_named_pair() {
        let sizes = vec![20.0, 30.0, 50.0];
        let constraints = vec![(0.0, 100.0); 3];
        let out = resize_pair(&sizes, 1, 10.0, &constraints);
        assert_eq!(out[0], 20.0, "panel outside the pair must be untouched");
        assert_eq!(out[1], 40.0);
        assert_eq!(out[2], 40.0);
    }

    #[test]
    fn resize_pair_is_a_noop_for_an_invalid_boundary() {
        let sizes = vec![50.0, 50.0];
        let constraints = vec![(0.0, 100.0), (0.0, 100.0)];
        assert_eq!(resize_pair(&sizes, 5, 10.0, &constraints), sizes);
    }

    #[test]
    fn resize_pair_supports_collapsible_snap_below_normal_min() {
        // Home on a collapsible primary pane targets `collapsed_size` (0.0 here), below its
        // normal `min_size` (20.0) -- the caller lowers the constraint's own min to let the
        // boundary travel there; see ResizableHandle's Home/Enter handling.
        let sizes = vec![50.0, 50.0];
        let collapsed_min = 0.0;
        let constraints = vec![(collapsed_min, 100.0), (0.0, 100.0)];
        let delta = collapsed_min - sizes[0];
        let out = resize_pair(&sizes, 0, delta, &constraints);
        assert_eq!(out[0], 0.0, "primary collapses fully");
        assert_eq!(out[1], 100.0, "secondary absorbs the collapsed space");
    }

    #[test]
    fn initial_sizes_splits_evenly_with_no_defaults() {
        assert_eq!(initial_sizes(&[None, None]), vec![50.0, 50.0]);
        let thirds = initial_sizes(&[None, None, None]);
        for v in &thirds {
            assert!((v - 100.0 / 3.0).abs() < 1e-9);
        }
    }

    #[test]
    fn initial_sizes_gives_leftover_to_panels_without_a_default() {
        assert_eq!(
            initial_sizes(&[Some(25.0), None, None]),
            vec![25.0, 37.5, 37.5]
        );
    }

    #[test]
    fn initial_sizes_all_explicit_passes_through() {
        assert_eq!(initial_sizes(&[Some(30.0), Some(70.0)]), vec![30.0, 70.0]);
    }

    #[test]
    fn home_end_target_uses_collapsed_size_when_collapsible() {
        let collapsible = PanelConstraints {
            min_size: 20.0,
            max_size: 100.0,
            default_size: None,
            collapsible: true,
            collapsed_size: 0.0,
        };
        assert_eq!(home_end_target(&Key::Home, &collapsible), Some(0.0));
        assert_eq!(home_end_target(&Key::End, &collapsible), Some(100.0));

        let plain = PanelConstraints {
            min_size: 20.0,
            max_size: 100.0,
            default_size: None,
            collapsible: false,
            collapsed_size: 0.0,
        };
        assert_eq!(home_end_target(&Key::Home, &plain), Some(20.0));
        assert_eq!(home_end_target(&Key::ArrowUp, &plain), None);
    }
}

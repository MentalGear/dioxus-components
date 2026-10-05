//! The `/charts/` gallery: every chart demo of the chart component pages on
//! one page, one chart type per tab, modeled on shadcn's `/charts/<type>`
//! (toolbar above each card with a Copy button and a "View Code" button that
//! opens the variant's source in a side sheet on desktop and a bottom drawer
//! on phones).
//!
//! Nothing here owns demo content. The cards render the same
//! `ComponentVariantDemoData` entries `components::DEMOS` already builds for
//! the component pages (`area_chart`, `bar_chart`, `line_chart`,
//! `pie_chart`, `radar_chart`, `radial_chart`, `chart_tooltip`), so a new
//! variant added to a chart component's list in `components/mod.rs` shows up
//! here (after the ordered ones, see [`ChartKind::order`]) with no gallery
//! change, and the source in the Copy button / code view is the exact
//! compile-time-embedded text of that variant's `mod.rs`.
//!
//! Routing: `/charts/` is the Area tab, `/charts/<slug>/` every other tab
//! (and Area again). Both are prerendered by `server_static_routes`
//! (`main.rs`), which asks [`ChartKind::ALL`] for the slugs so the tab list
//! and the SSG list cannot drift apart.

use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    drawer::{Drawer, DrawerContent, DrawerDescription, DrawerHandle, DrawerSide, DrawerTitle},
    sheet::{Sheet, SheetContentClose, SheetDescription, SheetTitle},
    tooltip::{Tooltip, TooltipContent, TooltipTrigger},
    ComponentCategory,
};
use crate::{
    components, installed_source, variant_title, ComponentVariantDemoData, HighlightedCode, Navbar,
    PreviewCode, Route, MAIN_ID,
};
use dioxus::html::input_data::MouseButton;
use dioxus::prelude::*;
use dioxus_icons::lucide::{
    ChartArea, ChartColumn, ChartLine, ChartPie, Check, Copy, FileCode, Gauge, MessageSquare, Radar,
};

/// The seven tabs, in tab order.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum ChartKind {
    Area,
    Bar,
    Line,
    Pie,
    Radar,
    Radial,
    Tooltip,
}

impl ChartKind {
    pub const ALL: [ChartKind; 7] = [
        ChartKind::Area,
        ChartKind::Bar,
        ChartKind::Line,
        ChartKind::Pie,
        ChartKind::Radar,
        ChartKind::Radial,
        ChartKind::Tooltip,
    ];

    /// The path segment of `/charts/<slug>/`.
    pub const fn slug(self) -> &'static str {
        match self {
            ChartKind::Area => "area",
            ChartKind::Bar => "bar",
            ChartKind::Line => "line",
            ChartKind::Pie => "pie",
            ChartKind::Radar => "radar",
            ChartKind::Radial => "radial",
            ChartKind::Tooltip => "tooltip",
        }
    }

    pub fn from_slug(slug: &str) -> Option<ChartKind> {
        Self::ALL.into_iter().find(|kind| kind.slug() == slug)
    }

    /// The tab text.
    pub const fn tab_label(self) -> &'static str {
        match self {
            ChartKind::Area => "Area Charts",
            ChartKind::Bar => "Bar Charts",
            ChartKind::Line => "Line Charts",
            ChartKind::Pie => "Pie Charts",
            ChartKind::Radar => "Radar Charts",
            ChartKind::Radial => "Radial Charts",
            ChartKind::Tooltip => "Tooltips",
        }
    }

    /// The label on the left of every card's toolbar.
    pub const fn card_label(self) -> &'static str {
        match self {
            ChartKind::Area => "Area Chart",
            ChartKind::Bar => "Bar Chart",
            ChartKind::Line => "Line Chart",
            ChartKind::Pie => "Pie Chart",
            ChartKind::Radar => "Radar Chart",
            ChartKind::Radial => "Radial Chart",
            ChartKind::Tooltip => "Tooltip",
        }
    }

    /// The `components::DEMOS` entry the cards come from; also the folder
    /// name shown in the code view's filename bar.
    pub const fn demo_name(self) -> &'static str {
        match self {
            ChartKind::Area => "area_chart",
            ChartKind::Bar => "bar_chart",
            ChartKind::Line => "line_chart",
            ChartKind::Pie => "pie_chart",
            ChartKind::Radar => "radar_chart",
            ChartKind::Radial => "radial_chart",
            ChartKind::Tooltip => "chart_tooltip",
        }
    }

    /// Card order, following shadcn's gallery (the `interactive` hero first
    /// where shadcn has one). A variant of the demo that is not listed
    /// sorts after the listed ones in its own `DEMOS` order, so adding a
    /// variant never needs this list touched; a name listed here that no
    /// longer exists fails `ordered_variants_exist` below.
    const fn order(self) -> &'static [&'static str] {
        match self {
            ChartKind::Area => &[
                "interactive",
                "main",
                "linear",
                "step",
                "legend",
                "stacked",
                "stacked_expand",
                "icons",
                "gradient",
                "axes",
            ],
            ChartKind::Bar => &[
                "interactive",
                "main",
                "horizontal",
                "multiple",
                "stacked",
                "stacked_legend",
                "label",
                "label_custom",
                "mixed",
                "active",
                "negative",
            ],
            ChartKind::Line => &[
                "interactive",
                "main",
                "linear",
                "step",
                "multiple",
                "dots",
                "dots_custom",
                "dots_colors",
                "label",
                "label_custom",
            ],
            ChartKind::Pie => &[
                "main",
                "separator_none",
                "label",
                "label_custom",
                "label_list",
                "legend",
                "donut",
                "donut_active",
                "donut_text",
                "stacked",
                "interactive",
            ],
            ChartKind::Radar => &[
                "main",
                "dots",
                "lines_only",
                "label_custom",
                "grid_custom",
                "grid_none",
                "grid_circle",
                "grid_circle_no_lines",
                "grid_circle_fill",
                "grid_fill",
                "multiple",
                "legend",
                "icons",
                "radius",
            ],
            ChartKind::Radial => &["main", "label", "grid", "text", "shape", "stacked"],
            ChartKind::Tooltip => &[
                "main",
                "indicator_line",
                "indicator_none",
                "label_none",
                "label_custom",
                "label_formatter",
                "formatter",
                "icons",
                "advanced",
            ],
        }
    }

    /// Whether a variant is the full-width "hero" card (shadcn: the
    /// `interactive` chart of the Area, Bar and Line tabs; the Pie one is
    /// an ordinary card).
    fn is_hero(self, variant: &str) -> bool {
        variant == "interactive"
            && matches!(self, ChartKind::Area | ChartKind::Bar | ChartKind::Line)
    }

    fn icon(self) -> Element {
        let size = "0.9rem";
        match self {
            ChartKind::Area => rsx! {
                ChartArea { size, "aria-hidden": "true" }
            },
            ChartKind::Bar => rsx! {
                ChartColumn { size, "aria-hidden": "true" }
            },
            ChartKind::Line => rsx! {
                ChartLine { size, "aria-hidden": "true" }
            },
            ChartKind::Pie => rsx! {
                ChartPie { size, "aria-hidden": "true" }
            },
            ChartKind::Radar => rsx! {
                Radar { size, "aria-hidden": "true" }
            },
            ChartKind::Radial => rsx! {
                Gauge { size, "aria-hidden": "true" }
            },
            ChartKind::Tooltip => rsx! {
                MessageSquare { size, "aria-hidden": "true" }
            },
        }
    }
}

/// The variants of `kind`'s demo, in gallery order.
fn ordered_variants(kind: ChartKind) -> Vec<&'static ComponentVariantDemoData> {
    let Some(demo) = components::DEMOS
        .iter()
        .find(|demo| demo.name == kind.demo_name())
    else {
        return Vec::new();
    };
    let order = kind.order();
    let mut variants: Vec<_> = demo.variants.iter().collect();
    // Stable: variants missing from `order` keep their `DEMOS` order, last.
    variants.sort_by_key(|variant| {
        order
            .iter()
            .position(|name| *name == variant.name)
            .unwrap_or(order.len())
    });
    variants
}

/// `/charts/` -- the Area tab.
#[component]
pub fn Charts(dark_mode: Option<bool>) -> Element {
    rsx! {
        ChartsPage { kind: ChartKind::Area }
    }
}

/// `/charts/<kind>/`.
#[component]
pub fn ChartsKind(kind: String, dark_mode: Option<bool>) -> Element {
    match ChartKind::from_slug(&kind) {
        Some(kind) => rsx! {
            ChartsPage { kind }
        },
        // Not `dx-component-demo-not-found`: `scripts/deploy-preview.sh`
        // greps every prerendered page for that marker, and an unknown
        // slug is never prerendered anyway (`server_static_routes` only
        // lists `ChartKind::ALL`).
        None => rsx! {
            document::Title { "Chart type not found \u{2013} {SITE_NAME}" }
            Navbar {}
            main { id: MAIN_ID, tabindex: "-1", class: "dx-charts-not-found",
                h1 { "Chart type not found" }
                p { "There is no chart gallery called \"{kind}\"." }
                Link { to: Route::charts(ChartKind::Area), "Browse the area charts" }
            }
        },
    }
}

/// The site name in a page title: `Area Charts - dioxus-components`.
const SITE_NAME: &str = "dioxus-components";

/// Runs when a tab link is activated, before the router navigates (an inline
/// handler runs ahead of Dioxus's own delegated one): remembers where the tab
/// row sits in the viewport. Plain primary clicks only, like the handler.
const REMEMBER_TAB_ROW: &str = "if (event.button === 0 && !event.ctrlKey && !event.metaKey && !event.shiftKey && !event.altKey) { const row = document.querySelector('.dx-charts-tabs'); window.__dxChartsTab = { top: row ? row.getBoundingClientRect().top : null, at: Date.now() }; }";

/// Runs after a gallery page renders. The router scrolls to the top on every
/// navigation and the clicked tab may have been unmounted with the previous
/// page, so after a tab switch this scrolls the page back so the tab row sits
/// where it did before the click (never under the sticky navbar, never off
/// screen) and focuses the now-active tab. `behavior: 'instant'` also
/// overrides the page's `scroll-behavior: smooth`: switching tabs is not a
/// scroll to watch, and it keeps the motion-averse from seeing any.
const RESTORE_TAB_ROW: &str = r#"
const pending = window.__dxChartsTab;
window.__dxChartsTab = undefined;
if (pending && Date.now() - pending.at < 5000) {
    const active = document.querySelector('.dx-charts-tab[aria-current="page"]');
    const row = active && active.closest('.dx-charts-tabs');
    if (active && row) {
        active.focus({ preventScroll: true });
        const navbar = parseFloat(
            getComputedStyle(document.documentElement).getPropertyValue('--dx-navbar-height')
        ) || 60;
        const want = typeof pending.top === 'number' ? Math.max(pending.top, navbar) : navbar;
        window.scrollTo({
            top: window.scrollY + row.getBoundingClientRect().top - want,
            behavior: 'instant',
        });
        const rect = row.getBoundingClientRect();
        if (rect.top < navbar || rect.bottom > window.innerHeight) {
            row.scrollIntoView({ block: 'start', behavior: 'instant' });
        }
    }
}
"#;

/// What the code view shows: one variant's source, and on which surface.
#[derive(Clone, PartialEq)]
struct CodeView {
    filename: String,
    title: String,
    code: HighlightedCode,
    /// A bottom drawer (phone widths) rather than a right-hand sheet.
    mobile: bool,
}

#[component]
fn ChartsPage(kind: ChartKind) -> Element {
    let mut selected = use_signal(|| None::<CodeView>);
    let variants = ordered_variants(kind);

    // After a tab switch (and only then): put the tab row back where it was
    // and focus the new tab. Re-runs when `kind` changes in place (Bar ->
    // Line keeps this component); the first run, on page load, has no
    // pending switch and does nothing.
    use_effect(use_reactive!(|kind| {
        let _ = kind;
        document::eval(RESTORE_TAB_ROW);
    }));

    rsx! {
        document::Title { "{kind.tab_label()} \u{2013} {SITE_NAME}" }
        Navbar {}
        main { id: MAIN_ID, tabindex: "-1", class: "dx-charts-page",
            ChartsHero {}
            div { id: "charts", class: "dx-charts-browse",
                ChartsTabs { active: kind }
                section { class: "dx-charts-gallery",
                    h2 { class: "dx-charts-sr-only", "{kind.tab_label()}" }
                    ul { class: "dx-charts-grid",
                        for variant in variants {
                            ChartsCard {
                                key: "{kind.slug()}-{variant.name}",
                                kind,
                                variant: variant.clone(),
                                on_view_code: move |view| selected.set(Some(view)),
                            }
                        }
                    }
                }
            }
        }
        CodeViewer { selected }
    }
}

#[component]
fn ChartsHero() -> Element {
    rsx! {
        section { class: "dx-charts-hero",
            h1 { class: "dx-charts-hero-title", "Beautiful Charts for Dioxus" }
            p { class: "dx-charts-hero-summary",
                "Themed, accessible SVG charts for Dioxus. Browse the gallery, then copy the source of any chart into your app."
            }
            div { class: "dx-charts-hero-actions",
                a { class: "dx-charts-hero-button", "data-style": "primary", href: "#charts", "Browse charts" }
                Link {
                    to: Route::component("chart"),
                    class: "dx-charts-hero-button",
                    "data-style": "secondary",
                    "Documentation"
                }
            }
        }
    }
}

#[component]
fn ChartsTabs(active: ChartKind) -> Element {
    rsx! {
        nav { class: "dx-charts-tabs", aria_label: "Chart types",
            for kind in ChartKind::ALL {
                ChartsTab { key: "{kind.slug()}", kind, active: kind == active }
            }
        }
    }
}

/// One tab link. A plain anchor with the router's own navigation, not
/// `Link`: `Link` computes `aria-current` itself by comparing the target
/// against the current URL, which renders on the server (SSG) but not after
/// hydration under a base path, and passing the attribute as well makes the
/// pair a duplicate that renders neither. Deriving it from the `active` prop
/// alone gives the server and the client the same markup.
#[component]
fn ChartsTab(kind: ChartKind, active: bool) -> Element {
    let href = format!(
        "{}{}",
        router().prefix().unwrap_or_default(),
        Route::charts(kind)
    );
    rsx! {
        a {
            class: "dx-charts-tab",
            href,
            aria_current: active.then_some("page"),
            "onclick": REMEMBER_TAB_ROW,
            onclick: move |event: MouseEvent| {
                // Modified and non-primary clicks keep the browser's own
                // behavior (new tab, ...), like `Link`.
                if event.modifiers().is_empty()
                    && event.trigger_button() == Some(MouseButton::Primary)
                {
                    event.prevent_default();
                    navigator().push(Route::charts(kind));
                }
            },
            "{kind.tab_label()}"
        }
    }
}

#[component]
fn ChartsCard(
    kind: ChartKind,
    variant: ComponentVariantDemoData,
    on_view_code: EventHandler<CodeView>,
) -> Element {
    let ComponentVariantDemoData {
        name,
        rs_highlighted,
        css_highlighted: _,
        component: Comp,
    } = variant;
    // What `dx components add` makes of the file, so Copy / View Code hand
    // out code that compiles in the user's app (see `installed_source`).
    let code = installed_source::installed(kind.demo_name(), name, &rs_highlighted);
    let title = variant_title(name);
    let filename = format!("{}/variants/{name}/mod.rs", kind.demo_name());
    let span = if kind.is_hero(name) { "full" } else { "single" };

    let view_code = {
        let (filename, title, code) = (filename.clone(), title.clone(), code.clone());
        move |_| {
            let view = CodeView {
                filename: filename.clone(),
                title: title.clone(),
                code: code.clone(),
                mobile: false,
            };
            spawn(async move {
                // The surface is picked at click time (a modal can only be
                // one of the two), against the same 768px line the grid and
                // the sidebar use.
                let mut probe = document::eval(
                    "dioxus.send(window.matchMedia('(max-width: 767.98px)').matches)",
                );
                let mobile = probe.recv::<bool>().await.unwrap_or(false);
                on_view_code.call(CodeView { mobile, ..view });
            });
        }
    };

    rsx! {
        li {
            class: "dx-charts-cell",
            "data-kind": kind.slug(),
            "data-variant": name,
            "data-span": span,
            div { class: "dx-charts-toolbar",
                div { class: "dx-charts-toolbar-label",
                    {kind.icon()}
                    span { "{kind.card_label()}" }
                }
                div { class: "dx-charts-toolbar-actions",
                    CopyCodeButton {
                        source: code.source.source().to_string(),
                        label: format!("Copy code ({title})"),
                    }
                    span { class: "dx-charts-toolbar-separator", aria_hidden: "true" }
                    Button {
                        variant: ButtonVariant::Outline,
                        size: ButtonSize::Sm,
                        onclick: view_code,
                        "View Code"
                        span { class: "dx-charts-sr-only", " ({title})" }
                    }
                }
            }
            div { class: "dx-charts-card-body", Comp {} }
        }
    }
}

/// An icon-only ghost button that copies `source` to the clipboard. The
/// check icon shows for a moment afterwards. `tooltip` adds the hover/focus
/// "Copy code" tooltip; the code view leaves it off, because its copy button
/// is the first focusable element of a freshly opened dialog and would show
/// the tooltip the moment the dialog opens.
#[component]
fn CopyCodeButton(
    source: String,
    label: String,
    #[props(default = true)] tooltip: bool,
) -> Element {
    let mut copied = use_signal(|| false);

    let button = move |attributes: Vec<Attribute>| {
        let source = source.clone();
        let label = label.clone();
        rsx! {
            Button {
                variant: ButtonVariant::Ghost,
                size: ButtonSize::IconSm,
                attributes,
                aria_label: label,
                "data-copied": copied(),
                onclick: move |_| {
                    let source = source.clone();
                    spawn(async move {
                        let mut clip = document::eval(
                            r#"
                            const text = await dioxus.recv();
                            try {
                                await navigator.clipboard.writeText(text);
                            } catch (e) {
                                const area = document.createElement('textarea');
                                area.value = text;
                                area.setAttribute('readonly', '');
                                area.style.position = 'fixed';
                                area.style.opacity = '0';
                                document.body.appendChild(area);
                                area.select();
                                document.execCommand('copy');
                                area.remove();
                            }
                            await new Promise((resolve) => setTimeout(resolve, 1600));
                            dioxus.send(true);
                            "#,
                        );
                        let _ = clip.send(source);
                        copied.set(true);
                        let _ = clip.recv::<bool>().await;
                        copied.set(false);
                    });
                },
                if copied() {
                    Check { size: "1rem", "aria-hidden": "true" }
                } else {
                    Copy { size: "1rem", "aria-hidden": "true" }
                }
            }
        }
    };

    if tooltip {
        rsx! {
            Tooltip {
                TooltipTrigger { r#as: button }
                TooltipContent { "Copy code" }
            }
        }
    } else {
        button(Vec::new())
    }
}

/// The single code view of the page: a right-hand `Sheet` at tablet and
/// desktop widths, a bottom `Drawer` on phones. Both are driven by the one
/// `selected` signal (`CodeView::mobile` picks which opens), so at most one
/// is ever open and closing either clears the selection. The native
/// `<dialog>` underneath returns focus to the "View Code" button that
/// opened it.
#[component]
fn CodeViewer(selected: Signal<Option<CodeView>>) -> Element {
    let current = selected();
    let sheet_view = current.clone().filter(|view| !view.mobile);
    let drawer_view = current.filter(|view| view.mobile);

    rsx! {
        Sheet {
            class: "dx-charts-sheet",
            open: sheet_view.is_some(),
            on_open_change: move |open: bool| {
                if !open {
                    selected.set(None);
                }
            },
            if let Some(view) = sheet_view {
                CodeViewBody { view: view.clone(),
                    SheetTitle { class: "dx-charts-code-filename", "{view.filename}" }
                    SheetDescription { class: "dx-charts-sr-only",
                        "Source of the {view.title} chart. Press Escape to close."
                    }
                }
            }
            SheetContentClose {}
        }
        Drawer {
            open: drawer_view.is_some(),
            side: DrawerSide::Bottom,
            on_open_change: move |open: bool| {
                if !open {
                    selected.set(None);
                }
            },
            DrawerContent {
                DrawerHandle {}
                if let Some(view) = drawer_view {
                    CodeViewBody { view: view.clone(),
                        DrawerTitle { class: "dx-charts-code-filename", "{view.filename}" }
                        DrawerDescription { class: "dx-charts-sr-only",
                            "Source of the {view.title} chart. Swipe down or press Escape to close."
                        }
                    }
                }
            }
        }
    }
}

/// Filename bar (the dialog's title, plus a copy button) over the
/// highlighted source. `children` carries the surface's own title and
/// description components.
#[component]
fn CodeViewBody(view: CodeView, children: Element) -> Element {
    let CodeView { code, title, .. } = view;
    rsx! {
        div { class: "dx-charts-code",
            div { class: "dx-charts-code-bar",
                FileCode { size: "1rem", "aria-hidden": "true" }
                {children}
                CopyCodeButton {
                    source: code.source.source().to_string(),
                    label: format!("Copy code ({title})"),
                    tooltip: false,
                }
            }
            div { class: "dx-charts-code-body",
                PreviewCode { source: code.source }
            }
        }
    }
}

/// Whether a component page offers the "Browse all charts" link: the chart
/// component pages, which are exactly the sidebar's Charts group.
pub fn offers_gallery_link(component_name: &str) -> bool {
    components::category_of(component_name) == ComponentCategory::Charts
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn slugs_round_trip() {
        for kind in ChartKind::ALL {
            assert_eq!(ChartKind::from_slug(kind.slug()), Some(kind));
        }
        assert_eq!(ChartKind::from_slug("scatter"), None);
        assert_eq!(ChartKind::from_slug(""), None);
    }

    #[test]
    fn routes_are_canonical_trailing_slash_paths() {
        // `Route::to_string` keeps the empty query marker (`/charts/?`), as for
        // every route; the path part is what must be canonical.
        let path_of = |route: &Route| route.to_string().trim_end_matches('?').to_string();
        assert_eq!(path_of(&Route::Charts { dark_mode: None }), "/charts/");
        for kind in ChartKind::ALL {
            let route = Route::ChartsKind {
                kind: kind.slug().to_string(),
                dark_mode: None,
            };
            let path = format!("/charts/{}/", kind.slug());
            assert_eq!(path_of(&route), path);
            assert!(
                Route::from_str(&path).is_ok_and(|parsed| parsed == route),
                "{path} must parse back to the same route"
            );
        }
        assert!(
            Route::from_str("/charts/").is_ok_and(|parsed| matches!(parsed, Route::Charts { .. }))
        );
    }

    /// Every chart type has its demo, and every name in `order` is a real
    /// variant of it (a renamed or removed variant would silently drop a
    /// card from its shadcn position otherwise).
    #[test]
    fn ordered_variants_exist() {
        for kind in ChartKind::ALL {
            let demo = components::DEMOS
                .iter()
                .find(|demo| demo.name == kind.demo_name())
                .unwrap_or_else(|| panic!("no DEMOS entry for {}", kind.demo_name()));
            for name in kind.order() {
                assert!(
                    demo.variants.iter().any(|variant| variant.name == *name),
                    "{} has no variant `{name}` (listed in ChartKind::{kind:?}::order)",
                    kind.demo_name()
                );
            }
            // Every variant is shown exactly once.
            let shown = ordered_variants(kind);
            assert_eq!(shown.len(), demo.variants.len());
        }
    }

    #[test]
    fn hero_cards_are_the_interactive_area_bar_line_charts() {
        let heroes: Vec<_> = ChartKind::ALL
            .into_iter()
            .filter(|kind| kind.is_hero("interactive"))
            .collect();
        assert_eq!(heroes, [ChartKind::Area, ChartKind::Bar, ChartKind::Line]);
        assert!(!ChartKind::Area.is_hero("main"));
        // ... and lead their tab, like shadcn's.
        for kind in heroes {
            assert_eq!(ordered_variants(kind)[0].name, "interactive");
        }
    }

    #[test]
    fn chart_component_pages_offer_the_gallery_link() {
        assert!(offers_gallery_link("chart"));
        assert!(offers_gallery_link("pie_chart"));
        assert!(!offers_gallery_link("button"));
    }
}

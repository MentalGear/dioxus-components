use crate::components::{
    avatar::{AvatarImageSize, ImageAvatar},
    badge::{Badge, BadgeVariant, VerifiedIcon},
    button::{Button, ButtonVariant},
    checkbox::Checkbox,
    color_picker::ColorPicker,
    combobox::{Combobox, ComboboxEmpty, ComboboxOption},
    drag_and_drop_list::DragAndDropList,
    input::Input,
    item::{
        Item, ItemContent, ItemDescription, ItemMedia, ItemMediaVariant, ItemTitle, ItemVariant,
    },
    label::Label,
    progress::Progress,
    radio_group::{RadioGroup, RadioItem},
    sidebar::{
        Sidebar, SidebarCollapsible, SidebarContent, SidebarCtx, SidebarGroup, SidebarGroupLabel,
        SidebarInset, SidebarMenu, SidebarMenuButton, SidebarMenuItem, SidebarMenuSub,
        SidebarMenuSubButton, SidebarMenuSubItem, SidebarProvider, SidebarSide, SidebarTrigger,
    },
    slider::Slider,
    switch::Switch,
    tabs::{TabContent, TabList, TabTrigger, Tabs, TabsVariant},
    textarea::{Textarea, TextareaVariant},
    toggle_group::{ToggleGroup, ToggleItem},
};
use core::panic;
use dioxus::prelude::{dioxus_router::LinkProps, *};
use dioxus_code::{advanced::HighlightedSource, Code, CodeTheme, Theme};
use dioxus_i18n::prelude::{i18n, use_init_i18n, I18nConfig};
use dioxus_icons::lucide::{
    ArrowRight, ArrowUpRight, Bell, BookOpen, Check, ChevronDown, ChevronLeft, ChevronsUpDown,
    Compass, Copy, ExternalLink, FileText, Hash, House, Layers, LayoutGrid, Mail, Pause, Play,
    SkipBack, SkipForward, SquareCheck,
};
use std::str::FromStr;
use strum::{Display, EnumIter, EnumString, IntoEnumIterator};
use unic_langid::{langid, LanguageIdentifier};

mod components;
mod dashboard;
mod theme;

#[derive(Copy, Clone, PartialEq)]
enum ComponentType {
    /// Normal component as default.
    Normal,
    /// Component that render the preview inside an iframe for isolation.
    Block,
}

#[derive(Clone, PartialEq)]
struct ComponentDemoData {
    name: &'static str,
    r#type: ComponentType,
    description: &'static str,
    docs: &'static str,
    component: HighlightedCode,
    style: CssHighlight,
    variants: &'static [ComponentVariantDemoData],
}

#[allow(unpredictable_function_pointer_comparisons)]
#[derive(Clone, PartialEq)]
struct ComponentVariantDemoData {
    name: &'static str,
    rs_highlighted: HighlightedCode,
    css_highlighted: Option<CssHighlight>,
    component: fn() -> Element,
}

#[cfg(not(feature = "server"))]
fn main() {
    dioxus::launch(App);
}

#[cfg(feature = "server")]
fn main() {
    use dioxus::server::axum::{routing::post, Json, Router};
    use dioxus::server::{DioxusRouterExt, IncrementalRendererConfig, ServeConfig};

    dioxus::server::serve(|| async {
        let cfg = ServeConfig::builder()
            // Enable incremental rendering
            .incremental(
                IncrementalRendererConfig::new()
                    // Store static files in the public directory where other static assets like wasm are stored
                    .static_dir(
                        std::env::current_exe()
                            .unwrap()
                            .parent()
                            .unwrap()
                            .join("public"),
                    )
                    // Don't clear the public folder on every build. The public folder has other files including the wasm
                    // binary and static assets required for the app to run
                    .clear_cache(false),
            )
            .enable_out_of_order_streaming();

        // Workaround for dioxus-cli 0.7.6: with `--base-path`, the `static_routes`
        // server function ends up under `/<base>/api/static_routes`, but the SSG
        // step POSTs to the unprefixed `/api/static_routes` and fails to parse
        // the empty body. Expose a shim at the root that returns the route list.
        //
        // This shim is ALSO the fix for dev-docs/backlog.md row 46 -- see
        // `server_static_routes`'s own doc comment for why.
        let router = Router::new()
            .route(
                "/api/static_routes",
                post(|| async { Json(server_static_routes()) }),
            )
            .serve_dioxus_application(cfg, App);

        Ok(router)
    })
}

/// The full list of routes `dx build --ssg` should prerender, as route
/// strings (`Route::to_string()`). Served by the `/api/static_routes` shim
/// above, which is the only caller.
///
/// `Route::static_routes()` (dioxus-router-0.7.9's own default,
/// `routable.rs`) only ever enumerates route variants whose path is made
/// ENTIRELY of literal segments -- it `filter_map`s away any variant
/// containing a `Dynamic`/`CatchAll` segment, so it can never expand
/// `ComponentDemoPath`'s `:name` (or `ComponentBlockDemoPath`'s
/// `:name`/`:variant`) into one concrete entry per `components::DEMOS` item;
/// it would just silently omit those routes entirely (dev-docs/backlog.md
/// row 46). Appending one resolved route string per demo (and per
/// block-demo variant) below is what actually makes every component page
/// SSG-enumerable. `Route::static_routes()` still supplies every genuinely
/// all-static route: `/`, `/docs`, `/demos`, `/dashboard/email-client`, and
/// the legacy bare `/component/`/`/component/block/` query-form shells (see
/// `ComponentDemo`/`ComponentBlockDemo`'s own doc comments for why those two
/// stay deliberately name-agnostic rather than being enumerated here too).
#[cfg(feature = "server")]
fn server_static_routes() -> Vec<String> {
    let mut routes: Vec<String> = Route::static_routes()
        .iter()
        .map(ToString::to_string)
        .collect();
    for demo in components::DEMOS {
        routes.push(
            Route::ComponentDemoPath {
                name: demo.name.to_string(),
                iframe: None,
                dark_mode: None,
            }
            .to_string(),
        );
        if demo.r#type == ComponentType::Block {
            for variant in demo.variants {
                routes.push(
                    Route::ComponentBlockDemoPath {
                        name: demo.name.to_string(),
                        variant: variant.name.to_string(),
                        dark_mode: None,
                    }
                    .to_string(),
                );
            }
        }
    }
    routes
}

#[component]
pub fn App() -> Element {
    use_init_i18n(|| {
        I18nConfig::new(langid!("en-US"))
            .with_locale((langid!("en-US"), include_str!("i18n/en-US.ftl")))
            .with_locale((langid!("fr-FR"), include_str!("i18n/fr-FR.ftl")))
            .with_locale((langid!("es-ES"), include_str!("i18n/es-ES.ftl")))
            .with_locale((langid!("de-DE"), include_str!("i18n/de-DE.ftl")))
    });

    rsx! {
        Router::<Route> {}
    }
}

#[derive(Routable, Clone, PartialEq)]
pub enum Route {
    #[layout(AppLayout)]
    #[layout(NavigationLayout)]
    #[route("/?:iframe&:dark_mode")]
    Home {
        iframe: Option<bool>,
        dark_mode: Option<bool>,
    },
    #[route("/docs?:dark_mode")]
    Docs { dark_mode: Option<bool> },
    #[route("/demos?:dark_mode")]
    Demos { dark_mode: Option<bool> },
    // Legacy query-string deep link (dev-docs/backlog.md row 46). Kept ONLY
    // so old bookmarks/external links/the hundreds of existing Playwright
    // specs using this URL form keep working -- it is NOT SSG-enumerable by
    // construction (a query string can never become a distinct static FILE:
    // `dioxus-server-0.7.9`'s `FileSystemCache::map_path` strips everything
    // from `?` onward before mapping a route to a file path, so every
    // `name=X` value would collide onto the same `component/index.html`
    // even if this app's `/api/static_routes` enumerated every X). Renders a
    // name-agnostic loading shell (see `ComponentDemo`'s doc comment) and
    // redirects client-side to `ComponentDemoPath` once mounted.
    #[route("/component/?:name&:iframe&:dark_mode")]
    ComponentDemo {
        name: String,
        iframe: Option<bool>,
        dark_mode: Option<bool>,
    },
    // Canonical, SSG-enumerable component page (row 46's construction):
    // `name` lives in the PATH, so every `components::DEMOS` entry maps to
    // its own static file (`component/<name>/index.html`) instead of every
    // one collapsing onto a single query-keyed file. Every internal link
    // (`Route::component`) points here; `ComponentDemo` above only exists to
    // catch old links and hand them off to this route.
    #[route("/component/:name/?:iframe&:dark_mode")]
    ComponentDemoPath {
        name: String,
        iframe: Option<bool>,
        dark_mode: Option<bool>,
    },
    #[end_layout]
    // Legacy query-string block-demo deep link -- same reasoning and same
    // redirect-shell construction as `ComponentDemo` above;
    // `ComponentBlockDemoPath` is the canonical form.
    #[route("/component/block/?:name&:variant&:dark_mode")]
    ComponentBlockDemo {
        name: String,
        variant: Option<String>,
        dark_mode: Option<bool>,
    },
    // Canonical, SSG-enumerable block-demo page: both `name` AND `variant`
    // are path segments (`variant` required, unlike the legacy query form's
    // `Option` -- the canonical route has no "default variant" concept of
    // its own; `ComponentBlockDemo`'s redirect resolves that default before
    // handing off here), so every (demo, variant) pair -- not just every
    // demo -- gets its own static file.
    #[route("/component/block/:name/:variant/?:dark_mode")]
    ComponentBlockDemoPath {
        name: String,
        variant: String,
        dark_mode: Option<bool>,
    },
    #[route("/dashboard/email-client?:dark_mode")]
    EmailClientDashboard { dark_mode: Option<bool> },
}

impl Route {
    pub fn iframe(&self) -> Option<bool> {
        match self {
            Route::Home { iframe, .. } => *iframe,
            Route::Docs { .. } => None,
            Route::Demos { .. } => None,
            Route::ComponentDemo { iframe, .. } => *iframe,
            Route::ComponentDemoPath { iframe, .. } => *iframe,
            Route::ComponentBlockDemo { .. } => None,
            Route::ComponentBlockDemoPath { .. } => None,
            Route::EmailClientDashboard { .. } => None,
        }
    }

    pub fn in_iframe() -> Option<bool> {
        let route: Self = router().current();
        route.iframe()
    }

    pub fn dark_mode(&self) -> Option<bool> {
        match self {
            Route::Home { dark_mode, .. } => *dark_mode,
            Route::Docs { dark_mode, .. } => *dark_mode,
            Route::Demos { dark_mode, .. } => *dark_mode,
            Route::ComponentDemo { dark_mode, .. } => *dark_mode,
            Route::ComponentDemoPath { dark_mode, .. } => *dark_mode,
            Route::ComponentBlockDemo { dark_mode, .. } => *dark_mode,
            Route::ComponentBlockDemoPath { dark_mode, .. } => *dark_mode,
            Route::EmailClientDashboard { dark_mode, .. } => *dark_mode,
        }
    }

    pub fn in_dark_mode() -> Option<bool> {
        let route: Self = router().current();
        route.dark_mode()
    }

    pub fn home() -> Self {
        let iframe = Self::in_iframe();
        let dark_mode = Self::in_dark_mode();
        Self::Home { iframe, dark_mode }
    }

    pub fn docs() -> Self {
        let dark_mode = Self::in_dark_mode();
        Self::Docs { dark_mode }
    }

    pub fn demos() -> Self {
        let dark_mode = Self::in_dark_mode();
        Self::Demos { dark_mode }
    }

    /// The canonical component-page link every internal caller (sidebar,
    /// home gallery cards, the navbar demo fixture) should use --
    /// `ComponentDemoPath` (row 46), never the legacy query-string
    /// `ComponentDemo`, so every link this app renders itself is already
    /// SSG-enumerable.
    pub fn component(name: impl ToString) -> Self {
        let iframe = Self::in_iframe();
        let dark_mode = Self::in_dark_mode();
        Self::ComponentDemoPath {
            name: name.to_string(),
            iframe,
            dark_mode,
        }
    }

    /// The canonical block-demo iframe-content link (mirrors `component`
    /// above) -- `ComponentBlockDemoPath`, used by
    /// `BlockComponentVariantHighlight` to build the `<iframe src>` for a
    /// given demo's variant.
    pub fn component_block(name: impl ToString, variant: impl ToString) -> Self {
        let dark_mode = Self::in_dark_mode();
        Self::ComponentBlockDemoPath {
            name: name.to_string(),
            variant: variant.to_string(),
            dark_mode,
        }
    }
}

/// The outermost layout, applied to EVERY route (never popped by an
/// `#[end_layout]` anywhere in `Route`) -- this is deliberately where
/// `GlobalHead` renders (dev-docs/backlog.md row 46 finding, below), not
/// `NavigationLayout`, precisely because it stays mounted across a
/// client-side transition between ANY two routes in the app, including two
/// routes that both sit outside `NavigationLayout`
/// (`ComponentBlockDemo`/`ComponentBlockDemoPath`/`EmailClientDashboard`).
///
/// **Row 46 finding:** before this construction, those three routes each
/// called `GlobalHead {}` themselves (the same shape, three separate call
/// sites -- CLAUDE.md's "two or more occurrences is a class" case). That
/// was harmless as long as the ONLY way to reach any of them was a hard
/// page load, but `ComponentBlockDemo`'s new client-side redirect (this
/// same row) to `ComponentBlockDemoPath` made it the first construction in
/// this app to ever SPA-navigate between two routes that each mount their
/// own independent `GlobalHead` instance -- and doing so silently dropped
/// `main.css`/`dx-components-theme.css`/the Google Fonts `<link>`s
/// entirely (confirmed by reading `document.styleSheets` before/after:
/// present on a hard load of the destination route directly, absent after
/// the redirect transition) rather than erroring, a hydration-adjacent
/// silent-CSS-loss defect of the exact same *class* `oracle/tier2-html/
/// global-stylesheet.spec.ts` already exists to catch for the `@import`
/// case. Root cause: `document::Link`'s own head-tag bookkeeping does not
/// re-insert a href it believes is already present, and unmounting the
/// FIRST `GlobalHead` instance (when the "from" route's tree is torn down)
/// does not clear that bookkeeping, so the SECOND instance's identical
/// `document::Link`s silently no-op. Fixed by construction, not by patching
/// each call site: `GlobalHead` now mounts exactly once, here, and simply
/// never unmounts for the lifetime of the app, so there is no unmount+
/// remount pair for the bug to trigger on, regardless of which route
/// transitions to which. Subsumes all three prior call sites
/// (`NavigationLayout`, `ComponentBlockDemo`/`ComponentBlockDemoPath`,
/// `EmailClientDashboard`); does not need a matching fix for
/// `NavigationLayout`'s own `hero.css` link, which stays where it is --
/// no evidence of the same defect there (nothing outside `NavigationLayout`
/// ever needed `hero.css`, so no cross-layout transition has ever unmounted
/// it under a sibling that also renders it).
#[component]
fn AppLayout() -> Element {
    use_effect(move || {
        theme::theme_seed();
        if let Some(dark_mode) = Route::in_dark_mode() {
            theme::set_theme(dark_mode);
        }
    });

    rsx! {
        GlobalHead {}
        Outlet::<Route> {}
    }
}

#[component]
fn NavigationLayout() -> Element {
    // Send the route to the parent window if in an iframe
    let mut initial_route = use_hook(|| CopyValue::new(true));
    use_effect(move || {
        let route: Route = router().current();

        // Only send route changes, not the initial route
        if initial_route() || !Route::in_iframe().unwrap_or_default() {
            initial_route.set(false);
            return;
        }

        let eval = document::eval(
            "let route = await dioxus.recv();
            window.top.postMessage({ 'route': route }, '*');",
        );
        let _ = eval.send(route.to_string());
    });

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/assets/hero.css") }
        Outlet::<Route> {}
        Footer {}
    }
}

/// The site's persistent top nav. Rendered as a descendant of `DocsLayout`'s
/// `SidebarProvider` on every route that has a sidebar (`Docs`, `Home`,
/// `ComponentDemo`/`ComponentHighlight`) so its leading `SidebarTrigger` can
/// sit inside this same row instead of a separate strip below it (the
/// previous layout: `NavigationLayout` rendered `Navbar {}` as a sibling of,
/// and before, `Outlet::<Route> {}`, so it was never a descendant of
/// `DocsLayout`'s `SidebarProvider` -- `DocsLayout` instead rendered its own
/// separate `header { class: "dx-docs-inset-topbar", SidebarTrigger {} }`
/// strip). `Demos` has no sidebar and renders `Navbar {}` directly (as does
/// `ComponentDemo`'s own "not found" branch, which also bypasses
/// `DocsLayout`) -- `try_consume_context` (not `consume_context`, same
/// defensive-read pattern as `HomeSidebarControls` above) is what makes
/// omitting the trigger there safe rather than a panic, since neither page
/// has a `SidebarProvider` ancestor.
#[component]
fn Navbar() -> Element {
    let in_iframe = Route::in_iframe().unwrap_or_default();
    let in_component = matches!(
        router().current(),
        Route::ComponentDemo { .. } | Route::ComponentDemoPath { .. }
    );
    let has_sidebar = try_consume_context::<SidebarCtx>().is_some();
    if in_iframe {
        return rsx! {
            nav {
                class: "dx-preview-navbar",
                aria_label: "Primary",
                border: "none",
                padding: "1rem",
                justify_content: "flex-start",
                if has_sidebar {
                    SidebarTrigger { class: "dx-navbar-sidebar-trigger" }
                }
                if in_component {
                    Link {
                        to: Route::home(),
                        class: "dx-navbar-brand",
                        aria_label: "Back",
                        ChevronLeft {
                            size: "2rem",
                            stroke: "var(--secondary-color-4)",
                        }
                    }
                }
            }
        };
    }

    rsx! {
        nav { class: "dx-preview-navbar", aria_label: "Primary",
            div { class: "dx-navbar-inner",
                div { class: "dx-navbar-primary",
                    if has_sidebar {
                        SidebarTrigger { class: "dx-navbar-sidebar-trigger" }
                    }
                    Link { to: Route::home(), class: "dx-navbar-brand",
                        img {
                            src: asset!("/assets/dioxus_color.svg"),
                            alt: "Dioxus Logo",
                            width: "18",
                            height: "18",
                        }
                        span { "dioxus-components" }
                    }
                    Link { to: Route::docs(), class: "dx-navbar-link", "Docs" }
                    Link { to: Route::demos(), class: "dx-navbar-link", "Demos" }
                }
                div { class: "dx-navbar-utilities",
                    // TODO: restore once the primitives crate is published
                    // Link {
                    //     to: "https://crates.io/crates/dioxus-components",
                    //     class: "dx-navbar-link",
                    //     aria_label: "Dioxus-Components crates.io",
                    //     Icon {
                    //         width: "24px",
                    //         height: "24px",
                    //         viewBox: ViewBox::new(0, 0, 576, 512),
                    //         path {
                    //             d: "M290.8 48.6l78.4 29.7L288 109.5 206.8 78.3l78.4-29.7c1.8-.7 3.8-.7 5.7 0zM136 92.5l0 112.2c-1.3 .4-2.6 .8-3.9 1.3l-96 36.4C14.4 250.6 0 271.5 0 294.7L0 413.9c0 22.2 13.1 42.3 33.5 51.3l96 42.2c14.4 6.3 30.7 6.3 45.1 0L288 457.5l113.5 49.9c14.4 6.3 30.7 6.3 45.1 0l96-42.2c20.3-8.9 33.5-29.1 33.5-51.3l0-119.1c0-23.3-14.4-44.1-36.1-52.4l-96-36.4c-1.3-.5-2.6-.9-3.9-1.3l0-112.2c0-23.3-14.4-44.1-36.1-52.4l-96-36.4c-12.8-4.8-26.9-4.8-39.7 0l-96 36.4C150.4 48.4 136 69.3 136 92.5zM392 210.6l-82.4 31.2 0-89.2L392 121l0 89.6zM154.8 250.9l78.4 29.7L152 311.7 70.8 280.6l78.4-29.7c1.8-.7 3.8-.7 5.7 0zm18.8 204.4l0-100.5L256 323.2l0 95.9-82.4 36.2zM421.2 250.9c1.8-.7 3.8-.7 5.7 0l78.4 29.7L424 311.7l-81.2-31.1 78.4-29.7zM523.2 421.2l-77.6 34.1 0-100.5L528 323.2l0 90.7c0 3.2-1.9 6-4.8 7.3z",
                    //             fill: "currentColor",
                    //             fill_rule: "nonzero",
                    //         }
                    //     }
                    // }
                    Link {
                        to: "https://github.com/DioxusLabs/components",
                        class: "dx-navbar-link",
                        img {
                            class: "dx-light-mode-only",
                            src: asset!("/assets/github-mark/github-mark.svg"),
                            alt: "GitHub",
                            width: "22",
                            height: "22",
                        }
                        img {
                            class: "dx-dark-mode-only",
                            src: asset!("/assets/github-mark/github-mark-white.svg"),
                            alt: "GitHub",
                            width: "22",
                            height: "22",
                        }
                    }
                    theme::DarkModeToggle {}
                    LanguageSelect {}
                }
            }
        }
    }
}

#[component]
fn Footer() -> Element {
    if Route::in_iframe().unwrap_or_default() {
        return rsx! {};
    }

    rsx! {
        footer { class: "dx-preview-footer",
            div { class: "dx-footer-inner",
                div { class: "dx-footer-brand",
                    Link { to: Route::home(), class: "dx-footer-brand-link",
                        img {
                            src: asset!("/assets/dioxus_color.svg"),
                            alt: "Dioxus Logo",
                            width: "22",
                            height: "22",
                        }
                        span { "Dioxus Components" }
                    }
                    p { class: "dx-footer-tagline",
                        "Accessible, themeable interface pieces for Dioxus apps."
                    }
                }
                nav { class: "dx-footer-nav", aria_label: "Footer",
                    div { class: "dx-footer-nav-group",
                        span { class: "dx-footer-nav-heading", "Library" }
                        Link { to: Route::home(), class: "dx-footer-link", "Components" }
                        Link { to: Route::docs(), class: "dx-footer-link", "Docs" }
                        Link { to: Route::demos(), class: "dx-footer-link", "Demos" }
                    }
                    div { class: "dx-footer-nav-group",
                        span { class: "dx-footer-nav-heading", "Project" }
                        Link {
                            to: "https://github.com/DioxusLabs/dioxus-components",
                            class: "dx-footer-link",
                            "GitHub"
                        }
                        Link {
                            to: "https://dioxuslabs.com",
                            class: "dx-footer-link",
                            "Dioxus"
                        }
                    }
                }
            }
            div { class: "dx-footer-base",
                span { class: "dx-footer-copy", "Built with Dioxus." }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
pub struct HighlightedCode {
    pub source: HighlightedSource,
}

#[component]
fn CodeBlock(source: HighlightedCode) -> Element {
    rsx! {
        div {
            class: "dx-code-block",
            tabindex: "0",
            PreviewCode { source: source.source }
        }
        CopyButton { position: "absolute", top: "0.5em", right: "0.5em" }
    }
}

#[component]
fn PreviewCode(source: HighlightedSource) -> Element {
    rsx! {
        div {
            class: "dx-preview-code-theme",
            tabindex: "0",
            Code {
                src: source,
                theme: CodeTheme::system(Theme::GITHUB_LIGHT, Theme::GITHUB_DARK),
            }
        }
    }
}

/// The Style tab's CSS source for one component file (`style.css`, or a
/// Block-kind variant's `demo.css`).
///
/// In release/SSG builds this carries the text highlighted (and embedded)
/// at compile time via `dioxus_code::code!()`, exactly like
/// `HighlightedCode` -- unchanged from before this type existed. In debug
/// builds (`dx serve`'s own dev loop) `code!()` is deliberately NOT used:
/// it expands to `include_str!(path)`, which makes rustc track the `.css`
/// file as a source dependency of this crate. `dx serve`'s file watcher
/// checks that dependency list (dioxus-cli's
/// `serve/runner.rs::handle_file_change`, via the compiled artifact's own
/// rustc dep-info `.d` file) and, finding the edited file listed there,
/// classifies the edit as `needs_full_rebuild` -- a full ~30-60s rebuild,
/// instead of the sub-second asset hot-reload every *other* `style.css`
/// edit already gets from its own separate `asset!()` stylesheet link
/// (`hotreload_bundled_assets`, checked first, but overridden once
/// `needs_full_rebuild` is also true for that same file). Measured
/// before/after and the full mechanism: `dev-docs/dev-loop.md`'s CSS
/// section. Debug builds instead carry only the file's own bundled
/// `asset!()` URL -- `asset!()` does not embed the file's text at compile
/// time, so referencing it costs nothing extra; `CssCodeBlock` below
/// fetches and highlights that URL's text lazily, client-side, the first
/// time the Style tab is rendered.
#[derive(Clone, PartialEq)]
struct CssHighlight {
    /// Pre-highlighted content -- release/SSG builds only.
    #[cfg(not(debug_assertions))]
    embedded: HighlightedCode,
    /// This file's own bundled asset URL -- debug builds only.
    #[cfg(debug_assertions)]
    asset: Asset,
}

/// Renders a CSS Style tab from a [`CssHighlight`]. Release/SSG builds
/// show the compile-time-highlighted text directly, identical to
/// `CodeBlock`.
#[cfg(not(debug_assertions))]
#[component]
fn CssCodeBlock(source: CssHighlight) -> Element {
    rsx! {
        CodeBlock { source: source.embedded }
    }
}

/// Debug builds' half of [`CssCodeBlock`]: fetch the CSS from its own
/// asset URL and highlight it at runtime instead of at compile time --
/// see `CssHighlight`'s doc comment for why.
#[cfg(debug_assertions)]
#[component]
fn CssCodeBlock(source: CssHighlight) -> Element {
    rsx! {
        LazyCssCodeBlock { asset: source.asset }
    }
}

/// Fetches `asset`'s own text over HTTP -- the same URL its
/// `document::Link`/`document::Stylesheet` sibling already loads as a live
/// stylesheet -- and highlights it client-side once it arrives, so a
/// `style.css` edit is visible here exactly like the live stylesheet
/// already is: via `dx serve`'s asset hot-reload, not a crate rebuild.
#[cfg(debug_assertions)]
#[component]
fn LazyCssCodeBlock(asset: Asset) -> Element {
    let url = asset.to_string();
    let css = use_resource(move || {
        let url = url.clone();
        async move { fetch_asset_text(url).await }
    });

    // Fully qualified rather than imported: this file already has its own,
    // unrelated local `enum Language` (the i18n language switcher above),
    // and importing `dioxus_code::Language` under that same bare name
    // shadows it -- confirmed live: it breaks the i18n enum's own inherent
    // `impl` block ("cannot define inherent `impl` for a type outside of
    // the crate") the moment both are in scope together.
    let placeholder = |text: &'static str| HighlightedCode {
        source: HighlightedSource::from_static_parts(text, dioxus_code::Language::Css, &[]),
    };

    // Cloned out of the resource's read guard into an owned value up front
    // (rather than matching on `&*css.read()` directly) so the guard drops
    // immediately, before any of the `rsx!` arms below run -- avoids tying
    // the returned `Element` to that guard's borrow.
    let state: Option<Option<String>> = css.read().clone();

    match state {
        Some(Some(text)) => rsx! {
            CodeBlock {
                source: HighlightedCode {
                    source: dioxus_code::SourceCode::new(dioxus_code::Language::Css, text).into(),
                },
            }
        },
        Some(None) => rsx! { CodeBlock { source: placeholder("/* failed to load style.css */") } },
        None => rsx! { CodeBlock { source: placeholder("/* loading style.css... */") } },
    }
}

/// One-shot fetch of a same-origin asset's raw text, used only by
/// [`LazyCssCodeBlock`] (debug builds). Mirrors this codebase's other
/// one-shot `document::eval` helpers (e.g. `primitives/src/input_otp.rs`'s
/// `snap_caret_to_slot`) -- a scoped async JS snippet, not a persistent
/// listener.
#[cfg(debug_assertions)]
async fn fetch_asset_text(url: String) -> Option<String> {
    let mut eval = document::eval(
        r#"
        const url = await dioxus.recv();
        try {
            const res = await fetch(url);
            dioxus.send(res.ok ? await res.text() : null);
        } catch (e) {
            dioxus.send(null);
        }
        "#,
    );
    let _ = eval.send(url);
    eval.recv::<Option<String>>().await.ok().flatten()
}

#[component]
fn CopyButton(#[props(extends=GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    let mut copied = use_signal(|| false);

    rsx! {
        button {
            class: "dx-copy-button",
            r#type: "button",
            aria_label: "Copy code",
            "data-copied": copied,
            "onclick": "const visiblePre = Array.from(this.parentNode.querySelectorAll('pre')).find((pre) => pre.offsetParent !== null); navigator.clipboard.writeText(visiblePre ? visiblePre.innerText : Array.from(this.parentNode.childNodes).filter((node) => node !== this).map((node) => node.textContent).join('').trim());",
            onclick: move |_| copied.set(true),
            ..attributes,
            if copied() {
                CheckIcon {}
            } else {
                CopyIcon {}
            }
        }
    }
}

#[component]
fn CopyIcon() -> Element {
    rsx! {
        Copy {
            width: "24px",
            height: "24px",
        }
    }
}

#[component]
fn CheckIcon() -> Element {
    rsx! {
        Check {
            width: "24px",
            height: "24px",
        }
    }
}

#[derive(PartialEq, Display, EnumIter, EnumString)]
enum Language {
    English,
    French,
    Spanish,
    German,
}

impl Language {
    const fn id(&self) -> LanguageIdentifier {
        match self {
            Language::English => langid!("en-US"),
            Language::French => langid!("fr-FR"),
            Language::Spanish => langid!("es-ES"),
            Language::German => langid!("de-DE"),
        }
    }

    const fn flag(&self) -> &'static str {
        match self {
            Language::English => "🇬🇧",
            Language::French => "🇫🇷",
            Language::Spanish => "🇪🇸",
            Language::German => "🇩🇪",
        }
    }

    fn display_name(&self) -> String {
        format!("{} {}", self.flag(), self.localize_name())
    }

    const fn localize_name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::French => "Français",
            Language::Spanish => "Español",
            Language::German => "Deutsch",
        }
    }
}

#[component]
fn LanguageSelect() -> Element {
    let mut current_lang = use_signal(|| Language::English);

    rsx! {
        document::Stylesheet { href: asset!("/assets/language-select.css") }
        div { class: "dx-language-container",
            span { class: "dx-language-select-container",
                select {
                    class: "dx-language-select",
                    aria_label: "Language",
                    onchange: move |e| {
                        let name = e.value().parse().unwrap_or(current_lang.to_string());
                        if let Ok(lang) = Language::from_str(&name) {
                            current_lang.set(lang);
                        }
                        let id = current_lang.read().id();
                        tracing::info!("Current lang: {id}");
                        // backlog row 84 finding 5: this call was commented
                        // out, so the dropdown changed its own displayed
                        // selection but never touched the app's actual
                        // locale -- every `tid!`/`t!` call site (e.g. the
                        // date_picker/calendar internationalized demos'
                        // `on_format_month`/`on_format_*_placeholder`
                        // callbacks) stayed on `en-US` regardless. `i18n()`
                        // (`dioxus_i18n::prelude`) reads the same `I18n`
                        // context `use_init_i18n` provided in `App` above;
                        // `set_language` writes its `active_bundle` signal,
                        // which every `tid!` call reads reactively
                        // (`I18n::try_translate_with_args`'s own
                        // `self.active_bundle.read()`), so this now
                        // propagates live to every already-mounted
                        // translated string, not just future ones.
                        i18n().set_language(id);
                    },
                    for lang in Language::iter() {
                        option {
                            value: lang.to_string(),
                            selected: lang == *current_lang.read(),
                            {lang.display_name()}
                        }
                    }
                }
                span { class: "dx-language-select-value",
                    {current_lang.read().flag()}
                    ChevronDown {
                        class: "dx-select-expand-icon",
                        size: "24px",
                        stroke: "var(--secondary-color-4)",
                    }
                }
            }
        }
    }
}

#[component]
fn ComponentCode(
    rs_highlighted: HighlightedCode,
    css_highlighted: CssHighlight,
    #[props(default = ComponentType::Normal)] component_type: ComponentType,
) -> Element {
    rsx! {
        Tabs {
            default_value: "main.rs",
            border_bottom_left_radius: "0.5rem",
            border_bottom_right_radius: "0.5rem",
            horizontal: true,
            width: "100%",
            TabList {
                TabTrigger { value: "main.rs", index: 0usize, "main.rs" }
                TabTrigger { value: "style.css", index: 1usize, "style.css" }
                if component_type != ComponentType::Block {
                    TabTrigger { value: "dx-components-theme.css", index: 2usize, "dx-components-theme.css" }
                }
            }
            div {
                width: "100%",
                height: "100%",
                display: "flex",
                flex_direction: "column",
                justify_content: "center",
                align_items: "center",
                TabContent {
                    index: 0usize,
                    padding: 0,
                    value: "main.rs",
                    width: "100%",
                    position: "relative",
                    CodeBlock { source: rs_highlighted }
                }
                TabContent {
                    index: 1usize,
                    padding: 0,
                    value: "style.css",
                    width: "100%",
                    position: "relative",
                    CssCodeBlock { source: css_highlighted }
                }
                if component_type != ComponentType::Block {
                    TabContent {
                        index: 2usize,
                        padding: 0,
                        value: "dx-components-theme.css",
                        width: "100%",
                        position: "relative",
                        CssCodeBlock { source: THEME_CSS }
                    }
                }
            }
        }
    }
}

/// The `/docs` "Overview" page's own section headings, in the order
/// `Docs()` renders them. Single-sourced here so `Docs()`'s `h2`s and
/// `DocsLayout`'s "Start" sidebar sub-items (rendered off the
/// `page_sections` prop `Docs()` passes it, see `DocsLayout`) can never
/// drift out of sync with each other -- each `h2`'s visible text and the
/// matching sub-link's label both read from this one array, and
/// `docs_section_slug` derives both the `h2`'s `id` and the sub-link's
/// `href` from that same text rather than a hand-typed anchor.
///
/// `Home()`'s own `HOME_SECTIONS` (near `Home()`, below) is the same
/// pattern applied to the homepage's own sections -- both are plain
/// `&'static [&'static str]` so `DocsLayout`'s `page_sections` prop can
/// carry either one without caring which page it came from.
const DOCS_SECTIONS: &[&str] = &["How it works", "Add a component", "Recommended workflow"];

/// Turns a heading's visible text into a same-page anchor id: lowercased,
/// with every run of non-alphanumeric characters (spaces included)
/// collapsed to a single hyphen and none left dangling at either end.
/// "How it works" -> "how-it-works". Generic over any page's headings --
/// `Home()` reuses this as-is for its own sections.
fn docs_section_slug(title: &str) -> String {
    let mut slug = String::with_capacity(title.len());
    let mut pending_hyphen = false;
    for ch in title.chars() {
        if ch.is_ascii_alphanumeric() {
            if pending_hyphen && !slug.is_empty() {
                slug.push('-');
            }
            slug.push(ch.to_ascii_lowercase());
            pending_hyphen = false;
        } else {
            pending_hyphen = true;
        }
    }
    slug
}

#[component]
fn Docs(dark_mode: Option<bool>) -> Element {
    rsx! {
        DocsLayout { active: DocsNavActive::Overview, page_sections: Some(DOCS_SECTIONS),
            article { class: "dx-docs-page dx-docs-prose",
                header { class: "dx-docs-page-header",
                    p { class: "dx-docs-eyebrow", "Docs" }
                    h1 { "Build with dioxus-components" }
                    p {
                        "dioxus-components is a collection of styled, accessible Dioxus components designed to be copied into your app. Use the CLI when you want the fastest path, or copy the source when you want complete ownership."
                    }
                }
                section { class: "dx-docs-section",
                    h2 { id: "{docs_section_slug(DOCS_SECTIONS[0])}", "{DOCS_SECTIONS[0]}" }
                    p {
                        "dioxus-components is not yet on crates.io. For now, components ship from this Git repository — you point your app at the primitives library here, then pull individual styled components into your source tree with the Dioxus CLI."
                    }
                    p {
                        "Start by adding the underlying primitives library to your app's "
                        code { "Cargo.toml" }
                        " from the Git path:"
                    }
                    pre {
                        code { r#"dioxus-primitives = {{ git = "https://github.com/DioxusLabs/components" }}"# }
                    }
                    p {
                        "The styled components live in this same repository as a registry. The "
                        code { "dx components" }
                        " subcommand of the Dioxus CLI is what reads from it. To see everything that's available:"
                    }
                    div { class: "dx-docs-command",
                        code { "dx components list" }
                        CopyCommandButton { command: "dx components list".to_string() }
                    }
                    p {
                        "Then add a specific component to your app — swap "
                        code { "button" }
                        " for any name from the list:"
                    }
                    div { class: "dx-docs-command",
                        code { "dx components add button" }
                        CopyCommandButton { command: "dx components add button".to_string() }
                    }
                    p {
                        "Each "
                        code { "dx components add" }
                        " copies the component's Rust source and its stylesheet directly into your project. Once it's in your tree, the code is yours: keep the included CSS as-is, replace the class names with Tailwind utilities, or rewrite the styles from scratch. There is no runtime dependency on this registry after the copy."
                    }
                }
                section { class: "dx-docs-section",
                    h2 { id: "{docs_section_slug(DOCS_SECTIONS[1])}", "{DOCS_SECTIONS[1]}" }
                    p { "Run the add command from your Dioxus app. Swap the final name for any component in the sidebar." }
                    div { class: "dx-docs-command",
                        code { "dx components add button" }
                        CopyCommandButton { command: "dx components add button".to_string() }
                    }
                    p { class: "dx-docs-muted",
                        "If you do not have the Dioxus CLI yet, install it once with cargo install dioxus-cli."
                    }
                }
                section { class: "dx-docs-section",
                    h2 { id: "{docs_section_slug(DOCS_SECTIONS[2])}", "{DOCS_SECTIONS[2]}" }
                    ol {
                        li { "Pick a component from the sidebar or catalog." }
                        li { "Preview the default example and variants." }
                        li { "Run the CLI command shown on the component page." }
                        li { "Customize the generated Rust and CSS to fit your app." }
                    }
                }
            }
        }
    }
}

/// Which sidebar nav entry (if any) corresponds to the page currently being
/// rendered inside `DocsLayout`, for active-link highlighting.
#[derive(Clone, Copy, PartialEq)]
enum DocsNavActive {
    /// The `/docs` written-guide page ("Start" > "Overview").
    Overview,
    /// A `/component/?name=...` page, keyed by the component's raw name.
    Component(&'static str),
    /// The homepage (`/`) -- part of this same nav now, but distinct from
    /// `Overview`: before the homepage had a sidebar at all, `None` here
    /// meant "on Docs", so a bare `Option<&'static str>` would have wrongly
    /// lit up "Overview" while browsing the gallery too.
    Home,
}

/// The heading-derived sub-item list nested under whichever "Start" nav
/// item corresponds to the page currently being viewed (see `DocsLayout`'s
/// `page_sections` prop and its "Start" group markup, below). Written once
/// and called from both the "Home" and "Overview" `SidebarMenuItem`s so
/// their sub-items can never diverge in markup shape -- same `SidebarMenuSub`/
/// `SidebarMenuSubItem`/`SidebarMenuSubButton` structure, same plain same-
/// page `a` anchor (not the router's `Link`) with a `Hash` icon, regardless
/// of which page's sections are being listed.
fn page_sections_submenu(sections: &'static [&'static str]) -> Element {
    rsx! {
        SidebarMenuSub {
            for title in sections.iter().copied() {
                SidebarMenuSubItem { key: "{title}",
                    SidebarMenuSubButton {
                        as: move |attributes: Vec<Attribute>| rsx! {
                            a {
                                href: "#{docs_section_slug(title)}",
                                ..attributes,
                                Hash { size: "0.875rem", "aria-hidden": "true" }
                                span { "{title}" }
                            }
                        },
                    }
                }
            }
        }
    }
}

/// The site's real navigation shell: `SidebarProvider`/`Sidebar`/
/// `SidebarInset` composed the same way `sidebar/variants/main/mod.rs`'s own
/// demo (`Demo()`) composes them -- a `SidebarContent` of `SidebarGroup`s
/// holding `SidebarMenu`/`SidebarMenuItem`/`SidebarMenuButton`s, and a
/// `SidebarInset` whose own leading `header` holds just the `SidebarTrigger`
/// (mirrors both that demo's `Demo()` and the email-client dashboard's
/// `EmailClient()`, `dashboard/views/email_client/mod.rs`). `page` content
/// renders as the inset's children -- callers no longer wrap it in a `<main>`
/// themselves, since `SidebarInset` already renders the page's one `<main>`
/// landmark (see `EmailClient()`'s own comment on that same point).
#[component]
fn DocsLayout(
    active: DocsNavActive,
    // Only the homepage passes these (`Home()`, via its own `Signal`s that
    // its `ComponentGalleryPreview`'s Sidebar card also writes to) so its
    // card's Side/Collapse buttons can reconfigure this same, real nav
    // in place. Every other call site (`/docs`, component pages) leaves
    // both `None` and gets its own fresh, fixed-default `Sidebar` state
    // here -- Dioxus already unmounts/remounts this whole layout on route
    // change (a new component instance per route), so that default can't
    // leak in from whatever the homepage's own sidebar was last set to.
    #[props(default)] side: Option<Signal<SidebarSide>>,
    #[props(default)] collapsible: Option<Signal<SidebarCollapsible>>,
    // Only `Home()` passes `Some(false)`, so the homepage's real site nav
    // starts closed/out-of-the-way behind its hero + gallery content (that
    // page already has its own primary draw, and the same `SidebarTrigger`
    // in the topbar still opens it). Every other call site (`/docs`,
    // component pages) leaves this `None` and keeps the previous, open-by-
    // default behavior -- those pages need the nav immediately usable.
    // `SidebarProvider`'s own `default_open` (`components/sidebar/
    // component.rs`) already exists for exactly this and just needed
    // threading through here.
    #[props(default)] default_open: Option<bool>,
    // The CURRENT page's own section headings, if it has any -- `Docs()`
    // passes its `DOCS_SECTIONS`, `Home()` passes its `HOME_SECTIONS`.
    // Page-agnostic on purpose: which top-level "Start" nav item this
    // grows a heading-derived sub-list under is decided below purely from
    // `active` (`DocsNavActive::Home` nests it under "Home",
    // `DocsNavActive::Overview` under "Overview"), so this same prop
    // shape carries either page's list without `DocsLayout` needing a
    // separate prop -- or a separate `SidebarMenuSub` rendering block --
    // per page. `ComponentHighlight()` leaves this `None` and keeps
    // rendering both "Home" and "Overview" as plain, sub-item-less
    // entries, same as before this page had sections of its own.
    #[props(default)] page_sections: Option<&'static [&'static str]>,
    children: Element,
) -> Element {
    // Always call both hooks (rather than only inside an `unwrap_or_else`
    // closure) so hook order never depends on whether the caller passed
    // `side`/`collapsible` -- these locals are simply unused/discarded
    // when a real signal was supplied.
    let default_side = use_signal(|| SidebarSide::Left);
    let default_collapsible = use_signal(|| SidebarCollapsible::Offcanvas);
    let side = side.unwrap_or(default_side);
    let collapsible = collapsible.unwrap_or(default_collapsible);

    rsx! {
        SidebarProvider { class: "dx-docs-shell", default_open: default_open.unwrap_or(true),
            // `Navbar` renders first, as a direct child of this
            // `SidebarProvider` (whose own render is just `div { class:
            // "dx-sidebar-wrapper", {children} }` -- see `component.rs`) so
            // its `SidebarTrigger` finds this provider's `SidebarCtx` via
            // `use_context`. `Sidebar`/`SidebarInset` are grouped under
            // their own `.dx-docs-shell-body` row beneath it rather than
            // sitting as siblings of `Navbar` directly, because the base
            // `.dx-sidebar-wrapper` rule (`sidebar/style.css`) is a flex ROW
            // sized for exactly that pair -- a third flex child would sit
            // beside them instead of stacking above (see `main.css`'s
            // `.dx-docs-shell-body` comment for the full rationale).
            Navbar {}
            div { class: "dx-docs-shell-body",
                Sidebar { side: side(), collapsible: collapsible(),
                    SidebarContent {
                        SidebarGroup {
                            SidebarGroupLabel {
                                BookOpen { size: "1rem", "aria-hidden": "true" }
                                span { "Start" }
                            }
                            SidebarMenu {
                                SidebarMenuItem {
                                    SidebarMenuButton {
                                        is_active: active == DocsNavActive::Home,
                                        as: move |attributes: Vec<Attribute>| rsx! {
                                            Link { to: Route::home(), attributes,
                                                House { size: "1rem", "aria-hidden": "true" }
                                                span { "Home" }
                                            }
                                        },
                                    }
                                    // Only rendered while `Home()` is the page being viewed
                                    // AND it passed its own `page_sections` (`HOME_SECTIONS`)
                                    // -- see `DocsLayout`'s prop doc comment and
                                    // `page_sections_submenu`. On `/docs` or a component page
                                    // this is `None`/doesn't match `active`, so "Home" stays
                                    // the plain, sub-item-less link it always was.
                                    if active == DocsNavActive::Home {
                                        if let Some(sections) = page_sections {
                                            {page_sections_submenu(sections)}
                                        }
                                    }
                                }
                                SidebarMenuItem {
                                    SidebarMenuButton {
                                        is_active: active == DocsNavActive::Overview,
                                        as: move |attributes: Vec<Attribute>| rsx! {
                                            Link { to: Route::docs(), attributes,
                                                FileText { size: "1rem", "aria-hidden": "true" }
                                                span { "Overview" }
                                            }
                                        },
                                    }
                                    // Mirrors the "Home" item above, but scoped to `Docs()`
                                    // being the page being viewed instead -- same shared
                                    // `page_sections_submenu`, so `/docs`'s 3 sub-items
                                    // (`DOCS_SECTIONS`) render exactly as they did before
                                    // "Home" gained this same treatment.
                                    if active == DocsNavActive::Overview {
                                        if let Some(sections) = page_sections {
                                            {page_sections_submenu(sections)}
                                        }
                                    }
                                }
                            }
                        }
                        for cat in components::ComponentCategory::ALL.iter().copied() {
                            SidebarGroup { key: "{cat.label()}",
                                SidebarGroupLabel {
                                    match cat {
                                        components::ComponentCategory::Forms => rsx! {
                                            SquareCheck { size: "1rem", "aria-hidden": "true" }
                                        },
                                        components::ComponentCategory::Navigation => rsx! {
                                            Compass { size: "1rem", "aria-hidden": "true" }
                                        },
                                        components::ComponentCategory::Overlays => rsx! {
                                            Layers { size: "1rem", "aria-hidden": "true" }
                                        },
                                        components::ComponentCategory::Feedback => rsx! {
                                            Bell { size: "1rem", "aria-hidden": "true" }
                                        },
                                        components::ComponentCategory::Disclosure => rsx! {
                                            ChevronsUpDown { size: "1rem", "aria-hidden": "true" }
                                        },
                                        components::ComponentCategory::DataDisplay => rsx! {
                                            LayoutGrid { size: "1rem", "aria-hidden": "true" }
                                        },
                                    }
                                    span { "{cat.label()}" }
                                }
                                SidebarMenu {
                                    for component in components::DEMOS.iter().filter(|c| components::category_of(c.name) == cat) {
                                        SidebarMenuItem { key: "{component.name}",
                                            SidebarMenuButton {
                                                is_active: active == DocsNavActive::Component(component.name),
                                                as: move |attributes: Vec<Attribute>| rsx! {
                                                    Link {
                                                        to: Route::component(component.name),
                                                        attributes,
                                                        {component.name.replace("_", " ")}
                                                    }
                                                },
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
                SidebarInset { {children} }
            }
        }
    }
}

struct DemoEntry {
    tag: &'static str,
    title: &'static str,
    description: &'static str,
    route: fn() -> Route,
    thumb: fn() -> Element,
}

fn email_client_thumb() -> Element {
    rsx! {
        Mail { size: "56", stroke_width: "1.4" }
    }
}

const DEMO_ENTRIES: &[DemoEntry] = &[DemoEntry {
    tag: "Dashboard",
    title: "Email client",
    description:
        "Multi-pane mail app composed from the sidebar, item list, reading pane, and compose modal.",
    route: || Route::EmailClientDashboard {
        dark_mode: Route::in_dark_mode(),
    },
    thumb: email_client_thumb,
}];

#[component]
fn Demos(dark_mode: Option<bool>) -> Element {
    rsx! {
        // No sidebar on this page -- `Navbar`'s own `try_consume_context`
        // check (see its doc comment) means the trigger just doesn't
        // render here, rather than panicking on a missing `SidebarCtx`.
        Navbar {}
        main { class: "dx-home-page", role: "main",
            section { class: "dx-home-section",
                header { class: "dx-section-header",
                    span { class: "dx-section-eyebrow", "Demos" }
                    h1 { class: "dx-section-title", "Demo apps" }
                    p { class: "dx-section-summary",
                        "End-to-end app demos assembled from these primitives. Open one to explore the layout and try it live."
                    }
                }
                ul { class: "dx-demos-grid",
                    for entry in DEMO_ENTRIES {
                        li { class: "dx-demos-item",
                            Link {
                                to: (entry.route)(),
                                class: "dx-demos-card",
                                div { class: "dx-demos-card-thumb", {(entry.thumb)()} }
                                div { class: "dx-demos-card-meta",
                                    span { class: "dx-demos-card-tag", "{entry.tag}" }
                                    h2 { class: "dx-demos-card-title", "{entry.title}" }
                                    p { class: "dx-demos-card-description", "{entry.description}" }
                                    span { class: "dx-demos-card-cta",
                                        "Open demo"
                                        ArrowRight { size: "16", stroke_width: "1.6" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Legacy query-string deep link (`/component/?name=<x>&`, dev-docs/
/// backlog.md row 46). Renders the SAME markup regardless of `name` -- a
/// generic, name-agnostic loading shell -- on every platform, so there is
/// nothing for a server prerender and a client hydration to ever disagree
/// on structurally (unlike the old behavior this replaces, which tried to
/// resolve `name` against `components::DEMOS` here and rendered the
/// "Component not found" shell server-side for every X, since `dx build
/// --ssg` only ever prerenders this route with an EMPTY query -- see
/// `Route::ComponentDemo`'s own doc comment). Once mounted client-side (in
/// a real browser, where `name` IS available from the real URL), redirects
/// to the canonical `ComponentDemoPath` route so old links/bookmarks/specs
/// keep landing on the right component.
#[component]
fn ComponentDemo(iframe: Option<bool>, dark_mode: Option<bool>, name: String) -> Element {
    let nav = navigator();
    use_effect(move || {
        nav.replace(Route::ComponentDemoPath {
            name: name.clone(),
            iframe,
            dark_mode,
        });
    });

    rsx! {
        Navbar {}
        main { class: "dx-component-demo-redirect", role: "main",
            p { "Loading component…" }
        }
    }
}

/// Canonical, SSG-enumerable component page (`/component/<name>/`, row 46).
/// `name` is a PATH segment here, so the server prerender for a given `name`
/// and the client's initial render for that same URL always resolve to the
/// same branch below -- unlike the legacy `ComponentDemo` query route, a
/// genuinely nonexistent `name` shows "Component not found" identically on
/// both sides rather than on every name unconditionally.
#[component]
fn ComponentDemoPath(iframe: Option<bool>, dark_mode: Option<bool>, name: String) -> Element {
    let route = router().current::<Route>();
    tracing::info!("route: {route}");
    let Some(demo) = components::DEMOS
        .iter()
        .find(|demo| demo.name == name)
        .cloned()
    else {
        // Bypasses `ComponentHighlight`/`DocsLayout` entirely, so -- like
        // `Demos` -- there is no `SidebarProvider` ancestor here either;
        // `Navbar {}` renders its trigger-less fallback (see its own doc
        // comment) rather than being left off this page altogether.
        return rsx! {
            Navbar {}
            main { class: "dx-component-demo-not-found",
                h3 { "Component not found" }
                p { "The requested component does not exist." }
            }
        };
    };
    rsx! {
        ComponentHighlight { demo }
    }
}

#[component]
fn ComponentHighlight(demo: ComponentDemoData) -> Element {
    let ComponentDemoData {
        name: raw_name,
        r#type,
        docs,
        description,
        variants,
        component,
        style,
    } = demo;
    let name = raw_name.replace("_", " ");
    let [main, variants @ ..] = variants else {
        unreachable!("Expected at least one variant for component: {}", name);
    };

    rsx! {
        DocsLayout { active: DocsNavActive::Component(raw_name),
            article { class: "dx-component-page",
                header { class: "dx-component-page-header",
                    p { class: "dx-docs-eyebrow", "Component" }
                    div { class: "dx-component-page-title-row",
                        h1 { "{name}" }
                        ComponentInstallCommand { name: raw_name }
                    }
                    p { "{description}" }
                }
                section { class: "dx-component-section",
                    match r#type {
                        ComponentType::Normal => rsx! {
                            ComponentVariantHighlight { variant: main.clone(), main_variant: true, component_name: None }
                        },
                        ComponentType::Block => rsx! {
                            BlockComponentVariantHighlight { variant: main.clone(), main_variant: true, component_name: raw_name, show_install: false }
                        },
                    }
                }
                section { class: "dx-component-section",
                    div { class: "dx-component-section-heading",
                        h2 { "Installation" }
                        p { "Use the CLI command for the common path, or copy the component files manually." }
                    }
                    details { class: "dx-component-manual-install dx-component-manual-install-code",
                        summary { "Manual installation files" }
                        ManualComponentInstallation { component, style }
                    }
                }
                section { class: "dx-component-section dx-docs-prose",
                    div { class: "dx-component-section-heading",
                        h2 { "Usage notes" }
                    }
                    div { class: "dx-component-description",
                        div { dangerous_inner_html: docs }
                    }
                }
                if !variants.is_empty() {
                    section { class: "dx-component-section",
                        div { class: "dx-component-section-heading",
                            h2 { "Variants" }
                            p { "Alternative examples for common configurations." }
                        }
                        for variant in variants {
                            div { class: "dx-component-variant",
                                match r#type {
                                    ComponentType::Normal => rsx! {
                                        ComponentVariantHighlight { variant: variant.clone(), main_variant: false, component_name: None }
                                    },
                                    ComponentType::Block => rsx! {
                                        BlockComponentVariantHighlight { variant: variant.clone(), main_variant: false, component_name: raw_name, show_install: false }
                                    },
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn ComponentInstallCommand(name: &'static str) -> Element {
    let command = format!("dx components add {name}");

    rsx! {
        div { class: "dx-component-inline-command",
            code { "{command}" }
            CopyCommandButton { command: command.clone() }
        }
    }
}

#[component]
fn ManualComponentInstallation(component: HighlightedCode, style: CssHighlight) -> Element {
    rsx! {
        div { class: "dx-component-manual-copy",
            p { class: "dx-docs-muted",
                "Copy the component source and CSS into your app. Import the shared theme CSS once near your app root."
            }
        }
        div { class: "dx-component-manual-code",
            ComponentCode {
                rs_highlighted: component,
                css_highlighted: style,
                component_type: ComponentType::Normal,
            }
        }
    }
}

#[component]
fn ComponentVariantHighlight(
    variant: ComponentVariantDemoData,
    main_variant: bool,
    component_name: Option<&'static str>,
) -> Element {
    let ComponentVariantDemoData {
        name,
        rs_highlighted: highlighted,
        css_highlighted: _,
        component: Comp,
    } = variant;
    // Every variant of a "Normal" component renders on the SAME page (the
    // "Variants" section below loops over all of them, main included --
    // see `ComponentHighlight` above), so a literal `"component-preview-
    // frame"` id can't be shared across more than one of these `TabContent`s
    // without producing duplicate ids -- confirmed by execution: adding
    // `popover`'s `non_modal` variant (dev-docs/backlog.md rows 19/7) gave
    // the popover page two, and `popover.spec.ts`'s basic test (`page.
    // locator("#component-preview-frame")`, no `.first()`) failed with a
    // strict-mode "2 elements" violation. Fixed by construction, not
    // per-instance: `main_variant` (already the exact signal distinguishing
    // the one call site above from every entry in the loop below) picks
    // exactly `component-preview-frame` for `main` -- unchanged, so every
    // existing single-variant component page and every spec that already
    // targets that literal id keeps working untouched -- and
    // `component-preview-frame-<name>` for every other variant, unique per
    // variant name so two additional variants on the same page (e.g.
    // `calendar[simple, internationalized, range, multi_month,
    // unavailable_dates]`, already exposed to this same defect before this
    // fix, silently tolerated via `.first()` in `calendar.spec.ts`) can
    // never collide with each other either.
    let frame_id = if main_variant {
        "component-preview-frame".to_string()
    } else {
        format!("component-preview-frame-{name}")
    };
    rsx! {
        if !main_variant {
            h3 { class: "dx-component-variant-title", "{name}" }
        }
        Tabs {
            default_value: "Demo",
            border_bottom_left_radius: "0.5rem",
            border_bottom_right_radius: "0.5rem",
            horizontal: true,
            width: "100%",
            variant: TabsVariant::Ghost,
            div { class: "dx-component-tabs-header",
                TabList {
                    TabTrigger { value: "Demo", index: 0usize, "DEMO" }
                    TabTrigger { value: "Code", index: 1usize, "CODE" }
                }
                if let Some(component_name) = component_name {
                    ComponentInstallCommand { name: component_name }
                }
            }
            div {
                width: "100%",
                height: "100%",
                display: "flex",
                flex_direction: "column",
                justify_content: "center",
                align_items: "center",
                TabContent {
                    index: 0usize,
                    class: "dx-component-preview-frame",
                    id: "{frame_id}",
                    value: "Demo",
                    width: "100%",
                    position: "relative",
                    Comp {}
                }
                TabContent {
                    index: 1usize,
                    class: "dx-component-preview-frame",
                    value: "Code",
                    width: "100%",
                    position: "relative",
                    CodeBlock { source: highlighted }
                }
            }
        }
    }
}

#[component]
fn BlockComponentVariantHighlight(
    component_name: &'static str,
    variant: ComponentVariantDemoData,
    main_variant: bool,
    show_install: bool,
) -> Element {
    let ComponentVariantDemoData {
        name,
        rs_highlighted: highlighted,
        css_highlighted,
        component: _,
    } = variant;

    // The canonical, SSG-enumerable path route (row 46) -- NOT the legacy
    // `Route::ComponentBlockDemo` query form, so every block demo's iframe
    // content this app renders itself is already a page `dx build --ssg`
    // can prerender per (name, variant) pair.
    let route_path = Route::component_block(component_name, name).to_string();

    let iframe_src = match router().prefix() {
        Some(prefix) => format!("{prefix}{route_path}"),
        None => route_path,
    };

    // Same defect, same construction as `ComponentVariantHighlight`'s
    // identical `frame_id` above (see its doc comment for the full
    // rationale) -- this is the Block-kind sibling of that function, and
    // every "Block" component with more than one variant (e.g. `sidebar(
    // block)[floating, inset]`) renders all of them on this same
    // `/component/?name=<name>&` page (`ComponentHighlight` above doesn't
    // branch on kind for *that* -- only for which of these two functions
    // renders each variant), so it was exposed to the identical duplicate-
    // id defect, just not yet caught by a spec: `sidebar.spec.ts` only ever
    // drives the single-variant `/component/block/?name=sidebar&variant=
    // ...&` route (`ComponentBlockDemo`, a different function entirely,
    // one variant per page by construction), never this one.
    let frame_id = if main_variant {
        "component-preview-frame".to_string()
    } else {
        format!("component-preview-frame-{name}")
    };

    rsx! {
        if !main_variant {
            h3 { class: "dx-component-variant-title", "{name}" }
        }
        Tabs {
            default_value: "Preview",
            border_bottom_left_radius: "0.5rem",
            border_bottom_right_radius: "0.5rem",
            horizontal: true,
            width: "100%",
            variant: TabsVariant::Ghost,
            div { class: "dx-component-tabs-header",
                TabList {
                    TabTrigger { value: "Preview", index: 0usize, "PREVIEW" }
                    TabTrigger { value: "Code", index: 1usize, "CODE" }
                }
                if show_install {
                    ComponentInstallCommand { name: component_name }
                }
            }
            div {
                width: "100%",
                height: "100%",
                display: "flex",
                flex_direction: "column",
                justify_content: "center",
                align_items: "center",
                TabContent {
                    index: 0usize,
                    id: "{frame_id}",
                    value: "Preview",
                    width: "100%",
                    position: "relative",
                    // `overflow-x: clip` is set globally on `html`/`body`
                    // (`assets/main.css`) to keep the page itself from ever
                    // growing a horizontal scrollbar -- so an iframe wider
                    // than the space this page happens to leave it would
                    // otherwise just get silently clipped, not pushed into
                    // a scrollbar. This local `auto` opts *this* frame back
                    // into scrolling on its own, so `.dx-block-demo-iframe`'s
                    // desktop-only `min-width` (`assets/main.css`) stays
                    // reachable instead of being cut off.
                    overflow_x: "auto",
                    iframe {
                        src: "{iframe_src}",
                        width: "100%",
                        // Block demos embed here at whatever width this
                        // page's own layout leaves them -- since the site
                        // nav itself became a real `Sidebar` (`DocsLayout`,
                        // this file), that can now be under 768px on a
                        // normal desktop viewport (e.g. ~684px measured on
                        // the live `sidebar` demo). Any block component
                        // that (like `Sidebar` itself, `components/sidebar/
                        // component.rs`'s `MOBILE_BREAKPOINT`) switches to
                        // a different, closed-by-default layout below that
                        // breakpoint would silently render nothing in the
                        // preview: its own `window.innerWidth` is this
                        // iframe's content width, not the outer page's, so
                        // it has no way to know it's being squeezed rather
                        // than genuinely viewed on a narrow screen.
                        //
                        // A PREVIOUS fix forced a flat, always-on
                        // `min-width: 820px` here to keep the preview
                        // desktop-sized -- but that ALSO fired for a
                        // genuinely narrow real device/window, forcing an
                        // 820px-wide iframe into e.g. a 390px mobile
                        // viewport (confirmed live + locally: the iframe's
                        // own `getBoundingClientRect()` reported `width:
                        // 822` inside a 390px viewport, clipped/garbled
                        // rather than scrolled to, since `overflow-x: clip`
                        // on `html`/`body` hides the escape without giving
                        // a scrollbar). A real mobile visitor SHOULD see
                        // this block's own mobile rendering (e.g.
                        // `Sidebar`'s Sheet-trigger button) -- forcing
                        // desktop width there is wrong, not just ugly.
                        //
                        // The fix is `.dx-block-demo-iframe` in
                        // `assets/main.css`: the `min-width: 820px` rule
                        // lives behind `@media (min-width: 768px)` in THAT
                        // file, not this iframe's own document. A media
                        // query written inside the iframe's own stylesheet
                        // would evaluate against the iframe's own (possibly
                        // squeezed) viewport -- the exact ambiguous signal
                        // at the heart of this bug -- but `main.css` is
                        // loaded by the OUTER top-level page, so its media
                        // query evaluates against the outer page's real
                        // `window.innerWidth`, i.e. the actual visitor
                        // viewport/device width. That correctly tells apart
                        // "the outer desktop page's own sidebar nav is
                        // squeezing an otherwise-plenty-wide viewport"
                        // (outer viewport >= 768: force the comfortable
                        // desktop min-width; `overflow-x: auto` above makes
                        // the extra width reachable by scrolling within
                        // this card instead of overflowing the page) from
                        // "this actually is a narrow device/window" (outer
                        // viewport < 768: no forced min-width, so the
                        // iframe sizes to its real, narrow container and
                        // the embedded block component's own breakpoint
                        // check sees a genuinely narrow width and renders
                        // its real mobile layout, matching what a mobile
                        // visitor should see).
                        class: "dx-block-demo-iframe",
                        height: "600px",
                        border: "1px solid var(--primary-color-6)",
                        border_radius: "0.5em",
                    }
                }
                TabContent {
                    index: 1usize,
                    value: "Code",
                    width: "100%",
                    position: "relative",
                    if let Some(css) = css_highlighted {
                        ComponentCode {
                            rs_highlighted: highlighted,
                            css_highlighted: css,
                            component_type: ComponentType::Block,
                        }
                    } else {
                        CodeBlock { source: highlighted }
                    }
                }
            }
        }
    }
}

/// The head links every route shares, in one place so the three route roots
/// cannot drift apart (they used to repeat this block by hand).
///
/// The Geist web fonts are loaded here as `<link>`s rather than via
/// `@import` inside `main.css`: a stylesheet whose `@import` is still
/// loading is not applied by the browser at all (the sheet parses but stays
/// out of `document.styleSheets`), so with the font CDN slow or blocked the
/// whole of `main.css` was inert -- found by execution 2026-09-02, see the
/// comment at the top of `assets/main.css` and
/// `playwright/oracle/tier2-html/global-stylesheet.spec.ts`. Separate links
/// let `main.css` apply immediately and the fonts swap in when they arrive.
#[component]
fn GlobalHead() -> Element {
    rsx! {
        document::Link { rel: "preconnect", href: "https://fonts.googleapis.com" }
        document::Link { rel: "preconnect", href: "https://fonts.gstatic.com", crossorigin: "anonymous" }
        document::Link {
            rel: "stylesheet",
            href: "https://fonts.googleapis.com/css2?family=Geist:wght@100..900&family=Geist+Mono:wght@400;500;700&display=swap",
        }
        document::Link { rel: "stylesheet", href: asset!("/assets/main.css") }
        document::Link { rel: "stylesheet", href: asset!("/assets/dx-components-theme.css") }
    }
}

#[component]
fn EmailClientDashboard(dark_mode: Option<bool>) -> Element {
    // `GlobalHead` renders once, in `AppLayout` (this route's own ancestor
    // layout) -- see that component's doc comment (row 46) for why it moved
    // there rather than being called per-route as it used to be here.
    rsx! {
        dashboard::views::email_client::EmailClient {}
    }
}

// Visually-hidden clip-rect, for `ComponentBlockDemo`'s heading below. The
// same construction already exists, duplicated per css_module, in this repo
// (e.g. `preview/src/components/sidebar/style.css`'s `.dx-sr-only`, with its
// own "TODO: abstract as Utility class" note) -- inlined here as a style
// string instead of a shared class because `main.rs` has no css_module of
// its own (it links `preview/assets/main.css` as a plain global stylesheet
// instead), and this fix's own file lane does not include any stylesheet.
const SR_ONLY_STYLE: &str = "position: absolute; overflow: hidden; width: 1px; height: 1px; padding: 0; border: 0; margin: -1px; clip-path: inset(50%); white-space: nowrap;";

/// Legacy query-string block-demo deep link (`/component/block/?name=<x>&
/// variant=<y>&`, dev-docs/backlog.md row 46) -- same reasoning as
/// `ComponentDemo`'s doc comment: a name-agnostic loading shell, identical
/// on server and client, that redirects to the canonical
/// `ComponentBlockDemoPath` once mounted in a real browser. Resolves the
/// same "no variant specified -> use the demo's first (\"main\") variant"
/// default the old direct-render code used to apply, so an old link that
/// never named a variant still redirects to the content it used to render
/// directly.
#[component]
fn ComponentBlockDemo(name: String, variant: Option<String>, dark_mode: Option<bool>) -> Element {
    let nav = navigator();
    use_effect(move || {
        let name = name.clone();
        let resolved_variant = variant.clone().unwrap_or_else(|| {
            components::DEMOS
                .iter()
                .find(|d| d.name == name)
                .map(|d| d.variants[0].name.to_string())
                .unwrap_or_default()
        });
        nav.replace(Route::ComponentBlockDemoPath {
            name,
            variant: resolved_variant,
            dark_mode,
        });
    });

    // `GlobalHead` renders once, in `AppLayout` (see that component's doc
    // comment, row 46) -- NOT called here. This route's own redirect to
    // `ComponentBlockDemoPath` is exactly the transition that finding
    // documents: calling `GlobalHead` independently in both of two routes a
    // client-side navigation can move between silently drops its `<link>`s
    // on the destination once the source unmounts, which a per-route call
    // here would reintroduce.
    rsx! {
        main {
            style: "min-height: 100vh; display: flex; align-items: center; justify-content: center; padding: 2rem;",
            p { "Loading component…" }
        }
    }
}

/// Canonical, SSG-enumerable block-demo page
/// (`/component/block/<name>/<variant>/`, row 46): both `name` and `variant`
/// are PATH segments -- unlike the legacy `ComponentBlockDemo` query route's
/// `Option<String>` variant with an implicit "first variant" default, this
/// route always names a concrete variant, so every (demo, variant) pair gets
/// its own static file and its own server/client-agreeing render.
#[component]
fn ComponentBlockDemoPath(name: String, variant: String, dark_mode: Option<bool>) -> Element {
    // `GlobalHead` renders once, in `AppLayout` -- not per-route here; see
    // `AppLayout`'s doc comment (row 46) for why.
    let Some(demo) = components::DEMOS.iter().find(|d| d.name == name).cloned() else {
        return rsx! {
            main {
                h1 { "Block component not found" }
            }
        };
    };

    let Some(variant) = demo.variants.iter().find(|v| v.name == variant) else {
        return rsx! {
            main {
                style: "min-height: 100vh; display: flex; align-items: center; justify-content: center; padding: 2rem;",
                h1 { "Variant content not found: {variant}" }
            }
        };
    };

    let Comp = variant.component;

    // axe `page-has-heading-one`/`region` (docs/backlog.md row 38): visited
    // directly -- rather than embedded in the main preview page's
    // `<iframe>`, where a separate document root makes the absence a
    // non-issue, see `sidebar.spec.ts`'s "preview page renders block" test
    // -- this route rendered no page-level heading or landmark of its own
    // at all. A visually-hidden `<h1>` below names the block for that
    // direct-visit case; it's wrapped in its own `<header>` (an implicit
    // "banner" landmark here, since it isn't nested inside any sectioning
    // element) rather than added as an outer `<main>` around the whole
    // route: every current block demo (`sidebar`'s `main`/`floating`/
    // `inset` variants) already renders its own `<main>` via
    // `SidebarInset`, so a second, outer `<main>` here would nest one
    // `<main>` inside another (`landmark-no-duplicate-main`/
    // `landmark-main-is-top-level` -- the same defect row 34 already had
    // to fix in the dashboard). This way the block's own landmarks stay
    // untouched and nothing is duplicated; see `Sidebar`'s own component
    // (`components/sidebar/component.rs`) for the matching fix that gives
    // the block's *sidebar* content itself a landmark, so `region` has no
    // remaining orphan content between the two.
    let block_label = demo.name.replace('_', " ");
    let heading = if variant.name == "main" {
        format!("{block_label} block preview")
    } else {
        format!(
            "{block_label} block preview ({} variant)",
            variant.name.replace('_', " ")
        )
    };

    rsx! {
        header {
            h1 { style: "{SR_ONLY_STYLE}", "{heading}" }
        }
        div { style: "min-height: 100vh;", Comp {} }
    }
}

/// Lets the homepage's own `ComponentGalleryPreview` (deep inside
/// `DocsLayout`'s `children`, via `ComponentGallery`) reach the same
/// `side`/`collapsible` signals `Home()` hands to `DocsLayout` itself, so
/// the Sidebar gallery card's Side/Collapse buttons reconfigure the real,
/// on-page nav live rather than a separate demo instance. Provided once
/// here and read with `try_consume_context` (not `consume_context`) by
/// `ComponentGalleryPreview`, since that component has no other reason to
/// require a `Home()` ancestor -- reading it defensively keeps it reusable
/// on a future page that doesn't provide this context, where its Sidebar
/// card then just falls back to the plain "Open full preview" link.
#[derive(Clone, Copy, PartialEq)]
struct HomeSidebarControls {
    side: Signal<SidebarSide>,
    collapsible: Signal<SidebarCollapsible>,
}

/// The homepage's own section headings, in the order `Home()` renders
/// them -- the same single-source-of-truth pattern as `DOCS_SECTIONS`
/// (see its doc comment): `docs_section_slug` derives both each `h2`'s
/// `id` and `DocsLayout`'s "Start" > "Home" sub-link `href`s from this
/// same text, so they can't drift apart. `HOME_SECTIONS[0]` ("Sample
/// interfaces") is `WidgetMasonry()`'s own heading, rendered by that
/// separate `#[component]` fn rather than here, so `Home()` passes its
/// slug down as a prop instead of duplicating the `h2` here.
const HOME_SECTIONS: &[&str] = &["Sample interfaces", "All components"];

#[component]
fn Home(iframe: Option<bool>, dark_mode: Option<bool>) -> Element {
    let side = use_signal(|| SidebarSide::Left);
    let collapsible = use_signal(|| SidebarCollapsible::Offcanvas);
    use_context_provider(|| HomeSidebarControls { side, collapsible });

    rsx! {
        DocsLayout {
            active: DocsNavActive::Home,
            side: Some(side),
            collapsible: Some(collapsible),
            default_open: Some(false),
            page_sections: Some(HOME_SECTIONS),
            // No `role: "main"` here -- `DocsLayout`'s `SidebarInset` already
            // renders the page's one `<main>` landmark (axe
            // `landmark-no-duplicate-main`, see `DocsLayout`'s own comment).
            div { class: "dx-home-page",
                div { id: "hero",
                    div { class: "dx-hero-shell",
                        h1 { class: "dx-hero-heading",
                            span { class: "dx-hero-title", "dioxus-components" }
                            span { class: "dx-hero-subtitle",
                                "beautiful, accessible, responsive components for dioxus apps"
                            }
                        }
                        p { class: "dx-hero-summary",
                            "Dioxus components by the Dioxus team. Browse the catalog, copy the CLI command, and pull only what you need into your project. Thoughtfully designed with powerful accessibility features."
                        }
                        div { class: "dx-hero-cta",
                            Link { to: Route::docs(), class: "dx-hero-cta-primary",
                                "get started"
                                ArrowRight { size: "18", stroke_width: "1.8" }
                            }
                            div { class: "dx-hero-command",
                                span { class: "dx-hero-prompt", "$" }
                                code { "dx components list" }
                                CopyCommandButton { command: "dx components list".to_string() }
                            }
                        }
                    }
                }
                WidgetMasonry { heading_id: docs_section_slug(HOME_SECTIONS[0]) }
                section { class: "dx-home-section dx-catalog-section",
                    header { class: "dx-section-header",
                        span { class: "dx-section-eyebrow", "Catalog" }
                        h2 { id: "{docs_section_slug(HOME_SECTIONS[1])}", class: "dx-section-title", "{HOME_SECTIONS[1]}" }
                        p { class: "dx-section-summary",
                            "Every primitive in the library, with live previews and a copy-paste install command for each one."
                        }
                    }
                    ComponentGallery {}
                }
            }
        }
    }
}

struct MasonryEntry {
    component: fn() -> Element,
    popout: bool,
}

const BLOCKS: &[MasonryEntry] = &[
    MasonryEntry {
        component: BlockSignIn,
        popout: false,
    },
    MasonryEntry {
        component: BlockProfile,
        popout: false,
    },
    MasonryEntry {
        component: BlockStats,
        popout: false,
    },
    MasonryEntry {
        component: BlockInbox,
        popout: false,
    },
    MasonryEntry {
        component: BlockTasks,
        popout: false,
    },
    MasonryEntry {
        component: BlockNotifications,
        popout: false,
    },
    MasonryEntry {
        component: BlockPlayer,
        popout: false,
    },
    MasonryEntry {
        component: BlockCommand,
        popout: true,
    },
    MasonryEntry {
        component: BlockComposer,
        popout: false,
    },
    MasonryEntry {
        component: BlockPricing,
        popout: false,
    },
    MasonryEntry {
        component: BlockFilters,
        popout: false,
    },
    MasonryEntry {
        component: BlockColorPalette,
        popout: true,
    },
    MasonryEntry {
        component: BlockTabs,
        popout: false,
    },
    MasonryEntry {
        component: BlockSchedule,
        popout: false,
    },
];

// `heading_id` is `Home()`'s `docs_section_slug(HOME_SECTIONS[0])` --
// threaded in as a prop (rather than this fn reaching for `HOME_SECTIONS`
// itself) because the id belongs to whichever page mounts this section,
// and `Home()` is the only caller today. The heading TEXT below still
// reads from `HOME_SECTIONS[0]` directly (same module, no need to thread
// that too), so id and text can't drift apart from each other.
#[component]
fn WidgetMasonry(heading_id: String) -> Element {
    rsx! {
        section { class: "dx-home-section dx-masonry-section",
            header { class: "dx-section-header",
                span { class: "dx-section-eyebrow", "Showcase" }
                h2 { id: "{heading_id}", class: "dx-section-title", "{HOME_SECTIONS[0]}" }
                p { class: "dx-section-summary",
                    "Live, interactive UI blocks composed from the primitives below. Use your keyboard to test the accessibility interactions."
                }
            }
            div { class: "dx-widget-masonry",
                for entry in BLOCKS {
                    MasonryCard {
                        component: move |()| (entry.component)(),
                        popout: entry.popout,
                    }
                }
            }
        }
    }
}

/// `component` takes `Callback<(), Element>` rather than a bare
/// `fn() -> Element`: dioxus's `#[component]` macro derives `PartialEq` for
/// this function's generated props struct by comparing every field with
/// `==`, and a raw function-pointer field triggers rustc's
/// `unpredictable_function_pointer_comparisons` lint from *inside* that
/// macro-generated `impl PartialEq` -- a separate item the macro emits
/// itself, so an `#[allow]` on this function (tried first; still present in
/// history) cannot reach it. `Callback`'s own `PartialEq` compares a
/// `GenerationalBox` pointer + `ScopeId` instead of a function pointer, so
/// routing the prop through it (the crate's own idiom for this, used by
/// every other dynamic-render/event prop in this codebase, e.g.
/// `on_change: Callback<bool, ()>`) sidesteps the lint by construction
/// instead of suppressing it.
#[component]
fn MasonryCard(component: Callback<(), Element>, #[props(default)] popout: bool) -> Element {
    let class = if popout {
        "dx-widget-card dx-widget-card-popout"
    } else {
        "dx-widget-card"
    };
    rsx! {
        div { class,
            {component.call(())}
        }
    }
}

#[component]
fn BlockSignIn() -> Element {
    rsx! {
        div { style: "display: grid; gap: 0.3rem; margin-bottom: 1.1rem;",
            h3 { style: "margin: 0; font-size: 1.05rem; font-weight: 660; color: var(--secondary-color-3);", "Welcome back" }
            p { style: "margin: 0; color: var(--secondary-color-5); font-size: 0.85rem;", "Sign in to your workspace." }
        }
        div { style: "display: grid; gap: 0.75rem; margin-bottom: 1rem;",
            div { style: "display: grid; gap: 0.35rem;",
                Label { html_for: "blk-signin-email", "Email" }
                Input { id: "blk-signin-email", r#type: "email", placeholder: "you@example.com" }
            }
            div { style: "display: grid; gap: 0.35rem;",
                div { style: "display: flex; align-items: center;",
                    Label { html_for: "blk-signin-pw", "Password" }
                    span { style: "margin-left: auto; font-size: 0.78rem; color: var(--secondary-color-5); text-decoration: underline; text-underline-offset: 3px;",
                        "Forgot?"
                    }
                }
                Input { id: "blk-signin-pw", r#type: "password", placeholder: "••••••••" }
            }
        }
        div { style: "display: grid; gap: 0.5rem;",
            Button { style: "width: 100%;", "Sign in" }
            Button { variant: ButtonVariant::Outline, style: "width: 100%;", "Continue with Google" }
        }
    }
}

#[component]
fn BlockProfile() -> Element {
    rsx! {
        div { style: "display: flex; align-items: center; gap: 0.75rem;",
            ImageAvatar {
                size: AvatarImageSize::Medium,
                src: "https://avatar.vercel.sh/avery-lin",
                alt: "Avery Lin",
                aria_label: "Avatar",
                "AL"
            }
            div { style: "flex: 1; display: grid; gap: 0.1rem; min-width: 0;",
                div { style: "display: flex; align-items: center; gap: 0.4rem;",
                    span { style: "font-weight: 600; color: var(--secondary-color-3);", "Avery Lin" }
                    Badge {
                        variant: BadgeVariant::Secondary,
                        style: "padding: 0.15rem 0.3rem; background-color: var(--focused-border-color); color: white;",
                        VerifiedIcon {}
                    }
                }
                span { style: "color: var(--secondary-color-5); font-size: 0.85rem;", "@averylin" }
            }
            Button { variant: ButtonVariant::Outline, "Follow" }
        }
        p { style: "margin: 1.1rem 0 0; color: var(--secondary-color-5); font-size: 0.9rem; line-height: 1.55;",
            "Building UI primitives that ship to web, desktop, and mobile. Mostly Rust, mostly weekends."
        }
        div { style: "display: flex; gap: 0.35rem; margin-top: 0.85rem; flex-wrap: wrap;",
            Badge { variant: BadgeVariant::Outline, "Rust" }
            Badge { variant: BadgeVariant::Outline, "WebAssembly" }
            Badge { variant: BadgeVariant::Outline, "UI" }
        }
    }
}

#[component]
fn BlockStats() -> Element {
    rsx! {
        div { style: "display: grid; gap: 0.45rem;",
            p { style: "margin: 0; color: var(--secondary-color-5); font-size: 0.74rem; text-transform: uppercase; letter-spacing: 0.1em; font-weight: 600;",
                "Active users · 30d"
            }
            div { style: "display: flex; align-items: baseline; gap: 0.6rem;",
                span { style: "font-size: 2rem; font-weight: 720; color: var(--secondary-color-3); line-height: 1.1;",
                    "24,815"
                }
                Badge {
                    variant: BadgeVariant::Secondary,
                    // axe `color-contrast` (docs/backlog.md row 39, filed
                    // 2026-09-03): the previous text color, rgb(21, 128,
                    // 61), measured 4.17:1 against this badge's rendered
                    // background (#d4f1df, the flattened
                    // rgba(34,197,94,0.18) over the page) -- below WCAG's
                    // 4.5:1. rgb(19, 115, 55) is the same green, ~10%
                    // darker, and clears 4.5:1 (measured 4.94:1).
                    style: "background-color: rgba(34, 197, 94, 0.18); color: rgb(19, 115, 55);",
                    "+12.4%"
                }
            }
        }
        div { style: "margin-top: 1rem;",
            Progress {
                value: 68.0,
                aria_label: "Toward Q2 target",
                style: "width: 100%;",
            }
        }
        p { style: "margin: 0.65rem 0 0; color: var(--secondary-color-5); font-size: 0.82rem;",
            "On track for the 36k Q2 target."
        }
    }
}

#[component]
fn BlockNotifications() -> Element {
    rsx! {
        div { style: "display: grid; gap: 0.3rem; margin-bottom: 1rem;",
            h3 { style: "margin: 0; font-size: 1rem; font-weight: 660; color: var(--secondary-color-3);", "Notifications" }
            p { style: "margin: 0; color: var(--secondary-color-5); font-size: 0.85rem;", "Pick what we ping you about." }
        }
        div { style: "display: grid; gap: 0.95rem;",
            NotificationRow { id: "blk-notif-comments", name: "Comments", description: "Replies on your posts", default_on: true }
            NotificationRow { id: "blk-notif-mentions", name: "Mentions", description: "When someone @'s you", default_on: true }
            NotificationRow { id: "blk-notif-weekly", name: "Weekly digest", description: "A Monday morning recap", default_on: false }
            NotificationRow { id: "blk-notif-updates", name: "Product updates", description: "New features and releases", default_on: false }
        }
    }
}

#[component]
fn NotificationRow(id: String, name: String, description: String, default_on: bool) -> Element {
    let mut checked = use_signal(|| default_on);
    rsx! {
        div { style: "display: flex; align-items: center; gap: 0.75rem;",
            div { style: "flex: 1; display: grid; gap: 0.1rem; min-width: 0;",
                span { style: "font-weight: 540; font-size: 0.92rem; color: var(--secondary-color-3);", "{name}" }
                span { style: "color: var(--secondary-color-5); font-size: 0.8rem;", "{description}" }
            }
            Switch {
                id: "{id}",
                checked: checked(),
                aria_label: "{name}",
                on_checked_change: move |v| checked.set(v),
            }
        }
    }
}

#[component]
fn BlockPlayer() -> Element {
    const TRACK_DURATION_SECONDS: f64 = 212.0;
    const TRACK_START_SECONDS: f64 = 84.0;

    let mut playing = use_signal(|| true);
    let mut progress_seconds = use_signal(|| Some(TRACK_START_SECONDS));
    let current_time = use_memo(move || format_track_time(progress_seconds().unwrap_or(0.0)));
    let duration_time = format_track_time(TRACK_DURATION_SECONDS);

    use_effect(move || {
        let mut timer = document::eval(
            "setInterval(() => {
                dioxus.send(performance.now());
            }, 100);",
        );

        spawn(async move {
            let mut last_tick_ms: Option<f64> = None;

            while let Ok(now_ms) = timer.recv::<f64>().await {
                let elapsed_seconds = last_tick_ms
                    .map(|last_ms| ((now_ms - last_ms) / 1000.0).clamp(0.0, 0.25))
                    .unwrap_or(0.0);
                last_tick_ms = Some(now_ms);

                if !playing() {
                    continue;
                }

                let current = progress_seconds().unwrap_or(0.0);
                let next = if current >= TRACK_DURATION_SECONDS {
                    0.0
                } else {
                    (current + elapsed_seconds).min(TRACK_DURATION_SECONDS)
                };
                progress_seconds.set(Some(next));
            }
        });
    });

    rsx! {
        div { style: "display: flex; gap: 0.85rem; align-items: center;",
            img {
                src: "https://avatar.vercel.sh/midnight-city",
                alt: "Midnight City album art",
                width: "64",
                height: "64",
                style: "width: 64px; height: 64px; border-radius: 0.45rem; object-fit: cover; flex-shrink: 0; box-shadow: 0 6px 18px -8px rgba(0,0,0,0.35);",
            }
            div { style: "flex: 1; min-width: 0;",
                p { style: "margin: 0; font-weight: 600; color: var(--secondary-color-3); overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                    "Midnight City"
                }
                p { style: "margin: 0.15rem 0 0; color: var(--secondary-color-5); font-size: 0.85rem; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                    "M83 · Hurry Up, We're Dreaming"
                }
            }
        }
        div { style: "margin-top: 1.1rem;",
            Slider {
                horizontal: true,
                min: 0.0,
                max: TRACK_DURATION_SECONDS,
                step: 1.0,
                value: progress_seconds,
                on_value_change: move |value| progress_seconds.set(Some(value)),
                label: "Track progress",
            }
            div { style: "display: flex; justify-content: space-between; margin-top: 0.45rem; color: var(--secondary-color-5); font-size: 0.78rem;",
                span { "{current_time}" }
                span { "{duration_time}" }
            }
        }
        div { style: "display: flex; align-items: center; justify-content: center; gap: 0.5rem; margin-top: 0.6rem;",
            Button {
                variant: ButtonVariant::Ghost,
                aria_label: "Previous",
                onclick: move |_| progress_seconds.set(Some(0.0)),
                SkipBack { size: "18", fill: "currentColor", stroke_width: "1.5" }
            }
            Button {
                aria_label: "Play or pause",
                onclick: move |_| { let v = !playing(); playing.set(v); },
                if playing() {
                    Pause { size: "18", fill: "currentColor", stroke_width: "1.5" }
                } else {
                    Play { size: "18", fill: "currentColor", stroke_width: "1.5" }
                }
            }
            Button {
                variant: ButtonVariant::Ghost,
                aria_label: "Next",
                onclick: move |_| progress_seconds.set(Some(0.0)),
                SkipForward { size: "18", fill: "currentColor", stroke_width: "1.5" }
            }
        }
    }
}

fn format_track_time(seconds: f64) -> String {
    let seconds = if seconds.is_finite() { seconds } else { 0.0 };
    let seconds = seconds.max(0.0).floor() as u64;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

#[component]
fn BlockPricing() -> Element {
    rsx! {
        div { style: "display: flex; align-items: center; gap: 0.5rem; margin-bottom: 0.6rem;",
            h3 { style: "margin: 0; font-size: 1rem; font-weight: 660; color: var(--secondary-color-3);", "Team" }
            Badge { variant: BadgeVariant::Secondary, "Most popular" }
        }
        div { style: "display: flex; align-items: baseline; gap: 0.3rem; margin-bottom: 0.55rem;",
            span { style: "font-size: 2.4rem; font-weight: 720; color: var(--secondary-color-3); line-height: 1;", "$12" }
            span { style: "color: var(--secondary-color-5);", "/ seat / mo" }
        }
        p { style: "margin: 0 0 1rem; color: var(--secondary-color-5); font-size: 0.86rem; line-height: 1.55;",
            "Everything in Pro, plus shared workspaces and audit logs."
        }
        ul { style: "list-style: none; padding: 0; margin: 0 0 1rem; display: grid; gap: 0.55rem; color: var(--secondary-color-4); font-size: 0.88rem;",
            for feature in ["Unlimited projects", "Role-based access", "SSO + SAML", "Priority support"] {
                li { style: "display: flex; align-items: center; gap: 0.55rem;",
                    svg {
                        width: "16",
                        height: "16",
                        view_box: "0 0 24 24",
                        fill: "none",
                        stroke: "var(--highlight-color-tertiary)",
                        stroke_width: "2.5",
                        "aria-hidden": "true",
                        polyline { points: "20 6 9 17 4 12" }
                    }
                    "{feature}"
                }
            }
        }
        Button { style: "width: 100%;", "Start free trial" }
    }
}

#[component]
fn BlockFilters() -> Element {
    rsx! {
        div { style: "display: grid; gap: 0.3rem; margin-bottom: 1rem;",
            h3 { style: "margin: 0; font-size: 1rem; font-weight: 660; color: var(--secondary-color-3);", "Filter results" }
            p { style: "margin: 0; color: var(--secondary-color-5); font-size: 0.85rem;", "Narrow down what's shown below." }
        }
        div { style: "display: grid; gap: 1.1rem;",
            div { style: "display: grid; gap: 0.45rem;",
                span { style: "color: var(--secondary-color-5); font-size: 0.78rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em;",
                    "Status"
                }
                RadioGroup { default_value: "active".to_string(),
                    RadioItem { value: "active".to_string(), index: 0usize, "Active" }
                    RadioItem { value: "draft".to_string(), index: 1usize, "Drafts" }
                    RadioItem { value: "archived".to_string(), index: 2usize, "Archived" }
                }
            }
            div { style: "display: grid; gap: 0.45rem;",
                span { style: "color: var(--secondary-color-5); font-size: 0.78rem; font-weight: 600; text-transform: uppercase; letter-spacing: 0.08em;",
                    "Tags"
                }
                div { style: "display: grid; gap: 0.4rem;",
                    for tag in [("ft-design", "Design", true), ("ft-eng", "Engineering", false), ("ft-research", "Research", false)] {
                        // `var(--dx-space-3)` (12px), not the `0.55rem`
                        // (8.8px) every other unrelated gap in this
                        // dashboard mockup still uses -- this is the one
                        // row that actually pairs a `Checkbox` with its
                        // label text, so it should match the same
                        // checkbox+label spacing convention every other
                        // composed checkbox/radio pairing in this codebase
                        // (Field's horizontal layout, RadioGroup) already
                        // uses.
                        div { style: "display: flex; align-items: center; gap: var(--dx-space-3);",
                            Checkbox {
                                id: tag.0,
                                name: tag.0,
                                default_checked: if tag.2 { dioxus_primitives::checkbox::CheckboxState::Checked } else { dioxus_primitives::checkbox::CheckboxState::Unchecked },
                                aria_label: tag.1,
                            }
                            Label { html_for: tag.0, "{tag.1}" }
                        }
                    }
                }
            }
            Button { style: "width: 100%; margin-top: 0.2rem;", "Apply filters" }
        }
    }
}

#[component]
fn BlockColorPalette() -> Element {
    use dioxus_primitives::color_picker::Color;
    use palette::{encoding, Hsv, IntoColor};

    let mut color = use_signal(|| -> Hsv<encoding::Srgb, f64> {
        Color::new(124, 58, 237).into_format::<f64>().into_color()
    });

    rsx! {
        div { style: "display: grid; gap: 0.3rem; margin-bottom: 1.1rem;",
            h3 { style: "margin: 0; font-size: 1rem; font-weight: 660; color: var(--secondary-color-3);", "Theme accent" }
            p { style: "margin: 0; color: var(--secondary-color-5); font-size: 0.85rem;", "Tune the accent that shows up across the workspace." }
        }
        ColorPicker {
            label: "Theme accent color",
            color: color(),
            on_color_change: move |c| color.set(c),
        }
    }
}

#[component]
fn BlockTabs() -> Element {
    let members: &[(&str, &str, &str, &str)] = &[
        ("Avery Lin", "Eng lead", "online", "AL"),
        ("Casey Park", "Design", "away", "CP"),
        ("Robin Hayes", "PM", "offline", "RH"),
    ];
    let activity: &[(&str, &str, &str)] = &[
        ("Casey", "shipped v2.4.1", "12m ago"),
        ("Avery", "opened PR #482", "1h ago"),
        ("Robin", "moved 4 tasks", "3h ago"),
    ];
    rsx! {
        div { style: "display: grid; gap: 0.3rem; margin-bottom: 1.1rem;",
            h3 { style: "margin: 0; font-size: 1rem; font-weight: 660; color: var(--secondary-color-3);", "Workspace" }
            p { style: "margin: 0; color: var(--secondary-color-5); font-size: 0.85rem;", "Team activity at a glance." }
        }
        Tabs {
            default_value: "members".to_string(),
            horizontal: true,
            width: "100%",
            TabList {
                TabTrigger { value: "members".to_string(), index: 0usize, "Members" }
                TabTrigger { value: "activity".to_string(), index: 1usize, "Activity" }
                TabTrigger { value: "files".to_string(), index: 2usize, "Files" }
            }
            TabContent { index: 0usize, value: "members".to_string(),
                div { style: "display: grid; gap: 0.85rem;",
                    for member in members.iter() {
                        div { style: "display: flex; align-items: center; gap: 0.7rem;",
                            ImageAvatar {
                                size: AvatarImageSize::Small,
                                src: "https://avatar.vercel.sh/{member.0}",
                                alt: "{member.0}",
                                aria_label: "{member.0}",
                                "{member.3}"
                            }
                            div { style: "flex: 1; min-width: 0;",
                                div { style: "font-weight: 540; color: var(--secondary-color-3); font-size: 0.9rem;", "{member.0}" }
                                div { style: "color: var(--secondary-color-5); font-size: 0.78rem;", "{member.1}" }
                            }
                            span {
                                style: match member.2 {
                                    "online" => "width: 0.55rem; height: 0.55rem; border-radius: 999px; background-color: rgb(34,197,94);",
                                    "away" => "width: 0.55rem; height: 0.55rem; border-radius: 999px; background-color: rgb(234,179,8);",
                                    _ => "width: 0.55rem; height: 0.55rem; border-radius: 999px; background-color: var(--primary-color-6);",
                                },
                            }
                        }
                    }
                }
            }
            TabContent { index: 1usize, value: "activity".to_string(),
                div { style: "display: grid; gap: 0.85rem;",
                    for entry in activity.iter() {
                        div { style: "display: flex; align-items: baseline; gap: 0.45rem; font-size: 0.88rem;",
                            span { style: "font-weight: 600; color: var(--secondary-color-3);", "{entry.0}" }
                            span { style: "color: var(--secondary-color-5);", "{entry.1}" }
                            span { style: "margin-left: auto; color: var(--secondary-color-5); font-size: 0.78rem; white-space: nowrap;", "{entry.2}" }
                        }
                    }
                }
            }
            TabContent { index: 2usize, value: "files".to_string(),
                div { style: "display: grid; gap: 0.6rem; color: var(--secondary-color-4); font-size: 0.88rem;",
                    div { style: "display: flex; align-items: center; gap: 0.5rem;",
                        span { style: "font-family: monospace; color: var(--secondary-color-5);", "/" }
                        span { "Roadmap Q2.md" }
                        Badge { variant: BadgeVariant::Outline, style: "margin-left: auto;", "Draft" }
                    }
                    div { style: "display: flex; align-items: center; gap: 0.5rem;",
                        span { style: "font-family: monospace; color: var(--secondary-color-5);", "/" }
                        span { "Brand guidelines.pdf" }
                    }
                    div { style: "display: flex; align-items: center; gap: 0.5rem;",
                        span { style: "font-family: monospace; color: var(--secondary-color-5);", "/" }
                        span { "Onboarding deck.key" }
                    }
                }
            }
        }
    }
}

#[component]
fn BlockSchedule() -> Element {
    rsx! {
        div { style: "display: flex; align-items: center; gap: 0.6rem; margin-bottom: 0.85rem;",
            div { style: "flex: 1;",
                h3 { style: "margin: 0; font-size: 1rem; font-weight: 660; color: var(--secondary-color-3);", "Schedule" }
                p { style: "margin: 0; color: var(--secondary-color-5); font-size: 0.85rem;", "Pick a day for the standup." }
            }
            Badge { variant: BadgeVariant::Outline, "Mar 2026" }
        }
        div { style: "display: grid; justify-items: center;",
            components::calendar::variants::main::Demo {}
        }
    }
}

#[component]
fn BlockCommand() -> Element {
    let mut query = use_signal(String::new);
    let workspaces: &[(&str, &str)] = &[
        ("acme", "Acme Inc."),
        ("orbit", "Orbit Studio"),
        ("nimbus", "Nimbus Labs"),
        ("strata", "Strata Health"),
        ("vela", "Vela Robotics"),
        ("riverstone", "Riverstone Capital"),
    ];
    rsx! {
        div { style: "display: grid; gap: 0.3rem; margin-bottom: 1rem;",
            h3 { style: "margin: 0; font-size: 1rem; font-weight: 660; color: var(--secondary-color-3);", "Switch workspace" }
            p { style: "margin: 0; color: var(--secondary-color-5); font-size: 0.85rem;", "Jump between projects your team owns." }
        }
        Combobox::<String> {
            query: Some(query()),
            on_query_change: move |next| query.set(next),
            placeholder: "Search workspaces...",
            aria_label: "Switch workspace",
            list_aria_label: "Workspaces",
            ComboboxEmpty { "No workspaces match." }
            for (i , (value , label)) in workspaces.iter().enumerate() {
                ComboboxOption::<String> {
                    index: i,
                    value: value.to_string(),
                    text_value: label.to_string(),
                    "{label}"
                }
            }
        }
    }
}

#[component]
fn BlockInbox() -> Element {
    let messages: &[(&str, &str, &str)] = &[
        ("Sarah Chen", "Left 3 comments on the auth flow", "2m"),
        ("Marcus Wright", "Roadmap sync notes attached", "1h"),
        ("Lena Park", "Refactored the sidebar layout", "4h"),
    ];
    rsx! {
        div { style: "display: flex; align-items: center; gap: 0.55rem; margin-bottom: 0.85rem;",
            div { style: "flex: 1;",
                h3 { style: "margin: 0; font-size: 1rem; font-weight: 660; color: var(--secondary-color-3);", "Inbox" }
                p { style: "margin: 0; color: var(--secondary-color-5); font-size: 0.85rem;", "3 new conversations." }
            }
            Badge { variant: BadgeVariant::Secondary, "3" }
        }
        div { style: "display: grid; gap: 0.5rem;",
            for (sender , preview , time) in messages.iter() {
                Item { variant: ItemVariant::Outline,
                    ItemMedia { variant: ItemMediaVariant::Icon,
                        ImageAvatar {
                            size: AvatarImageSize::Small,
                            src: "https://avatar.vercel.sh/{sender}",
                            alt: "{sender}",
                            aria_label: "{sender}",
                            "{sender.chars().next().unwrap_or('?')}"
                        }
                    }
                    ItemContent {
                        ItemTitle { "{sender}" }
                        ItemDescription { "{preview}" }
                    }
                    ItemContent { flex: "none",
                        ItemDescription { "{time}" }
                    }
                }
            }
        }
    }
}

#[component]
fn BlockTasks() -> Element {
    let tasks: &[(&str, &str, &str, &str)] = &[
        ("LNC-128", "Ship Q2 product roadmap", "Today", "AL"),
        ("LNC-142", "Redesign onboarding flow", "Apr 24", "CP"),
        ("LNC-147", "Audit payment webhook logs", "Apr 29", "RH"),
        ("LNC-151", "Draft changelog for v2.4", "May 02", "AL"),
    ];
    let items: Vec<Element> = tasks
        .iter()
        .map(|t| {
            rsx! {
                div { key: "{t.0}", style: "display: flex; align-items: center; gap: 0.75rem; min-width: 0;",
                    div { style: "flex: 1; min-width: 0; display: grid; gap: 0.2rem;",
                        div { style: "color: var(--secondary-color-3); font-size: 0.9rem; font-weight: 540; overflow: hidden; text-overflow: ellipsis; white-space: nowrap;",
                            "{t.1}"
                        }
                        div { style: "display: flex; align-items: center; gap: 0.45rem; color: var(--secondary-color-5); font-size: 0.78rem;",
                            span { style: "font-family: monospace;", "{t.0}" }
                            span { style: "width: 3px; height: 3px; border-radius: 999px; background-color: var(--primary-color-7);" }
                            span { "{t.2}" }
                        }
                    }
                    ImageAvatar {
                        size: AvatarImageSize::Small,
                        src: "https://avatar.vercel.sh/{t.3}",
                        alt: "{t.3}",
                        aria_label: "Assignee {t.3}",
                        "{t.3}"
                    }
                }
            }
        })
        .collect();

    rsx! {
        div { style: "display: flex; align-items: center; gap: 0.55rem; margin-bottom: 1.1rem;",
            div { style: "flex: 1;",
                h3 { style: "margin: 0; font-size: 1rem; font-weight: 660; color: var(--secondary-color-3);", "Launch priorities" }
                p { style: "margin: 0; color: var(--secondary-color-5); font-size: 0.85rem;", "Drag to reorder — top is highest priority." }
            }
            Badge { variant: BadgeVariant::Outline, "4 active" }
        }
        DragAndDropList { items }
    }
}

#[component]
fn BlockComposer() -> Element {
    let mut draft = use_signal(|| {
        "Big thanks to the team for landing the new roadmap view — looks great!".to_string()
    });
    rsx! {
        div { style: "display: flex; align-items: center; gap: 0.65rem; margin-bottom: 1rem;",
            ImageAvatar {
                size: AvatarImageSize::Small,
                src: "https://avatar.vercel.sh/avery-lin",
                alt: "Avery Lin",
                aria_label: "Avery Lin",
                "AL"
            }
            div { style: "flex: 1; display: grid; gap: 0.1rem;",
                span { style: "font-weight: 600; color: var(--secondary-color-3); font-size: 0.9rem;", "Reply to roadmap thread" }
                span { style: "color: var(--secondary-color-5); font-size: 0.78rem;", "Posting as @averylin · #product" }
            }
        }
        Textarea {
            variant: TextareaVariant::Default,
            value: draft,
            oninput: move |e: FormEvent| draft.set(e.value()),
            placeholder: "Share an update…",
            style: "width: 100%; min-height: 5.5rem; resize: vertical;",
        }
        div { style: "display: flex; align-items: center; gap: 0.55rem; margin-top: 0.85rem;",
            ToggleGroup { horizontal: true, allow_multiple_pressed: true, aria_label: "Text formatting",
                ToggleItem { index: 0usize, aria_label: "Bold",
                    b { "B" }
                }
                ToggleItem { index: 1usize, aria_label: "Italic",
                    i { "I" }
                }
                ToggleItem { index: 2usize, aria_label: "Underline",
                    u { "U" }
                }
            }
            div { style: "margin-left: auto; display: flex; gap: 0.45rem;",
                Button { variant: ButtonVariant::Ghost, "Save draft" }
                Button { "Post" }
            }
        }
    }
}

#[component]
fn ComponentGallery() -> Element {
    rsx! {
        div { class: "dx-component-gallery",
            // `top_layer` is excluded here, deliberately: it is not a real,
            // installable component (there is no `dx components add
            // top_layer`) but an oracle fixture --
            // `preview/src/components/top_layer/component.rs`'s
            // `TopLayerFixture`, a large probe surface for
            // `playwright/oracle/tier2-html/top-layer.spec.ts` and
            // `native-dialog.spec.ts` (clipping-escape triggers, background-
            // inertness probes, native `<dialog>`/`<div popover>` references,
            // ...). Embedding it inline on `/` put dozens of raw
            // `dioxus_primitives::` elements and duplicate landmark-shaped
            // markup into the one page every visitor and every gallery-wide
            // oracle (`preview.spec.ts`, hydration-parity Rule 4b) scans,
            // for a fixture no one browsing the catalog is here to see. It
            // stays fully reachable at its own page,
            // `/component/?name=top_layer&` (still listed under Overlays in
            // `DocsSidebar`, since `components::DEMOS` itself is unchanged
            // and this filter only narrows the *gallery grid's* iteration)
            // -- see docs/backlog.md for the landed row and
            // docs/conformance-harness.md's hydration-parity section for
            // Rule 4b's replacement subject, now that its old one
            // (the fixture's toast region) no longer renders on `/`.
            for component in components::DEMOS.iter().filter(|c| c.name != "top_layer").cloned() {
                ComponentGalleryPreview { component }
            }
        }
    }
}

#[component]
fn ComponentGalleryPreview(component: ComponentDemoData) -> Element {
    let ComponentDemoData {
        name,
        r#type,
        description,
        variants,
        ..
    } = component;

    let first_variant = &variants[0];
    let Comp = first_variant.component;
    let display_name = name.replace("_", " ");
    let install_command = format!("dx components add {name}");

    // Only the `sidebar` card, and only when rendered under `Home()` (which
    // is the sole provider of this context -- see `HomeSidebarControls`'s
    // own doc comment), gets live controls instead of the plain
    // "Open full preview" link every other `ComponentType::Block` entry
    // still falls back to. `try_consume_context` (not `consume_context`)
    // is what makes that fallback safe rather than a panic: this same
    // component would otherwise need every future caller to remember to
    // provide the context, for a card that doesn't use it.
    let home_sidebar_controls = (name == "sidebar")
        .then(try_consume_context::<HomeSidebarControls>)
        .flatten();

    let preview = match (r#type, home_sidebar_controls) {
        (ComponentType::Normal, _) => rsx! {
            Comp {}
        },
        (ComponentType::Block, Some(controls)) => rsx! {
            SidebarGalleryCardControls { controls }
        },
        (ComponentType::Block, None) => rsx! {
            Link {
                to: Route::component(name),
                class: "dx-component-card-block-link",
                "Open full preview"
                ArrowUpRight { size: "18", stroke_width: "1.6" }
            }
        },
    };

    rsx! {
        article { class: "dx-component-card",
            div { class: "dx-component-card-meta",
                h3 { class: "dx-component-card-title",
                    Link {
                        to: Route::component(name),
                        class: "dx-component-card-title-link",
                        "{display_name}"
                        ArrowUpRight { size: "18", stroke_width: "1.6" }
                    }
                }
                p { class: "dx-component-card-description", "{description}" }
                div { class: "dx-component-card-actions",
                    div { class: "dx-component-card-command",
                        code { "{install_command}" }
                        CopyCommandButton { command: install_command.clone() }
                    }
                }
            }
            div { class: "dx-component-card-preview", {preview} }
        }
    }
}

/// The homepage's Sidebar gallery card's own controls -- Side/Collapse
/// buttons in the same shape as `sidebar/variants/main/mod.rs`'s
/// `DemoSettingControls` (that file's own demo page), but writing to
/// `Home()`'s signals instead of a demo-local pair, so they reconfigure
/// the real, on-page site nav (`DocsLayout`'s `Sidebar`) live.
#[component]
fn SidebarGalleryCardControls(controls: HomeSidebarControls) -> Element {
    let HomeSidebarControls {
        mut side,
        mut collapsible,
    } = controls;

    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 0.75rem; width: 100%;",
            div { style: "display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; flex-wrap: wrap;",
                span { style: "font-size: 0.75rem; font-weight: 600; color: var(--secondary-color-4);",
                    "Side"
                }
                div { style: "display: inline-flex; gap: 0.5rem;",
                    Button {
                        variant: if side() == SidebarSide::Left { ButtonVariant::Primary } else { ButtonVariant::Outline },
                        onclick: move |_| side.set(SidebarSide::Left),
                        style: "padding: 0.4rem 0.6rem; font-size: 0.75rem;",
                        "Left"
                    }
                    Button {
                        variant: if side() == SidebarSide::Right { ButtonVariant::Primary } else { ButtonVariant::Outline },
                        onclick: move |_| side.set(SidebarSide::Right),
                        style: "padding: 0.4rem 0.6rem; font-size: 0.75rem;",
                        "Right"
                    }
                }
            }
            div { style: "display: flex; align-items: center; justify-content: space-between; gap: 0.75rem; flex-wrap: wrap;",
                span { style: "font-size: 0.75rem; font-weight: 600; color: var(--secondary-color-4);",
                    "Collapse"
                }
                div { style: "display: inline-flex; gap: 0.5rem; flex-wrap: wrap;",
                    Button {
                        variant: if collapsible() == SidebarCollapsible::Offcanvas { ButtonVariant::Primary } else { ButtonVariant::Outline },
                        onclick: move |_| collapsible.set(SidebarCollapsible::Offcanvas),
                        style: "padding: 0.4rem 0.6rem; font-size: 0.75rem;",
                        "Offcanvas"
                    }
                    Button {
                        variant: if collapsible() == SidebarCollapsible::Icon { ButtonVariant::Primary } else { ButtonVariant::Outline },
                        onclick: move |_| collapsible.set(SidebarCollapsible::Icon),
                        style: "padding: 0.4rem 0.6rem; font-size: 0.75rem;",
                        "Icon"
                    }
                    Button {
                        variant: if collapsible() == SidebarCollapsible::None { ButtonVariant::Primary } else { ButtonVariant::Outline },
                        onclick: move |_| collapsible.set(SidebarCollapsible::None),
                        style: "padding: 0.4rem 0.6rem; font-size: 0.75rem;",
                        "None"
                    }
                }
            }
        }
    }
}

#[component]
fn CopyCommandButton(command: String) -> Element {
    let mut copied = use_signal(|| false);

    rsx! {
        button {
            class: "dx-copy-button dx-component-card-copy",
            r#type: "button",
            aria_label: "Copy install command",
            "data-command": "{command}",
            "data-copied": copied,
            "onclick": "navigator.clipboard.writeText(this.dataset.command);",
            onclick: move |_| copied.set(true),
            if copied() {
                CheckIcon {}
            } else {
                CopyIcon {}
            }
        }
    }
}

#[component]
fn GotoIcon(mut props: LinkProps) -> Element {
    props.children = rsx! {
        ExternalLink {
            size: "20px",
            stroke: "var(--secondary-color-4)",
        }
    };
    Link(props)
}

// Same class as `CssHighlight` (see its doc comment): this shared theme
// stylesheet lives under `preview/assets/`, not a component folder, but it
// is *also* embedded via `dioxus_code::code!()` for this same Style tab --
// so editing it hits the identical rustc-dep-info full-rebuild path a
// component's own `style.css` does, for the identical reason. It gets the
// identical fix.
#[cfg(not(debug_assertions))]
const THEME_CSS: CssHighlight = CssHighlight {
    embedded: HighlightedCode {
        source: dioxus_code::code!("/assets/dx-components-theme.css"),
    },
};

#[cfg(debug_assertions)]
const THEME_CSS: CssHighlight = CssHighlight {
    asset: asset!("/assets/dx-components-theme.css"),
};

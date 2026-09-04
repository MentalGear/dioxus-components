use crate::components::button::{Button, ButtonVariant};
use crate::components::separator::Separator;
use crate::components::sheet::{
    Sheet, SheetContentClose, SheetDescription, SheetHeader, SheetSide, SheetTitle,
};
use crate::components::skeleton::Skeleton;
use crate::components::tooltip::{Tooltip, TooltipContent, TooltipTrigger};
use dioxus::core::use_drop;
use dioxus::prelude::*;
use dioxus_icons::lucide::PanelLeft;
use dioxus_primitives::dioxus_attributes::attributes;
use dioxus_primitives::merge_attributes;
use dioxus_primitives::use_controlled;

// docs/backlog.md row 32: `#[css_module]` is gone -- see checkbox/component.rs's
// header comment for the full delivery-mechanism rationale (asset!() +
// document::Link). This is the heaviest component on the migration: every
// exported entry point in this file gets its own `document::Link` (dedup on
// (href, rel) makes that free), and `variants/{main,inset,floating}/mod.rs`
// each carried their OWN `#[css_module("/src/components/sidebar/style.css")]
// struct Styles;` pointing at this same sheet -- those are simply deleted
// (their `Styles::` refs swapped to literals, same as here) rather than given
// their own `Link`, because every one of those variant demo pages always
// also renders the real themed wrapper (`SidebarProvider`/`Sidebar`) from
// THIS file, whose `Link` already covers the sheet. Those three variant
// files also each carry a second, genuinely separate
// `#[css_module("/src/components/sidebar/variants/demo.css")] struct
// DemoStyles;` for their own demo-only decorative classes (team-switcher
// icon/info-block/chevron, `dx-sidebar-hide-on-collapse`) -- that sheet is
// NOT this component's own `style.css` and nothing else links it, so each
// variant's own `Demo()` now carries its own `asset!()` + `document::Link`
// for `demo.css` instead.
//
// This component is also on the row-32 "not already namespaced" lane: its
// screen-reader-only helper used to be the bare `dx-sr-only`, which
// `pagination/style.css` also defines (byte-identical). `#[css_module]`'s
// hash kept the two apart; renamed to `dx-sidebar-sr-only` in the same
// change that drops the macro (`pagination` gets its own
// `dx-pagination-sr-only` copy) -- see `style.css`'s comment on that rule
// and `scripts/check-dx-class-prefix.sh`.

// constants
const SIDEBAR_WIDTH: &str = "16rem";
const SIDEBAR_WIDTH_MOBILE: &str = "18rem";
const SIDEBAR_WIDTH_ICON: &str = "3rem";
const SIDEBAR_KEYBOARD_SHORTCUT: &str = "b";
const MOBILE_BREAKPOINT: u32 = 768;

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum SidebarState {
    #[default]
    Expanded,
    Collapsed,
}

impl SidebarState {
    pub fn as_str(&self) -> &'static str {
        match self {
            SidebarState::Expanded => "expanded",
            SidebarState::Collapsed => "collapsed",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum SidebarSide {
    #[default]
    Left,
    Right,
}

impl SidebarSide {
    pub fn as_str(&self) -> &'static str {
        match self {
            SidebarSide::Left => "left",
            SidebarSide::Right => "right",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum SidebarVariant {
    #[default]
    Sidebar,
    Floating,
    Inset,
}

impl SidebarVariant {
    pub fn as_str(&self) -> &'static str {
        match self {
            SidebarVariant::Sidebar => "sidebar",
            SidebarVariant::Floating => "floating",
            SidebarVariant::Inset => "inset",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub enum SidebarCollapsible {
    #[default]
    Offcanvas,
    Icon,
    None,
}

impl SidebarCollapsible {
    pub fn as_str(&self) -> &'static str {
        match self {
            SidebarCollapsible::Offcanvas => "offcanvas",
            SidebarCollapsible::Icon => "icon",
            SidebarCollapsible::None => "none",
        }
    }
}

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct SidebarCtx {
    pub state: Memo<SidebarState>,
    pub side: Signal<SidebarSide>,
    pub is_mobile: Signal<bool>,
    // From use_controlled:
    open: Memo<bool>,
    set_open: Callback<bool>,
    // Mobile state:
    open_mobile: Signal<bool>,
}

impl SidebarCtx {
    /// Toggle the sidebar open/closed state
    pub fn toggle(&self) {
        if (self.is_mobile)() {
            let current = (self.open_mobile)();
            let mut open_mobile = self.open_mobile;
            open_mobile.set(!current);
        } else {
            self.set_open.call(!self.open());
        }
    }

    /// Set the mobile sidebar open state
    pub fn set_open_mobile(&self, value: bool) {
        let mut open_mobile = self.open_mobile;
        open_mobile.set(value);
    }

    /// Get the current open state (desktop)
    pub fn open(&self) -> bool {
        self.open.cloned()
    }
}

pub fn use_sidebar() -> SidebarCtx {
    use_context::<SidebarCtx>()
}

pub fn use_is_mobile() -> Signal<bool> {
    let mut is_mobile = use_signal(|| false);

    use_effect(move || {
        spawn(async move {
            let js_code = format!(
                r#"
                function checkMobile() {{
                    return window.innerWidth < {MOBILE_BREAKPOINT};
                }}
                function handleResize() {{
                    dioxus.send(checkMobile());
                }}
                window.__sidebarResizeHandler = handleResize;
                window.addEventListener('resize', window.__sidebarResizeHandler);
                dioxus.send(checkMobile());
                "#
            );
            let mut eval = document::eval(&js_code);

            while let Ok(result) = eval.recv::<bool>().await {
                is_mobile.set(result);
            }
        });
    });

    use_drop(|| {
        _ = document::eval(
            r#"
            window.removeEventListener('resize', window.__sidebarResizeHandler);
            delete window.__sidebarResizeHandler;
            "#,
        );
    });

    is_mobile
}

#[component]
pub fn SidebarProvider(
    #[props(default = true)] default_open: bool,
    #[props(default)] open: ReadSignal<Option<bool>>,
    #[props(default)] on_open_change: Callback<bool>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let is_mobile = use_is_mobile();
    let side = use_signal(|| SidebarSide::Left);
    let open_mobile = use_signal(|| false);

    let (open, set_open) = use_controlled(open, default_open, on_open_change);

    let state = use_memo(move || {
        if open() {
            SidebarState::Expanded
        } else {
            SidebarState::Collapsed
        }
    });

    let ctx = SidebarCtx {
        state,
        side,
        is_mobile,
        open,
        set_open,
        open_mobile,
    };

    use_context_provider(|| ctx);

    use_effect(move || {
        spawn(async move {
            let js_code = format!(
                r#"
                function sidebarKeyHandler(event) {{
                    if (event.key === '{SIDEBAR_KEYBOARD_SHORTCUT}' && (event.metaKey || event.ctrlKey)) {{
                        event.preventDefault();
                        dioxus.send(true);
                    }}
                }}
                window.__sidebarKeyHandler = sidebarKeyHandler;
                window.addEventListener('keydown', window.__sidebarKeyHandler);
                "#
            );
            let mut eval = document::eval(&js_code);

            loop {
                if eval.recv::<bool>().await.is_ok() {
                    ctx.toggle();
                }
            }
        });
    });

    use_drop(|| {
        _ = document::eval(
            r#"
            window.removeEventListener('keydown', window.__sidebarKeyHandler);
            delete window.__sidebarKeyHandler;
            "#,
        );
    });

    let sidebar_style = format!(
        r#"
        --dx-sidebar-width: {SIDEBAR_WIDTH};
        --dx-sidebar-width-mobile: {SIDEBAR_WIDTH_MOBILE};
        --dx-sidebar-width-icon: {SIDEBAR_WIDTH_ICON}
        "#
    );

    let base = attributes!(div {
        class: "dx-sidebar-wrapper",
        "data-slot": "sidebar-wrapper",
        style: sidebar_style,
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        div { ..merged, {children} }
    }
}

#[component]
pub fn Sidebar(
    #[props(default)] side: SidebarSide,
    #[props(default)] variant: SidebarVariant,
    #[props(default)] collapsible: SidebarCollapsible,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let ctx = use_sidebar();
    let mut ctx_side = ctx.side;
    if *ctx_side.peek() != side {
        ctx_side.set(side);
    }

    let is_mobile = ctx.is_mobile;
    let state = ctx.state;
    let open_mobile = ctx.open_mobile;

    if collapsible == SidebarCollapsible::None {
        // axe `region` (docs/backlog.md row 38): this was a plain `<div>`
        // with no landmark role at all, so its content (menu items,
        // footer) sat outside every landmark on the page -- surfaced both
        // by the block-route direct-visit case (`ComponentBlockDemo`,
        // `preview/src/main.rs`) and, independently, by the dashboard
        // email client's own embedded sidebar (`dashboard/views/
        // email_client/sidebar.rs`), which renders `Sidebar` directly and
        // was never routed through the block preview at all. `role:
        // "complementary"` (the `<aside>` landmark) fixes both call sites
        // at their one shared source; labelled so it stays unambiguous if
        // a future page ever has more than one.
        let base = attributes!(div {
            class: "dx-sidebar-static",
            "data-slot": "sidebar",
            role: "complementary",
            aria_label: "Sidebar",
        });
        let merged = merge_attributes(vec![base, attributes]);

        return rsx! {
            document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
            div { ..merged, {children} }
        };
    }

    if is_mobile() {
        let sheet_side = match side {
            SidebarSide::Left => SheetSide::Left,
            SidebarSide::Right => SheetSide::Right,
        };

        return rsx! {
            document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
            Sheet {
                open: open_mobile(),
                on_open_change: move |v| ctx.set_open_mobile(v),
                "data-side": sheet_side.as_str(),
                class: "dx-sidebar-sheet".to_string(),
                "data-sidebar": "sidebar",
                "data-slot": "sidebar",
                "data-mobile": "true",
                SheetContentClose { class: "dx-sidebar-sheet-close" }
                SheetHeader { class: "dx-sidebar-sr-only",
                    SheetTitle { "Sidebar" }
                    SheetDescription { "Displays the mobile sidebar." }
                }
                div { class: "dx-sidebar-mobile-inner", {children} }
            }
        };
    }

    let collapsible_str = if state() == SidebarState::Collapsed {
        collapsible.as_str()
    } else {
        ""
    };

    let container_base = attributes!(div {
        class: "dx-sidebar-container",
        "data-slot": "sidebar-container",
    });
    let container_attrs = merge_attributes(vec![container_base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        div {
            class: "dx-sidebar-desktop",
            "data-state": state().as_str(),
            "data-collapsible": collapsible_str,
            "data-variant": variant.as_str(),
            "data-side": side.as_str(),
            "data-slot": "sidebar",
            // axe `region` (docs/backlog.md row 38) -- see the identical
            // comment on the `collapsible == SidebarCollapsible::None`
            // branch above; this is the desktop-expanded/-collapsed
            // branch actually rendered by both known affected call sites
            // (the block route's `sidebar` demo and the dashboard email
            // client's sidebar), so it's the one that mattered for the
            // reported gap.
            role: "complementary",
            aria_label: "Sidebar",
            div { class: "dx-sidebar-gap", "data-slot": "sidebar-gap" }
            div {
                ..container_attrs,
                div {
                    class: "dx-sidebar-inner",
                    "data-sidebar": "sidebar",
                    "data-slot": "sidebar-inner",
                    {children}
                }
            }
        }
    }
}

#[component]
pub fn SidebarTrigger(
    #[props(default)] onclick: Option<EventHandler<MouseEvent>>,
    #[props(extends = GlobalAttributes)]
    #[props(extends = button)]
    attributes: Vec<Attribute>,
) -> Element {
    let ctx = use_sidebar();

    let base = attributes!(button {
        class: "dx-sidebar-trigger",
        "data-sidebar": "trigger",
        "data-slot": "sidebar-trigger",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        Button {
            variant: ButtonVariant::Ghost,
            onclick: move |e| {
                if let Some(handler) = &onclick {
                    handler.call(e);
                }
                ctx.toggle();
            },
            attributes: merged,
            PanelLeft {
                class: "dx-sidebar-trigger-icon",
                size: "1rem",
            }
            span { class: "dx-sidebar-sr-only", "Toggle Sidebar" }
        }
    }
}

#[component]
pub fn SidebarRail(#[props(extends = GlobalAttributes)] attributes: Vec<Attribute>) -> Element {
    let ctx = use_sidebar();

    let base = attributes!(button {
        class: "dx-sidebar-rail",
        "data-sidebar": "rail",
        "data-slot": "sidebar-rail",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        button {
            aria_label: "Toggle Sidebar",
            tabindex: -1,
            onclick: move |_| ctx.toggle(),
            title: "Toggle Sidebar",
            ..merged,
        }
    }
}

#[component]
pub fn SidebarInset(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(main {
        class: "dx-sidebar-inset",
        "data-slot": "sidebar-inset",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        main { ..merged, {children} }
    }
}

#[component]
pub fn SidebarHeader(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-sidebar-header",
        "data-slot": "sidebar-header",
        "data-sidebar": "header",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        div { ..merged, {children} }
    }
}

#[component]
pub fn SidebarContent(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-sidebar-content",
        "data-slot": "sidebar-content",
        "data-sidebar": "content",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        div { ..merged, {children} }
    }
}

#[component]
pub fn SidebarFooter(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-sidebar-footer",
        "data-slot": "sidebar-footer",
        "data-sidebar": "footer",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        div { ..merged, {children} }
    }
}

#[component]
pub fn SidebarSeparator(
    #[props(default = true)] horizontal: bool,
    #[props(default = true)] decorative: bool,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let base = attributes!(div {
        class: "dx-sidebar-separator",
        "data-slot": "sidebar-separator",
        "data-sidebar": "separator",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        Separator { horizontal, decorative, attributes: merged }
    }
}

#[component]
pub fn SidebarGroup(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-sidebar-group",
        "data-slot": "sidebar-group",
        "data-sidebar": "group",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        div { ..merged, {children} }
    }
}

#[component]
pub fn SidebarGroupLabel(
    r#as: Option<Callback<Vec<Attribute>, Element>>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-sidebar-group-label",
        "data-slot": "sidebar-group-label",
        "data-sidebar": "group-label",
    });
    let merged = merge_attributes(vec![base, attributes]);

    if let Some(dynamic) = r#as {
        dynamic.call(merged)
    } else {
        rsx! {
            document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
            div { ..merged,{children} }
        }
    }
}

#[component]
pub fn SidebarGroupAction(
    r#as: Option<Callback<Vec<Attribute>, Element>>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(button {
        class: "dx-sidebar-group-action",
        "data-slot": "sidebar-group-action",
        "data-sidebar": "group-action",
    });
    let merged = merge_attributes(vec![base, attributes]);

    if let Some(dynamic) = r#as {
        dynamic.call(merged)
    } else {
        rsx! {
            document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
            button { ..merged,{children} }
        }
    }
}

#[component]
pub fn SidebarGroupContent(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-sidebar-group-content",
        "data-slot": "sidebar-group-content",
        "data-sidebar": "group-content",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        div { ..merged, {children} }
    }
}

#[component]
pub fn SidebarMenu(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(ul {
        class: "dx-sidebar-menu",
        "data-slot": "sidebar-menu",
        "data-sidebar": "menu",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        ul { ..merged, {children} }
    }
}

#[component]
pub fn SidebarMenuItem(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(li {
        class: "dx-sidebar-menu-item",
        "data-slot": "sidebar-menu-item",
        "data-sidebar": "menu-item",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        li { ..merged, {children} }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[allow(dead_code)]
pub enum SidebarMenuButtonVariant {
    #[default]
    Default,
    Outline,
}

impl SidebarMenuButtonVariant {
    pub fn as_str(&self) -> &'static str {
        match self {
            SidebarMenuButtonVariant::Default => "default",
            SidebarMenuButtonVariant::Outline => "outline",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[allow(dead_code)]
pub enum SidebarMenuButtonSize {
    #[default]
    Default,
    Sm,
    Lg,
}

impl SidebarMenuButtonSize {
    pub fn as_str(&self) -> &'static str {
        match self {
            SidebarMenuButtonSize::Default => "default",
            SidebarMenuButtonSize::Sm => "sm",
            SidebarMenuButtonSize::Lg => "lg",
        }
    }
}

#[component]
pub fn SidebarMenuButton(
    #[props(default = false)] is_active: bool,
    #[props(default)] variant: SidebarMenuButtonVariant,
    #[props(default)] size: SidebarMenuButtonSize,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    #[props(default)] tooltip: Option<Element>,
    r#as: Option<Callback<Vec<Attribute>, Element>>,
    children: Element,
) -> Element {
    let ctx = use_sidebar();
    let is_mobile = ctx.is_mobile;
    let state = ctx.state;

    let base = attributes!(button {
        class: "dx-sidebar-menu-button",
        "data-slot": "sidebar-menu-button",
        "data-sidebar": "menu-button",
        "data-size": size.as_str(),
        "data-variant": variant.as_str(),
        "data-active": if is_active { "true" } else { "false" },
    });
    let merged = merge_attributes(vec![base, attributes]);

    let Some(tooltip_content) = tooltip else {
        return if let Some(dynamic) = r#as {
            dynamic.call(merged)
        } else {
            rsx! {
                document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
                button { ..merged, {children} }
            }
        };
    };

    let hidden = state() != SidebarState::Collapsed || is_mobile();
    let sidebar_side = ctx.side;

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        Tooltip {
            class: "dx-sidebar-tooltip",
            disabled: hidden,
            TooltipTrigger {
                as: move |tooltip_attrs: Vec<Attribute>| {
                    let final_attrs = merge_attributes(vec![tooltip_attrs, merged.clone()]);
                    let children = children.clone();
                    if let Some(dynamic) = &r#as {
                        dynamic.call(final_attrs)
                    } else {
                        rsx! { button { ..final_attrs, {children} } }
                    }
                },
            }
            TooltipContent {
                side: match sidebar_side() {
                    SidebarSide::Left => dioxus_primitives::ContentSide::Right,
                    SidebarSide::Right => dioxus_primitives::ContentSide::Left,
                },
                {tooltip_content}
            }
        }
    }
}

#[component]
pub fn SidebarMenuAction(
    #[props(default = false)] show_on_hover: bool,
    r#as: Option<Callback<Vec<Attribute>, Element>>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(button {
        class: "dx-sidebar-menu-action",
        "data-slot": "sidebar-menu-action",
        "data-sidebar": "menu-action",
        "data-show-on-hover": if show_on_hover { "true" } else { "false" },
    });
    let merged = merge_attributes(vec![base, attributes]);

    if let Some(dynamic) = r#as {
        dynamic.call(merged)
    } else {
        rsx! {
            document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
            button { ..merged,{children} }
        }
    }
}

#[component]
pub fn SidebarMenuBadge(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(div {
        class: "dx-sidebar-menu-badge",
        "data-slot": "sidebar-menu-badge",
        "data-sidebar": "menu-badge",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        div { ..merged, {children} }
    }
}

#[component]
pub fn SidebarMenuSkeleton(
    #[props(default = false)] show_icon: bool,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
) -> Element {
    let base = attributes!(div {
        class: "dx-sidebar-menu-skeleton",
        "data-slot": "sidebar-menu-skeleton",
        "data-sidebar": "menu-skeleton",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        div {
            ..merged,
            if show_icon {
                Skeleton { class: "dx-sidebar-menu-skeleton-icon" }
            }
            Skeleton { class: "dx-sidebar-menu-skeleton-text", width: "70%" }
        }
    }
}

#[component]
pub fn SidebarMenuSub(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(ul {
        class: "dx-sidebar-menu-sub",
        "data-slot": "sidebar-menu-sub",
        "data-sidebar": "menu-sub",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        ul { ..merged, {children} }
    }
}

#[component]
pub fn SidebarMenuSubItem(
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(li {
        class: "dx-sidebar-menu-sub-item",
        "data-slot": "sidebar-menu-sub-item",
        "data-sidebar": "menu-sub-item",
    });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
        li { ..merged, {children} }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[allow(dead_code)]
pub enum SidebarMenuSubButtonSize {
    Sm,
    #[default]
    Md,
}

impl SidebarMenuSubButtonSize {
    pub fn as_str(&self) -> &'static str {
        match self {
            SidebarMenuSubButtonSize::Sm => "sm",
            SidebarMenuSubButtonSize::Md => "md",
        }
    }
}

#[component]
pub fn SidebarMenuSubButton(
    #[props(default = false)] is_active: bool,
    #[props(default)] size: SidebarMenuSubButtonSize,
    r#as: Option<Callback<Vec<Attribute>, Element>>,
    #[props(extends = GlobalAttributes)] attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(a {
        class: "dx-sidebar-menu-sub-button",
        "data-slot": "sidebar-menu-sub-button",
        "data-sidebar": "menu-sub-button",
        "data-size": size.as_str(),
        "data-active": if is_active { "true" } else { "false" },
    });
    let merged = merge_attributes(vec![base, attributes]);

    if let Some(dynamic) = r#as {
        dynamic.call(merged)
    } else {
        rsx! {
            document::Link { rel: "stylesheet", href: asset!("/src/components/sidebar/style.css") }
            a { ..merged, {children} }
        }
    }
}

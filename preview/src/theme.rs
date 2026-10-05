use crate::components::{
    button::{Button, ButtonSize, ButtonVariant},
    drawer::{
        Drawer, DrawerClose, DrawerContent, DrawerDescription, DrawerFooter, DrawerHandle,
        DrawerTitle,
    },
    popover::{PopoverContent, PopoverRoot, PopoverTrigger},
    sidebar::{SidebarFooter, SidebarMenu, SidebarMenuButton, SidebarMenuItem},
};
use dioxus::prelude::*;
use dioxus_icons::lucide::{Moon, Palette, Sun};
use dioxus_primitives::ContentAlign;

const CHANNEL_NAME: &str = "dx-theme";

/// The site's persisted display preferences. Each is one `<html data-<key>>`
/// attribute AND one cookie (`dx_<key with - as _>`), so adding one here, to
/// [`PREPAINT_JS`]'s key list and to `theme-presets.css` is the whole job.
///
/// * `Mode` is `data-theme="dark|light"`, switched by `dx-components-theme.css`
///   (absent = follow `prefers-color-scheme`).
/// * `Base` / `Accent` / `Radius` are `data-theme-base` / `-accent` / `-radius`,
///   matched by the blocks in `preview/assets/theme-presets.css`
///   (absent = no preset = the library's own look).
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Pref {
    Mode,
    Base,
    Accent,
    Radius,
}

impl Pref {
    const fn key(self) -> &'static str {
        match self {
            Pref::Mode => "theme",
            Pref::Base => "theme-base",
            Pref::Accent => "theme-accent",
            Pref::Radius => "theme-radius",
        }
    }
}

/// shadcn's base colours, in picker order: `(attribute value, label)`. Every slug must have a
/// `[data-theme-base="<slug>"]` block in `theme-presets.css` (and vice versa; see the tests below).
pub const BASES: &[(&str, &str)] = &[
    ("neutral", "Neutral"),
    ("stone", "Stone"),
    ("zinc", "Zinc"),
    ("gray", "Gray"),
    ("slate", "Slate"),
    ("mauve", "Mauve"),
    ("olive", "Olive"),
    ("mist", "Mist"),
    ("taupe", "Taupe"),
];

/// shadcn's accent themes: `[data-theme-accent="<slug>"]` blocks.
pub const ACCENTS: &[(&str, &str)] = &[
    ("amber", "Amber"),
    ("blue", "Blue"),
    ("cyan", "Cyan"),
    ("emerald", "Emerald"),
    ("fuchsia", "Fuchsia"),
    ("green", "Green"),
    ("indigo", "Indigo"),
    ("lime", "Lime"),
    ("orange", "Orange"),
    ("pink", "Pink"),
    ("purple", "Purple"),
    ("red", "Red"),
    ("rose", "Rose"),
    ("sky", "Sky"),
    ("teal", "Teal"),
    ("violet", "Violet"),
    ("yellow", "Yellow"),
];

/// Radius choices in rem. [`DEFAULT_RADIUS`] is the library default and is stored as "no attribute",
/// so there is exactly one way to be on it.
pub const RADII: &[&str] = &["0", "0.3", "0.5", "0.625", "0.75", "1"];
const DEFAULT_RADIUS: &str = "0.625";

/// Applies every preference from its cookie to `<html>`, and defines `window.__dxTheme(key, value)`,
/// the one place a value is validated and written to / removed from an attribute (the live
/// cross-tab sync and [`set_pref`] reuse it).
///
/// This text is embedded VERBATIM in `preview/index.html`'s `<head>` (a unit test below enforces it),
/// which is what makes it run before first paint: the SSG HTML is static, the server cannot read a
/// cookie, and everything in `AppLayout`'s effect only runs after hydration. With the attributes
/// already on `<html>` and `theme-presets.css` / `dx-components-theme.css` render-blocking in
/// `<head>`, the first paint is already in the chosen mode, colours and radius. [`theme_seed`] runs it
/// again for any host that has no such `index.html`.
pub const PREPAINT_JS: &str = "(function(){var K=['theme','theme-base','theme-accent','theme-radius'],R=document.documentElement;function A(k,v){if(K.indexOf(k)<0)return;if(v&&/^[a-z0-9.]{1,16}$/.test(v)&&(k!=='theme'||v==='dark'||v==='light'))R.setAttribute('data-'+k,v);else R.removeAttribute('data-'+k)}window.__dxTheme=A;try{var C=document.cookie.split(';');K.forEach(function(k){var n='dx_'+k.replace('-','_')+'=',v=null;C.forEach(function(c){c=c.trim();if(c.indexOf(n)===0)v=decodeURIComponent(c.slice(n.length))});A(k,v)})}catch(e){}})()";

pub fn theme_seed() {
    _ = document::eval(&format!(
        r#"
        (function () {{
          if (window.__dx_theme_seeded) return;
          window.__dx_theme_seeded = true;

          {PREPAINT_JS};

          try {{
            const ch = new BroadcastChannel('{CHANNEL_NAME}');
            ch.addEventListener('message', (event) => {{
              const data = event.data;
              if (data && typeof data.key === 'string') window.__dxTheme(data.key, data.value);
            }});
            window.__dx_theme_channel = ch;
          }} catch (_) {{}}
        }})();
        "#,
    ));
}

/// Sets (or, with `None`, clears) one preference: attribute now, cookie for the next page load,
/// and a broadcast so other tabs and the demo iframes follow live. `value` is only ever one of the
/// slugs above; anything else is dropped rather than interpolated into script text.
pub fn set_pref(pref: Pref, value: Option<&str>) {
    let key = pref.key();
    let js_value = match value {
        Some(v) if !v.is_empty() && v.chars().all(|c| c.is_ascii_alphanumeric() || c == '.') => {
            format!("'{v}'")
        }
        _ => "null".to_string(),
    };

    _ = document::eval(&format!(
        r#"
        (function () {{
          const key = '{key}';
          const value = {js_value};
          const cookieName = 'dx_' + key.replace('-', '_');

          function getCookie(name) {{
            const prefix = name + '=';
            for (let p of document.cookie.split(';')) {{
              p = p.trim();
              if (p.startsWith(prefix)) return decodeURIComponent(p.slice(prefix.length));
            }}
            return '';
          }}

          if (window.__dxTheme) window.__dxTheme(key, value);
          if (getCookie(cookieName) === (value || '')) return;

          document.cookie = value
            ? cookieName + '=' + encodeURIComponent(value) + '; path=/; max-age=31536000; samesite=lax'
            : cookieName + '=; path=/; max-age=0; samesite=lax';

          try {{
            const ch = window.__dx_theme_channel;
            if (ch && typeof ch.postMessage === 'function') {{
              ch.postMessage({{ key, value }});
            }} else {{
              const tmp = new BroadcastChannel('{CHANNEL_NAME}');
              tmp.postMessage({{ key, value }});
              tmp.close();
            }}
          }} catch (_) {{}}
        }})();
        "#
    ));
}

pub fn set_theme(dark_mode: bool) {
    set_pref(Pref::Mode, Some(if dark_mode { "dark" } else { "light" }));
}

#[component]
pub fn DarkModeToggle() -> Element {
    rsx! {
        button {
            class: "dx-dark-mode-toggle dx-dark-mode-only",
            onclick: move |_| set_theme(false),
            r#type: "button",
            aria_label: "Enable light mode",
            DarkModeIcon {}
        }
        button {
            class: "dx-dark-mode-toggle dx-light-mode-only",
            onclick: move |_| set_theme(true),
            r#type: "button",
            aria_label: "Enable dark mode",
            LightModeIcon {}
        }
    }
}

#[component]
fn DarkModeIcon() -> Element {
    rsx! {
        Moon { size: "24px" }
    }
}

#[component]
fn LightModeIcon() -> Element {
    rsx! {
        Sun { size: "24px" }
    }
}

/// The picker's three axes, mirrored from the `<html data-theme-*>` attributes for `aria-pressed` and
/// the selected ring. The attributes stay the single source of truth (so presets need no JS to apply and
/// the first paint is already right, see [`PREPAINT_JS`]); these signals are only ever read back from
/// them, on mount and whenever a panel opens ([`ThemeState::sync`]).
///
/// One per trigger: the header's [`ThemePicker`] and the phone menu's [`ThemePickerSidebarFooter`] each
/// own one and hand it to the shared [`ThemePickerPanel`], so the panel itself never has to fetch its
/// own state after it mounts (which would paint one frame of "Default" selected before the read-back).
#[derive(Clone, Copy, PartialEq)]
struct ThemeState {
    base: Signal<String>,
    accent: Signal<String>,
    radius: Signal<String>,
}

impl ThemeState {
    /// Re-reads the three attributes: another tab, the cookie or the pre-paint script may have changed
    /// them since this component last looked.
    fn sync(self) {
        let (mut base, mut accent, mut radius) = (self.base, self.accent, self.radius);
        spawn(async move {
            let mut eval = document::eval(
                r#"const r = document.documentElement;
                dioxus.send([r.getAttribute('data-theme-base') || '', r.getAttribute('data-theme-accent') || '', r.getAttribute('data-theme-radius') || ''].join('|'));"#,
            );
            if let Ok(state) = eval.recv::<String>().await {
                let mut parts = state.split('|');
                base.set(parts.next().unwrap_or_default().to_string());
                accent.set(parts.next().unwrap_or_default().to_string());
                radius.set(parts.next().unwrap_or_default().to_string());
            }
        });
    }

    fn reset(self) {
        let (mut base, mut accent, mut radius) = (self.base, self.accent, self.radius);
        base.set(String::new());
        accent.set(String::new());
        radius.set(String::new());
        set_pref(Pref::Base, None);
        set_pref(Pref::Accent, None);
        set_pref(Pref::Radius, None);
    }
}

fn use_theme_state() -> ThemeState {
    let state = ThemeState {
        base: use_signal(String::new),
        accent: use_signal(String::new),
        radius: use_signal(String::new),
    };
    use_effect(move || state.sync());
    state
}

/// shadcn-style theme customizer: colour swatches (base colour, accent) and a radius row.
///
/// Why swatches rather than our `Select`: the choice is two independent colour axes plus a radius, and
/// a colour is picked by looking at it. A listbox shows one option at a time behind a click, three of
/// them would crowd the header, and shadcn's own customizer is swatches too. The panel opens from one
/// icon button and every option is visible at once.
///
/// The swatch dots take their colours from the very same `[data-theme-*]` blocks the presets use
/// (`main.css`, `.dx-theme-swatch`), so no colour is written down twice.
///
/// # One panel, two presentations
///
/// [`ThemePickerPanel`] is the content and exists once. Where it opens is a layout question, so it is
/// answered by CSS rather than by measuring the window in Rust (which would render different markup
/// before and after hydration, and flash): this component, the header trigger that opens it as a
/// popover, is always rendered and `main.css` hides it at phone width (`<= 760px`, the header's own
/// breakpoint); [`ThemePickerSidebarFooter`], the phone menu's trigger that opens the same panel as a
/// bottom sheet, is always rendered inside the sidebar and `main.css` shows it only at that width.
/// Exactly one trigger is ever visible, so exactly one panel is ever mounted.
#[component]
pub fn ThemePicker() -> Element {
    let state = use_theme_state();

    rsx! {
        PopoverRoot {
            is_modal: false,
            on_open_change: move |open| {
                if open {
                    state.sync();
                }
            },
            PopoverTrigger { class: "dx-theme-picker-trigger", aria_label: "Theme", title: "Theme",
                Palette { size: "24px" }
            }
            PopoverContent { class: "dx-theme-picker", align: ContentAlign::End,
                ThemePickerPanel {
                    state,
                    scope: "popover",
                    title: rsx! {
                        span { class: "dx-theme-picker-title", "Theme" }
                    },
                }
            }
        }
    }
}

/// The picker on phones: a "Theme" entry at the foot of the mobile navigation sheet that opens
/// [`ThemePickerPanel`] as a bottom [`Drawer`].
///
/// The header has no room for the popover's trigger at phone width (it overlapped "Charts" and the
/// GitHub link) and the popover itself would open from a point near the middle of the screen, so on
/// phones the picker lives where the rest of the navigation does and opens as a bottom sheet, the
/// native phone idiom. Rendered inside the desktop sidebar too (the sidebar only becomes a sheet below
/// 768px, in Rust); `.dx-theme-sidebar-footer` is `display: none` above the 760px breakpoint, which is
/// what keeps it out of the accessibility tree there.
///
/// The drawer is a child of the sidebar sheet, so it is a modal opened on top of a modal. That is the
/// shape the dialog primitive supports (it ships a nested-dialog demo): the browser stacks the two
/// `<dialog>`s in the top layer, so Escape and the focus trap act on the topmost one only, the scroll
/// lock is one refcounted lock shared along the component chain, and closing the drawer returns focus
/// to this trigger inside the still-open sheet.
#[component]
pub fn ThemePickerSidebarFooter() -> Element {
    let state = use_theme_state();
    let mut open = use_signal(|| false);

    rsx! {
        SidebarFooter { class: "dx-theme-sidebar-footer",
            SidebarMenu {
                SidebarMenuItem {
                    SidebarMenuButton {
                        as: move |attributes: Vec<Attribute>| rsx! {
                            button {
                                r#type: "button",
                                aria_haspopup: "dialog",
                                onclick: move |_| open.set(true),
                                ..attributes,
                                Palette { size: "1rem", "aria-hidden": "true" }
                                span { "Theme" }
                            }
                        },
                    }
                }
            }
            Drawer {
                open: open(),
                on_open_change: move |next| {
                    open.set(next);
                    if next {
                        state.sync();
                    }
                },
                DrawerContent {
                    DrawerHandle {}
                    ThemePickerPanel {
                        state,
                        scope: "drawer",
                        title: rsx! {
                            DrawerTitle { "Theme" }
                        },
                        DrawerDescription { "Colours and corner radius for this site." }
                    }
                    DrawerFooter {
                        DrawerClose {
                            as: |attributes| rsx! {
                                Button { variant: ButtonVariant::Outline, attributes, "Done" }
                            },
                        }
                    }
                }
            }
        }
    }
}

/// The picker's content: reset, base colour, accent, radius. Rendered inside either the header's
/// `Popover` (`scope: "popover"`) or the phone menu's `Drawer` (`scope: "drawer"`); `scope` is also
/// the `data-presentation` CSS hooks onto (larger touch targets in the drawer) and keeps the label ids
/// unique per presentation.
///
/// `title` is the heading in the head row (a `DrawerTitle` names the drawer's dialog, a plain span
/// does in the popover) and `children` render right under that row (the drawer's description), the two
/// kept together as one intro block.
#[component]
fn ThemePickerPanel(
    state: ThemeState,
    scope: &'static str,
    title: Element,
    children: Element,
) -> Element {
    let ThemeState {
        mut base,
        mut accent,
        mut radius,
    } = state;

    let base_label = label_of(BASES, &base.read());
    let accent_label = label_of(ACCENTS, &accent.read());

    rsx! {
        div { class: "dx-theme-picker-panel", "data-presentation": scope,
            div { class: "dx-theme-picker-intro",
                div { class: "dx-theme-picker-head",
                    {title}
                    Button {
                        variant: ButtonVariant::Ghost,
                        size: ButtonSize::Xs,
                        r#type: "button",
                        onclick: move |_| state.reset(),
                        "Reset"
                    }
                }
                {children}
            }

            div { class: "dx-theme-picker-section",
                span { class: "dx-theme-picker-label", id: "dx-theme-base-label-{scope}",
                    "Base colour"
                    span { class: "dx-theme-picker-current", "{base_label}" }
                }
                div {
                    class: "dx-theme-swatches",
                    role: "group",
                    aria_labelledby: "dx-theme-base-label-{scope}",
                    button {
                        class: "dx-theme-swatch",
                        r#type: "button",
                        aria_label: "Default base colour",
                        title: "Default",
                        aria_pressed: pressed(base.read().is_empty()),
                        onclick: move |_| {
                            base.set(String::new());
                            set_pref(Pref::Base, None);
                        },
                        span { class: "dx-theme-swatch-dot", "data-none": "true" }
                    }
                    for (slug , label) in BASES.iter().copied() {
                        button {
                            key: "{slug}",
                            class: "dx-theme-swatch",
                            r#type: "button",
                            aria_label: "{label}",
                            title: "{label}",
                            aria_pressed: pressed(*base.read() == slug),
                            "data-theme-base": slug,
                            onclick: move |_| {
                                base.set(slug.to_string());
                                set_pref(Pref::Base, Some(slug));
                            },
                            span { class: "dx-theme-swatch-dot" }
                        }
                    }
                }
            }

            div { class: "dx-theme-picker-section",
                span { class: "dx-theme-picker-label", id: "dx-theme-accent-label-{scope}",
                    "Accent"
                    span { class: "dx-theme-picker-current", "{accent_label}" }
                }
                div {
                    class: "dx-theme-swatches",
                    role: "group",
                    aria_labelledby: "dx-theme-accent-label-{scope}",
                    button {
                        class: "dx-theme-swatch",
                        r#type: "button",
                        aria_label: "Default accent",
                        title: "Default",
                        aria_pressed: pressed(accent.read().is_empty()),
                        onclick: move |_| {
                            accent.set(String::new());
                            set_pref(Pref::Accent, None);
                        },
                        span { class: "dx-theme-swatch-dot", "data-none": "true" }
                    }
                    for (slug , label) in ACCENTS.iter().copied() {
                        button {
                            key: "{slug}",
                            class: "dx-theme-swatch",
                            r#type: "button",
                            aria_label: "{label}",
                            title: "{label}",
                            aria_pressed: pressed(*accent.read() == slug),
                            "data-theme-accent": slug,
                            onclick: move |_| {
                                accent.set(slug.to_string());
                                set_pref(Pref::Accent, Some(slug));
                            },
                            span { class: "dx-theme-swatch-dot" }
                        }
                    }
                }
            }

            div { class: "dx-theme-picker-section",
                span { class: "dx-theme-picker-label", id: "dx-theme-radius-label-{scope}",
                    "Radius"
                    span { class: "dx-theme-picker-current", "rem" }
                }
                div {
                    class: "dx-theme-chips",
                    role: "group",
                    aria_labelledby: "dx-theme-radius-label-{scope}",
                    for value in RADII.iter().copied() {
                        {
                            let is_default = value == DEFAULT_RADIUS;
                            let selected = if is_default {
                                radius.read().is_empty()
                            } else {
                                *radius.read() == value
                            };
                            rsx! {
                                Button {
                                    key: "{value}",
                                    variant: if selected { ButtonVariant::Primary } else { ButtonVariant::Outline },
                                    size: ButtonSize::Xs,
                                    r#type: "button",
                                    title: if is_default { "0.625 (default)" } else { value },
                                    aria_pressed: pressed(selected),
                                    onclick: move |_| {
                                        if is_default {
                                            radius.set(String::new());
                                            set_pref(Pref::Radius, None);
                                        } else {
                                            radius.set(value.to_string());
                                            set_pref(Pref::Radius, Some(value));
                                        }
                                    },
                                    "{value}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn pressed(on: bool) -> &'static str {
    if on {
        "true"
    } else {
        "false"
    }
}

fn label_of(list: &[(&'static str, &'static str)], slug: &str) -> &'static str {
    list.iter()
        .find(|(s, _)| *s == slug)
        .map_or("Default", |(_, label)| label)
}

#[cfg(test)]
mod tests {
    use super::*;

    const INDEX_HTML: &str = include_str!("../index.html");
    const PRESETS_CSS: &str = include_str!("../assets/theme-presets.css");

    /// The pre-paint snippet exists twice (a Rust const the runtime evals, and the `<script>` in
    /// `index.html` that actually runs before first paint). They are the same text or the page flashes.
    #[test]
    fn index_html_embeds_the_prepaint_snippet_verbatim() {
        assert!(
            INDEX_HTML.contains(&format!("<script>{PREPAINT_JS}</script>")),
            "preview/index.html must contain `<script>` + theme::PREPAINT_JS + `</script>` exactly"
        );
        assert!(!PREPAINT_JS.contains("</script"));
    }

    /// Every cookie key the snippet reads is a `Pref`, and vice versa.
    #[test]
    fn prepaint_snippet_handles_every_pref() {
        for pref in [Pref::Mode, Pref::Base, Pref::Accent, Pref::Radius] {
            assert!(
                PREPAINT_JS.contains(&format!("'{}'", pref.key())),
                "PREPAINT_JS does not list {}",
                pref.key()
            );
        }
    }

    fn blocks(attr: &str) -> Vec<String> {
        let needle = format!("[data-{attr}=\"");
        let mut out: Vec<String> = PRESETS_CSS
            .lines()
            .filter(|l| l.starts_with(&needle) && l.trim_end().ends_with('{'))
            .map(|l| {
                let rest = &l[needle.len()..];
                rest[..rest.find('"').unwrap()].to_string()
            })
            .collect();
        out.sort();
        out
    }

    fn sorted(list: &[&str]) -> Vec<String> {
        let mut v: Vec<String> = list.iter().map(|s| s.to_string()).collect();
        v.sort();
        v
    }

    /// The picker's lists and the generated stylesheet cannot drift apart: a swatch with no block
    /// would silently do nothing, a block with no swatch would be unreachable.
    #[test]
    fn picker_lists_match_the_preset_stylesheet() {
        let bases: Vec<&str> = BASES.iter().map(|(s, _)| *s).collect();
        let accents: Vec<&str> = ACCENTS.iter().map(|(s, _)| *s).collect();
        assert_eq!(blocks("theme-base"), sorted(&bases));
        assert_eq!(blocks("theme-accent"), sorted(&accents));
        let non_default: Vec<&str> = RADII
            .iter()
            .copied()
            .filter(|r| *r != DEFAULT_RADIUS)
            .collect();
        let mut radius_blocks = blocks("theme-radius");
        radius_blocks.retain(|r| r != DEFAULT_RADIUS);
        assert_eq!(radius_blocks, sorted(&non_default));
    }

    /// Accent blocks override base blocks through the cascade, so they must come after every base block.
    #[test]
    fn accents_follow_bases_in_the_stylesheet() {
        let last_base = PRESETS_CSS.rfind("[data-theme-base=\"").unwrap();
        let first_accent = PRESETS_CSS.find("[data-theme-accent=\"").unwrap();
        assert!(last_base < first_accent);
    }
}

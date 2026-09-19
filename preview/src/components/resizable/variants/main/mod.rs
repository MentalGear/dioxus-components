use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::GripVertical;

// shadcn/ui v4's own demo: `flex h-[200px] items-center justify-center p-6` per panel, label
// `font-semibold`. `height: 100%` (fill whatever the *panel* actually gives it) replaces
// shadcn's own literal `h-[200px]` here deliberately: react-resizable-panels measures each
// panel's pixel height with JS and grows the group to fit, but this crate's ResizablePanel is
// pure-CSS percentage-of-the-group's-real-size with `overflow: hidden` (primitives/src/
// resizable.rs's own documented contract), so a FIXED 200px content height silently clips
// (confirmed by screenshot: the nested group's 25%-share "Top" panel, well under 200px tall,
// truncated its own label mid-glyph). `height: 100%` is the construction that can never
// overflow its panel regardless of split percentage or a later drag/resize -- the two
// `DemoGroup` calls below restore an explicit group height precisely so this has a reliably
// generous share to fill, matching this demo's own original (pre-shadcn-translation) height.
fn panel_content(label: &'static str) -> Element {
    rsx! {
        div {
            style: "display: flex; align-items: center; justify-content: center; height: 100%; padding: var(--dx-space-6); box-sizing: border-box;",
            span { style: "font-weight: 600;", "{label}" }
        }
    }
}

// shadcn's own handle grip: `bg-border z-10 flex h-4 w-3 items-center justify-center rounded-xs
// border` (the classes live in style.css's `.dx-resizable-handle-grip`) around a
// `GripVerticalIcon` at `size-2.5` (0.625rem).
fn handle_grip() -> Element {
    rsx! {
        span { class: "dx-resizable-handle-grip", GripVertical { size: "0.625rem" } }
    }
}

// shadcn's own demo container classes: `max-w-md rounded-lg border md:min-w-[450px]`, applied
// directly to the demo's own ResizablePanelGroup instance -- the same way every other demo in
// this crate applies its own presentational chrome (border/height/overflow) via props rather
// than a class shipped in style.css, so a consumer installing this component gets none of this
// page's own demo framing. `--primary-color-6`/`-7` is this repo's established "border" token
// pair (`.dx-card`, `.dx-button[data-style="outline"]`); `--dx-radius-lg` is shadcn's
// `rounded-lg`.
#[component]
fn DemoGroup(direction: ResizableDirection, height: &'static str, children: Element) -> Element {
    rsx! {
        ResizablePanelGroup {
            direction,
            height,
            max_width: "28rem",
            min_width: "450px",
            border: "1px solid var(--light, var(--primary-color-6)) var(--dark, var(--primary-color-7))",
            border_radius: "var(--dx-radius-lg)",
            overflow: "hidden",
            {children}
        }
    }
}

#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 2rem;",

            // A horizontal group (50/50) whose second panel contains a nested vertical group
            // (25/75) -- shadcn's own canonical "Resizable" demo shape.
            DemoGroup { direction: ResizableDirection::Horizontal, height: "20rem",
                ResizablePanel { index: 0usize, default_size: 50.0, min_size: 20.0,
                    {panel_content("Sidebar")}
                }
                ResizableHandle { index: 0usize, aria_label: "Sidebar", {handle_grip()} }
                ResizablePanel { index: 1usize, default_size: 50.0, min_size: 20.0,
                    ResizablePanelGroup { direction: ResizableDirection::Vertical, height: "100%",
                        ResizablePanel { index: 0usize, default_size: 25.0, min_size: 15.0,
                            {panel_content("Top")}
                        }
                        ResizableHandle { index: 0usize, aria_label: "Top", {handle_grip()} }
                        ResizablePanel { index: 1usize, default_size: 75.0, min_size: 15.0,
                            {panel_content("Content")}
                        }
                    }
                }
            }

            // A second, variant-free example: a collapsible first panel, styled the same.
            // Home/Enter on its handle collapse it to `collapsed_size` (0); Enter again
            // restores it.
            DemoGroup { direction: ResizableDirection::Horizontal, height: "10rem",
                ResizablePanel {
                    index: 0usize,
                    default_size: 30.0,
                    min_size: 15.0,
                    collapsible: true,
                    collapsed_size: 0.0,
                    {panel_content("Collapsible Panel")}
                }
                ResizableHandle { index: 0usize, aria_label: "Collapsible Panel", {handle_grip()} }
                ResizablePanel { index: 1usize, default_size: 70.0, min_size: 20.0,
                    {panel_content("Content")}
                }
            }
        }
    }
}

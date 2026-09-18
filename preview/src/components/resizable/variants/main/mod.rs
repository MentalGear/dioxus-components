use super::super::component::*;
use dioxus::prelude::*;
use dioxus_icons::lucide::GripVertical;

fn panel_content(label: &'static str) -> Element {
    rsx! {
        div {
            style: "display: flex; align-items: center; justify-content: center; height: 100%; padding: 1rem; box-sizing: border-box;",
            "{label}"
        }
    }
}

#[component]
pub fn Demo() -> Element {
    rsx! {
        div { style: "display: flex; flex-direction: column; gap: 2rem;",

            // A horizontal group (50/50) whose second panel contains a nested vertical group
            // (25/75) -- shadcn's own canonical "Resizable" demo shape.
            ResizablePanelGroup {
                direction: ResizableDirection::Horizontal,
                height: "20rem",
                border: "1px solid var(--secondary-color-3)",
                border_radius: "0.5rem",
                overflow: "hidden",

                ResizablePanel { index: 0usize, default_size: 50.0, min_size: 20.0,
                    {panel_content("Sidebar")}
                }
                ResizableHandle { index: 0usize, aria_label: "Sidebar",
                    span { class: "dx-resizable-handle-grip", GripVertical { size: "0.75rem" } }
                }
                ResizablePanel { index: 1usize, default_size: 50.0, min_size: 20.0,
                    ResizablePanelGroup { direction: ResizableDirection::Vertical, height: "100%",
                        ResizablePanel { index: 0usize, default_size: 25.0, min_size: 15.0,
                            {panel_content("Top")}
                        }
                        ResizableHandle { index: 0usize, aria_label: "Top",
                            span { class: "dx-resizable-handle-grip", GripVertical { size: "0.75rem" } }
                        }
                        ResizablePanel { index: 1usize, default_size: 75.0, min_size: 15.0,
                            {panel_content("Content")}
                        }
                    }
                }
            }

            // A second, variant-free example: a collapsible first panel. Home/Enter on its
            // handle collapse it to `collapsed_size` (0); Enter again restores it.
            ResizablePanelGroup {
                direction: ResizableDirection::Horizontal,
                height: "10rem",
                border: "1px solid var(--secondary-color-3)",
                border_radius: "0.5rem",
                overflow: "hidden",

                ResizablePanel {
                    index: 0usize,
                    default_size: 30.0,
                    min_size: 15.0,
                    collapsible: true,
                    collapsed_size: 0.0,
                    {panel_content("Collapsible Panel")}
                }
                ResizableHandle { index: 0usize, aria_label: "Collapsible Panel",
                    span { class: "dx-resizable-handle-grip", GripVertical { size: "0.75rem" } }
                }
                ResizablePanel { index: 1usize, default_size: 70.0, min_size: 20.0,
                    {panel_content("Content")}
                }
            }
        }
    }
}

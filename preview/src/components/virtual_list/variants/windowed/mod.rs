use super::super::component::*;
use crate::components::button::{Button, ButtonVariant};
use dioxus::document::Eval;
use dioxus::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

const MANY: usize = 100_000;
/// The row counts the buttons switch between: either side of `Auto`'s default threshold (1,000).
const COUNTS: [usize; 4] = [MANY, 5_000, 1_001, 1_000];

// Demo instrumentation, not part of the component: a render of the list calls `render_item`
// for consecutive indices, so a jump in the index marks the start of a new render. The readout
// below shows how many times Rust rendered the list while you scroll.
static RENDERS: AtomicUsize = AtomicUsize::new(0);
static LAST_INDEX: AtomicUsize = AtomicUsize::new(usize::MAX);

fn count_render(idx: usize) {
    let previous = LAST_INDEX.swap(idx, Ordering::Relaxed);
    if previous == usize::MAX || previous + 1 != idx {
        RENDERS.fetch_add(1, Ordering::Relaxed);
    }
}

// Reports how many rows are in the DOM and which mode the list resolved to, whenever the
// observed subtree changes.
const READOUT_JS: &str = r#"
    const root = document.querySelector(await dioxus.recv());
    if (!root) return;
    let frame = 0;
    const report = () => {
        frame = 0;
        const list = root.querySelector('[role="list"]');
        dioxus.send([
            root.querySelectorAll('[role="listitem"]').length,
            list ? list.dataset.virtualize : "unmounted",
        ]);
    };
    const observer = new MutationObserver(() => {
        if (!frame) frame = requestAnimationFrame(report);
    });
    observer.observe(root, { childList: true, subtree: true, attributes: true });
    report();
    await dioxus.recv();
    observer.disconnect();
    if (frame) cancelAnimationFrame(frame);
"#;

#[component]
fn Readout(total: usize) -> Element {
    let mut rows = use_signal(|| 0usize);
    let mut mode = use_signal(String::new);
    let mut renders = use_signal(|| 0usize);
    let mut bridge = use_signal(|| None::<Eval>);

    use_effect(move || {
        let mut eval = document::eval(READOUT_JS);
        let _ = eval.send("[data-vl-rows-in=\"windowed\"]");
        spawn(async move {
            while let Ok((n, resolved)) = eval.recv::<(usize, String)>().await {
                if *rows.peek() != n {
                    rows.set(n);
                }
                if *mode.peek() != resolved {
                    mode.set(resolved);
                }
                let renders_now = RENDERS.load(Ordering::Relaxed);
                if *renders.peek() != renders_now {
                    renders.set(renders_now);
                }
            }
        });
        bridge.set(Some(eval));
    });
    use_drop(move || {
        if let Some(eval) = bridge.take() {
            let _ = eval.send("stop");
        }
    });

    rsx! {
        p { class: "dx-virtual-list-readout",
            "Mode: "
            strong { "data-vl-mode": "", "{mode}" }
            " · Rows in the DOM: "
            strong { "data-vl-rows": "", "{rows}" }
            " of {total} · Rust renders of the list: "
            strong { "data-vl-renders": "", "{renders}" }
        }
    }
}

#[component]
pub fn Demo() -> Element {
    let mut shown = use_signal(|| true);
    let mut rows = use_signal(|| MANY);

    rsx! {
        div { class: "dx-virtual-list-demo", "data-vl-demo": "windowed",
            p { class: "dx-virtual-list-subtitle",
                "Only the rows in view (plus a buffer) are mounted, however long the list. Native find-in-page cannot reach rows that are not mounted. Rust renders once per few rows of scrolling, not once per scroll event. Up to 1,000 rows the default mode keeps every row in the DOM instead: switch the count to see it change."
            }
            style { r#".dx-virtual-list-container {{
  position: relative;
  max-height: 36rem;
  contain: layout paint;
  overflow-y: auto;
}}

.dx-virtual-list-demo {{
  display: flex;
  flex-direction: column;
  margin: 0 auto;
  gap: 0.75rem;
}}

.dx-virtual-list-demo .dx-virtual-list-subtitle,
.dx-virtual-list-demo .dx-virtual-list-readout {{
  margin: 0;
  color: var(--dx-muted-foreground);
  font-size: 0.9rem;
}}

.dx-virtual-list-readout strong {{
  color: var(--dx-foreground);
  font-variant-numeric: tabular-nums;
}}

.dx-virtual-list-card {{
  padding: 0.75rem 0.9rem;
  border: 1px solid var(--dx-border);
  border-radius: 0.625rem;
  background: var(--dx-card);
}}

.dx-virtual-list-card-title {{
  margin: 0 0 0.3rem;
  color: var(--dx-foreground);
  font-size: 0.95rem;
  font-weight: 660;
}}

.dx-virtual-list-card p {{
  margin: 0;
  color: var(--dx-muted-foreground);
  font-size: 0.875rem;
  line-height: 1.4;
}}"# }
            div { "data-vl-rows-in": "windowed",
                if shown() {
                    VirtualList {
                        count: rows,
                        // No `virtualize`: `Auto` (the default) windows any list longer than
                        // 1,000 rows and keeps every row in the DOM up to that. Pass
                        // `VirtualListMode::Windowed` to window a short list too.
                        buffer: 12usize,
                        estimate_size: |idx: usize| 68 + (idx % 6).div_ceil(2) as u32 * 20,
                        "aria-label": "One hundred thousand rows",
                        render_item: move |idx: usize| {
                            count_render(idx);
                            let extra_text = "Extra content to vary row height. ".repeat(idx % 6);
                            rsx! {
                                article { class: "dx-virtual-list-card",
                                    p { class: "dx-virtual-list-card-title", "Row {idx + 1} of {rows}" }
                                    p { "Mounted only while it is near the viewport. Index = {idx}" }
                                    p { "{extra_text}" }
                                }
                            }
                        },
                    }
                }
            }
            Readout { total: rows() }
            div { style: "display: flex; flex-wrap: wrap; align-items: center; gap: var(--dx-space-2);",
                for count in COUNTS {
                    Button {
                        key: "{count}",
                        variant: if rows() == count { ButtonVariant::Secondary } else { ButtonVariant::Outline },
                        "aria-pressed": (rows() == count).to_string(),
                        onclick: move |_| rows.set(count),
                        "{count} rows"
                    }
                }
                Button {
                    variant: ButtonVariant::Outline,
                    onclick: move |_| shown.toggle(),
                    if shown() {
                        "Unmount the list"
                    } else {
                        "Mount the list"
                    }
                }
            }
        }
    }
}

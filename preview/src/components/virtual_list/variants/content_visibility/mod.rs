use super::super::component::*;
use dioxus::document::Eval;
use dioxus::prelude::*;
use std::sync::atomic::{AtomicUsize, Ordering};

const ROWS: usize = 1_000;

// Demo instrumentation, not part of the component: a render of the list calls `render_item`
// for consecutive indices, so a jump in the index marks the start of a new render. The readout
// below shows how many times Rust rendered the list.
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
        let _ = eval.send("[data-vl-rows-in=\"content-visibility\"]");
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
    rsx! {
        div { class: "dx-virtual-list-demo", "data-vl-demo": "content-visibility",
            p { class: "dx-virtual-list-subtitle",
                "All {ROWS} rows are in the DOM and in the page's HTML. The browser skips laying out and painting the ones off-screen, so scrolling costs Rust nothing: Ctrl+F, selection and the accessibility tree reach every row."
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
            div { "data-vl-rows-in": "content-visibility",
                VirtualList {
                    count: ROWS,
                    // Keep every row in the DOM, whatever the count (`Auto` would also pick
                    // this up to 1,000 rows).
                    virtualize: VirtualListMode::ContentVisibility,
                    // The height the browser assumes for a row it has not rendered yet (summed
                    // over each chunk of 20 rows); it remembers the real height once it has.
                    // Close guesses keep the scrollbar steady.
                    estimate_size: |idx: usize| 68 + (idx % 6).div_ceil(2) as u32 * 20,
                    "aria-label": "One thousand rows",
                    render_item: move |idx: usize| {
                        count_render(idx);
                        let extra_text = "Extra content to vary row height. ".repeat(idx % 6);
                        rsx! {
                            article { class: "dx-virtual-list-card",
                                p { class: "dx-virtual-list-card-title", "Row {idx + 1} of {ROWS}" }
                                p { "Rendered by the browser only while it is on screen. Index = {idx}" }
                                p { "{extra_text}" }
                            }
                        }
                    },
                }
            }
            Readout { total: ROWS }
        }
    }
}

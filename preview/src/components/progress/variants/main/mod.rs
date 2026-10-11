use super::super::component::*;
use dioxus::prelude::*;
use dioxus_primitives::activity::use_motion;
use dioxus_primitives::interval::use_interval_while;

#[component]
pub fn Demo() -> Element {
    let mut progress = use_signal(|| 0);
    // xorshift32 state: the demo only needs steps that look irregular, not real randomness
    // (and no `rand` dependency, no `Math.random()` through `eval`).
    let mut seed = use_hook(|| CopyValue::new(0x2545_F491_u32));

    // Owned by this component and dropped with it (`use_interval`), never a bare JS `setInterval`.
    // Visibility-gated: no wakeups while the demo is off-screen or the tab is hidden.
    let motion = use_motion();
    use_interval_while(motion.active(), std::time::Duration::from_secs(1), move || {
        let mut x = seed.cloned();
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        seed.set(x);
        let step = (x % 30) as usize;
        let next = (*progress.peek() + step) % 101;
        progress.set(next);
    });

    rsx! {
        div { ..motion.attributes(),
            Progress { aria_label: "Progressbar Demo", value: progress() as f64 }
        }
    }
}

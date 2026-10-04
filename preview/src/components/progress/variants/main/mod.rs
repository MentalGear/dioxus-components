use super::super::component::*;
use dioxus::prelude::*;
use dioxus_primitives::interval::use_interval;

#[component]
pub fn Demo() -> Element {
    let mut progress = use_signal(|| 0);
    // xorshift32 state: the demo only needs steps that look irregular, not real randomness
    // (and no `rand` dependency, no `Math.random()` through `eval`).
    let mut seed = use_hook(|| CopyValue::new(0x2545_F491_u32));

    // Owned by this component and dropped with it (`use_interval`), never a bare JS `setInterval`.
    use_interval(std::time::Duration::from_secs(1), move || {
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
        Progress { aria_label: "Progressbar Demo", value: progress() as f64 }
    }
}

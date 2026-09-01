// Item 1 fix (2026-09-01, live-site report): "checkboxes and radio buttons
// collapse visually when toggled on/off." This fixture used to import the
// *raw* `dioxus_primitives::checkbox::{Checkbox, CheckboxIndicator}` and
// `dioxus_primitives::radio_group::{RadioGroup, RadioItem}` primitives
// directly, unlike every other consumer in this preview app -- see e.g.
// `components/slider/variants/dynamic_range/mod.rs` importing
// `crate::components::switch::Switch`, or `components/card/variants/main/
// mod.rs` importing `crate::components::{button, input, label}`: the
// established pattern in this workspace is for one component's fixture/demo
// to compose *another* component's themed wrapper (`preview/src/components/
// <name>/component.rs`), not the bare `dioxus_primitives` type, because the
// theme wrapper is what attaches the fixed-size CSS classes
// (`.dx-checkbox`/`.dx-checkbox-indicator` in `../checkbox/style.css`,
// `.dx-radio-item` in `../radio_group/style.css`). Without those classes,
// `Checkbox`'s `<button>` has no explicit `width`/`height` at all -- its box
// is sized purely by its content -- and `CheckboxIndicator` renders its
// children (the checkmark) *only* while checked (see
// `primitives/src/checkbox.rs`'s doc: "children will only be rendered when
// the checkbox is checked"), so the button visibly grows for the glyph when
// checked and collapses back to a zero-content box when unchecked.
// `RadioItem`'s entire visual (the circle) is a `.dx-radio-item::before`
// pseudo-element that plain `dioxus_primitives::radio_group::RadioItem`
// never gets without that class, so its `<button>` collapses to whatever a
// bare, empty `<button>` renders as in every state. Confirmed by measurement
// against `/component/?name=checkbox` and `/component/?name=radio_group`
// (both themed, both stable-sized in every state) vs. this fixture
// pre-fix (both collapsing) -- see this session's report for the exact
// `getBoundingClientRect()` numbers. Fixed here, not in
// `primitives/src/checkbox.rs` or `radio_group.rs`: the *conditional-
// children* behavior of `CheckboxIndicator` is correct, documented API (a
// consumer's indicator content need not be a fixed-size glyph), and
// `RadioItem`'s primitive renders no built-in visual by design (it is a
// headless primitive) -- the actual defect was this one fixture forgetting
// the theme layer every sibling fixture already applies.
use crate::components::checkbox::Checkbox;
use crate::components::radio_group::{RadioGroup, RadioItem};
use dioxus::prelude::*;
use dioxus_primitives::checkbox::CheckboxState;
use dioxus_primitives::select::{Select, SelectList, SelectOption, SelectTrigger, SelectValue};
use dioxus_primitives::switch::{Switch, SwitchThumb};

#[css_module("/src/components/form/style.css")]
struct Styles;

/// Builds JS that reads `new FormData(form)` and writes one `name=value` line
/// per entry, in insertion order, into the result element -- this is the
/// tier 2 (HTML) entry-list rule from `docs/conformance-harness.md`, read
/// straight from the browser's own algorithm rather than through any
/// Dioxus-side approximation of it. A `data-submit-count` bump lets
/// Playwright await the update deterministically.
fn read_form_data_js(form_id: &str, result_id: &str) -> String {
    format!(
        r#"
        const form = document.getElementById('{form_id}');
        const result = document.getElementById('{result_id}');
        if (form && result) {{
            const data = new FormData(form);
            const lines = [];
            for (const [key, value] of data.entries()) {{
                lines.push(key + '=' + value);
            }}
            result.textContent = lines.join('\n');
            const count = parseInt(result.getAttribute('data-submit-count') || '0', 10);
            result.setAttribute('data-submit-count', String(count + 1));
        }}
        "#
    )
}

/// Wires a single capturing `invalid` listener on the form. The `invalid`
/// event does not bubble, so a capturing listener on an ancestor is the only
/// way to observe every control the browser's constraint validation blocked
/// submission on -- including the hidden native inputs the form components
/// render internally. Invalid targets for one submit attempt are batched
/// (via a microtask) into a single, replacing write so repeated attempts
/// don't accumulate stale entries.
///
/// Item 2 fix (2026-09-01, live-site report): "Required blocking (form B)
/// seems broken, at least visually." The block itself always worked (tier-2
/// rule 4 in `form-participation.spec.ts` was already green) -- `required`
/// lives on each control's visually-hidden native mirror (`BubbleInput` in
/// `primitives/src/checkbox.rs`, the hidden `<input type="radio">` in
/// `radio_group.rs`, the hidden `<select>` in `select/components/select.rs`,
/// all marked `aria-hidden="true" tabindex="-1"`), and Chrome refuses to
/// focus a non-focusable hidden control or show its native validation
/// bubble on one -- confirmed via the browser console on submit: "An
/// invalid form control with name='...' is not focusable." So the missing
/// piece was purely user-visible feedback, not the blocking mechanics.
///
/// Fixed here, in the fixture, rather than in the primitives: bridging
/// native constraint-validation feedback onto a *visible* stand-in for an
/// invalid hidden control is a UI-layer decision each consumer's markup is
/// best placed to make (which element counts as "the visible one," how to
/// style it) -- a primitives-level validity bridge would need a public API
/// for a library-wide default that this one fixture's fix does not have to
/// invent (see this session's report, "follow-up candidates").
///
/// The mapping from an invalid hidden control to its visible counterpart is
/// the same trick across every control this fixture pairs a hidden mirror
/// with: the mirror is the element carrying `aria-hidden="true"`, and its
/// paired visible control is the nearest `<button>` inside the shared
/// `.dx-form-field` wrapper (the class both the `<div>` and `<fieldset>`
/// field containers use) -- `Checkbox`, `RadioItem`, `Switch`, and
/// `SelectTrigger` are all rendered as a `<button>` (see their respective
/// primitives), so one selector covers all four. A plain native reference
/// control (no hidden mirror, not `aria-hidden`) maps to itself, since it
/// already *is* the visible control. Only the *first* blocked control (in
/// `invalid`-firing order, which follows document order) receives focus --
/// matching a real browser's own reportValidity() behavior of focusing and
/// bubble-anchoring on just the first invalid control, not every one at
/// once -- while every blocked control's visible counterpart gets
/// `data-invalid="true"` (styled by this page's `style.css`) so the whole
/// batch is visible, not just the focused one. Cleared once the control's
/// underlying constraint validation actually resolves (checked via
/// `checkValidity()` on interaction, or unconditionally on the form's own
/// `reset`) -- see `reviewInvalidMarkers` below for why that has to be a
/// re-check against real validity rather than a simple "on change" listener.
fn watch_invalid_js(form_id: &str, report_id: &str) -> String {
    format!(
        r#"
        const form = document.getElementById('{form_id}');
        const report = document.getElementById('{report_id}');
        if (form && report && !form.dataset.dxInvalidWired) {{
            form.dataset.dxInvalidWired = '1';

            // `[class*="dx-form-field"]`, not `.dx-form-field`: `#[css_module]`
            // hash-suffixes every class it generates (confirmed by execution --
            // the rendered class here is `dx-form-field-<8 hex chars>`, not the
            // literal name `Styles::dx_form_field` reads in Rust source), so an
            // exact class selector never matches any real element.
            function fieldContainerOf(el) {{
                return el.closest('[class*="dx-form-field"]');
            }}
            function visibleCounterpart(target) {{
                if (target.getAttribute('aria-hidden') !== 'true') return target;
                const container = fieldContainerOf(target);
                return container ? container.querySelector('button') : null;
            }}

            let pending = new Set();
            let order = [];
            let scheduled = false;
            form.addEventListener('invalid', (event) => {{
                const target = event.target;
                pending.add(target.name || target.id || target.tagName);
                order.push(target);
                if (!scheduled) {{
                    scheduled = true;
                    setTimeout(() => {{
                        report.textContent = Array.from(pending).join('\n');
                        const count = parseInt(report.getAttribute('data-invalid-count') || '0', 10);
                        report.setAttribute('data-invalid-count', String(count + 1));

                        let focused = false;
                        for (const invalidTarget of order) {{
                            const visible = visibleCounterpart(invalidTarget);
                            if (!visible) continue;
                            visible.setAttribute('data-invalid', 'true');
                            if (!focused) {{
                                visible.focus();
                                focused = true;
                            }}
                        }}

                        pending = new Set();
                        order = [];
                        scheduled = false;
                    }}, 0);
                }}
            }}, true);

            // Clearing `data-invalid` again is *not* the mirror image of
            // setting it: the hidden mirrors this library's controls update
            // on interaction (`BubbleInput` in checkbox.rs, the hidden
            // `<input>`/`<select>` in radio_group.rs/select.rs) are all
            // driven by `document::eval` setting `.checked`/`.value`
            // directly -- which, unlike a real user interacting with a
            // native control, never dispatches a `change`/`input` event
            // (confirmed by execution: a plain `change` listener here never
            // fired for any library control, only the native reference
            // ones). So instead of reacting to a specific event, any
            // plausible interaction (click, key release, or a real
            // `change`/`input` from a native control) triggers a re-check
            // of every currently-marked control's *actual* validity --
            // correct regardless of how that control updates its hidden
            // mirror, and self-correcting if a marker and reality ever
            // disagree. Listened for on `document`, not `form`: `Select`'s
            // option list can render through this repo's top-layer/popover
            // machinery outside the form's own DOM subtree, where a
            // listener on `form` itself (even capturing) would never see
            // the click. Deferred a macrotask out (`setTimeout(_, 0)`): the
            // click/keyup that toggles a library control fires *before*
            // that control's own Dioxus effect has re-run and pushed the
            // new checked/value onto its hidden mirror (confirmed by
            // execution -- checking synchronously on click always saw the
            // mirror's *pre*-click validity), so this needs to wait for
            // that settle first, same as the `reset` listener below.
            //
            // Reads `.validity.valid`, never calls `.checkValidity()`:
            // confirmed by execution, the latter is not the passive read it
            // looks like -- per spec it *fires `invalid` on the element
            // when the check fails*, which this file's own capturing
            // `invalid` listener above would immediately see and re-mark
            // `data-invalid` from, undoing the very clear this function was
            // trying to make (a self-reinforcing loop that, before this
            // fix, made every required library control's marker outlive a
            // form reset -- the native references were unaffected only
            // because their reset finishes synchronously, before this
            // function's deferred re-check ever ran). `.validity` is a
            // live `ValidityState` read with no such side effect.
            function reviewInvalidMarkers() {{
                setTimeout(() => {{
                    form.querySelectorAll('[data-invalid=\"true\"]').forEach((visible) => {{
                        let hidden = null;
                        if (visible.getAttribute('aria-hidden') === 'true') {{
                            hidden = visible;
                        }} else {{
                            const container = fieldContainerOf(visible);
                            hidden = container ? container.querySelector('[aria-hidden=\"true\"]') : null;
                        }}
                        if (!hidden || !hidden.willValidate || hidden.validity.valid) {{
                            visible.removeAttribute('data-invalid');
                        }}
                    }});
                }}, 0);
            }}
            document.addEventListener('click', reviewInvalidMarkers, true);
            document.addEventListener('keyup', reviewInvalidMarkers, true);
            form.addEventListener('change', reviewInvalidMarkers, true);
            form.addEventListener('input', reviewInvalidMarkers, true);

            // Deferred a macrotask out (`setTimeout(_, 0)`, same reasoning as
            // the `invalid` batching above): each library control's own
            // `use_form_reset_listener` is *also* a `reset` listener on this
            // same form, resyncing its Dioxus-side state and re-rendering --
            // confirmed by execution, clearing synchronously here raced that
            // re-render and the marker came right back on every library
            // control (only the native references' cleared for good), so
            // this waits for that settle first.
            form.addEventListener('reset', () => {{
                setTimeout(() => {{
                    form.querySelectorAll('[data-invalid]').forEach((el) => el.removeAttribute('data-invalid'));
                }}, 0);
            }});
        }}
        "#
    )
}

/// A real `<form>` fixture pairing this library's form controls with native
/// reference controls, for the tier 2 (HTML) rule checklist in
/// `docs/conformance-harness.md`.
///
/// `RadioGroup` and `Select` currently document `name` (and `RadioGroup`
/// also documents `required`) without rendering a submittable element for
/// either -- see `docs/plan.md` Phase 1. This fixture sets those props per
/// their documented API anyway, so it fails today and passes once Phase 1
/// lands.
#[component]
pub fn FormFixture() -> Element {
    // Wire the capturing `invalid` listener once, after the required-blocking
    // form mounts. This reads no reactive signals, so it only runs once.
    use_effect(move || {
        let _ = document::eval(&watch_invalid_js("form-required", "invalid-report"));
    });

    rsx! {
        div { class: Styles::dx_form_fixture,

            section { class: Styles::dx_form_section,
                h2 { "Entry list" }
                p { class: Styles::dx_form_hint,
                    "Every row pairs this library's control with a native reference control sharing a parallel "
                    code { "name" }
                    ". Submitting prevents navigation, builds "
                    code { "new FormData(form)" }
                    ", and renders one "
                    code { "name=value" }
                    " line per entry, in insertion order, below."
                }
                form {
                    id: "entries-form",
                    class: Styles::dx_form,
                    onsubmit: move |evt: FormEvent| {
                        evt.prevent_default();
                        let _ = document::eval(&read_form_data_js("entries-form", "form-result"));
                    },

                    div { class: Styles::dx_form_row,
                        div { class: Styles::dx_form_field,
                            label { r#for: "chk-lib", "Accept terms (library)" }
                            Checkbox {
                                id: "chk-lib",
                                name: "terms-lib",
                                value: "accepted",
                                default_checked: CheckboxState::Checked,
                            }
                        }
                        div { class: Styles::dx_form_field,
                            label { r#for: "chk-native", "Accept terms (native)" }
                            input {
                                id: "chk-native",
                                r#type: "checkbox",
                                name: "terms-native",
                                value: "accepted",
                                // `initial_checked` (-> `defaultChecked`) rather than `checked`
                                // (-> the live `.checked` IDL property, per dioxus-interpreter-js
                                // `set_attribute.ts`): only the former sets the `checked` content
                                // attribute the HTML reset algorithm reads, so this reference
                                // control actually restores on <form> reset like a plain static
                                // `<input checked>` would. See docs/conformance-harness.md rule 6.
                                initial_checked: true,
                            }
                        }
                    }

                    div { class: Styles::dx_form_row,
                        div { class: Styles::dx_form_field,
                            label { r#for: "chk-disabled-lib", "Promo opt-in, disabled (library)" }
                            Checkbox {
                                id: "chk-disabled-lib",
                                name: "promo-lib",
                                value: "yes",
                                disabled: true,
                                default_checked: CheckboxState::Checked,
                            }
                        }
                        div { class: Styles::dx_form_field,
                            label { r#for: "chk-disabled-native", "Promo opt-in, disabled (native)" }
                            input {
                                id: "chk-disabled-native",
                                r#type: "checkbox",
                                name: "promo-native",
                                value: "yes",
                                // See the `chk-native` comment above: `initial_checked`, not
                                // `checked`, so this reference matches static-HTML reset semantics.
                                initial_checked: true,
                                disabled: true,
                            }
                        }
                    }

                    div { class: Styles::dx_form_row,
                        div { class: Styles::dx_form_field,
                            label { r#for: "switch-lib", "Notifications (library switch)" }
                            Switch { id: "switch-lib", name: "notify-lib", value: "subscribed",
                                SwitchThumb {}
                            }
                        }
                        div { class: Styles::dx_form_field,
                            label { r#for: "switch-native", "Notifications (native reference)" }
                            input {
                                id: "switch-native",
                                r#type: "checkbox",
                                name: "notify-native",
                                value: "subscribed",
                            }
                        }
                    }

                    div { class: Styles::dx_form_row,
                        fieldset { class: Styles::dx_form_field,
                            legend { "Plan (library radio group)" }
                            RadioGroup { name: "plan-lib", aria_label: "Plan (library)",
                                label { r#for: "plan-lib-starter",
                                    RadioItem { id: "plan-lib-starter", value: "starter".to_string(), index: 0usize }
                                    "Starter"
                                }
                                label { r#for: "plan-lib-pro",
                                    RadioItem { id: "plan-lib-pro", value: "pro".to_string(), index: 1usize }
                                    "Pro"
                                }
                                label { r#for: "plan-lib-enterprise",
                                    RadioItem { id: "plan-lib-enterprise", value: "enterprise".to_string(), index: 2usize }
                                    "Enterprise"
                                }
                            }
                        }
                        fieldset { class: Styles::dx_form_field,
                            legend { "Plan (native reference)" }
                            label { r#for: "plan-native-starter",
                                input { id: "plan-native-starter", r#type: "radio", name: "plan-native", value: "starter" }
                                "Starter"
                            }
                            label { r#for: "plan-native-pro",
                                input { id: "plan-native-pro", r#type: "radio", name: "plan-native", value: "pro" }
                                "Pro"
                            }
                            label { r#for: "plan-native-enterprise",
                                input { id: "plan-native-enterprise", r#type: "radio", name: "plan-native", value: "enterprise" }
                                "Enterprise"
                            }
                        }
                    }

                    div { class: Styles::dx_form_row,
                        div { class: Styles::dx_form_field,
                            span { "Fruit (library select)" }
                            Select::<String> { name: "fruit-lib", default_value: Some("apple".to_string()),
                                SelectTrigger { aria_label: "Fruit (library)",
                                    SelectValue {}
                                }
                                SelectList { aria_label: "Fruit options (library)",
                                    SelectOption::<String> { index: 0usize, value: "apple", "Apple" }
                                    SelectOption::<String> { index: 1usize, value: "banana", "Banana" }
                                    SelectOption::<String> { index: 2usize, value: "cherry", "Cherry" }
                                    SelectOption::<String> { index: 3usize, value: "date", "Date" }
                                }
                            }
                        }
                        div { class: Styles::dx_form_field,
                            label { r#for: "fruit-native", "Fruit (native reference)" }
                            select { id: "fruit-native", name: "fruit-native",
                                option { value: "apple", "Apple" }
                                option { value: "banana", "Banana" }
                                option { value: "cherry", "Cherry" }
                                option { value: "date", "Date" }
                            }
                        }
                    }

                    div { class: Styles::dx_form_actions,
                        button { id: "entries-submit", r#type: "submit", "Submit" }
                        button { id: "entries-reset", r#type: "reset", "Reset" }
                    }
                }
                pre { id: "form-result", class: Styles::dx_form_result, "data-submit-count": "0" }
            }

            section { class: Styles::dx_form_section,
                h2 { "Required blocking" }
                p { class: Styles::dx_form_hint,
                    "Demonstrates that "
                    code { "required" }
                    " controls block submission: submitting with any required control unsatisfied refuses to submit, "
                    "outlines the offending control(s) in red and moves focus to the first one, and lists their names below."
                }
                p { class: Styles::dx_form_hint,
                    "Every library control below sets "
                    code { "required" }
                    " per its documented API, including "
                    code { "Select" }
                    " -- see docs/plan.md Phase 1.3. "
                    "Blocked submits fire "
                    code { "invalid" }
                    " on the offending controls, listed below in insertion order."
                }
                form {
                    id: "form-required",
                    class: Styles::dx_form,
                    onsubmit: move |evt: FormEvent| {
                        evt.prevent_default();
                        let _ = document::eval(&read_form_data_js("form-required", "required-result"));
                    },

                    div { class: Styles::dx_form_row,
                        div { class: Styles::dx_form_field,
                            label { r#for: "chk-required-lib", "Accept terms, required (library)" }
                            Checkbox { id: "chk-required-lib", name: "terms-required-lib", value: "accepted", required: true }
                        }
                        div { class: Styles::dx_form_field,
                            label { r#for: "chk-required-native", "Accept terms, required (native)" }
                            input {
                                id: "chk-required-native",
                                r#type: "checkbox",
                                name: "terms-required-native",
                                value: "accepted",
                                required: true,
                            }
                        }
                    }

                    div { class: Styles::dx_form_row,
                        div { class: Styles::dx_form_field,
                            label { r#for: "switch-required-lib", "Opt-in, required (library switch)" }
                            Switch { id: "switch-required-lib", name: "opt-in-required-lib", value: "subscribed", required: true,
                                SwitchThumb {}
                            }
                        }
                        div { class: Styles::dx_form_field,
                            label { r#for: "switch-required-native", "Opt-in, required (native reference)" }
                            input {
                                id: "switch-required-native",
                                r#type: "checkbox",
                                name: "opt-in-required-native",
                                value: "subscribed",
                                required: true,
                            }
                        }
                    }

                    div { class: Styles::dx_form_row,
                        fieldset { class: Styles::dx_form_field,
                            legend { "Tier, required (library radio group)" }
                            RadioGroup { name: "tier-required-lib", required: true, aria_label: "Tier, required (library)",
                                label { r#for: "tier-lib-small",
                                    RadioItem { id: "tier-lib-small", value: "small".to_string(), index: 0usize }
                                    "Small"
                                }
                                label { r#for: "tier-lib-medium",
                                    RadioItem { id: "tier-lib-medium", value: "medium".to_string(), index: 1usize }
                                    "Medium"
                                }
                                label { r#for: "tier-lib-large",
                                    RadioItem { id: "tier-lib-large", value: "large".to_string(), index: 2usize }
                                    "Large"
                                }
                            }
                        }
                        fieldset { class: Styles::dx_form_field,
                            legend { "Tier, required (native reference)" }
                            label { r#for: "tier-native-small",
                                input { id: "tier-native-small", r#type: "radio", name: "tier-required-native", value: "small", required: true }
                                "Small"
                            }
                            label { r#for: "tier-native-medium",
                                input { id: "tier-native-medium", r#type: "radio", name: "tier-required-native", value: "medium", required: true }
                                "Medium"
                            }
                            label { r#for: "tier-native-large",
                                input { id: "tier-native-large", r#type: "radio", name: "tier-required-native", value: "large", required: true }
                                "Large"
                            }
                        }
                    }

                    div { class: Styles::dx_form_row,
                        div { class: Styles::dx_form_field,
                            span { "Fruit, required (library select)" }
                            Select::<String> { name: "fruit-required-lib", required: true,
                                SelectTrigger { aria_label: "Fruit, required (library)",
                                    SelectValue { placeholder: "Choose a fruit" }
                                }
                                SelectList { aria_label: "Fruit options, required (library)",
                                    SelectOption::<String> { index: 0usize, value: "apple", "Apple" }
                                    SelectOption::<String> { index: 1usize, value: "banana", "Banana" }
                                    SelectOption::<String> { index: 2usize, value: "cherry", "Cherry" }
                                }
                            }
                        }
                        div { class: Styles::dx_form_field,
                            label { r#for: "fruit-required-native", "Fruit, required (native reference)" }
                            select { id: "fruit-required-native", name: "fruit-required-native", required: true,
                                option { value: "", selected: true, "Choose a fruit" }
                                option { value: "apple", "Apple" }
                                option { value: "banana", "Banana" }
                                option { value: "cherry", "Cherry" }
                            }
                        }
                    }

                    div { class: Styles::dx_form_actions,
                        button { id: "required-submit", r#type: "submit", "Submit" }
                        button { id: "required-reset", r#type: "reset", "Reset" }
                    }
                }
                pre { id: "invalid-report", class: Styles::dx_form_result, "data-invalid-count": "0" }
                pre { id: "required-result", class: Styles::dx_form_result, "data-submit-count": "0" }
            }
        }
    }
}

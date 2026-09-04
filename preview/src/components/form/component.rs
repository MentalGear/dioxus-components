// Composition rule for this fixture (and every other preview page): compose
// ONLY themed wrappers from `crate::components::*`, never a bare
// `dioxus_primitives::` component directly. `preview/src/components/<name>/
// component.rs` is the one place per component where the raw primitive is
// imported and given its `css_module` theme class (e.g. `.dx-checkbox` in
// `../checkbox/style.css`, `.dx-switch` in `../switch/style.css`); every
// *other* file, this one included, is expected to reach for that themed
// wrapper the same way `components/slider/variants/dynamic_range/mod.rs`
// imports `crate::components::switch::Switch` or `components/card/variants/
// main/mod.rs` imports `crate::components::{button, input, label}`. This
// file being itself a `component.rs` does not exempt it: unlike the ~45
// wrapper files, there is no `dioxus_primitives::form` primitive for it to
// wrap -- it is a fixture composing *other* components, so the rule applies
// to it exactly as it does to a variant demo. `scripts/check-preview-
// composition.sh` enforces this over `preview/src`; see
// `docs/preview-composition.md` for the full rule and rationale.
//
// Two confirmed incidents of violating it, both by construction (execution-
// verified, not just plausible):
//
// Item 1 fix (2026-09-01, live-site report): "checkboxes and radio buttons
// collapse visually when toggled on/off." This fixture imported the *raw*
// `dioxus_primitives::checkbox::{Checkbox, CheckboxIndicator}` and
// `dioxus_primitives::radio_group::{RadioGroup, RadioItem}` primitives
// directly. Without the theme class, `Checkbox`'s `<button>` has no explicit
// `width`/`height` at all -- its box is sized purely by its content -- and
// `CheckboxIndicator` renders its children (the checkmark) *only* while
// checked (see `primitives/src/checkbox.rs`'s doc: "children will only be
// rendered when the checkbox is checked"), so the button visibly grows for
// the glyph when checked and collapses back to a zero-content box when
// unchecked. `RadioItem`'s entire visual (the circle) is a
// `.dx-radio-item::before` pseudo-element that plain
// `dioxus_primitives::radio_group::RadioItem` never gets without that class,
// so its `<button>` collapses to whatever a bare, empty `<button>` renders as
// in every state. Confirmed by measurement against `/component/?name=checkbox`
// and `/component/?name=radio_group` (both themed, both stable-sized in every
// state) vs. this fixture pre-fix (both collapsing). Fixed here, not in
// `primitives/src/checkbox.rs` or `radio_group.rs`: the *conditional-children*
// behavior of `CheckboxIndicator` is correct, documented API (a consumer's
// indicator content need not be a fixed-size glyph), and `RadioItem`'s
// primitive renders no built-in visual by design (it is a headless
// primitive) -- the actual defect was this one fixture forgetting the theme
// layer every sibling fixture already applies.
//
// Item 2 fix (2026-09-01, live-site report): "Notifications (library switch)"
// and "Opt-in, required (library switch)" render visually collapsed, the same
// way. This fixture also imported the *raw* `dioxus_primitives::switch::
// {Switch, SwitchThumb}` and `dioxus_primitives::select::{Select, SelectList,
// SelectOption, SelectTrigger, SelectValue}` primitives directly, the same
// root cause as Item 1: `Switch`'s `<button>` gets no `width`/`height`
// without `.dx-switch` (`../switch/style.css`: `all: unset; width: 2rem;
// height: 1.15rem; ...`), so it collapses to an empty button's intrinsic
// size. Confirmed by an SSR test below asserting `#switch-lib` carries a
// `dx-switch`-prefixed class, red before this fix (no `class` attribute on
// that button at all) and green after. Fixed here by switching to
// `crate::components::switch::Switch` (which renders its own themed
// `SwitchThumb` internally -- the explicit `SwitchThumb {}` child this
// fixture previously passed is gone) and `crate::components::select::*`.
use crate::components::checkbox::Checkbox;
use crate::components::radio_group::{RadioGroup, RadioItem};
use crate::components::select::{Select, SelectOption};
use crate::components::switch::Switch;
use dioxus::prelude::*;
use dioxus_primitives::checkbox::CheckboxState;

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

            // `[class*="dx-form-field"]` rather than `.dx-form-field`: this is
            // deliberately loose (docs/backlog.md row 32 dropped `#[css_module]`'s
            // hashing, so the rendered class is now the plain `dx-form-field`
            // literal and an exact selector would work too), kept this way so it
            // keeps matching regardless of any extra classes a future edit appends
            // alongside it.
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
        document::Link { rel: "stylesheet", href: asset!("/src/components/form/style.css") }
        div { class: "dx-form-fixture",

            section { class: "dx-form-section",
                h2 { "Entry list" }
                p { class: "dx-form-hint",
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
                    class: "dx-form",
                    onsubmit: move |evt: FormEvent| {
                        evt.prevent_default();
                        let _ = document::eval(&read_form_data_js("entries-form", "form-result"));
                    },

                    div { class: "dx-form-row",
                        div { class: "dx-form-field",
                            label { r#for: "chk-lib", "Accept terms (library)" }
                            Checkbox {
                                id: "chk-lib",
                                name: "terms-lib",
                                value: "accepted",
                                default_checked: CheckboxState::Checked,
                            }
                        }
                        div { class: "dx-form-field",
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

                    div { class: "dx-form-row",
                        div { class: "dx-form-field",
                            label { r#for: "chk-disabled-lib", "Promo opt-in, disabled (library)" }
                            Checkbox {
                                id: "chk-disabled-lib",
                                name: "promo-lib",
                                value: "yes",
                                disabled: true,
                                default_checked: CheckboxState::Checked,
                            }
                        }
                        div { class: "dx-form-field",
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

                    div { class: "dx-form-row",
                        div { class: "dx-form-field",
                            label { r#for: "switch-lib", "Notifications (library switch)" }
                            Switch { id: "switch-lib", name: "notify-lib", value: "subscribed" }
                        }
                        div { class: "dx-form-field",
                            label { r#for: "switch-native", "Notifications (native reference)" }
                            input {
                                id: "switch-native",
                                r#type: "checkbox",
                                name: "notify-native",
                                value: "subscribed",
                            }
                        }
                    }

                    div { class: "dx-form-row",
                        fieldset { class: "dx-form-field",
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
                        fieldset { class: "dx-form-field",
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

                    div { class: "dx-form-row",
                        div { class: "dx-form-field",
                            span { "Fruit (library select)" }
                            Select::<String> {
                                name: "fruit-lib",
                                default_value: Some("apple".to_string()),
                                trigger_aria_label: "Fruit (library)",
                                list_aria_label: "Fruit options (library)",
                                SelectOption::<String> { index: 0usize, value: "apple", "Apple" }
                                SelectOption::<String> { index: 1usize, value: "banana", "Banana" }
                                SelectOption::<String> { index: 2usize, value: "cherry", "Cherry" }
                                SelectOption::<String> { index: 3usize, value: "date", "Date" }
                            }
                        }
                        div { class: "dx-form-field",
                            label { r#for: "fruit-native", "Fruit (native reference)" }
                            select { id: "fruit-native", name: "fruit-native",
                                option { value: "apple", "Apple" }
                                option { value: "banana", "Banana" }
                                option { value: "cherry", "Cherry" }
                                option { value: "date", "Date" }
                            }
                        }
                    }

                    div { class: "dx-form-actions",
                        button { id: "entries-submit", r#type: "submit", "Submit" }
                        button { id: "entries-reset", r#type: "reset", "Reset" }
                    }
                }
                pre { id: "form-result", class: "dx-form-result", "data-submit-count": "0" }
            }

            section { class: "dx-form-section",
                h2 { "Required blocking" }
                p { class: "dx-form-hint",
                    "Demonstrates that "
                    code { "required" }
                    " controls block submission: submitting with any required control unsatisfied refuses to submit, "
                    "outlines the offending control(s) in red and moves focus to the first one, and lists their names below."
                }
                p { class: "dx-form-hint",
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
                    class: "dx-form",
                    onsubmit: move |evt: FormEvent| {
                        evt.prevent_default();
                        let _ = document::eval(&read_form_data_js("form-required", "required-result"));
                    },

                    div { class: "dx-form-row",
                        div { class: "dx-form-field",
                            label { r#for: "chk-required-lib", "Accept terms, required (library)" }
                            Checkbox { id: "chk-required-lib", name: "terms-required-lib", value: "accepted", required: true }
                        }
                        div { class: "dx-form-field",
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

                    div { class: "dx-form-row",
                        div { class: "dx-form-field",
                            label { r#for: "switch-required-lib", "Opt-in, required (library switch)" }
                            Switch {
                                id: "switch-required-lib",
                                name: "opt-in-required-lib",
                                value: "subscribed",
                                required: true,
                            }
                        }
                        div { class: "dx-form-field",
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

                    div { class: "dx-form-row",
                        fieldset { class: "dx-form-field",
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
                        fieldset { class: "dx-form-field",
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

                    div { class: "dx-form-row",
                        div { class: "dx-form-field",
                            span { "Fruit, required (library select)" }
                            Select::<String> {
                                name: "fruit-required-lib",
                                required: true,
                                trigger_aria_label: "Fruit, required (library)",
                                list_aria_label: "Fruit options, required (library)",
                                placeholder: "Choose a fruit",
                                SelectOption::<String> { index: 0usize, value: "apple", "Apple" }
                                SelectOption::<String> { index: 1usize, value: "banana", "Banana" }
                                SelectOption::<String> { index: 2usize, value: "cherry", "Cherry" }
                            }
                        }
                        div { class: "dx-form-field",
                            label { r#for: "fruit-required-native", "Fruit, required (native reference)" }
                            select { id: "fruit-required-native", name: "fruit-required-native", required: true,
                                option { value: "", selected: true, "Choose a fruit" }
                                option { value: "apple", "Apple" }
                                option { value: "banana", "Banana" }
                                option { value: "cherry", "Cherry" }
                            }
                        }
                    }

                    div { class: "dx-form-actions",
                        button { id: "required-submit", r#type: "submit", "Submit" }
                        button { id: "required-reset", r#type: "reset", "Reset" }
                    }
                }
                pre { id: "invalid-report", class: "dx-form-result", "data-invalid-count": "0" }
                pre { id: "required-result", class: "dx-form-result", "data-submit-count": "0" }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Returns the substring of `html` for the single opening tag containing
    /// `needle` (e.g. `id="switch-lib"`) -- from the tag's preceding `<` to
    /// its closing `>`. Panics with the full document on a miss, so a
    /// failure is easy to diagnose.
    fn tag_containing<'a>(html: &'a str, needle: &str) -> &'a str {
        let at = html
            .find(needle)
            .unwrap_or_else(|| panic!("expected to find `{needle}` in:\n{html}"));
        let start = html[..at].rfind('<').expect("tag has an opening `<`");
        let end = at + html[at..].find('>').expect("tag has a closing `>`");
        &html[start..=end]
    }

    /// Returns the value of `attr="..."` inside `tag`.
    fn attr_value<'a>(tag: &'a str, attr: &str) -> &'a str {
        let pat = format!("{attr}=\"");
        let start = tag
            .find(&pat)
            .unwrap_or_else(|| panic!("expected `{attr}` attribute in tag: {tag}"))
            + pat.len();
        let end = start + tag[start..].find('"').expect("attribute value is quoted");
        &tag[start..end]
    }

    /// Item 1 fix (2026-09-01, live-site report): a themed control's class
    /// is what actually carries its fixed size (see this file's header
    /// comment, and `../switch/style.css`'s `.dx-switch { all: unset; width:
    /// 2rem; ...}`). `#[css_module]` hash-suffixes every class it generates
    /// (confirmed by execution -- the rendered class is `dx-switch-<8 hex
    /// chars>`, never the literal `dx-switch`), so this matches on the
    /// prefix rather than the exact class name.
    #[test]
    fn switch_lib_carries_the_themed_dx_switch_class() {
        let mut dom = VirtualDom::new(FormFixture);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        let tag = tag_containing(&html, "id=\"switch-lib\"");
        let class = attr_value(tag, "class");
        // docs/backlog.md row 32: this used to look for a `dx-switch-<hash>`
        // token, because `#[css_module]` appended a scope hash to every class.
        // With the hashing dropped the themed class is the plain `dx-switch`,
        // so the old prefix match (`starts_with("dx-switch-")`, note the
        // trailing hyphen) no longer matches anything. Asserting the exact
        // token is a stricter check than the prefix ever was: it would now
        // also catch a stray re-hashed class coming back.
        assert!(
            class.split_whitespace().any(|token| token == "dx-switch"),
            "expected the unhashed `dx-switch` class on #switch-lib (see \
             crate::components::switch::Switch), got class=\"{class}\" from tag: {tag}"
        );
    }
}

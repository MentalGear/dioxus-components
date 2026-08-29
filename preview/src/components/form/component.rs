use dioxus::prelude::*;
use dioxus_primitives::checkbox::{Checkbox, CheckboxIndicator, CheckboxState};
use dioxus_primitives::radio_group::{RadioGroup, RadioItem};
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
fn watch_invalid_js(form_id: &str, report_id: &str) -> String {
    format!(
        r#"
        const form = document.getElementById('{form_id}');
        const report = document.getElementById('{report_id}');
        if (form && report && !form.dataset.dxInvalidWired) {{
            form.dataset.dxInvalidWired = '1';
            let pending = new Set();
            let scheduled = false;
            form.addEventListener('invalid', (event) => {{
                const target = event.target;
                pending.add(target.name || target.id || target.tagName);
                if (!scheduled) {{
                    scheduled = true;
                    setTimeout(() => {{
                        report.textContent = Array.from(pending).join('\n');
                        const count = parseInt(report.getAttribute('data-invalid-count') || '0', 10);
                        report.setAttribute('data-invalid-count', String(count + 1));
                        pending = new Set();
                        scheduled = false;
                    }}, 0);
                }}
            }}, true);
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
                                CheckboxIndicator { "\u{2713}" }
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
                                CheckboxIndicator { "\u{2713}" }
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
                            Checkbox { id: "chk-required-lib", name: "terms-required-lib", value: "accepted", required: true,
                                CheckboxIndicator { "\u{2713}" }
                            }
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

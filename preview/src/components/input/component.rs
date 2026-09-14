use dioxus::prelude::*;
use dioxus_primitives::{dioxus_attributes::attributes, merge_attributes};

/// Wraps a plain `<input>` in this crate's themed styling.
///
/// The base `"dx-input"` class is combined with any caller-supplied
/// `class` (e.g. `InputGroupInput`'s `"dx-input-group-control"`) via
/// [`merge_attributes`] rather than a bare `class: "dx-input"` field
/// followed by `..attributes` -- the latter lets Dioxus's normal
/// "later attribute wins" rule for a duplicate name silently *replace*
/// `"dx-input"` instead of appending to it, dropping this component's own
/// UA-style reset (border/appearance/background) for any caller that
/// passes its own `class`. `merge_attributes` special-cases `class` to
/// concatenate (space-joined) instead, which is what every caller here
/// actually wants: their own class alongside `"dx-input"`, never instead
/// of it. See `preview/src/components/input_group/component.rs` for the
/// consumer this was found through: without this, `InputGroupInput`'s
/// wrapped `<input>` rendered with *only* `"dx-input-group-control"`
/// (`"dx-input"` silently dropped), so none of `"dx-input"`'s own
/// border/background reset applied and the browser's native default
/// input chrome showed through inside `InputGroup`'s own border --  a
/// visible double-border/"pill within a pill" seam.
#[component]
pub fn Input(
    oninput: Option<EventHandler<FormEvent>>,
    onchange: Option<EventHandler<FormEvent>>,
    oninvalid: Option<EventHandler<FormEvent>>,
    onselect: Option<EventHandler<SelectionEvent>>,
    onselectionchange: Option<EventHandler<SelectionEvent>>,
    onfocus: Option<EventHandler<FocusEvent>>,
    onblur: Option<EventHandler<FocusEvent>>,
    onfocusin: Option<EventHandler<FocusEvent>>,
    onfocusout: Option<EventHandler<FocusEvent>>,
    onkeydown: Option<EventHandler<KeyboardEvent>>,
    onkeypress: Option<EventHandler<KeyboardEvent>>,
    onkeyup: Option<EventHandler<KeyboardEvent>>,
    onwheel: Option<EventHandler<WheelEvent>>,
    oncompositionstart: Option<EventHandler<CompositionEvent>>,
    oncompositionupdate: Option<EventHandler<CompositionEvent>>,
    oncompositionend: Option<EventHandler<CompositionEvent>>,
    oncopy: Option<EventHandler<ClipboardEvent>>,
    oncut: Option<EventHandler<ClipboardEvent>>,
    onpaste: Option<EventHandler<ClipboardEvent>>,
    #[props(extends=GlobalAttributes)]
    #[props(extends=input)]
    attributes: Vec<Attribute>,
    children: Element,
) -> Element {
    let base = attributes!(input { class: "dx-input" });
    let merged = merge_attributes(vec![base, attributes]);

    rsx! {
        document::Link { rel: "stylesheet", href: asset!("/src/components/input/style.css") }
        input {
            oninput: move |e| _ = oninput.map(|callback| callback(e)),
            onchange: move |e| _ = onchange.map(|callback| callback(e)),
            oninvalid: move |e| _ = oninvalid.map(|callback| callback(e)),
            onselect: move |e| _ = onselect.map(|callback| callback(e)),
            onselectionchange: move |e| _ = onselectionchange.map(|callback| callback(e)),
            onfocus: move |e| _ = onfocus.map(|callback| callback(e)),
            onblur: move |e| _ = onblur.map(|callback| callback(e)),
            onfocusin: move |e| _ = onfocusin.map(|callback| callback(e)),
            onfocusout: move |e| _ = onfocusout.map(|callback| callback(e)),
            onkeydown: move |e| _ = onkeydown.map(|callback| callback(e)),
            onkeypress: move |e| _ = onkeypress.map(|callback| callback(e)),
            onkeyup: move |e| _ = onkeyup.map(|callback| callback(e)),
            onwheel: move |e| _ = onwheel.map(|callback| callback(e)),
            oncompositionstart: move |e| _ = oncompositionstart.map(|callback| callback(e)),
            oncompositionupdate: move |e| _ = oncompositionupdate.map(|callback| callback(e)),
            oncompositionend: move |e| _ = oncompositionend.map(|callback| callback(e)),
            oncopy: move |e| _ = oncopy.map(|callback| callback(e)),
            oncut: move |e| _ = oncut.map(|callback| callback(e)),
            onpaste: move |e| _ = onpaste.map(|callback| callback(e)),
            ..merged,
            {children}
        }
    }
}

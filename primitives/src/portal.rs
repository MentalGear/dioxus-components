//! A portal renders content somewhere other than where it's declared in the
//! component tree -- e.g. an overlay's content mounted next to the document
//! root while the trigger that opens it stays wherever it's actually used in
//! the app's own markup.
//!
//! Call [`use_portal`] once to get a [`PortalId`], then pass that id to one
//! [`PortalIn`] (wherever you want to *declare* the content) and one
//! [`PortalOut`] (wherever you want it to actually *render*). `PortalIn`
//! itself renders nothing; `PortalOut` renders whatever the matching
//! `PortalIn` last passed it as children.
//!
//! ## Example
//!
//! ```rust
//! use dioxus::prelude::*;
//! use dioxus_primitives::portal::{use_portal, PortalIn, PortalOut};
//!
//! #[component]
//! fn Demo() -> Element {
//!     let portal = use_portal();
//!
//!     rsx! {
//!         // Declared here...
//!         PortalIn { portal, "Hello from the portal!" }
//!
//!         // ...but rendered here.
//!         div { id: "portal-target",
//!             PortalOut { portal }
//!         }
//!     }
//! }
//! ```

use crate::dioxus_core::provide_root_context;
use dioxus::prelude::*;
use std::collections::HashMap;

use crate::use_effect_cleanup;

/// A handle identifying one portal, returned by [`use_portal`]. Connects a
/// [`PortalIn`] to the [`PortalOut`] that renders its children -- see the
/// [module docs](self) for a full example.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PortalId(usize);

#[derive(Clone, Copy, PartialEq)]
struct PortalCtx {
    portals: Signal<HashMap<usize, Signal<Element>>>,
}

/// Create a portal and return its [`PortalId`].
///
/// Pass the returned id to one [`PortalIn`] (wherever the portal's content
/// is produced) and one [`PortalOut`] (wherever it should actually render)
/// -- see the [module docs](self) for a full example. The first call
/// anywhere in the app registers a document-root context the portal's slot
/// lives in; the slot is cleaned up again when the component that called
/// `use_portal` unmounts.
pub fn use_portal() -> PortalId {
    static NEXT_ID: GlobalSignal<usize> = Signal::global(|| 0);

    let (sig, id) = use_hook(|| {
        let mut next_id = NEXT_ID.write();
        let id = *next_id;
        *next_id += 1;

        let mut ctx = match try_consume_context::<PortalCtx>() {
            Some(ctx) => ctx,
            None => {
                let portals = Signal::new_in_scope(HashMap::new(), ScopeId::ROOT);
                let ctx = PortalCtx { portals };
                provide_root_context(ctx)
            }
        };

        let sig = Signal::new_in_scope(VNode::empty(), ScopeId::ROOT);
        ctx.portals.write().insert(id, sig);

        (sig, PortalId(id))
    });

    // Cleanup the portal.
    use_effect_cleanup(move || {
        let mut ctx = consume_context::<PortalCtx>();
        ctx.portals.write().remove(&id.0);
        sig.manually_drop();
    });

    id
}

/// Declares a portal's content. Renders nothing itself -- its `children` are
/// instead rendered by the [`PortalOut`] with the same [`PortalId`],
/// wherever that is in the tree. See the [module docs](self) for a full
/// example.
#[component]
pub fn PortalIn(portal: PortalId, children: Element) -> Element {
    if let Some(mut ctx) = try_use_context::<PortalCtx>() {
        let mut portals = ctx.portals.write();
        if let Some(portal) = portals.get_mut(&portal.0) {
            portal.set(children);
        }
    }

    rsx! {}
}

/// Renders whatever the [`PortalIn`] with the same [`PortalId`] last passed
/// it as children (or nothing, if that `PortalIn` hasn't rendered yet). See
/// the [module docs](self) for a full example.
#[component]
pub fn PortalOut(portal: PortalId) -> Element {
    if let Some(ctx) = try_use_context::<PortalCtx>() {
        if let Some(children) = ctx.portals.peek().get(&portal.0) {
            return rsx! {
                {*children}
            };
        }
    }

    rsx! {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[component]
    fn PortalDemo() -> Element {
        let portal = use_portal();

        rsx! {
            div { id: "in-place",
                PortalIn { portal, "portal content" }
            }
            div { id: "out-place",
                PortalOut { portal }
            }
        }
    }

    #[test]
    fn portal_in_children_render_at_portal_out_site() {
        let mut dom = VirtualDom::new(PortalDemo);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        // `PortalIn` itself renders nothing -- if "portal content" shows up
        // anywhere, it can only be because `PortalOut` rendered it.
        assert!(html.contains("portal content"));
        // It must render inside `PortalOut`'s own wrapper (`#out-place`),
        // not inside `PortalIn`'s (`#in-place`), proving the content moved.
        assert!(html.contains(r#"<div id="in-place"></div>"#));
        assert!(html.contains(r#"<div id="out-place">portal content</div>"#));
    }

    #[test]
    fn portal_out_renders_nothing_before_portal_in_runs() {
        #[component]
        fn OutOnly() -> Element {
            let portal = use_portal();
            rsx! {
                div { id: "out-place",
                    PortalOut { portal }
                }
            }
        }

        let mut dom = VirtualDom::new(OutOnly);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        assert!(html.contains(r#"<div id="out-place"></div>"#));
    }
}

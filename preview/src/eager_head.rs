//! Head elements that cannot be lost to a subtree that is torn down the turn it first renders.
//!
//! # The defect this exists for
//!
//! Every component wrapper ships its CSS as `document::Link { rel: "stylesheet", href: asset!(..) }`.
//! In Dioxus 0.7.9 that is two steps, and they are not atomic:
//!
//! 1. at RENDER, `Link`'s `use_hook` records `href|rel` in a de-duplication set kept in the ROOT
//!    context (`dioxus-document/src/elements/link.rs:123-137`, `elements/mod.rs`
//!    `DeduplicationContext`) and decides whether this call should insert at all;
//! 2. the `<link>` itself is appended later, from `queue_effect` on the Link's own scope
//!    (`dioxus-web/src/document.rs:160-164`).
//!
//! `Runtime::remove_scope` drops a removed scope's queued effects (`dioxus-core/src/runtime.rs:190-193`),
//! and effects only run once no scope is dirty (`virtual_dom.rs:529-535` returns the moment an effect
//! dirties one). So a subtree that is removed in the turn it first rendered never gets its links
//! inserted, yet the set says they are present -- and every LATER `document::Link` for the same href,
//! on the page that replaced it, is skipped as a duplicate. Nothing errors; the component simply
//! renders with no CSS.
//!
//! The condition, exactly: *an ancestor's first-run `use_effect` dirties a scope that unmounts the
//! subtree* (effects run parents-first, `scheduler.rs` `ScopeOrder`). The legacy
//! `/component/?name=X` shell is that: it renders `Navbar`, and its own `use_effect` calls
//! `nav.replace(ComponentDemoPath)`. The header's `LanguageSelect` stylesheet and the theme picker's
//! `Popover` stylesheet were recorded and never inserted, so `/component/X/` rendered them unstyled
//! (the picker's panel computed `border 3px / radius 0` instead of `1px / 10px`).
//!
//! It is a CLASS, not an instance (CLAUDE.md "when the same problem shows up more than once"): it is
//! what dropped `main.css`/`dx-components-theme.css` after `ComponentBlockDemo`'s redirect (backlog
//! row 46, whose fix moved `GlobalHead` into `AppLayout` -- sound for those four sheets, but it left
//! every other component's link exposed), it is what once left a page with no `Button` stylesheet, and
//! it hit `language-select-*.css` long before the theme picker existed.
//!
//! # The construction
//!
//! Put the insertion back where the de-duplication happens. [`EagerHeadDocument`] wraps whatever
//! `Document` the platform provided and leaves `create_link`/`create_meta`/`create_script`/
//! `create_style` at the trait's own defaults, which run `create_element_in_head` through `eval` --
//! and `WebDocument::eval` calls its JS synchronously (`dioxus-web/src/document.rs`
//! `WebEvaluator::create`). The `<link>` is therefore in `<head>` before `Link` returns, in the same
//! breath that the set records it; no later teardown can separate the two. It is installed once, in
//! `App`, so it covers EVERY component -- the shell, all of `preview/src/components`, and the
//! installable wrappers `dx components add` copies out -- without any of them knowing.
//!
//! `create_head_component` is delegated untouched: it is the fullstack hydration handshake (a head
//! element the server already wrote to the HTML must not be inserted again), and the SSG pages depend
//! on it. The server never installs this (`App` gates it on `not(feature = "server")`), because there
//! `ServerDocument` must keep collecting the head for the SSR `<head>`.
//!
//! `set_title` is delegated as is: a title is not de-duplicated, so a lost one is replaced by the next.
//!
//! Guards: `scripts/check-eager-head-document.sh` (static: the wrapper exists, never uses
//! `queue_effect`, and `App` installs it) and `playwright/oracle/tier2-html/stylesheets-present.spec.ts`
//! (behavioural: every rendered component's stylesheet is applied on fresh loads of every route type).

use std::rc::Rc;

use dioxus::document::{self, Document, Eval};
use dioxus::prelude::*;

/// A [`Document`] that inserts head elements synchronously. See the module docs.
pub struct EagerHeadDocument {
    inner: Rc<dyn Document>,
}

impl EagerHeadDocument {
    pub fn new(inner: Rc<dyn Document>) -> Self {
        Self { inner }
    }
}

impl Document for EagerHeadDocument {
    fn eval(&self, js: String) -> Eval {
        self.inner.eval(js)
    }

    fn set_title(&self, title: String) {
        self.inner.set_title(title);
    }

    // The fullstack hydration handshake: `false` for a head element the server already wrote.
    fn create_head_component(&self) -> bool {
        self.inner.create_head_component()
    }

    // `create_meta`, `create_script`, `create_style` and `create_link` are deliberately NOT overridden:
    // the trait defaults insert through `eval` immediately, which is the whole point. Overriding them to
    // delegate to `inner` would bring back the web document's `queue_effect`.
}

/// Install [`EagerHeadDocument`] over the platform's document for this app. Call once, first, in `App`
/// (before anything that can render a `document::Link`), on every client build.
pub fn use_eager_head_document() {
    use_hook(|| {
        let inner = document::document();
        provide_context(Rc::new(EagerHeadDocument::new(inner)) as Rc<dyn Document>)
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    use dioxus::document::{LinkProps, NoOpDocument};

    /// Records what the platform document is asked to do. Its `create_link` stands in for
    /// `WebDocument::create_link`, the `queue_effect` path the wrapper must bypass.
    #[derive(Default)]
    struct Platform {
        evals: Rc<RefCell<Vec<String>>>,
        deferred_links: Rc<Cell<usize>>,
        written_by_server: bool,
    }

    impl Document for Platform {
        fn eval(&self, js: String) -> Eval {
            self.evals.borrow_mut().push(js);
            NoOpDocument.eval(String::new())
        }

        fn create_link(&self, _: LinkProps) {
            self.deferred_links.set(self.deferred_links.get() + 1);
        }

        fn create_head_component(&self) -> bool {
            !self.written_by_server
        }
    }

    fn stylesheet() -> LinkProps {
        LinkProps::builder()
            .rel("stylesheet".to_string())
            .href("/assets/popover.css".to_string())
            .build()
    }

    #[test]
    fn a_link_is_inserted_now_not_through_the_platforms_deferred_path() {
        let platform = Platform::default();
        let (evals, deferred) = (platform.evals.clone(), platform.deferred_links.clone());
        let doc = EagerHeadDocument::new(Rc::new(platform));

        doc.create_link(stylesheet());

        assert_eq!(
            deferred.get(),
            0,
            "must not reach the platform's queued create_link"
        );
        let evals = evals.borrow();
        assert_eq!(evals.len(), 1, "exactly one synchronous head insertion");
        assert!(evals[0].contains("createElementInHead"), "{}", evals[0]);
        assert!(evals[0].contains("/assets/popover.css"), "{}", evals[0]);
    }

    #[test]
    fn the_hydration_handshake_is_the_platforms() {
        let fresh = EagerHeadDocument::new(Rc::new(Platform::default()));
        assert!(fresh.create_head_component(), "client-only render: insert");

        let hydrating = EagerHeadDocument::new(Rc::new(Platform {
            written_by_server: true,
            ..Platform::default()
        }));
        assert!(
            !hydrating.create_head_component(),
            "a head element the server already wrote must not be inserted again"
        );
    }
}

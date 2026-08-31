use crate::dioxus_core::{queue_effect, Runtime};
use dioxus::html::geometry::ClientPoint;
use dioxus::prelude::*;

#[derive(Debug)]
struct Pointer {
    id: i32,
    position: ClientPoint,
}

static POINTERS: GlobalSignal<Vec<Pointer>> = Global::new(|| {
    let runtime = Runtime::current();
    queue_effect(move || {
        runtime.spawn(ScopeId::ROOT, async move {
            let mut pointer_updates = dioxus::document::eval(
                // clientX/clientY (not pageX/pageY) must match element handlers
                // that store `evt.client_coordinates()` and viewport-relative
                // rects from getBoundingClientRect.
                "window.addEventListener('pointerdown', (e) => {
                    dioxus.send(['down', [e.pointerId, e.clientX, e.clientY]]);
                });
                window.addEventListener('pointermove', (e) => {
                    dioxus.send(['move', [e.pointerId, e.clientX, e.clientY]]);
                });
                window.addEventListener('pointerup', (e) => {
                    dioxus.send(['up', [e.pointerId, e.clientX, e.clientY]]);
                });
                window.addEventListener('pointercancel', (e) => {
                    dioxus.send(['up', [e.pointerId, e.clientX, e.clientY]]);
                });",
            );

            while let Ok((event_type, (pointer_id, x, y))) =
                pointer_updates.recv::<(String, (i32, f64, f64))>().await
            {
                let position = ClientPoint::new(x, y);

                match event_type.as_str() {
                    "down" => add_pointer(pointer_id, position),
                    "move" => update_pointer(pointer_id, position),
                    "up" => remove_pointer(pointer_id),
                    _ => {}
                }
            }
        });
    });

    Vec::new()
});

pub(crate) fn track_pointer_down(pointer_id: i32, position: ClientPoint) {
    add_pointer(pointer_id, position);
}

pub(crate) fn pointer_position(pointer_id: i32) -> Option<ClientPoint> {
    POINTERS
        .read()
        .iter()
        .find(|pointer| pointer.id == pointer_id)
        .map(|pointer| pointer.position)
}

fn add_pointer(pointer_id: i32, position: ClientPoint) {
    let mut pointers = POINTERS.write();
    upsert_pointer(&mut pointers, pointer_id, position);
}

fn upsert_pointer(pointers: &mut Vec<Pointer>, pointer_id: i32, position: ClientPoint) {
    if let Some(pointer) = pointers.iter_mut().find(|pointer| pointer.id == pointer_id) {
        pointer.position = position;
    } else {
        pointers.push(Pointer {
            id: pointer_id,
            position,
        });
    }
}

fn update_pointer(pointer_id: i32, position: ClientPoint) {
    if let Some(pointer) = POINTERS
        .write()
        .iter_mut()
        .find(|pointer| pointer.id == pointer_id)
    {
        pointer.position = position;
    }
}

fn remove_pointer(pointer_id: i32) {
    POINTERS.write().retain(|pointer| pointer.id != pointer_id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use std::rc::Rc;

    #[test]
    fn upsert_pointer_updates_existing_pointer() {
        let mut pointers = vec![Pointer {
            id: 1,
            position: ClientPoint::new(10.0, 20.0),
        }];

        upsert_pointer(&mut pointers, 1, ClientPoint::new(30.0, 40.0));

        assert_eq!(pointers.len(), 1);
        assert_eq!(pointers[0].position, ClientPoint::new(30.0, 40.0));
    }

    /// Run a closure inside a Dioxus runtime context. The `POINTERS` `GlobalSignal` (and the
    /// `queue_effect`/`Runtime::current` machinery its initializer touches) require a live
    /// runtime, so its public API can only be exercised this way (mirrors
    /// `virtual/virtualizer.rs`'s `with_runtime`).
    fn with_runtime(f: impl Fn() + 'static) {
        let result = Rc::new(Cell::new(false));
        let result2 = result.clone();
        let test_fn = Rc::new(f);
        let mut dom = VirtualDom::new_with_props(
            |props: TestHarnessProps| {
                (props.test_fn)();
                props.result.set(true);
                rsx! { div {} }
            },
            TestHarnessProps {
                test_fn,
                result: result2,
            },
        );
        dom.rebuild_in_place();
        assert!(result.get(), "Test component did not run");
    }

    #[derive(Clone, Props)]
    struct TestHarnessProps {
        test_fn: Rc<dyn Fn()>,
        result: Rc<Cell<bool>>,
    }

    impl PartialEq for TestHarnessProps {
        fn eq(&self, _: &Self) -> bool {
            true
        }
    }

    /// Each test picks pointer ids not used by other tests in this file: `POINTERS` is a
    /// process-wide `GlobalSignal` (not reset between tests), and `cargo test` runs tests for
    /// a binary in parallel by default.
    #[test]
    fn track_pointer_down_then_pointer_position_reports_it() {
        with_runtime(|| {
            assert_eq!(pointer_position(101), None, "unknown pointer starts absent");

            track_pointer_down(101, ClientPoint::new(1.0, 2.0));
            assert_eq!(pointer_position(101), Some(ClientPoint::new(1.0, 2.0)));
        });
    }

    #[test]
    fn add_pointer_then_update_pointer_moves_it() {
        with_runtime(|| {
            add_pointer(102, ClientPoint::new(5.0, 5.0));
            assert_eq!(pointer_position(102), Some(ClientPoint::new(5.0, 5.0)));

            update_pointer(102, ClientPoint::new(9.0, 9.0));
            assert_eq!(
                pointer_position(102),
                Some(ClientPoint::new(9.0, 9.0)),
                "update_pointer must move the existing entry, not add a new one"
            );
        });
    }

    #[test]
    fn update_pointer_on_unknown_id_is_a_no_op() {
        with_runtime(|| {
            // No add_pointer(103, ..) beforehand.
            update_pointer(103, ClientPoint::new(9.0, 9.0));
            assert_eq!(
                pointer_position(103),
                None,
                "update_pointer must not create an entry for an unknown pointer id"
            );
        });
    }

    #[test]
    fn remove_pointer_clears_it_and_leaves_others() {
        with_runtime(|| {
            add_pointer(104, ClientPoint::new(1.0, 1.0));
            add_pointer(105, ClientPoint::new(2.0, 2.0));

            remove_pointer(104);

            assert_eq!(pointer_position(104), None, "removed pointer must be gone");
            assert_eq!(
                pointer_position(105),
                Some(ClientPoint::new(2.0, 2.0)),
                "removing one pointer must not affect another"
            );
        });
    }
}

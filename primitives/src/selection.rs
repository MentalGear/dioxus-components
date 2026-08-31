//! Shared option selection helpers.

use dioxus::prelude::{Signal, WritableExt};
use std::{any::Any, rc::Rc};

trait DynPartialEq: Any {
    fn eq(&self, other: &dyn Any) -> bool;
}

impl<T: PartialEq + 'static> DynPartialEq for T {
    fn eq(&self, other: &dyn Any) -> bool {
        other.downcast_ref::<T>() == Some(self)
    }
}

/// Type-erased value that still supports equality.
#[derive(Clone)]
pub(crate) struct RcPartialEqValue {
    value: Rc<dyn DynPartialEq>,
}

impl RcPartialEqValue {
    /// Create a new type-erased value.
    pub(crate) fn new<T: PartialEq + 'static>(value: T) -> Self {
        Self {
            value: Rc::new(value),
        }
    }

    /// Borrow this value as [`Any`].
    pub(crate) fn as_any(&self) -> &dyn Any {
        (&*self.value) as &dyn Any
    }

    /// Downcast this value to its concrete type.
    pub(crate) fn as_ref<T: PartialEq + 'static>(&self) -> Option<&T> {
        self.as_any().downcast_ref::<T>()
    }
}

impl PartialEq for RcPartialEqValue {
    fn eq(&self, other: &Self) -> bool {
        self.value.eq(&*other.value)
    }
}

/// Registered option metadata shared by select-like components.
#[derive(PartialEq)]
pub(crate) struct OptionState {
    /// Stable option identity.
    pub(crate) id: String,
    /// Collection index.
    pub(crate) index: usize,
    /// Programmatic option value.
    pub(crate) value: RcPartialEqValue,
    /// Display/search text.
    pub(crate) text_value: String,
}

/// Resolve an option's searchable text value.
pub(crate) fn option_text_value<T: 'static>(
    value: &T,
    text_value: Option<String>,
    component_name: &str,
) -> String {
    text_value.unwrap_or_else(|| {
        let as_any: &dyn Any = value;
        as_any
            .downcast_ref::<String>()
            .cloned()
            .or_else(|| as_any.downcast_ref::<&str>().map(|s| s.to_string()))
            .unwrap_or_else(|| {
                tracing::warn!(
                    "{component_name} with non-string types requires text_value to be set"
                );
                String::new()
            })
    })
}

/// Display text for selected values in selection order.
pub(crate) fn selected_text<'a>(
    values: impl IntoIterator<Item = &'a RcPartialEqValue>,
    options: &[OptionState],
) -> Option<String> {
    let parts: Vec<String> = values
        .into_iter()
        .filter_map(|value| {
            options
                .iter()
                .find(|option| &option.value == value)
                .map(|option| option.text_value.clone())
        })
        .collect();

    (!parts.is_empty()).then(|| parts.join(", "))
}

/// Insert or update a registered option.
pub(crate) fn sync_option(mut options: Signal<Vec<OptionState>>, option_state: OptionState) {
    sync_option_state(&mut options.write(), option_state);
}

fn sync_option_state(options: &mut Vec<OptionState>, option_state: OptionState) {
    if let Some(position) = options
        .iter()
        .position(|option| option.id == option_state.id)
    {
        if options[position].index == option_state.index {
            options[position] = option_state;
            return;
        }
        options.remove(position);
    }
    insert_option(options, option_state);
}

fn insert_option(options: &mut Vec<OptionState>, option_state: OptionState) {
    let insert_at = options.partition_point(|option| option.index <= option_state.index);
    options.insert(insert_at, option_state);
}

/// Remove a registered option by id.
pub(crate) fn remove_option(mut options: Signal<Vec<OptionState>>, id: &str) {
    remove_option_state(&mut options.write(), id);
}

fn remove_option_state(options: &mut Vec<OptionState>, id: &str) {
    options.retain(|option| option.id != id);
}

#[cfg(test)]
mod tests {
    use super::*;
    use dioxus::prelude::*;

    fn option(id: &str, index: usize) -> OptionState {
        OptionState {
            id: id.to_string(),
            index,
            value: RcPartialEqValue::new(id.to_string()),
            text_value: id.to_string(),
        }
    }

    fn ids(options: &[OptionState]) -> Vec<&str> {
        options
            .iter()
            .map(|option| option.text_value.as_str())
            .collect()
    }

    fn indices(options: &[OptionState]) -> Vec<usize> {
        options.iter().map(|option| option.index).collect()
    }

    #[test]
    fn sync_option_state_keeps_sorted_order() {
        let mut options = vec![option("a", 0), option("b", 1), option("c", 2)];

        sync_option_state(&mut options, option("d", 3));

        assert_eq!(ids(&options), ["a", "b", "c", "d"]);
        assert_eq!(indices(&options), [0, 1, 2, 3]);
    }

    #[test]
    fn sync_option_state_updates_matching_id_and_reorders() {
        let mut options = vec![option("a", 0), option("b", 1), option("c", 2)];

        sync_option_state(&mut options, option("b", 3));

        assert_eq!(ids(&options), ["a", "c", "b"]);
        assert_eq!(indices(&options), [0, 2, 3]);
    }

    #[test]
    fn removing_stale_option_does_not_remove_option_that_moved_to_same_index() {
        let mut options = vec![option("a", 0), option("b", 1)];

        sync_option_state(&mut options, option("b", 0));
        remove_option_state(&mut options, "a");

        assert_eq!(ids(&options), ["b"]);
        assert_eq!(indices(&options), [0]);
    }

    #[test]
    fn dyn_partial_eq_compares_by_downcast_and_value() {
        let a: Rc<dyn DynPartialEq> = Rc::new(1i32);
        let same_value: Rc<dyn DynPartialEq> = Rc::new(1i32);
        let different_value: Rc<dyn DynPartialEq> = Rc::new(2i32);
        let different_type: Rc<dyn DynPartialEq> = Rc::new("1".to_string());

        assert!(a.eq(&*same_value as &dyn Any));
        assert!(!a.eq(&*different_value as &dyn Any));
        assert!(
            !a.eq(&*different_type as &dyn Any),
            "a mismatched concrete type must never compare equal"
        );
    }

    #[test]
    fn rc_partial_eq_value_equality_is_by_downcast_value() {
        let a = RcPartialEqValue::new(42i32);
        let same = RcPartialEqValue::new(42i32);
        let different = RcPartialEqValue::new(7i32);
        let different_type = RcPartialEqValue::new("42".to_string());

        assert!(a == same);
        assert!(a != different);
        assert!(
            a != different_type,
            "values of different concrete types must never be equal"
        );

        assert_eq!(a.as_ref::<i32>(), Some(&42));
        assert_eq!(a.as_ref::<String>(), None, "wrong-type downcast must fail");
    }

    #[test]
    fn option_text_value_prefers_explicit_text_value() {
        let value = "ignored".to_string();
        let text = option_text_value(&value, Some("explicit".to_string()), "Select");
        assert_eq!(text, "explicit");
    }

    #[test]
    fn option_text_value_falls_back_to_string_value() {
        let value = "from-string".to_string();
        let text = option_text_value(&value, None, "Select");
        assert_eq!(text, "from-string");
    }

    #[test]
    fn option_text_value_falls_back_to_str_value() {
        let value: &str = "from-str";
        let text = option_text_value(&value, None, "Select");
        assert_eq!(text, "from-str");
    }

    #[test]
    fn option_text_value_defaults_to_empty_for_non_string_types_without_text_value() {
        let value = 42i32;
        let text = option_text_value(&value, None, "Select");
        assert_eq!(text, "");
    }

    #[test]
    fn selected_text_joins_matching_options_in_selection_order() {
        let options = vec![option("a", 0), option("b", 1), option("c", 2)];
        let a_value = RcPartialEqValue::new("a".to_string());
        let c_value = RcPartialEqValue::new("c".to_string());

        // Selection order (c, a) is preserved, not option registration order.
        let text = selected_text([&c_value, &a_value], &options);
        assert_eq!(text, Some("c, a".to_string()));
    }

    #[test]
    fn selected_text_skips_values_with_no_matching_option() {
        let options = vec![option("a", 0)];
        let stale_value = RcPartialEqValue::new("gone".to_string());

        let text = selected_text([&stale_value], &options);
        assert_eq!(
            text, None,
            "no matching option means no text, not an empty joined string"
        );
    }

    #[test]
    fn selected_text_returns_none_for_no_values() {
        let options = vec![option("a", 0)];
        let values: Vec<&RcPartialEqValue> = Vec::new();

        assert_eq!(selected_text(values, &options), None);
    }

    /// Run a closure inside a Dioxus runtime context so `Signal` is available
    /// (mirrors `virtual/virtualizer.rs`'s `with_runtime`). `sync_option` and
    /// `remove_option` are thin `Signal<Vec<OptionState>>` wrappers around
    /// `sync_option_state`/`remove_option_state`, so exercising *them*
    /// specifically (not just the inner pure functions) needs a real signal.
    fn with_runtime(f: impl Fn() + 'static) {
        use std::cell::Cell;

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
        result: Rc<std::cell::Cell<bool>>,
    }

    impl PartialEq for TestHarnessProps {
        fn eq(&self, _: &Self) -> bool {
            true
        }
    }

    #[test]
    fn sync_option_and_remove_option_operate_through_a_real_signal() {
        with_runtime(|| {
            let options_signal = Signal::new(vec![option("a", 0)]);

            sync_option(options_signal, option("b", 1));
            assert_eq!(
                options_signal
                    .read()
                    .iter()
                    .map(|o| o.id.clone())
                    .collect::<Vec<_>>(),
                vec!["a".to_string(), "b".to_string()],
                "sync_option must write the new option into the signal"
            );

            remove_option(options_signal, "a");
            assert_eq!(
                options_signal
                    .read()
                    .iter()
                    .map(|o| o.id.clone())
                    .collect::<Vec<_>>(),
                vec!["b".to_string()],
                "remove_option must remove the matching option from the signal"
            );
        });
    }
}

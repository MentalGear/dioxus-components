//! Defines the [`DatePicker`] and [`DateRangePicker`] components and its subcomponents, which allowing users to enter or select a date value

use crate::{
    calendar::{
        weekday_abbreviation, AvailableRanges, CalendarProps, DateRange, RangeCalendarProps,
    },
    collection::{collection_item, use_collection_provider, use_item, CollectionState},
    dioxus_core::Properties,
    popover::*,
    use_unique_id, LocalDateExt as _,
};

use dioxus::prelude::*;
use num_integer::Integer;
use std::{fmt::Display, str::FromStr};
use time::{macros::date, Date, Month, OffsetDateTime, Weekday};

/// The context provided by the [`DatePicker`] component to its children.
#[derive(Copy, Clone)]
struct BaseDatePickerContext {
    // State
    open: Signal<bool>,
    read_only: ReadSignal<bool>,

    // Configuration
    disabled: ReadSignal<bool>,
    focus: CollectionState,
    enabled_date_range: DateRange,
    available_ranges: Memo<AvailableRanges>,
}

/// The context provided by the [`DatePicker`] component to its children.
#[derive(Copy, Clone)]
struct DatePickerContext {
    on_value_change: Callback<Option<Date>>,
    selected_date: ReadSignal<Option<Date>>,
}

impl DatePickerContext {
    fn set_date(&mut self, date: Option<Date>) {
        let value = { self.selected_date.peek().cloned() };
        if value != date {
            self.on_value_change.call(date);
        }
    }
}

/// The props for the [`DatePicker`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DatePickerProps {
    /// Callback when value changes
    #[props(default)]
    pub on_value_change: Callback<Option<Date>>,

    /// The selected date
    #[props(default)]
    pub selected_date: ReadSignal<Option<Date>>,

    /// Whether the date picker is disabled
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Whether the date picker is enable user input
    #[props(default = ReadSignal::new(Signal::new(false)))]
    pub read_only: ReadSignal<bool>,

    /// Lower limit of the range of available dates
    #[props(default = date!(1925-01-01))]
    pub min_date: Date,

    /// Upper limit of the range of available dates
    #[props(default = date!(2050-12-31))]
    pub max_date: Date,

    /// Unavailable dates
    #[props(default)]
    pub disabled_ranges: ReadSignal<Vec<DateRange>>,

    /// Whether focus should loop around when reaching the end.
    #[props(default = ReadSignal::new(Signal::new(false)))]
    pub roving_loop: ReadSignal<bool>,

    /// Additional attributes to extend the date picker element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the date picker element
    pub children: Element,
}

/// # DatePicker
///
/// The [`DatePicker`] component provides an accessible date input interface.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::{calendar::Calendar, date_picker::*, popover::*, ContentAlign};
/// use time::Date;
/// #[component]
/// fn Demo() -> Element {
///    let mut selected_date = use_signal(|| None::<Date>);
///    rsx! {
///        div {
///            DatePicker {
///                selected_date: selected_date(),
///                on_value_change: move |date| {
///                    tracing::info!("Date changed to: {date:?}");
///                    selected_date.set(date);
///               },
///                DatePickerPopover {
///                    DatePickerInput {
///                        PopoverTrigger {
///                            "Select date"
///                        }
///                        PopoverContent {
///                            align: ContentAlign::End,
///                            DatePickerCalendar {
///                                calendar: Calendar,
///                            }
///                        }
///                    }
///                }
///            }
///        }
///    }
///}
/// ```
///
/// # Styling
///
/// The [`DatePicker`] component defines the following data attributes you can use to control styling:
/// - `data-disabled`: Indicates if the DatePicker is disabled. Possible values are `true` or `false`.
#[component]
pub fn DatePicker(props: DatePickerProps) -> Element {
    let open = use_signal(|| false);
    let focus = use_collection_provider(props.roving_loop);
    let available_ranges = use_memo(move || AvailableRanges::new(&props.disabled_ranges.read()));

    // Create context provider for child components
    use_context_provider(|| BaseDatePickerContext {
        open,
        read_only: props.read_only,
        disabled: props.disabled,
        focus,
        enabled_date_range: DateRange::new(props.min_date, props.max_date),
        available_ranges,
    });

    use_context_provider(|| DatePickerContext {
        on_value_change: props.on_value_change,
        selected_date: props.selected_date,
    });

    rsx! {
        div {
            role: "group",
            aria_label: "Date",
            "data-disabled": (props.disabled)(),
            ..props.attributes,
            {props.children}
        }
    }
}

/// The context provided by the [`DateRangePicker`] component to its children.
#[derive(Copy, Clone)]
pub struct DateRangePickerContext {
    // Currently selected date range
    date_range: ReadSignal<Option<DateRange>>,
    set_selected_range: Callback<Option<DateRange>>,
}

impl DateRangePickerContext {
    /// Set the selected date
    pub fn set_range(&mut self, range: Option<DateRange>) {
        if (self.date_range)() != range {
            self.set_selected_range.call(range);
        }
    }
}

/// The props for the [`DatePicker`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DateRangePickerProps {
    /// Callback when value changes
    #[props(default)]
    pub on_range_change: Callback<Option<DateRange>>,

    /// The selected date
    #[props(default)]
    pub selected_range: ReadSignal<Option<DateRange>>,

    /// Whether the date picker is disabled
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// Whether the date picker is enable user input
    #[props(default = ReadSignal::new(Signal::new(false)))]
    pub read_only: ReadSignal<bool>,

    /// Lower limit of the range of available dates
    #[props(default = date!(1925-01-01))]
    pub min_date: Date,

    /// Upper limit of the range of available dates
    #[props(default = date!(2050-12-31))]
    pub max_date: Date,

    /// Unavailable dates
    #[props(default)]
    pub disabled_ranges: ReadSignal<Vec<DateRange>>,

    /// Whether focus should loop around when reaching the end.
    #[props(default = ReadSignal::new(Signal::new(false)))]
    pub roving_loop: ReadSignal<bool>,

    /// Additional attributes to extend the date picker element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the date picker element
    pub children: Element,
}

/// # DateRangePicker
///
/// The [`DateRangePicker`] component provides an accessible date range input interface.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::{calendar::{DateRange, RangeCalendar}, date_picker::*, popover::*, ContentAlign};
/// #[component]
/// fn Demo() -> Element {
///    let mut selected_range = use_signal(|| None::<DateRange>);
///    rsx! {
///        div {
///            DateRangePicker {
///                selected_range: selected_range(),
///                on_range_change: move |range| {
///                    tracing::info!("Selected range: {:?}", range);
///                    selected_range.set(range);
///               },
///                DatePickerPopover {
///                    DatePickerInput {
///                        PopoverTrigger {
///                            "Select date"
///                        }
///                        PopoverContent {
///                            align: ContentAlign::End,
///                            DateRangePickerCalendar {
///                                calendar: RangeCalendar,
///                            }
///                        }
///                    }
///                }
///            }
///        }
///    }
///}
/// ```
///
/// # Styling
///
/// The [`DateRangePicker`] component defines the following data attributes you can use to control styling:
/// - `data-disabled`: Indicates if the DateRangePicker is disabled. Possible values are `true` or `false`.
#[component]
pub fn DateRangePicker(props: DateRangePickerProps) -> Element {
    let open = use_signal(|| false);
    let focus = use_collection_provider(props.roving_loop);

    let available_ranges = use_memo(move || AvailableRanges::new(&props.disabled_ranges.read()));

    // Create context provider for child components
    use_context_provider(|| BaseDatePickerContext {
        open,
        read_only: props.read_only,
        disabled: props.disabled,
        focus,
        enabled_date_range: DateRange::new(props.min_date, props.max_date),
        available_ranges,
    });

    use_context_provider(|| DateRangePickerContext {
        date_range: props.selected_range,
        set_selected_range: props.on_range_change,
    });

    rsx! {
        div {
            role: "group",
            aria_label: "Date Range",
            "data-disabled": (props.disabled)(),
            ..props.attributes,
            {props.children}
        }
    }
}

/// The props for the [`DatePickerPopover`] component.
#[allow(unpredictable_function_pointer_comparisons)]
#[derive(Props, Clone, PartialEq)]
pub struct DatePickerPopoverProps {
    /// Whether the popover is a modal and should capture focus.
    #[props(default = ReadSignal::new(Signal::new(true)))]
    pub is_modal: ReadSignal<bool>,

    /// The controlled open state of the popover.
    pub open: ReadSignal<Option<bool>>,

    /// The default open state when uncontrolled.
    #[props(default)]
    pub default_open: bool,

    /// Callback fired when the open state changes.
    #[props(default)]
    pub on_open_change: Callback<bool>,

    /// Additional attributes to apply to the popover root element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the popover root component.
    pub children: Element,

    /// The popover root component to use.
    #[props(default = PopoverRoot)]
    pub popover_root: fn(PopoverRootProps) -> Element,
}

/// # DatePickerPopover
///
/// The `DatePickerPopover` component wraps all the popover components and manages the state.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::{calendar::Calendar, date_picker::*, popover::*, ContentAlign};
/// use time::Date;
/// #[component]
/// fn Demo() -> Element {
///    let mut selected_date = use_signal(|| None::<Date>);
///    rsx! {
///        div {
///            DatePicker {
///                selected_date: selected_date(),
///                on_value_change: move |date| {
///                    tracing::info!("Date changed to: {date:?}");
///                    selected_date.set(date);
///               },
///                DatePickerPopover {
///                    DatePickerInput {
///                        PopoverTrigger {
///                            "Select date"
///                        }
///                        PopoverContent {
///                            align: ContentAlign::End,
///                            DatePickerCalendar {
///                                calendar: Calendar,
///                            }
///                        }
///                    }
///                }
///            }
///        }
///    }
///}
/// ```
#[component]
pub fn DatePickerPopover(props: DatePickerPopoverProps) -> Element {
    let ctx = use_context::<BaseDatePickerContext>();
    let mut open = ctx.open;

    let PopoverRoot = props.popover_root;

    rsx! {
        PopoverRoot {
            // Item 3 fix (2026-09-01, live-site report): `is_modal` was
            // declared on `DatePickerPopoverProps` (documented "whether the
            // popover is a modal and should capture focus") but never
            // actually forwarded here, so it silently had no effect --
            // every `DatePickerPopover` rendered with `PopoverRoot`'s own
            // default (`is_modal: true`) no matter what a caller passed.
            // See `preview/src/components/date_picker/component.rs` for why
            // this mattered in practice: `PopoverModalContent`'s DOM-
            // relative "centering trick" (`position: absolute; left: 50%;
            // transform: translateX(-50%)`, `../popover/style.css`) centers
            // the calendar under its *positioned ancestor* (the narrow
            // `.dx-date-picker` input group), not its trigger, and has no
            // collision/edge-avoidance -- so a calendar wider than that
            // ancestor renders partly off-screen whenever the ancestor sits
            // near a viewport edge, confirmed by execution (measured
            // ~-138px of a 276px-wide calendar sitting off the left edge on
            // this repo's dev server). Forwarding `is_modal` here is what
            // lets that preview component's `is_modal: false` actually
            // switch the calendar onto the non-modal, trigger-anchored arm
            // instead.
            is_modal: props.is_modal,
            open: open(),
            on_open_change: move |v| open.set(v),
            attributes: props.attributes,
            {props.children}
        }
    }
}

#[doc(hidden)]
/// A trait for types that can provide default calendar rendering.
pub trait DefaultCalendarProps {
    /// Provide a default calendar rendering function.
    fn default_calendar(self) -> Element;
}

/// The props for the Calendar component.
#[allow(unpredictable_function_pointer_comparisons)]
#[derive(Props, Clone, PartialEq)]
pub struct DatePickerCalendarProps<T: DefaultCalendarProps + Properties + PartialEq> {
    /// Callback when display weekday
    #[props(default = Callback::new(|weekday: Weekday| weekday_abbreviation(weekday).to_string()))]
    pub on_format_weekday: Callback<Weekday, String>,

    /// Callback when display month
    #[props(default = Callback::new(|month: Month| month.to_string()))]
    pub on_format_month: Callback<Month, String>,

    /// The month being viewed
    #[props(default = ReadSignal::new(Signal::new(OffsetDateTime::now_local_date())))]
    pub view_date: ReadSignal<Date>,

    /// The current date (used for highlighting today)
    #[props(default = OffsetDateTime::now_local_date())]
    pub today: Date,

    /// Callback when view date changes
    #[props(default)]
    pub on_view_change: Callback<Date>,

    /// Whether the calendar is disabled
    #[props(default)]
    pub disabled: ReadSignal<bool>,

    /// First day of the week
    #[props(default = Weekday::Sunday)]
    pub first_day_of_week: Weekday,

    /// Lower limit of the range of available dates
    #[props(default = date!(1925-01-01))]
    pub min_date: Date,

    /// Upper limit of the range of available dates
    #[props(default = date!(2050-12-31))]
    pub max_date: Date,

    /// Unavailable dates
    #[props(default)]
    pub disabled_ranges: ReadSignal<Vec<DateRange>>,

    /// Additional attributes to extend the calendar element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the calendar element
    pub children: Element,

    /// The calendar to render with
    #[props(default = T::default_calendar)]
    pub calendar: fn(T) -> Element,
}

/// # DatePickerCalendar
///
/// The [`DatePickerCalendar`] component provides an accessible calendar interface with arrow key navigation, month switching, and date selection.
/// Used as date picker popover component
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::{calendar::Calendar, date_picker::*, popover::*, ContentAlign};
/// use time::Date;
/// #[component]
/// fn Demo() -> Element {
///    let mut selected_date = use_signal(|| None::<Date>);
///    rsx! {
///        div {
///            DatePicker {
///                selected_date: selected_date(),
///                on_value_change: move |date| {
///                    tracing::info!("Date changed to: {date:?}");
///                    selected_date.set(date);
///               },
///                DatePickerPopover {
///                    DatePickerInput {
///                        PopoverTrigger {
///                            "Select date"
///                        }
///                        PopoverContent {
///                            align: ContentAlign::End,
///                            DatePickerCalendar {}
///                        }
///                    }
///                }
///            }
///        }
///    }
///}
/// ```
#[component]
pub fn DatePickerCalendar(props: DatePickerCalendarProps<CalendarProps>) -> Element {
    let mut base_ctx = use_context::<BaseDatePickerContext>();
    let mut ctx = use_context::<DatePickerContext>();

    #[allow(non_snake_case)]
    let Calendar = props.calendar;
    let mut view_date = use_signal(|| props.today);
    use_effect(move || {
        if let Some(date) = (ctx.selected_date)() {
            view_date.set(date);
        }
    });

    let min_date = base_ctx.enabled_date_range.start();
    let max_date = base_ctx.enabled_date_range.end();

    rsx! {
        Calendar {
            selected_date: ctx.selected_date,
            on_date_change: move |date| {
                ctx.set_date(date);
                base_ctx.open.set(false);
            },
            disabled_ranges: base_ctx.available_ranges.read().to_disabled_ranges(),
            on_format_weekday: props.on_format_weekday,
            on_format_month: props.on_format_month,
            view_date: view_date(),
            on_view_change: move |date| view_date.set(date),
            today: props.today,
            disabled: props.disabled,
            first_day_of_week: props.first_day_of_week,
            min_date,
            max_date,
            attributes: props.attributes,
            {props.children}
        }
    }
}

/// # DateRangePickerCalendar
///
/// The [`DateRangePickerCalendar`] component provides an accessible calendar interface with arrow key navigation, month switching, and date range selection.
/// Used as date picker popover component
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::{calendar::{DateRange, RangeCalendar}, date_picker::*, popover::*, ContentAlign};
/// #[component]
/// fn Demo() -> Element {
///    let mut selected_range = use_signal(|| None::<DateRange>);
///    rsx! {
///        div {
///            DateRangePicker {
///                selected_range: selected_range(),
///                on_range_change: move |range| {
///                    tracing::info!("Selected range: {:?}", range);
///                    selected_range.set(range);
///               },
///                DatePickerPopover {
///                    DateRangePickerInput {
///                        PopoverTrigger {
///                            "Select date"
///                        }
///                        PopoverContent {
///                            align: ContentAlign::End,
///                            DateRangePickerCalendar {}
///                        }
///                    }
///                }
///            }
///        }
///    }
///}
/// ```
#[component]
pub fn DateRangePickerCalendar(props: DatePickerCalendarProps<RangeCalendarProps>) -> Element {
    let mut base_ctx = use_context::<BaseDatePickerContext>();
    let mut ctx = use_context::<DateRangePickerContext>();

    #[allow(non_snake_case)]
    let RangeCalendar = props.calendar;
    let mut view_date = use_signal(|| props.today);
    use_effect(move || {
        if let Some(r) = (ctx.date_range)() {
            view_date.set(r.start());
        }
    });

    let min_date = base_ctx.enabled_date_range.start();
    let max_date = base_ctx.enabled_date_range.end();

    rsx! {
        RangeCalendar {
            selected_range: ctx.date_range,
            on_range_change: move |range| {
                ctx.set_range(range);
                base_ctx.open.set(false);
            },
            disabled_ranges: base_ctx.available_ranges.read().to_disabled_ranges(),
            on_format_weekday: props.on_format_weekday,
            on_format_month: props.on_format_month,
            view_date: view_date(),
            on_view_change: move |date| view_date.set(date),
            today: props.today,
            disabled: props.disabled,
            first_day_of_week: props.first_day_of_week,
            min_date,
            max_date,
            attributes: props.attributes,
            {props.children}
        }
    }
}

// The props for the [`DateSegment`] component
#[derive(Props, Clone, PartialEq)]
struct DateSegmentProps<T: Clone + Integer + 'static> {
    // The index of the segment
    pub index: ReadSignal<usize>,

    // The controlled value of the date picker
    pub value: ReadSignal<Option<T>>,

    // Default value
    pub default: T,

    // Callback when value changes
    #[props(default)]
    pub on_value_change: Callback<Option<T>>,

    // The minimum value
    pub min: ReadSignal<T>,

    // The maximum value
    pub max: ReadSignal<T>,

    // Max field length
    pub max_length: usize,

    /// Whether Up/Down past `min`/`max` wraps to the opposite bound (the
    /// default) or clamps and stays at the bound -- see
    /// `date_segment_key_effects`'s own doc for the calibration (backlog
    /// row 84 finding 4).
    #[props(default = true)]
    pub wrap: bool,

    // Callback when display placeholder
    pub on_format_placeholder: Callback<(), String>,

    /// An `aria-valuetext` alternative to the raw number, for a value whose
    /// number alone doesn't convey its meaning (e.g. a month name) -- per
    /// the APG spinbutton pattern's Roles/States/Properties section. `None`
    /// (the default) renders no `aria-valuetext` at all, exactly like an
    /// absent `value_text` always has. Always `None` while the segment has
    /// no value (see [`DateSegment`]'s own doc), matching `aria-valuenow`'s
    /// own absent-while-untouched rule (backlog row 84 findings 2 and 3).
    #[props(default)]
    pub value_text: Option<String>,

    // Additional attributes for the value element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// A single side effect produced by interpreting a `DateSegment` keydown
/// event, in the exact order `DateSegment`'s `onkeydown` handler should apply
/// them. Keeping this as an ordered list (rather than, say, a handful of
/// independent booleans/options) preserves the original code's effect
/// ordering exactly -- which differs between branches (e.g. the empty-text
/// case emits the value before moving focus, while the overflow case moves
/// focus before emitting).
#[derive(Debug, Clone, PartialEq)]
enum DateSegmentEffect<T> {
    /// Call `on_value_change` with this value.
    EmitValue(Option<T>),
    /// Move focus to the previous segment.
    FocusPrevious,
    /// Move focus to the next segment.
    FocusNext,
    /// Clear the pending `reset_value` flag.
    ConsumeReset,
    /// Call `prevent_default` on the originating event.
    PreventDefault,
    /// Call `stop_propagation` on the originating event.
    StopPropagation,
}

/// Roll a value that has gone out of `[min, max]` around to the opposite
/// bound (used by the arrow-key handlers so incrementing past `max` wraps to
/// `min`, and vice versa). Used for the month and day segments -- see
/// [`date_segment_key_effects`]'s own doc for the calibration behind which
/// segments wrap and which (the year, [`clamp_value`]) don't.
fn roll_value<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        max
    } else if value > max {
        min
    } else {
        value
    }
}

/// Clamp a value that has gone out of `[min, max]` back to the nearest
/// bound, instead of [`roll_value`]'s wrap-to-the-opposite-end behavior.
/// Used by the year segment (backlog row 84 finding 4) -- see
/// [`date_segment_key_effects`]'s own doc for why.
fn clamp_value<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// Compute the effects of committing a new raw text buffer to a `DateSegment`
/// (shared by the Character/Backspace/Delete handlers, which all end by
/// re-deriving the segment's value from its updated text buffer).
fn push_commit_text_effects<T: Integer + Copy + FromStr>(
    effects: &mut Vec<DateSegmentEffect<T>>,
    text: &str,
    min: T,
    max: T,
) {
    if text.is_empty() {
        effects.push(DateSegmentEffect::EmitValue(None));
        effects.push(DateSegmentEffect::FocusPrevious);
        return;
    }

    let value = text.parse::<T>().map(|v| v.min(max)).ok();
    if let Some(value) = value {
        let in_range = value >= min && value <= max;
        // If adding a new digit would exceed max, move to the next segment.
        let new_value = format!("{text}0").parse::<T>().unwrap_or(value);
        if in_range && new_value > max {
            effects.push(DateSegmentEffect::FocusNext);
        }
    }
    effects.push(DateSegmentEffect::EmitValue(value));
}

/// Keyboard modifiers relevant to a `DateSegment` keydown (shortcut passthrough
/// and "clear all" detection).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct SegmentKeyModifiers {
    ctrl: bool,
    meta: bool,
    alt: bool,
}

/// The segment's raw text-buffer state at the time of the keydown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SegmentTextState<'a> {
    current_text: &'a str,
    max_length: usize,
    reset: bool,
}

/// The segment's current numeric value and its bounds/default.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SegmentValueBounds<T> {
    current_value: Option<T>,
    default: T,
    min: T,
    max: T,
    /// Whether Up/Down past `min`/`max` wraps to the opposite bound
    /// ([`roll_value`]) or clamps at it ([`clamp_value`]) -- see
    /// [`date_segment_key_effects`]'s own doc for the calibration.
    wrap: bool,
}

/// Compute what a `DateSegment` should do in response to a keydown, as a pure
/// function of the key, the relevant modifier flags, and the segment's
/// current text/value state. Side-effect free and directly testable --
/// mirrors `move_interaction.rs`'s `MoveEvent::from_keyboard`. The caller
/// (`DateSegment`'s `onkeydown` handler) applies the returned effects in
/// order.
///
/// ## Up/Down wrap-vs-clamp calibration (backlog row 84 finding 4)
///
/// `bounds.wrap` decides what Up/Down do at `min`/`max`: wrap to the
/// opposite bound ([`roll_value`]) or clamp and stay there ([`clamp_value`]).
/// Calibrated against two sources per segment, cited in each caller
/// (`DatePickerYearSegment`/`DatePickerMonthSegment`/`DatePickerDaySegment`):
/// native `<input type="date">` in Chromium (its month/day sub-fields wrap
/// on Up/Down at their own bounds; its year sub-field does not -- it just
/// stops incrementing/decrementing) and the APG spinbutton pattern (silent
/// on wrap-vs-clamp for a bounded value, but its own Roles/States/
/// Properties section frames `aria-valuemin`/`aria-valuemax` as real limits
/// the value stays within, which reads more naturally as "stop at the
/// bound" than "silently jump to the opposite end" for a non-cyclical unit
/// like a year). Month and day are cyclical (day 31 of one frame of
/// reference is adjacent to day 1 of the next), so wrapping matches both
/// user expectation and the native platform control; a year is not
/// cyclical, so clamping is the correct default for it specifically.
fn date_segment_key_effects<T: Integer + Copy + FromStr>(
    key: &Key,
    modifiers: SegmentKeyModifiers,
    text_state: SegmentTextState,
    bounds: SegmentValueBounds<T>,
) -> Vec<DateSegmentEffect<T>> {
    let SegmentKeyModifiers { ctrl, meta, alt } = modifiers;
    let SegmentTextState {
        current_text,
        max_length,
        reset,
    } = text_state;
    let SegmentValueBounds {
        current_value,
        default,
        min,
        max,
        wrap,
    } = bounds;

    let mut effects = Vec::new();
    match key {
        Key::Character(actual_char) => {
            // Don't block keyboard shortcuts
            if ctrl || meta || alt {
                return effects;
            }
            if actual_char.parse::<T>().is_ok() {
                let mut text = if current_text.len() == max_length || reset {
                    effects.push(DateSegmentEffect::ConsumeReset);
                    String::new()
                } else {
                    current_text.to_string()
                };
                text.push_str(actual_char);
                push_commit_text_effects(&mut effects, &text, min, max);
            }
            effects.push(DateSegmentEffect::PreventDefault);
            effects.push(DateSegmentEffect::StopPropagation);
        }
        Key::Backspace => {
            let mut text = current_text.to_string();
            if ctrl || meta {
                text.clear();
            } else {
                text.pop();
            }
            push_commit_text_effects(&mut effects, &text, min, max);
        }
        Key::Delete => {
            let mut text = current_text.to_string();
            text.remove(0);
            push_commit_text_effects(&mut effects, &text, min, max);
        }
        Key::ArrowLeft => {
            effects.push(DateSegmentEffect::FocusPrevious);
        }
        Key::ArrowRight => {
            effects.push(DateSegmentEffect::FocusNext);
        }
        Key::Enter => {
            effects.push(DateSegmentEffect::FocusNext);
            effects.push(DateSegmentEffect::PreventDefault);
            effects.push(DateSegmentEffect::StopPropagation);
        }
        Key::ArrowUp => {
            let value = match current_value {
                Some(mut value) => {
                    value.inc();
                    if wrap {
                        roll_value(value, min, max)
                    } else {
                        clamp_value(value, min, max)
                    }
                }
                None => default,
            };
            effects.push(DateSegmentEffect::EmitValue(Some(value)));
        }
        Key::ArrowDown => {
            let value = match current_value {
                Some(mut value) => {
                    value.dec();
                    if wrap {
                        roll_value(value, min, max)
                    } else {
                        clamp_value(value, min, max)
                    }
                }
                None => default,
            };
            effects.push(DateSegmentEffect::EmitValue(Some(value)));
        }
        // `aria-valuemin`/`aria-valuemax` are unconditionally present on every
        // `DateSegment` (see its `rsx!` below), so per the APG Spin Button
        // pattern's Keyboard Interaction section (non-optional, unlike Page
        // Up/Down which that same list marks "(Optional)"):
        // "Home: If the spinbutton has a minimum value, sets the value to
        // its minimum." / "End: ... sets the value to its maximum."
        // (`$S/aria-practices` content/patterns/spinbutton/spinbutton-
        // pattern.html#keyboard_interaction, commit 7e4034b -- the current,
        // non-deprecated pattern doc, not the deprecated `datepicker-
        // spinbuttons.html` example, which predates this rule and has no
        // Home/End support of its own to conflict with it). Previously
        // unhandled (fell through to the wildcard arm below), confirmed by
        // execution against this repo's dev server before this fix: Home/End
        // on a focused segment left `aria-valuenow` completely unchanged.
        Key::Home => {
            effects.push(DateSegmentEffect::EmitValue(Some(min)));
        }
        Key::End => {
            effects.push(DateSegmentEffect::EmitValue(Some(max)));
        }
        _ => (),
    }
    effects
}

#[component]
fn DateSegment<T: Clone + Copy + Integer + FromStr + Display + 'static>(
    props: DateSegmentProps<T>,
) -> Element {
    let mut text_value = use_signal(|| "".to_string());
    use_effect(move || {
        let text = match (props.value)() {
            Some(value) => value.to_string(),
            None => String::default(),
        };
        text_value.set(text);
    });

    let mut reset_value = use_signal(|| false);

    // The formatted text for the segment
    let display_value = use_memo(move || {
        let value = (props.value)();
        match value {
            Some(value) => format!("{:0>width$}", value, width = props.max_length),
            None => props
                .on_format_placeholder
                .call(())
                .repeat(props.max_length),
        }
    });

    // backlog row 84 findings 2/3/6 (the "untouched segment" state machine):
    // `aria-valuenow` must mirror `props.value` faithfully -- present once
    // the segment actually holds a value, absent while it doesn't -- rather
    // than substituting `props.default` for the "no value yet" case, which
    // is what this used to do (`(props.value)().unwrap_or(props.default)`).
    // That substitution silently claimed a value (typically today's date)
    // the user never set (finding 3), AND made the first Up/Down keypress
    // on a never-touched segment invisible from an aria/assistive-tech
    // standpoint (finding 6): `date_segment_key_effects`'s `None => default`
    // branch (unchanged by this fix -- see its own doc) already committed
    // exactly `default` on that first press, so when `aria-valuenow` was
    // ALREADY reporting `default` beforehand, the announced value was
    // identical before and after -- indistinguishable from a dropped
    // keystroke. Once "no value" stops being reported as "= default", that
    // same first press becomes a real, detectable transition (the attribute
    // goes from absent to present), fixing both findings with the one
    // change. `value_text` (below, in the rendered `span`) is gated on this
    // same `Option`, so `aria-valuetext` (finding 2) tracks `aria-valuenow`
    // in lockstep instead of drifting into its own "untouched" state.
    let now_value = use_memo(move || (props.value)());

    let mut ctx = use_context::<BaseDatePickerContext>();

    use_effect(move || {
        // If this item is not focused, always keep the value clamped
        if !ctx.focus.is_focused(props.index.cloned()) {
            if let Some(value) = (props.value)() {
                let clamped_value = value.clamp(props.min.cloned(), props.max.cloned());
                if clamped_value != value {
                    props.on_value_change.call(Some(clamped_value));
                }
            }
        }
    });

    let handle_keydown = move |event: Event<KeyboardData>| {
        let key = event.key();
        let modifiers = event.modifiers();
        let effects = date_segment_key_effects(
            &key,
            SegmentKeyModifiers {
                ctrl: modifiers.ctrl(),
                meta: modifiers.meta(),
                alt: modifiers.alt(),
            },
            SegmentTextState {
                current_text: &text_value(),
                max_length: props.max_length,
                reset: reset_value(),
            },
            SegmentValueBounds {
                current_value: (props.value)(),
                default: props.default,
                min: props.min.cloned(),
                max: props.max.cloned(),
                wrap: props.wrap,
            },
        );

        for effect in effects {
            match effect {
                DateSegmentEffect::EmitValue(value) => props.on_value_change.call(value),
                DateSegmentEffect::FocusPrevious => ctx.focus.focus_prev(),
                DateSegmentEffect::FocusNext => ctx.focus.focus_next(),
                DateSegmentEffect::ConsumeReset => reset_value.set(false),
                DateSegmentEffect::PreventDefault => event.prevent_default(),
                DateSegmentEffect::StopPropagation => event.stop_propagation(),
            }
        }
    };

    let disabled = move || (ctx.disabled)();
    let onmounted =
        use_item(collection_item(ctx.focus, props.index).disabled(disabled)).onmounted();

    let span_id = use_unique_id();
    let id = use_memo(move || format!("span-{span_id}"));

    rsx! {
        span {
            id,
            role: "spinbutton",
            aria_valuemin: props.min.to_string(),
            aria_valuemax: props.max.to_string(),
            // `Option<String>` (rather than always-a-string): Dioxus omits
            // the attribute entirely when `None`, the same convention this
            // crate already uses for other optional ARIA state (e.g.
            // `navigation_menu.rs`'s `aria_current: props.active.then_some
            // ("page")`) -- see `now_value`'s own doc above for why this
            // must be absent, not a substituted default, while untouched.
            aria_valuenow: now_value().map(|v| v.to_string()),
            aria_valuetext: props.value_text.clone(),
            // No `aria-labelledby` here: per the APG Spin Button pattern's
            // own Roles/States/Properties section ("If the spinbutton has a
            // visible label, it is referenced by aria-labelledby ... .
            // Otherwise, the spinbutton element has a label provided by
            // aria-label" -- `$S/aria-practices` content/patterns/
            // spinbutton/spinbutton-pattern.html#roles_states_properties,
            // commit 7e4034b), and every caller here (`DatePickerYearSegment`
            // etc., below) always passes a plain `aria_label: "year"`/
            // `"month"`/`"day"` -- there never is a separate *visible* label
            // element for `aria-labelledby` to reference. This span used to
            // also render `aria-labelledby="{id}-label"`, but nothing in
            // this module (or its preview consumers) ever rendered an
            // element with that id -- confirmed by execution before this
            // fix: every one of a live page's spinbuttons carried a
            // dangling reference (`document.getElementById(idref)` ===
            // `null`). Chromium's own accessible-name computation happens
            // to fall back to `aria-label` when `aria-labelledby`'s IDREFs
            // don't resolve (confirmed live via Playwright's
            // `ariaSnapshot()`, and axe-core's ruleset does not flag it
            // either), so this was not user-visible in this repo's own
            // harness -- but it is dead, misleading markup that contradicts
            // the cited rule outright (an `aria-labelledby` with nothing to
            // reference is not "referencing a visible label"), and relying
            // on a fallback another AT/browser might implement differently
            // is exactly the kind of fragility the cited either/or rule
            // exists to avoid. `aria-label` alone (already present via
            // `props.attributes`, spread below) is the correct, sufficient
            // source per that rule.
            inputmode: "numeric",
            contenteditable: !(ctx.read_only)(),
            spellcheck: false,
            tabindex: "0",
            enterkeyhint: "next",
            onkeydown: handle_keydown,
            onmounted,
            onfocus: move |_| {
                reset_value.set(true);
                ctx.focus.set_focus(Some(props.index.cloned()));
                if (ctx.open)() {
                    ctx.open.set(false);
                }
            },
            "no-date": (props.value)().is_none(),
            "data-disabled": (ctx.disabled)(),
            ..props.attributes,
            {display_value}
        }
    }
}

#[derive(Clone, Copy)]
struct DateElementContext {
    start_index: usize,
    year_value: Signal<Option<i32>>,
    month_value: Signal<Option<u8>>,
    day_value: Signal<Option<u8>>,
    on_format_day_placeholder: Callback<(), String>,
    on_format_month_placeholder: Callback<(), String>,
    on_format_year_placeholder: Callback<(), String>,
    /// Locale-aware month name for the month segment's `aria-valuetext`
    /// (backlog row 84 finding 2) -- see `DateElementProps::on_format_month`.
    on_format_month: Callback<Month, String>,
}

/// The props for the [`DatePickerYearSegment`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DatePickerYearSegmentProps {
    /// Additional attributes for the year segment element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// The props for the [`DatePickerMonthSegment`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DatePickerMonthSegmentProps {
    /// Additional attributes for the month segment element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// The props for the [`DatePickerDaySegment`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DatePickerDaySegmentProps {
    /// Additional attributes for the day segment element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// The props for the [`DatePickerSeparator`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DatePickerSeparatorProps {
    /// The separator symbol.
    #[props(default = '-')]
    pub symbol: char,

    /// Additional attributes for the separator element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// A year segment in a date input.
#[component]
pub fn DatePickerYearSegment(props: DatePickerYearSegmentProps) -> Element {
    let mut ctx = use_context::<DateElementContext>();
    let base_ctx = use_context::<BaseDatePickerContext>();
    let today = OffsetDateTime::now_local_date();
    let min_year = base_ctx.enabled_date_range.start().year();
    let max_year = base_ctx.enabled_date_range.end().year();

    rsx! {
        DateSegment {
            aria_label: "year",
            index: ctx.start_index,
            value: ctx.year_value,
            default: today.year(),
            on_value_change: move |value: Option<i32>| ctx.year_value.set(value),
            min: min_year,
            max: max_year,
            max_length: 4,
            // A year is not cyclical (unlike month/day) -- backlog row 84
            // finding 4, see `date_segment_key_effects`'s own doc for the
            // full calibration (native Chromium `<input type="date">` does
            // not wrap its year sub-field either).
            wrap: false,
            on_format_placeholder: ctx.on_format_year_placeholder,
            attributes: props.attributes,
        }
    }
}

/// Compute the min/max month allowed for the given year, based on the
/// enabled date range's boundary months (only the boundary *year* clamps the
/// month; any other year allows the full January-December range).
fn month_bounds_for_year(
    year_value: Option<i32>,
    min_year: i32,
    max_year: i32,
    min_date: Date,
    max_date: Date,
) -> (u8, u8) {
    let min_month = match year_value {
        Some(year) if year == min_year => min_date.month(),
        _ => Month::January,
    };
    let max_month = match year_value {
        Some(year) if year == max_year => max_date.month(),
        _ => Month::December,
    };
    (min_month as u8, max_month as u8)
}

/// A month segment in a date input.
#[component]
pub fn DatePickerMonthSegment(props: DatePickerMonthSegmentProps) -> Element {
    let mut ctx = use_context::<DateElementContext>();
    let base_ctx = use_context::<BaseDatePickerContext>();
    let today = OffsetDateTime::now_local_date();
    let min_date = base_ctx.enabled_date_range.start();
    let max_date = base_ctx.enabled_date_range.end();
    let min_year = min_date.year();
    let max_year = max_date.year();
    let (min_month, max_month) =
        month_bounds_for_year((ctx.year_value)(), min_year, max_year, min_date, max_date);

    // backlog row 84 finding 2: a locale-aware month *name* for
    // `aria-valuetext`, not a bare 1-12 number -- `ctx.on_format_month` is
    // the same kind of caller-suppliable, i18n-capable callback the
    // calendar grid's `CalendarMonthTitle`/`CalendarSelectMonthValue` (and
    // `DatePickerCalendarProps::on_format_month`) already use, defaulting to
    // `Month::to_string()` (see `DateElementProps::on_format_month`) but
    // overridable with e.g. `dioxus_i18n`'s `tid!` for a translated name.
    // `None` while the segment has no value yet (`ctx.month_value()` is
    // `None`), matching `aria-valuenow`'s own untouched-is-absent rule
    // (finding 3) so the two stay in lockstep -- see `DateSegment`'s
    // `now_value` doc.
    let value_text = (ctx.month_value)()
        .and_then(|m| Month::try_from(m).ok())
        .map(|m| ctx.on_format_month.call(m));

    rsx! {
        DateSegment {
            aria_label: "month",
            index: ctx.start_index + 1usize,
            value: ctx.month_value,
            default: today.month() as u8,
            on_value_change: move |value: Option<u8>| ctx.month_value.set(value),
            min: min_month,
            max: max_month,
            max_length: 2,
            // Cyclical (December -> January) -- backlog row 84 finding 4,
            // matches native Chromium `<input type="date">`'s month
            // sub-field. `wrap: true` is also `DateSegment`'s own default;
            // stated explicitly here (and on the day segment below) so the
            // calibration decision is visible at every call site, not just
            // the one (year) that overrides it.
            wrap: true,
            on_format_placeholder: ctx.on_format_month_placeholder,
            value_text,
            attributes: props.attributes,
        }
    }
}

/// Compute the min/max day allowed for the given year/month, based on the
/// enabled date range's boundary year+month. Falls back to the target
/// month's actual length (or 31, if the month value is out of range) when
/// not at a boundary.
fn day_bounds_for_year_month(
    year_value: Option<i32>,
    month_value: Option<u8>,
    min_year: i32,
    max_year: i32,
    min_date: Date,
    max_date: Date,
) -> (u8, u8) {
    let min_day = match (year_value, month_value) {
        (Some(year), Some(month)) if year == min_year && month == min_date.month() as u8 => {
            min_date.day()
        }
        _ => 1,
    };
    let max_day = match (year_value, month_value) {
        (Some(year), Some(month)) if year == max_year && month == max_date.month() as u8 => {
            max_date.day()
        }
        (Some(year), Some(month)) => {
            if let Ok(month) = Month::try_from(month) {
                month.length(year)
            } else {
                31
            }
        }
        _ => 31,
    };
    (min_day, max_day)
}

/// A day segment in a date input.
#[component]
pub fn DatePickerDaySegment(props: DatePickerDaySegmentProps) -> Element {
    let mut ctx = use_context::<DateElementContext>();
    let base_ctx = use_context::<BaseDatePickerContext>();
    let today = OffsetDateTime::now_local_date();
    let min_date = base_ctx.enabled_date_range.start();
    let max_date = base_ctx.enabled_date_range.end();
    let min_year = min_date.year();
    let max_year = max_date.year();
    let (min_day, max_day) = day_bounds_for_year_month(
        (ctx.year_value)(),
        (ctx.month_value)(),
        min_year,
        max_year,
        min_date,
        max_date,
    );

    rsx! {
        DateSegment {
            aria_label: "day",
            index: ctx.start_index + 2usize,
            value: ctx.day_value,
            default: today.day(),
            on_value_change: move |value: Option<u8>| ctx.day_value.set(value),
            min: min_day,
            max: max_day,
            max_length: 2,
            // Cyclical (the last day of a month -> the 1st) -- backlog row
            // 84 finding 4, matches native Chromium `<input type="date">`'s
            // day sub-field. See `DatePickerMonthSegment`'s identical note.
            wrap: true,
            on_format_placeholder: ctx.on_format_day_placeholder,
            attributes: props.attributes,
        }
    }
}

/// A separator in a date input.
#[component]
pub fn DatePickerSeparator(props: DatePickerSeparatorProps) -> Element {
    rsx! {
        span {
            aria_hidden: "true",
            tabindex: "-1",
            "is-separator": true,
            "no-date": true,
            ..props.attributes,
            "{props.symbol}"
        }
    }
}

/// The props for the [`DatePickerInputValue`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DatePickerInputValueProps {
    /// Callback when display day placeholder
    #[props(default = Callback::new(|_| "D".to_string()))]
    pub on_format_day_placeholder: Callback<(), String>,

    /// Callback when display month placeholder
    #[props(default = Callback::new(|_| "M".to_string()))]
    pub on_format_month_placeholder: Callback<(), String>,

    /// Callback when display year placeholder
    #[props(default = Callback::new(|_| "Y".to_string()))]
    pub on_format_year_placeholder: Callback<(), String>,

    /// See `DateElementProps::on_format_month` (backlog row 84 finding 2).
    #[props(default = Callback::new(|month: Month| month.to_string()))]
    pub on_format_month: Callback<Month, String>,

    /// The children of the date value.
    #[props(default)]
    pub children: Option<Element>,
}

/// The props for the [`DateRangePickerInputValue`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DateRangePickerInputValueProps {
    /// Callback when display day placeholder
    #[props(default = Callback::new(|_| "D".to_string()))]
    pub on_format_day_placeholder: Callback<(), String>,

    /// Callback when display month placeholder
    #[props(default = Callback::new(|_| "M".to_string()))]
    pub on_format_month_placeholder: Callback<(), String>,

    /// Callback when display year placeholder
    #[props(default = Callback::new(|_| "Y".to_string()))]
    pub on_format_year_placeholder: Callback<(), String>,

    /// See `DateElementProps::on_format_month` (backlog row 84 finding 2).
    #[props(default = Callback::new(|month: Month| month.to_string()))]
    pub on_format_month: Callback<Month, String>,

    /// The children of the date range value.
    #[props(default)]
    pub children: Option<Element>,
}

/// The props for the [`DateRangePickerStartValue`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DateRangePickerStartValueProps {
    /// The children of the start date value.
    #[props(default)]
    pub children: Option<Element>,
}

/// The props for the [`DateRangePickerEndValue`] component.
#[derive(Props, Clone, PartialEq)]
pub struct DateRangePickerEndValueProps {
    /// The children of the end date value.
    #[props(default)]
    pub children: Option<Element>,
}

#[derive(Clone, Copy)]
struct DateRangeInputContext {
    start_date: Signal<Option<Date>>,
    end_date: Signal<Option<Date>>,
    on_format_day_placeholder: Callback<(), String>,
    on_format_month_placeholder: Callback<(), String>,
    on_format_year_placeholder: Callback<(), String>,
    /// See `DateElementProps::on_format_month` (backlog row 84 finding 2).
    on_format_month: Callback<Month, String>,
}

impl DateRangeInputContext {
    /// Sets the start date, but only if it actually differs from the
    /// current value -- the single choke point every producer of
    /// `start_date` (the calendar-driven sync effect in
    /// `DateRangePickerInputValue` and the segment-driven `DateElement` for
    /// the start side) must go through, mirroring the guard
    /// `DatePickerContext::set_date` / `DateRangePickerContext::set_range`
    /// already use one level up. Without it, `start_date` and
    /// `DateElement`'s own `year_value`/`month_value`/`day_value` echo an
    /// unchanged value back and forth forever the instant all three
    /// segments first resolve (`DateElement`'s "sync down from
    /// selected_date" effect writes the segment signals, whose "sync up via
    /// on_date_change" effect writes this signal right back) -- see
    /// dev-docs/backlog.md's range-hang account for the full trace and the
    /// live reproduction (CPU pinned, RSS growing ~4MB/s, until the tab
    /// crashes) that confirmed it.
    fn set_start_date(&mut self, date: Option<Date>) {
        if *self.start_date.peek() != date {
            self.start_date.set(date);
        }
    }

    /// See [`Self::set_start_date`]; the same choke point for `end_date`.
    fn set_end_date(&mut self, date: Option<Date>) {
        if *self.end_date.peek() != date {
            self.end_date.set(date);
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct DateElementProps {
    /// The start index (used for focus)
    #[props(default = 0)]
    pub start_index: usize,

    /// The selected date
    pub selected_date: ReadSignal<Option<Date>>,

    /// Callback when selected date changes
    #[props(default)]
    pub on_date_change: Callback<Option<Date>>,

    /// Callback when display day placeholder
    #[props(default = Callback::new(|_| "D".to_string()))]
    pub on_format_day_placeholder: Callback<(), String>,

    /// Callback when display month placeholder
    #[props(default = Callback::new(|_| "M".to_string()))]
    pub on_format_month_placeholder: Callback<(), String>,

    /// Callback when display year placeholder
    #[props(default = Callback::new(|_| "Y".to_string()))]
    pub on_format_year_placeholder: Callback<(), String>,

    /// Locale-aware month name for the month segment's `aria-valuetext`
    /// (backlog row 84 finding 2). Defaults to `Month::to_string()` (plain
    /// English), matching `DatePickerCalendarProps::on_format_month`'s own
    /// default for the calendar grid -- pass e.g. `dioxus_i18n`'s `tid!` for
    /// a translated name, the same way the calendar side already can.
    #[props(default = Callback::new(|month: Month| month.to_string()))]
    pub on_format_month: Callback<Month, String>,

    /// The children of the date element.
    #[props(default)]
    pub children: Option<Element>,
}

#[component]
fn DateElement(props: DateElementProps) -> Element {
    let ctx = use_context::<BaseDatePickerContext>();
    let selected_date = props.selected_date.peek().cloned();

    let mut day_value = use_signal(move || selected_date.map(|date| date.day()));
    let mut month_value = use_signal(move || selected_date.map(|date| date.month() as u8));
    let mut year_value = use_signal(move || selected_date.map(|date| date.year()));

    use_effect(move || {
        // Sync down from `props.selected_date` into the three editable
        // segment signals -- but only where the decomposed value actually
        // differs from what the segment already holds. `on_date_change`
        // below re-derives `props.selected_date` from these same three
        // signals and calls back up through it every time all three
        // resolve; for a `DateRangePicker` side, that callback writes
        // straight back into the very signal this effect reads
        // (`DateRangeInputContext::set_start_date`/`set_end_date`). An
        // unconditional `.set()` here (the previous behavior) closes that
        // into an unbounded cycle the instant a date first completes, even
        // though `set_start_date`/`set_end_date` themselves already
        // no-op an unchanged value -- Dioxus's `Signal::set` marks a
        // signal dirty (and reschedules every subscriber) regardless of
        // whether the new value equals the old one, so the *other* half of
        // a two-way sync has to refuse the redundant write too, or the
        // "no-op" write still wakes this effect for nothing. See
        // dev-docs/backlog.md's range-hang account.
        let date = (props.selected_date)();
        let new_year = date.map(|d| d.year());
        if *year_value.peek() != new_year {
            year_value.set(new_year);
        }
        let new_month = date.map(|d| d.month() as u8);
        if *month_value.peek() != new_month {
            month_value.set(new_month);
        }
        let new_day = date.map(|d| d.day());
        if *day_value.peek() != new_day {
            day_value.set(new_day);
        }
    });

    use_effect(move || {
        if let (Some(year), Some(month), Some(day)) = (
            year_value(),
            month_value().and_then(|m| Month::try_from(m).ok()),
            day_value(),
        ) {
            if let Some(date) = Date::from_calendar_date(year, month, day)
                .ok()
                .filter(|date| ctx.enabled_date_range.contains(*date))
                .filter(|date| ctx.available_ranges.read().valid_interval(*date))
            {
                props.on_date_change.call(Some(date));
            }
        }
    });

    use_context_provider(|| DateElementContext {
        start_index: props.start_index,
        year_value,
        month_value,
        day_value,
        on_format_day_placeholder: props.on_format_day_placeholder,
        on_format_month_placeholder: props.on_format_month_placeholder,
        on_format_year_placeholder: props.on_format_year_placeholder,
        on_format_month: props.on_format_month,
    });

    let children = props.children.unwrap_or_else(|| {
        rsx! {
            DatePickerYearSegment {}
            DatePickerSeparator {}
            DatePickerMonthSegment {}
            DatePickerSeparator {}
            DatePickerDaySegment {}
        }
    });

    rsx! {
        {children}
    }
}

/// The editable date value for a single date picker input.
#[component]
pub fn DatePickerInputValue(props: DatePickerInputValueProps) -> Element {
    let mut base_ctx = use_context::<BaseDatePickerContext>();
    let mut ctx = use_context::<DatePickerContext>();

    rsx! {
        DateElement {
            selected_date: ctx.selected_date,
            on_date_change: move |date| {
                ctx.set_date(date);
                base_ctx.open.set(false);
            },
            on_format_day_placeholder: props.on_format_day_placeholder,
            on_format_month_placeholder: props.on_format_month_placeholder,
            on_format_year_placeholder: props.on_format_year_placeholder,
            on_format_month: props.on_format_month,
            children: props.children,
        }
    }
}

/// The editable date range value for a date range picker input.
#[component]
pub fn DateRangePickerInputValue(props: DateRangePickerInputValueProps) -> Element {
    let base_ctx = use_context::<BaseDatePickerContext>();
    let mut ctx = use_context::<DateRangePickerContext>();
    let selected_range = ctx.date_range.peek().cloned();

    let start_date = use_signal(move || selected_range.map(|range| range.start()));
    let end_date = use_signal(move || selected_range.map(|range| range.end()));
    // Built once, up front, so both this component's own sync-down effect
    // and every `DateElement` child (via the context provider below) write
    // `start_date`/`end_date` through the exact same guarded choke point --
    // see `DateRangeInputContext::set_start_date`'s doc comment for why an
    // unconditional write here forms an unbounded cycle with `DateElement`'s
    // own effects the instant a range first completes.
    let mut range_ctx = DateRangeInputContext {
        start_date,
        end_date,
        on_format_day_placeholder: props.on_format_day_placeholder,
        on_format_month_placeholder: props.on_format_month_placeholder,
        on_format_year_placeholder: props.on_format_year_placeholder,
        on_format_month: props.on_format_month,
    };

    use_effect(move || {
        let date_range = ctx.date_range.cloned();
        range_ctx.set_start_date(date_range.map(|r| r.start()));
        range_ctx.set_end_date(date_range.map(|r| r.end()));
    });

    use_effect(move || {
        if let (Some(start), Some(end)) = (start_date(), end_date()) {
            // force auto validation for input range
            if end < start {
                return;
            }

            // checking non-contiguous ranges
            if base_ctx
                .available_ranges
                .read()
                .available_range(start, base_ctx.enabled_date_range)
                .is_some_and(|r| r.contains(end))
            {
                let range = Some(DateRange::new(start, end));
                ctx.set_range(range);
            }
        };
    });

    use_context_provider(|| range_ctx);

    let children = props.children.unwrap_or_else(|| {
        rsx! {
            DateRangePickerStartValue {}
            DatePickerSeparator {
                symbol: '—',
            }
            DateRangePickerEndValue {}
        }
    });

    rsx! {
        {children}
    }
}

/// The editable start date value in a range picker input.
#[component]
pub fn DateRangePickerStartValue(props: DateRangePickerStartValueProps) -> Element {
    let mut ctx = use_context::<DateRangeInputContext>();

    rsx! {
        DateElement {
            selected_date: ctx.start_date,
            on_date_change: move |date| ctx.set_start_date(date),
            on_format_day_placeholder: ctx.on_format_day_placeholder,
            on_format_month_placeholder: ctx.on_format_month_placeholder,
            on_format_year_placeholder: ctx.on_format_year_placeholder,
            on_format_month: ctx.on_format_month,
            children: props.children,
        }
    }
}

/// The editable end date value in a range picker input.
#[component]
pub fn DateRangePickerEndValue(props: DateRangePickerEndValueProps) -> Element {
    let mut ctx = use_context::<DateRangeInputContext>();

    rsx! {
        DateElement {
            start_index: 3,
            selected_date: ctx.end_date,
            on_date_change: move |date| ctx.set_end_date(date),
            on_format_day_placeholder: ctx.on_format_day_placeholder,
            on_format_month_placeholder: ctx.on_format_month_placeholder,
            on_format_year_placeholder: ctx.on_format_year_placeholder,
            on_format_month: ctx.on_format_month,
            children: props.children,
        }
    }
}

/// The props for the [`DatePickerInput`] component
#[derive(Props, Clone, PartialEq)]
pub struct DatePickerInputProps {
    /// Callback when display day placeholder
    #[props(default = Callback::new(|_| "D".to_string()))]
    pub on_format_day_placeholder: Callback<(), String>,

    /// Callback when display month placeholder
    #[props(default = Callback::new(|_| "M".to_string()))]
    pub on_format_month_placeholder: Callback<(), String>,

    /// Callback when display year placeholder
    #[props(default = Callback::new(|_| "Y".to_string()))]
    pub on_format_year_placeholder: Callback<(), String>,

    /// See `DateElementProps::on_format_month` (backlog row 84 finding 2).
    /// Only applies to the default children (a caller-supplied `children`
    /// renders its own segments directly and must thread this itself, the
    /// same pre-existing limitation the placeholder callbacks above have).
    #[props(default = Callback::new(|month: Month| month.to_string()))]
    pub on_format_month: Callback<Month, String>,

    /// Additional attributes for the value element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the date picker element
    #[props(default)]
    pub children: Option<Element>,
}

/// # DatePickerInput
///
/// The input element for the [`DatePicker`] component which allow users to enter a date value.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::{calendar::Calendar, date_picker::*, popover::*, ContentAlign};
/// use time::Date;
/// #[component]
/// fn Demo() -> Element {
///    let mut selected_date = use_signal(|| None::<Date>);
///    rsx! {
///        div {
///            DatePicker {
///                selected_date: selected_date(),
///                on_value_change: move |date| {
///                    tracing::info!("Date changed to: {date:?}");
///                    selected_date.set(date);
///               },
///                DatePickerPopover {
///                    DatePickerInput {
///                        PopoverTrigger {
///                            "Select date"
///                        }
///                        PopoverContent {
///                            align: ContentAlign::End,
///                            DatePickerCalendar {
///                                calendar: Calendar,
///                            }
///                        }
///                    }
///                }
///            }
///        }
///    }
///}
/// ```
#[component]
pub fn DatePickerInput(props: DatePickerInputProps) -> Element {
    let children = props.children.unwrap_or_else(|| {
        rsx! {
            DatePickerInputValue {
                on_format_day_placeholder: props.on_format_day_placeholder,
                on_format_month_placeholder: props.on_format_month_placeholder,
                on_format_year_placeholder: props.on_format_year_placeholder,
                on_format_month: props.on_format_month,
            }
        }
    });

    rsx! {
        div { ..props.attributes,
            {children}
        }
    }
}

/// # DateRangePickerInput
///
/// The input element for the [`DateRangePicker`] component which allow users to enter a date range.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::{calendar::{DateRange, RangeCalendar}, date_picker::*, popover::*, ContentAlign};
/// #[component]
/// fn Demo() -> Element {
///    let mut selected_range = use_signal(|| None::<DateRange>);
///    rsx! {
///        div {
///            DateRangePicker {
///                selected_range: selected_range(),
///                on_range_change: move |range| {
///                    tracing::info!("Selected range: {:?}", range);
///                    selected_range.set(range);
///               },
///                DatePickerPopover {
///                    DateRangePickerInput {
///                        PopoverTrigger {
///                            "Select date"
///                        }
///                        PopoverContent {
///                            align: ContentAlign::End,
///                            DateRangePickerCalendar {
///                                calendar: RangeCalendar,
///                            }
///                        }
///                    }
///                }
///            }
///        }
///    }
///}
/// ```
#[component]
pub fn DateRangePickerInput(props: DatePickerInputProps) -> Element {
    let children = props.children.unwrap_or_else(|| {
        rsx! {
            DateRangePickerInputValue {
                on_format_day_placeholder: props.on_format_day_placeholder,
                on_format_month_placeholder: props.on_format_month_placeholder,
                on_format_year_placeholder: props.on_format_year_placeholder,
                on_format_month: props.on_format_month,
            }
        }
    });

    rsx! {
        div { ..props.attributes,
            {children}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::date;

    #[component]
    fn ControlledDatePicker() -> Element {
        rsx! {
            DatePicker {
                selected_date: Some(date!(2026 - 05 - 07)),
                DatePickerInput {}
            }
        }
    }

    #[component]
    fn ControlledDateRangePicker() -> Element {
        rsx! {
            DateRangePicker {
                selected_range: Some(DateRange::new(date!(2026 - 05 - 07), date!(2026 - 05 - 11))),
                DateRangePickerInput {}
            }
        }
    }

    #[test]
    fn date_picker_input_renders_controlled_date_on_first_render() {
        let mut dom = VirtualDom::new(ControlledDatePicker);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        assert!(html.contains("2026"));
        assert!(html.contains("05"));
        assert!(html.contains("07"));
        assert!(!html.contains("YYYY"));
        assert!(!html.contains("MM"));
        assert!(!html.contains("DD"));
    }

    #[component]
    fn UntouchedDatePicker() -> Element {
        rsx! {
            DatePicker {
                DatePickerInput {}
            }
        }
    }

    #[test]
    fn untouched_date_segments_omit_aria_valuenow_and_aria_valuetext() {
        // backlog row 84 findings 2/3/6: before any date is chosen, every
        // segment's `props.value` is `None` -- `aria-valuenow` (and the
        // month segment's new `aria-valuetext`) must be entirely absent
        // rather than substituting `default`/a formatted default. RED on
        // the unmodified tree: `now_value` used to be `(props.value)()
        // .unwrap_or(props.default)`, so `aria-valuenow="<today's year>"`
        // etc. were always present here.
        let mut dom = VirtualDom::new(UntouchedDatePicker);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        assert!(
            !html.contains("aria-valuenow"),
            "an untouched segment must not report any aria-valuenow: {html}"
        );
        assert!(
            !html.contains("aria-valuetext"),
            "an untouched month segment must not report any aria-valuetext: {html}"
        );
        // Sanity: the segments really did render (the assertions above
        // aren't vacuously true because nothing rendered at all).
        assert!(html.contains(r#"aria-label="year""#));
        assert!(html.contains("YYYY"));
    }

    #[test]
    fn touched_date_segments_report_aria_valuenow_and_month_reports_a_locale_aware_aria_valuetext()
    {
        // Counterpart to `untouched_date_segments_omit_aria_valuenow_and_
        // aria_valuetext` above: once a segment actually holds a value
        // (here, `ControlledDatePicker`'s fixed 2026-05-07), `aria-valuenow`
        // must report it, and the month segment's `aria-valuetext` must be
        // the default formatter's locale-aware month name (backlog row 84
        // finding 2 -- `Month::to_string()` by default, "May" for month 5),
        // not merely "some non-empty string". RED on the unmodified tree:
        // `aria-valuetext` never rendered at all (no such attribute
        // existed), so `html.contains(r#"aria-valuetext="May""#)` failed.
        let mut dom = VirtualDom::new(ControlledDatePicker);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        assert!(
            html.contains(r#"aria-valuenow="2026""#),
            "year segment must report its value: {html}"
        );
        assert!(
            html.contains(r#"aria-valuenow="5""#),
            "month segment must report its raw numeric value too: {html}"
        );
        assert!(
            html.contains(r#"aria-valuenow="7""#),
            "day segment must report its value: {html}"
        );
        assert!(
            html.contains(r#"aria-valuetext="May""#),
            "month segment must report a locale-aware name via aria-valuetext: {html}"
        );
        // Year and day are plain numbers -- no aria-valuetext for either
        // (backlog row 84 finding 2 is scoped to the month segment only,
        // per the APG spinbutton pattern's own rationale: a bare number is
        // "user-friendly" for a day-of-month or a year, unlike a month
        // index). `year value_text` is fixed structurally (only
        // `DatePickerMonthSegment` ever sets `DateSegmentProps::value_text`)
        // rather than by an attribute count here, since `aria-valuetext`
        // appearing exactly once total already proves it's not also on
        // year/day.
        assert_eq!(html.matches("aria-valuetext").count(), 1, "{html}");
    }

    #[test]
    fn date_segment_renders_aria_label_and_no_dangling_aria_labelledby() {
        // Regression guard for the `aria-labelledby` fix -- SSR-level rather
        // than the browser-level `playwright/date-picker.spec.ts` check, so
        // this runs (and was run, red-then-green) without needing a `dx
        // serve --web` build: this repo's own sandbox hit severe, batch-
        // wide disk pressure while this lane worked (see this lane's
        // report), so this cargo-only check is the primary evidence for the
        // RSX shape here, with the browser-level test as the fuller
        // (still-current-code, not yet server-verified) companion.
        let mut dom = VirtualDom::new(ControlledDatePicker);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        assert!(
            html.contains(r#"aria-label="year""#),
            "aria-label must still be present: {html}"
        );
        assert!(
            !html.contains("aria-labelledby"),
            "aria-labelledby must not be rendered at all (no visible label element exists to reference): {html}"
        );
    }

    #[test]
    fn date_range_picker_input_renders_controlled_range_on_first_render() {
        let mut dom = VirtualDom::new(ControlledDateRangePicker);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        assert!(html.contains("2026"));
        assert!(html.contains("05"));
        assert!(html.contains("07"));
        assert!(html.contains("11"));
        assert!(!html.contains("YYYY"));
        assert!(!html.contains("MM"));
        assert!(!html.contains("DD"));
    }

    // -----------------------------------------------------------------
    // `date_segment_key_effects` and friends (docs/mutants-baseline.md fix #7)
    //
    // These exercise the pure decision logic extracted from `DateSegment`'s
    // `onkeydown` handler (mechanical, behavior-preserving extraction --
    // precedent: `move_interaction.rs`'s `MoveEvent::from_keyboard`).
    // -----------------------------------------------------------------

    /// Test-only convenience wrapper matching the pre-refactor flat argument
    /// list, so each case below stays a single readable call.
    #[allow(clippy::too_many_arguments)]
    fn key_effects(
        key: &Key,
        ctrl: bool,
        meta: bool,
        alt: bool,
        current_text: &str,
        max_length: usize,
        reset: bool,
        current_value: Option<i32>,
        default: i32,
        min: i32,
        max: i32,
    ) -> Vec<DateSegmentEffect<i32>> {
        // `wrap: true` for every pre-existing call site below: all of them
        // predate the wrap-vs-clamp split (backlog row 84 finding 4) and
        // exercise the month/day-shaped ("cyclical") behavior, which is
        // unchanged by that fix -- see `key_effects_clamped` for the new,
        // year-shaped (`wrap: false`) cases.
        date_segment_key_effects(
            key,
            SegmentKeyModifiers { ctrl, meta, alt },
            SegmentTextState {
                current_text,
                max_length,
                reset,
            },
            SegmentValueBounds {
                current_value,
                default,
                min,
                max,
                wrap: true,
            },
        )
    }

    /// Like [`key_effects`], but with `wrap: false` -- the year segment's
    /// clamp-at-the-bound policy (backlog row 84 finding 4).
    #[allow(clippy::too_many_arguments)]
    fn key_effects_clamped(
        key: &Key,
        ctrl: bool,
        meta: bool,
        alt: bool,
        current_text: &str,
        max_length: usize,
        reset: bool,
        current_value: Option<i32>,
        default: i32,
        min: i32,
        max: i32,
    ) -> Vec<DateSegmentEffect<i32>> {
        date_segment_key_effects(
            key,
            SegmentKeyModifiers { ctrl, meta, alt },
            SegmentTextState {
                current_text,
                max_length,
                reset,
            },
            SegmentValueBounds {
                current_value,
                default,
                min,
                max,
                wrap: false,
            },
        )
    }

    fn no_modifiers() -> (bool, bool, bool) {
        (false, false, false)
    }

    #[test]
    fn roll_value_wraps_at_bounds() {
        assert_eq!(roll_value(5, 0, 10), 5, "in range is unchanged");
        assert_eq!(roll_value(-1, 0, 10), 10, "below min wraps to max");
        assert_eq!(roll_value(11, 0, 10), 0, "above max wraps to min");
        assert_eq!(roll_value(0, 0, 10), 0, "exactly at min stays");
        assert_eq!(roll_value(10, 0, 10), 10, "exactly at max stays");
    }

    #[test]
    fn key_effects_digit_within_bounds_just_emits_the_new_value() {
        let (ctrl, meta, alt) = no_modifiers();
        let effects = key_effects(
            &Key::Character("1".to_string()),
            ctrl,
            meta,
            alt,
            "0", // current text "0" -> becomes "01"; a 3rd digit ("010"=10) would stay in bounds
            2,
            false,
            Some(0),
            0,
            0,
            31,
        );

        assert_eq!(
            effects,
            vec![
                DateSegmentEffect::EmitValue(Some(1)),
                DateSegmentEffect::PreventDefault,
                DateSegmentEffect::StopPropagation,
            ]
        );
    }

    #[test]
    fn key_effects_digit_that_fills_max_length_advances_focus_before_emitting() {
        // Typing "5" onto day-segment text "3" with max 31: "35" clamps to
        // 31, and since (35+"0"="350" also clamps/parses to > max) focus
        // advances to the next segment. Order matters: focus before emit.
        let (ctrl, meta, alt) = no_modifiers();
        let effects = key_effects(
            &Key::Character("5".to_string()),
            ctrl,
            meta,
            alt,
            "3",
            2,
            false,
            Some(3),
            1,
            1,
            31,
        );

        assert_eq!(
            effects,
            vec![
                DateSegmentEffect::FocusNext,
                DateSegmentEffect::EmitValue(Some(31)),
                DateSegmentEffect::PreventDefault,
                DateSegmentEffect::StopPropagation,
            ]
        );
    }

    #[test]
    fn key_effects_digit_below_min_never_advances_focus_even_if_probe_overflows() {
        // value=3 is below min=5 (in_range must be false), even though the "add
        // one more digit" probe ("30") does exceed max=9. This distinguishes
        // `in_range && new_value > max` from a mistaken `||`.
        let (ctrl, meta, alt) = no_modifiers();
        let effects = key_effects(
            &Key::Character("3".to_string()),
            ctrl,
            meta,
            alt,
            "",
            2,
            false,
            None,
            5,
            5,
            9,
        );
        assert_eq!(
            effects,
            vec![
                DateSegmentEffect::EmitValue(Some(3)),
                DateSegmentEffect::PreventDefault,
                DateSegmentEffect::StopPropagation,
            ],
            "below-min values must never trigger the focus-advance heuristic"
        );
    }

    #[test]
    fn key_effects_digit_probe_exactly_at_max_does_not_advance_focus() {
        // value=1 is in range [0,10], and the "add one more digit" probe ("10")
        // equals max exactly (not greater than it). This distinguishes
        // `new_value > max` from a mistaken `>=`.
        let (ctrl, meta, alt) = no_modifiers();
        let effects = key_effects(
            &Key::Character("1".to_string()),
            ctrl,
            meta,
            alt,
            "",
            2,
            false,
            None,
            0,
            0,
            10,
        );
        assert_eq!(
            effects,
            vec![
                DateSegmentEffect::EmitValue(Some(1)),
                DateSegmentEffect::PreventDefault,
                DateSegmentEffect::StopPropagation,
            ],
            "a probe that lands exactly on max must not advance focus"
        );
    }

    #[test]
    fn key_effects_digit_resets_buffer_when_at_max_length_or_reset_pending() {
        let (ctrl, meta, alt) = no_modifiers();

        // Text is already at max_length: starts a fresh buffer instead of appending.
        let effects = key_effects(
            &Key::Character("9".to_string()),
            ctrl,
            meta,
            alt,
            "12",
            2,
            false,
            Some(12),
            1,
            1,
            31,
        );
        assert_eq!(
            effects,
            vec![
                // Restarting from "9" (not "129") means the "add one more digit"
                // overflow probe ("90") also exceeds max, so focus advances too.
                DateSegmentEffect::ConsumeReset,
                DateSegmentEffect::FocusNext,
                DateSegmentEffect::EmitValue(Some(9)),
                DateSegmentEffect::PreventDefault,
                DateSegmentEffect::StopPropagation,
            ],
            "a full buffer restarts rather than appending (would otherwise fail to parse or grow unbounded)"
        );

        // `reset` pending (e.g. segment was just focused): also starts fresh, and
        // reports that the reset flag should be consumed.
        let effects = key_effects(
            &Key::Character("9".to_string()),
            ctrl,
            meta,
            alt,
            "1",
            2,
            true,
            Some(1),
            1,
            1,
            31,
        );
        assert_eq!(
            effects,
            vec![
                DateSegmentEffect::ConsumeReset,
                DateSegmentEffect::FocusNext,
                DateSegmentEffect::EmitValue(Some(9)),
                DateSegmentEffect::PreventDefault,
                DateSegmentEffect::StopPropagation,
            ]
        );
    }

    #[test]
    fn key_effects_digit_ignores_keyboard_shortcuts() {
        for (ctrl, meta, alt) in [
            (true, false, false),
            (false, true, false),
            (false, false, true),
        ] {
            let effects = key_effects(
                &Key::Character("5".to_string()),
                ctrl,
                meta,
                alt,
                "1",
                2,
                false,
                Some(1),
                0,
                0,
                31,
            );
            assert_eq!(
                effects,
                Vec::new(),
                "ctrl/meta/alt+digit must pass through as a shortcut, untouched"
            );
        }
    }

    #[test]
    fn key_effects_non_digit_character_still_blocks_default_but_does_not_edit() {
        let (ctrl, meta, alt) = no_modifiers();
        let effects = key_effects(
            &Key::Character("x".to_string()),
            ctrl,
            meta,
            alt,
            "1",
            2,
            false,
            Some(1),
            0,
            0,
            31,
        );
        assert_eq!(
            effects,
            vec![
                DateSegmentEffect::PreventDefault,
                DateSegmentEffect::StopPropagation,
            ]
        );
    }

    #[test]
    fn key_effects_backspace_pops_last_character() {
        let (ctrl, meta, alt) = no_modifiers();
        let effects = key_effects(
            &Key::Backspace,
            ctrl,
            meta,
            alt,
            "12",
            2,
            false,
            Some(12),
            0,
            0,
            31,
        );
        assert_eq!(effects, vec![DateSegmentEffect::EmitValue(Some(1))]);
    }

    #[test]
    fn key_effects_backspace_with_ctrl_or_meta_clears_all() {
        let effects = key_effects(
            &Key::Backspace,
            true,
            false,
            false,
            "12",
            2,
            false,
            Some(12),
            0,
            0,
            31,
        );
        assert_eq!(
            effects,
            vec![
                DateSegmentEffect::EmitValue(None),
                DateSegmentEffect::FocusPrevious,
            ],
            "clearing to empty emits None then moves focus back"
        );
    }

    #[test]
    fn key_effects_delete_drops_first_character() {
        let (ctrl, meta, alt) = no_modifiers();
        let effects = key_effects(
            &Key::Delete,
            ctrl,
            meta,
            alt,
            "12",
            2,
            false,
            Some(12),
            0,
            0,
            31,
        );
        assert_eq!(effects, vec![DateSegmentEffect::EmitValue(Some(2))]);
    }

    #[test]
    fn key_effects_arrow_left_and_right_move_focus_only() {
        let (ctrl, meta, alt) = no_modifiers();
        let left = key_effects(
            &Key::ArrowLeft,
            ctrl,
            meta,
            alt,
            "1",
            2,
            false,
            Some(1),
            0,
            0,
            31,
        );
        assert_eq!(left, vec![DateSegmentEffect::FocusPrevious]);

        let right = key_effects(
            &Key::ArrowRight,
            ctrl,
            meta,
            alt,
            "1",
            2,
            false,
            Some(1),
            0,
            0,
            31,
        );
        assert_eq!(right, vec![DateSegmentEffect::FocusNext]);
    }

    #[test]
    fn key_effects_enter_moves_focus_next_and_blocks_default() {
        let (ctrl, meta, alt) = no_modifiers();
        let effects = key_effects(
            &Key::Enter,
            ctrl,
            meta,
            alt,
            "1",
            2,
            false,
            Some(1),
            0,
            0,
            31,
        );
        assert_eq!(
            effects,
            vec![
                DateSegmentEffect::FocusNext,
                DateSegmentEffect::PreventDefault,
                DateSegmentEffect::StopPropagation,
            ]
        );
    }

    #[test]
    fn key_effects_arrow_up_increments_and_wraps_from_max_to_min() {
        let (ctrl, meta, alt) = no_modifiers();
        let effects = key_effects(
            &Key::ArrowUp,
            ctrl,
            meta,
            alt,
            "",
            2,
            false,
            Some(5),
            0,
            0,
            10,
        );
        assert_eq!(effects, vec![DateSegmentEffect::EmitValue(Some(6))]);

        let wrapped = key_effects(
            &Key::ArrowUp,
            ctrl,
            meta,
            alt,
            "",
            2,
            false,
            Some(10),
            0,
            0,
            10,
        );
        assert_eq!(
            wrapped,
            vec![DateSegmentEffect::EmitValue(Some(0))],
            "incrementing past max wraps to min"
        );
    }

    #[test]
    fn key_effects_arrow_down_decrements_and_wraps_from_min_to_max() {
        let (ctrl, meta, alt) = no_modifiers();
        let effects = key_effects(
            &Key::ArrowDown,
            ctrl,
            meta,
            alt,
            "",
            2,
            false,
            Some(5),
            0,
            0,
            10,
        );
        assert_eq!(effects, vec![DateSegmentEffect::EmitValue(Some(4))]);

        let wrapped = key_effects(
            &Key::ArrowDown,
            ctrl,
            meta,
            alt,
            "",
            2,
            false,
            Some(0),
            0,
            0,
            10,
        );
        assert_eq!(
            wrapped,
            vec![DateSegmentEffect::EmitValue(Some(10))],
            "decrementing past min wraps to max"
        );
    }

    #[test]
    fn key_effects_arrow_up_down_clamp_at_bounds_instead_of_wrapping_when_wrap_is_false() {
        // backlog row 84 finding 4: the YEAR segment must clamp, not wrap.
        // `roll_value`'s own wrap test above (`key_effects_arrow_up_
        // increments_and_wraps_from_max_to_min`) is `wrap: true`'s
        // (month/day's) behavior; this is `wrap: false`'s (year's).
        let (ctrl, meta, alt) = no_modifiers();
        let up_at_max = key_effects_clamped(
            &Key::ArrowUp,
            ctrl,
            meta,
            alt,
            "",
            4,
            false,
            Some(2050),
            2000,
            1925,
            2050,
        );
        assert_eq!(
            up_at_max,
            vec![DateSegmentEffect::EmitValue(Some(2050))],
            "incrementing past max clamps at max instead of wrapping to min"
        );

        let down_at_min = key_effects_clamped(
            &Key::ArrowDown,
            ctrl,
            meta,
            alt,
            "",
            4,
            false,
            Some(1925),
            2000,
            1925,
            2050,
        );
        assert_eq!(
            down_at_min,
            vec![DateSegmentEffect::EmitValue(Some(1925))],
            "decrementing past min clamps at min instead of wrapping to max"
        );

        // Away from either bound, clamped and wrapped behavior agree --
        // this isolates the *boundary* as what actually differs.
        let up_in_range = key_effects_clamped(
            &Key::ArrowUp,
            ctrl,
            meta,
            alt,
            "",
            4,
            false,
            Some(2000),
            2000,
            1925,
            2050,
        );
        assert_eq!(up_in_range, vec![DateSegmentEffect::EmitValue(Some(2001))]);
    }

    #[test]
    fn key_effects_arrow_up_down_use_default_when_no_current_value() {
        let (ctrl, meta, alt) = no_modifiers();
        let up = key_effects(&Key::ArrowUp, ctrl, meta, alt, "", 2, false, None, 7, 0, 10);
        assert_eq!(up, vec![DateSegmentEffect::EmitValue(Some(7))]);

        let down = key_effects(
            &Key::ArrowDown,
            ctrl,
            meta,
            alt,
            "",
            2,
            false,
            None,
            7,
            0,
            10,
        );
        assert_eq!(down, vec![DateSegmentEffect::EmitValue(Some(7))]);
    }

    // -----------------------------------------------------------------
    // Home/End (APG spinbutton-pattern.html #keyboard_interaction --
    // playwright/date-picker.spec.ts's "Home/End" describe block is the
    // browser-level regression guard for the same fix; these are its pure-
    // function-level counterpart, mirroring every other `key_effects_*`
    // test above).
    // -----------------------------------------------------------------

    #[test]
    fn key_effects_home_sets_the_minimum_regardless_of_current_value() {
        let (ctrl, meta, alt) = no_modifiers();
        let effects = key_effects(
            &Key::Home,
            ctrl,
            meta,
            alt,
            "15",
            2,
            false,
            Some(15),
            1,
            1,
            31,
        );
        assert_eq!(
            effects,
            vec![DateSegmentEffect::EmitValue(Some(1))],
            "Home sets the value to the minimum, per the cited APG rule"
        );

        // Also true with no current value yet (the field still has a
        // well-defined `min`, so the rule still applies).
        let effects = key_effects(&Key::Home, ctrl, meta, alt, "", 2, false, None, 1, 1, 31);
        assert_eq!(effects, vec![DateSegmentEffect::EmitValue(Some(1))]);
    }

    #[test]
    fn key_effects_end_sets_the_maximum_regardless_of_current_value() {
        let (ctrl, meta, alt) = no_modifiers();
        let effects = key_effects(
            &Key::End,
            ctrl,
            meta,
            alt,
            "15",
            2,
            false,
            Some(15),
            1,
            1,
            31,
        );
        assert_eq!(
            effects,
            vec![DateSegmentEffect::EmitValue(Some(31))],
            "End sets the value to the maximum, per the cited APG rule"
        );

        let effects = key_effects(&Key::End, ctrl, meta, alt, "", 2, false, None, 1, 1, 31);
        assert_eq!(effects, vec![DateSegmentEffect::EmitValue(Some(31))]);
    }

    #[test]
    fn key_effects_unhandled_key_produces_no_effects() {
        let (ctrl, meta, alt) = no_modifiers();
        let effects = key_effects(&Key::Tab, ctrl, meta, alt, "1", 2, false, Some(1), 0, 0, 31);
        assert_eq!(effects, Vec::new());
    }

    // -----------------------------------------------------------------
    // Month/day segment min-max guards (docs/mutants-baseline.md fix #7)
    // -----------------------------------------------------------------

    #[test]
    fn month_bounds_clamp_only_at_the_boundary_year() {
        let min_date = date!(2020 - 03 - 01);
        let max_date = date!(2030 - 09 - 01);

        assert_eq!(
            month_bounds_for_year(Some(2020), 2020, 2030, min_date, max_date),
            (3, 12),
            "min-boundary year clamps the min month"
        );
        assert_eq!(
            month_bounds_for_year(Some(2030), 2020, 2030, min_date, max_date),
            (1, 9),
            "max-boundary year clamps the max month"
        );
        assert_eq!(
            month_bounds_for_year(Some(2025), 2020, 2030, min_date, max_date),
            (1, 12),
            "any other year allows the full range"
        );
        assert_eq!(
            month_bounds_for_year(None, 2020, 2030, min_date, max_date),
            (1, 12),
            "no year selected yet allows the full range"
        );
    }

    #[test]
    fn day_bounds_clamp_only_at_the_boundary_year_and_month() {
        let min_date = date!(2020 - 03 - 10);
        let max_date = date!(2030 - 09 - 20);

        assert_eq!(
            day_bounds_for_year_month(Some(2020), Some(3), 2020, 2030, min_date, max_date),
            (10, 31),
            "min-boundary year+month clamps the min day"
        );
        assert_eq!(
            day_bounds_for_year_month(Some(2030), Some(9), 2020, 2030, min_date, max_date),
            (1, 20),
            "max-boundary year+month clamps the max day"
        );
        // Not at a boundary: max day falls back to the target month's actual length.
        assert_eq!(
            day_bounds_for_year_month(Some(2024), Some(2), 2020, 2030, min_date, max_date),
            (1, 29),
            "falls back to the real month length (2024 is a leap year)"
        );
        assert_eq!(
            day_bounds_for_year_month(Some(2020), Some(3), 2020, 2030, min_date, max_date).0,
            10
        );
        assert_eq!(
            day_bounds_for_year_month(None, None, 2020, 2030, min_date, max_date),
            (1, 31),
            "no year/month selected yet allows the full range"
        );

        // Partial boundary matches (year matches but month doesn't, or vice versa)
        // must NOT clamp -- both year AND month must match the boundary.
        assert_eq!(
            day_bounds_for_year_month(Some(2020), Some(5), 2020, 2030, min_date, max_date).0,
            1,
            "min_year matches but month doesn't: falls through to the unclamped default"
        );
        assert_eq!(
            day_bounds_for_year_month(Some(2025), Some(3), 2020, 2030, min_date, max_date).0,
            1,
            "min_date's month matches but year doesn't: falls through to the unclamped default"
        );
        assert_eq!(
            day_bounds_for_year_month(Some(2030), Some(5), 2020, 2030, min_date, max_date).1,
            Month::May.length(2030),
            "max_year matches but month doesn't: falls through to the real month length"
        );
        assert_eq!(
            day_bounds_for_year_month(Some(2025), Some(9), 2020, 2030, min_date, max_date).1,
            Month::September.length(2025),
            "max_date's month matches but year doesn't: falls through to the real month length"
        );
    }

    // -----------------------------------------------------------------
    // Context accessors (bonus: same file, same mutants-baseline gap)
    // -----------------------------------------------------------------

    /// Run a closure inside a Dioxus runtime context so that Signal/Callback
    /// APIs are available (mirrors `virtual/virtualizer.rs`'s `with_runtime`).
    fn with_runtime(f: impl Fn() + 'static) {
        let result = std::rc::Rc::new(std::cell::Cell::new(false));
        let result2 = result.clone();
        let test_fn = std::rc::Rc::new(f);
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
        test_fn: std::rc::Rc<dyn Fn()>,
        result: std::rc::Rc<std::cell::Cell<bool>>,
    }

    impl PartialEq for TestHarnessProps {
        fn eq(&self, _: &Self) -> bool {
            true
        }
    }

    #[test]
    fn date_picker_context_set_date_only_emits_on_change() {
        with_runtime(|| {
            let selected = use_signal(|| Some(date!(2024 - 01 - 01)));
            let emitted = use_signal(Vec::<Option<Date>>::new);
            let mut ctx = DatePickerContext {
                selected_date: selected.into(),
                on_value_change: Callback::new(move |d| {
                    let mut emitted = emitted;
                    emitted.write().push(d);
                }),
            };

            // Same value: no-op.
            ctx.set_date(Some(date!(2024 - 01 - 01)));
            assert_eq!(emitted(), Vec::new());

            // Different value: emits.
            ctx.set_date(Some(date!(2024 - 06 - 15)));
            assert_eq!(emitted(), vec![Some(date!(2024 - 06 - 15))]);
        });
    }

    #[test]
    fn date_range_picker_context_set_range_only_emits_on_change() {
        with_runtime(|| {
            let initial = Some(DateRange::new(date!(2024 - 01 - 01), date!(2024 - 01 - 05)));
            let date_range = use_signal(move || initial);
            let emitted = use_signal(Vec::<Option<DateRange>>::new);
            let mut ctx = DateRangePickerContext {
                date_range: date_range.into(),
                set_selected_range: Callback::new(move |r| {
                    let mut emitted = emitted;
                    emitted.write().push(r);
                }),
            };

            ctx.set_range(initial);
            assert_eq!(emitted(), Vec::new(), "identical range must not re-emit");

            let changed = Some(DateRange::new(date!(2024 - 02 - 01), date!(2024 - 02 - 05)));
            ctx.set_range(changed);
            assert_eq!(emitted(), vec![changed]);
        });
    }
}

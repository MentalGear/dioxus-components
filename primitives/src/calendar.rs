//! Defines the [`Calendar`] component and its sub-components, which provide a calendar interface with date selection and navigation.

use dioxus::{
    core::{current_scope_id, ScopeId},
    prelude::*,
};
use std::{
    collections::HashSet,
    fmt::{self, Display},
    rc::Rc,
};

use dioxus_attributes::attributes;
use dioxus_core::AttributeValue::{Bool, Text};
use time::{ext::NumericalDuration, macros::date, Date, Month, OffsetDateTime, Weekday};

use crate::{
    date_picker::DefaultCalendarProps,
    direction::{use_direction, Direction, HorizontalNav},
    merge_attributes, use_effect_cleanup, LocalDateExt as _,
};

// A collection of [`Weekday`]s stored as a single byte
// Implemented as a bitmask where bits 1-7 correspond to Monday-Sunday
#[derive(Clone, Copy)]
struct WeekdaySet(u8); // the 8-th bit is always 0

impl WeekdaySet {
    // Get the first day in the collection, starting from Monday
    // Returns `None` if the collection is empty
    const fn first(self) -> Option<Weekday> {
        if self.is_empty() {
            return None;
        }

        // Find the first non-zero bit
        Some(Weekday::Monday.nth_next(self.0.trailing_zeros() as u8))
    }

    // Create a `WeekdaySet` from a single [`Weekday`]
    const fn single(weekday: Weekday) -> Self {
        Self(1 << weekday.number_days_from_monday())
    }

    // Iterate over the [`Weekday`]s in the collection starting from a given day
    // Wraps around from Sunday to Monday if necessary
    const fn iter(self, start: Weekday) -> WeekdaySetIter {
        WeekdaySetIter { days: self, start }
    }

    // Returns `true` if the collection is empty
    const fn is_empty(self) -> bool {
        self.0 == 0
    }

    // Split the collection in two at the given day. Returns a tuple `(before, after)`
    // `before` contains all days starting from Monday up to but NOT including `weekday`
    // `after` contains all days starting from `weekday` up to and including Sunday
    const fn split_at(self, weekday: Weekday) -> (Self, Self) {
        let days_after = 0b1000_0000 - Self::single(weekday).0;
        let days_before = days_after ^ 0b0111_1111;
        (Self(self.0 & days_before), Self(self.0 & days_after))
    }

    // Returns `true` if the collection contains the given day
    const fn contains(self, day: Weekday) -> bool {
        self.0 & Self::single(day).0 != 0
    }

    // Removes a day from the collection
    // Returns `true` if the collection did contain the day
    fn remove(&mut self, day: Weekday) -> bool {
        if self.contains(day) {
            self.0 &= !Self::single(day).0;
            return true;
        }

        false
    }
}

// An iterator over a collection of weekdays, starting from a given day
struct WeekdaySetIter {
    days: WeekdaySet,
    start: Weekday,
}

impl Iterator for WeekdaySetIter {
    type Item = Weekday;

    fn next(&mut self) -> Option<Self::Item> {
        if self.days.is_empty() {
            return None;
        }

        let (before, after) = self.days.split_at(self.start);
        let days = if after.is_empty() { before } else { after };

        let next = days.first().expect("the collection is not empty");
        self.days.remove(next);
        Some(next)
    }
}

pub(crate) fn weekday_abbreviation(weekday: Weekday) -> &'static str {
    match weekday {
        Weekday::Monday => "Mo",
        Weekday::Tuesday => "Tu",
        Weekday::Wednesday => "We",
        Weekday::Thursday => "Th",
        Weekday::Friday => "Fr",
        Weekday::Saturday => "Sa",
        Weekday::Sunday => "Su",
    }
}

// The number of days since the first weekday of current date
fn days_since(date: Date, weekday: Weekday) -> i64 {
    let lhs = date.replace_day(1).unwrap().weekday() as i64;
    let rhs = weekday as i64;
    if lhs < rhs {
        7 + lhs - rhs
    } else {
        lhs - rhs
    }
}

fn next_month(date: Date) -> Option<Date> {
    let next_month = date.month().next();
    let last_day = next_month.length(date.year());
    // Clamp the day to the length of the next month
    let current_day = date.day();
    let new_day = current_day.min(last_day);
    Date::from_calendar_date(
        date.year() + if next_month == Month::January { 1 } else { 0 },
        next_month,
        new_day,
    )
    .ok()
}

fn previous_month(date: Date) -> Option<Date> {
    let previous_month = date.month().previous();
    let last_day = previous_month.length(date.year());
    // Clamp the day to the length of the previous month
    let current_day = date.day();
    let new_day = current_day.min(last_day);
    Date::from_calendar_date(
        date.year()
            + if previous_month == Month::December {
                -1
            } else {
                0
            },
        previous_month,
        new_day,
    )
    .ok()
}

fn replace_month(date: Date, month: Month) -> Date {
    let year = date.year();
    let num_days = month.length(year);
    Date::from_calendar_date(year, month, std::cmp::min(date.day(), num_days))
        .expect("invalid or out-of-range date")
}

/// Move forward n months from the given date, handling year transitions
fn nth_month_next(date: Date, n: u8) -> Option<Date> {
    match n {
        0 => Some(date),
        n => {
            let month = date.month();
            let nth_month = month.nth_next(n);
            let year = date.year() + if month > nth_month { 1 } else { 0 };
            let max_day = nth_month.length(year);
            Date::from_calendar_date(year, nth_month, date.day().min(max_day)).ok()
        }
    }
}

/// Move backward n months from the given date, handling year transitions
fn nth_month_previous(date: Date, n: u8) -> Option<Date> {
    match n {
        0 => Some(date),
        n => {
            let month = date.month();
            let nth_month = month.nth_prev(n);
            let year = date.year() - if month < nth_month { 1 } else { 0 };
            let max_day = nth_month.length(year);
            Date::from_calendar_date(year, nth_month, date.day().min(max_day)).ok()
        }
    }
}

/// Resolve `ArrowLeft`/`ArrowRight` on the day grid to a `+1`/`-1` day step,
/// honoring [`Direction`] the same way every other roving-focus consumer in
/// this crate does (`Direction::resolve_horizontal`): the day "to the
/// right" is always `+1` day in LTR and `-1` day in RTL, since a calendar's
/// week row is itself a horizontal, direction-mirrored layout (the same
/// spatial-arrow-key convention this lane found unanimous across every
/// Radix source it read for roving focus, Slider, and the menu family --
/// see `$S/batch3/rtl-rust/reference.md`; no Radix/shadcn Calendar exists
/// to cite directly, since shadcn's own Calendar wraps `react-day-picker`,
/// not a Radix primitive). `ArrowUp`/`ArrowDown` (`±7 days`, a full week
/// row) are never direction-dependent and stay outside this function,
/// exactly like every other consumer's vertical arrows.
fn horizontal_day_step(key: &Key, direction: Direction) -> Option<i64> {
    match direction.resolve_horizontal(key)? {
        HorizontalNav::Next => Some(1),
        HorizontalNav::Prev => Some(-1),
    }
}

/// Calendar date range
#[derive(Copy, Clone, PartialEq, Debug)]
pub struct DateRange {
    /// The start date of the range
    start: Date,
    /// The end date of the range
    end: Date,
}

impl DateRange {
    /// Create a new date range
    pub fn new(start: Date, end: Date) -> Self {
        if start <= end {
            Self { start, end }
        } else {
            Self {
                start: end,
                end: start,
            }
        }
    }

    /// Returns true if date is contained in the range.
    pub fn contains(&self, date: Date) -> bool {
        self.start <= date && date <= self.end
    }

    fn contained_in_interval(&self, date: Date) -> bool {
        self.start < date && date < self.end
    }

    fn clamp(&self, date: Date) -> Date {
        date.clamp(self.start, self.end)
    }

    /// Get the start of the range
    pub fn start(&self) -> Date {
        self.start
    }

    /// Get the end of the range
    pub fn end(&self) -> Date {
        self.end
    }
}

impl Display for DateRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {}", self.start, self.end)
    }
}

/// Calendar available dates
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AvailableRanges {
    /// A sorted list of dates. Values after an odd number of elements are disabled.
    changes: Vec<Date>,
}

impl AvailableRanges {
    /// Create a new available dates
    pub fn new(disabled_ranges: &[DateRange]) -> Self {
        let mut sorted_range: Vec<_> = disabled_ranges
            .iter()
            .enumerate()
            .flat_map(|(index, date)| [(index, date.start), (index, date.end)])
            .collect();

        sorted_range.sort_by_key(|(_, date)| *date);

        // Merge any overlapping ranges
        let mut open_ranges = HashSet::new();
        let mut deduped_ranges = Vec::with_capacity(sorted_range.len());

        for (index, date) in sorted_range {
            let end_of_range = open_ranges.remove(&index);
            if open_ranges.is_empty() {
                deduped_ranges.push(date);
            }
            if !end_of_range {
                open_ranges.insert(index);
            }
        }

        Self {
            changes: deduped_ranges,
        }
    }

    /// Check the availability of given date
    pub fn valid_interval(&self, date: Date) -> bool {
        match self.changes.binary_search(&date) {
            Ok(_) => false,
            Err(index) => index % 2 == 0,
        }
    }

    /// Get the available range of given date
    pub fn available_range(&self, date: Date, date_range: DateRange) -> Option<DateRange> {
        let date_index = self.changes.binary_search(&date).err()?;
        let min_date = date_range.start();
        let max_date = date_range.end();

        let valid = date_index % 2 == 0;
        if !valid {
            return None;
        }

        let start = date_index
            .checked_sub(1)
            .and_then(|index| self.changes.get(index).copied())
            .map(|start| start.next_day().unwrap_or(start))
            .unwrap_or(min_date);
        let end = self
            .changes
            .get(date_index)
            .copied()
            .map(|end| end.previous_day().unwrap_or(end))
            .unwrap_or(max_date);

        Some(DateRange::new(start, end))
    }

    /// Get disabled ranges
    pub fn to_disabled_ranges(&self) -> Vec<DateRange> {
        self.changes
            .chunks(2)
            .map(|d| DateRange::new(d[0], d[1]))
            .collect()
    }
}

/// The base context provided by the [`Calendar`] and the [`RangeCalendar`] component to its children.
#[derive(Copy, Clone)]
pub struct BaseCalendarContext {
    // State
    focused_date: Signal<Option<Date>>,
    view_date: ReadSignal<Date>,
    available_ranges: Memo<AvailableRanges>,
    set_view_date: Callback<Date>,
    format_weekday: Callback<Weekday, String>,
    format_month: Callback<Month, String>,

    // Configuration
    disabled: ReadSignal<bool>,
    today: Date,
    first_day_of_week: Weekday,
    enabled_date_range: DateRange,
    view_registrations: Signal<Vec<CalendarViewRegistration>>,

    /// Text direction, for the day grid's `ArrowLeft`/`ArrowRight` step --
    /// see [`horizontal_day_step`]'s doc.
    direction: Direction,
}

#[derive(Clone, Copy, PartialEq)]
struct CalendarViewRegistration {
    id: ScopeId,
    offset: Option<u8>,
}

impl BaseCalendarContext {
    /// Get the currently focused date
    pub fn focused_date(&self) -> Option<Date> {
        self.focused_date.cloned()
    }

    /// Set the focused date
    pub fn set_focused_date(&mut self, date: Option<Date>) {
        self.focused_date.set(date);
    }

    /// Get the current view date
    pub fn view_date(&self) -> Date {
        self.view_date.cloned()
    }

    /// Set the view date
    pub fn set_view_date(&self, date: Date) {
        (self.set_view_date)(self.enabled_date_range.clamp(date));
    }

    /// Check if the calendar is disabled
    pub fn is_disabled(&self) -> bool {
        self.disabled.cloned()
    }

    /// Check if the selected date is unavailable
    pub fn is_unavailable(&self, date: Date) -> bool {
        !self.available_ranges.read().valid_interval(date)
    }

    /// Check if a date is focused
    pub fn is_focused(&self, date: Date) -> bool {
        self.focused_date().is_some_and(|d| d == date)
    }

    /// Return available date range by given date
    pub fn available_range(&self) -> Option<DateRange> {
        try_consume_context::<RangeCalendarContext>().and_then(|ctx| {
            ctx.anchor_date.cloned().and_then(|date| {
                self.available_ranges
                    .read()
                    .available_range(date, self.enabled_date_range)
            })
        })
    }

    fn visible_month_count(&self) -> u8 {
        self.view_registrations
            .read()
            .iter()
            .enumerate()
            .map(|(index, view)| {
                view.offset
                    .unwrap_or_else(|| u8::try_from(index).unwrap_or(u8::MAX))
                    .saturating_add(1)
            })
            .max()
            .unwrap_or(1)
    }

    fn calendar_view_offset(&self, id: ScopeId, offset: Option<u8>) -> u8 {
        offset.unwrap_or_else(|| {
            self.view_registrations
                .read()
                .iter()
                .position(|view| view.id == id)
                .and_then(|index| u8::try_from(index).ok())
                .unwrap_or_default()
        })
    }

    fn register_calendar_view(&self, id: ScopeId, offset: Option<u8>) {
        if self
            .view_registrations
            .read()
            .iter()
            .any(|view| view.id == id && view.offset == offset)
        {
            return;
        }

        let mut view_registrations_signal = self.view_registrations;
        let mut view_registrations = view_registrations_signal.write();
        if let Some(view) = view_registrations.iter_mut().find(|view| view.id == id) {
            view.offset = offset;
        } else {
            view_registrations.push(CalendarViewRegistration { id, offset });
        }
    }

    fn unregister_calendar_view(&self, id: ScopeId) {
        if !self
            .view_registrations
            .read()
            .iter()
            .any(|view| view.id == id)
        {
            return;
        }

        let mut view_registrations = self.view_registrations;
        view_registrations.write().retain(|view| view.id != id);
    }
}

/// The context provided by the [`Calendar`] component to its children.
#[derive(Copy, Clone)]
pub struct CalendarContext {
    selected_date: ReadSignal<Option<Date>>,
    set_selected_date: Callback<Option<Date>>,
}

impl CalendarContext {
    /// Get the currently selected date
    pub fn selected_date(&self) -> Option<Date> {
        self.selected_date.cloned()
    }

    /// Set the selected date
    pub fn set_selected_date(&self, date: Option<Date>) {
        (self.set_selected_date)(date);
    }
}

/// The props for the [`Calendar`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarProps {
    /// The selected date
    #[props(default)]
    pub selected_date: ReadSignal<Option<Date>>,

    /// Callback when selected date changes
    #[props(default)]
    pub on_date_change: Callback<Option<Date>>,

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

    /// The text direction for the day grid's `ArrowLeft`/`ArrowRight` step.
    /// Defaults to the nearest [`crate::direction::DirectionProvider`], or
    /// LTR if there is none.
    #[props(default)]
    pub dir: Option<Direction>,

    /// Additional attributes to extend the calendar element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the calendar element
    pub children: Element,
}

impl DefaultCalendarProps for CalendarProps {
    fn default_calendar(self) -> Element {
        Calendar(self)
    }
}

/// # Calendar
///
/// The [`Calendar`] component provides an accessible calendar interface with arrow key navigation, month switching, and date selection.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::calendar::{
///     Calendar, CalendarGrid, CalendarHeader, CalendarMonthTitle, CalendarNavigation, CalendarNextMonthButton, CalendarPreviousMonthButton
/// };
/// use time::{Date, Month, UtcDateTime};
/// #[component]
/// fn Demo() -> Element {
///     let mut selected_date = use_signal(|| None::<Date>);
///     let mut view_date = use_signal(|| UtcDateTime::now().date());
///     rsx! {
///         Calendar {
///             selected_date: selected_date(),
///             on_date_change: move |date| {
///                 tracing::info!("Selected date: {:?}", date);
///                 selected_date.set(date);
///             },
///             view_date: view_date(),
///             on_view_change: move |new_view: Date| {
///                 tracing::info!("View changed to: {}-{}", new_view.year(), new_view.month());
///                 view_date.set(new_view);
///             },
///             CalendarHeader {
///                 CalendarNavigation {
///                     CalendarPreviousMonthButton {
///                         "<"
///                     }
///                     CalendarMonthTitle {}
///                     CalendarNextMonthButton {
///                         ">"
///                     }
///                 }
///             }
///             CalendarGrid {}
///         }
///     }
/// }
/// ```
///
/// # Styling
///
/// The [`Calendar`] component defines the following data attributes you can use to control styling:
/// - `data-disabled`: Indicates if the calendar is disabled. Possible values are `true` or `false`.
#[component]
pub fn Calendar(props: CalendarProps) -> Element {
    let available_ranges = use_memo(move || AvailableRanges::new(&props.disabled_ranges.read()));
    let view_registrations = use_signal(Vec::new);
    let direction = use_direction(props.dir);

    // Create base context provider for child components
    let mut base_ctx = use_context_provider(|| BaseCalendarContext {
        focused_date: Signal::new(None),
        view_date: props.view_date,
        set_view_date: props.on_view_change,
        available_ranges,
        format_weekday: props.on_format_weekday,
        format_month: props.on_format_month,
        disabled: props.disabled,
        today: props.today,
        first_day_of_week: props.first_day_of_week,
        enabled_date_range: DateRange::new(props.min_date, props.max_date),
        view_registrations,
        direction,
    });
    // Create Calendar context provider for child components
    use_context_provider(|| CalendarContext {
        selected_date: props.selected_date,
        set_selected_date: props.on_date_change,
    });

    // `role`/the `data-*` state are owned by the component (roving-focus and
    // disabled-state wiring below reads them back); `aria_label` is an
    // overridable default. Merge with the caller's `attributes` so a caller
    // override survives hydration instead of colliding with these literals
    // on the same element (backlog row 93).
    let defaults = attributes!(div {
        aria_label: "Calendar"
    });
    let owned = attributes!(div {
        role: "application",
        "data-disabled": (props.disabled)(),
        "data-direction": direction.as_str(),
    });
    let merged = merge_attributes(vec![defaults, props.attributes.clone(), owned]);

    rsx! {
        div {
            dir: direction.as_str(),
            onkeydown: move |e| {
                let Some(focused_date) = (base_ctx.focused_date)() else {
                    return;
                };
                let mut set_focused_date = |new_date: Option<Date>| {
                    if let Some(date) = new_date {
                        let min_date = (base_ctx.view_date)().replace_day(1).unwrap();
                        if date < min_date {
                            let view_date = previous_month(min_date).unwrap_or(min_date);
                            (base_ctx.set_view_date)(view_date);
                        } else {
                            let max_date = nth_month_next(min_date, base_ctx.visible_month_count())
                                .unwrap_or(min_date);
                            if date >= max_date {
                                let view_date = next_month(min_date).unwrap_or(min_date);
                                (base_ctx.set_view_date)(view_date);
                            }
                        }
                    }
                    match new_date {
                        Some(date) => {
                            if base_ctx.enabled_date_range.contains(date) {
                                base_ctx.focused_date.set(new_date);
                            }
                        }
                        None => base_ctx.focused_date.set(None),
                    }
                };
                match e.key() {
                    Key::ArrowLeft | Key::ArrowRight => {
                        e.prevent_default();
                        if let Some(step) = horizontal_day_step(&e.key(), direction) {
                            let target = if step > 0 {
                                focused_date.next_day()
                            } else {
                                focused_date.previous_day()
                            };
                            set_focused_date(target);
                        }
                    }
                    Key::ArrowUp => {
                        e.prevent_default();
                        if e.modifiers().shift() {
                            if let Some(date) = previous_month(focused_date) {
                                set_focused_date(Some(date));
                            }
                        } else {
                            set_focused_date(Some(focused_date.saturating_sub(7.days())));
                        }
                    }
                    Key::ArrowDown => {
                        e.prevent_default();
                        if e.modifiers().shift() {
                            if let Some(date) = next_month(focused_date) {
                                set_focused_date(Some(date));
                            }
                        } else {
                            set_focused_date(Some(focused_date.saturating_add(7.days())));
                        }
                    }
                    _ => {}
                }
            },
            ..merged,
            {props.children}
        }
    }
}

/// The context provided by the [`RangeCalendar`] component to its children.
#[derive(Copy, Clone)]
pub struct RangeCalendarContext {
    // The date that the user clicked on to begin range selection
    anchor_date: Signal<Option<Date>>,
    // Currently highlighted date range
    highlighted_range: Signal<Option<DateRange>>,
    set_selected_range: Callback<Option<DateRange>>,
}

impl RangeCalendarContext {
    /// Set the selected date
    pub fn set_selected_date(&mut self, date: Option<Date>) {
        match (self.anchor_date)() {
            Some(anchor) => {
                if let Some(date) = date {
                    self.anchor_date.set(None);

                    let range = DateRange::new(date, anchor);
                    self.set_selected_range.call(Some(range));
                    self.highlighted_range.set(Some(range));
                }
            }
            None => {
                self.anchor_date.set(date);

                let range = date.map(|d| DateRange::new(d, d));
                self.highlighted_range.set(range);
            }
        }
    }

    /// Set the selected date range by hovered date
    pub fn set_hovered_date(&mut self, date: Date) {
        if let Some(anchor) = (self.anchor_date)() {
            let range = DateRange::new(anchor, date);
            self.highlighted_range.set(Some(range));
        }
    }

    /// Set previous selected range
    pub fn reset_selection(&mut self, range: Option<DateRange>) {
        self.anchor_date.set(None);
        self.highlighted_range.set(range);
    }
}

/// The props for the [`RangeCalendar`] component.
#[derive(Props, Clone, PartialEq)]
pub struct RangeCalendarProps {
    /// The selected range
    #[props(default)]
    pub selected_range: ReadSignal<Option<DateRange>>,

    /// Callback when selected date range changes
    #[props(default)]
    pub on_range_change: Callback<Option<DateRange>>,

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

    /// The text direction -- see [`CalendarProps::dir`]'s identical doc.
    #[props(default)]
    pub dir: Option<Direction>,

    /// Additional attributes to extend the calendar element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the calendar element
    pub children: Element,
}

impl DefaultCalendarProps for RangeCalendarProps {
    fn default_calendar(self) -> Element {
        RangeCalendar(self)
    }
}

/// # RangeCalendar
///
/// The [`RangeCalendar`] component provides an accessible calendar interface with arrow key navigation, month switching, and date selection.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::calendar::*;
/// use time::{Date, Month, UtcDateTime};
/// #[component]
/// fn Demo() -> Element {
///     let mut selected_range = use_signal(|| None::<DateRange>);
///     let mut view_date = use_signal(|| UtcDateTime::now().date());
///     rsx! {
///         RangeCalendar {
///             selected_range: selected_range(),
///             on_range_change: move |range| {
///                 tracing::info!("Selected range: {:?}", range);
///                 selected_range.set(range);
///             },
///             view_date: view_date(),
///             on_view_change: move |new_view: Date| {
///                 tracing::info!("View changed to: {}-{}", new_view.year(), new_view.month());
///                 view_date.set(new_view);
///             },
///             CalendarHeader {
///                 CalendarNavigation {
///                     CalendarPreviousMonthButton {
///                         "<"
///                     }
///                     CalendarMonthTitle {}
///                     CalendarNextMonthButton {
///                         ">"
///                     }
///                 }
///             }
///             CalendarGrid {}
///         }
///     }
/// }
/// ```
///
/// # Styling
///
/// The [`RangeCalendar`] component defines the following data attributes you can use to control styling:
/// - `data-disabled`: Indicates if the calendar is disabled. Possible values are `true` or `false`.
#[component]
pub fn RangeCalendar(props: RangeCalendarProps) -> Element {
    let focused_date = use_signal(|| {
        let range = (props.selected_range)();
        range.map(|r| r.end)
    });
    let anchor_date = use_signal(|| None::<Date>);
    let highlighted_range = use_signal(|| (props.selected_range)());
    let available_ranges = use_memo(move || AvailableRanges::new(&props.disabled_ranges.read()));
    let view_registrations = use_signal(Vec::new);
    let direction = use_direction(props.dir);

    // Create base context provider for child components
    let mut base_ctx = use_context_provider(|| BaseCalendarContext {
        focused_date,
        view_date: props.view_date,
        set_view_date: props.on_view_change,
        available_ranges,
        format_weekday: props.on_format_weekday,
        format_month: props.on_format_month,
        disabled: props.disabled,
        today: props.today,
        first_day_of_week: props.first_day_of_week,
        enabled_date_range: DateRange::new(props.min_date, props.max_date),
        view_registrations,
        direction,
    });

    // Create RangeCalendar context provider for child components
    let mut ctx = use_context_provider(|| RangeCalendarContext {
        anchor_date,
        highlighted_range,
        set_selected_range: props.on_range_change,
    });

    // Same reasoning as `Calendar` above: owned `role`/`data-*` state merged
    // with the caller's attributes so an override survives hydration.
    let defaults = attributes!(div {
        aria_label: "Calendar"
    });
    let owned = attributes!(div {
        role: "application",
        "data-disabled": (props.disabled)(),
        "data-direction": direction.as_str(),
    });
    let merged = merge_attributes(vec![defaults, props.attributes.clone(), owned]);

    rsx! {
        div {
            dir: direction.as_str(),
            onkeydown: move |e| {
                let Some(mut focused_date) = (base_ctx.focused_date)() else {
                    return;
                };
                if let (Some(range), Some(date)) = (
                    (ctx.highlighted_range)(),
                    (ctx.anchor_date)(),
                ) {
                    if date != range.start {
                        focused_date = range.start
                    } else {
                        focused_date = range.end
                    }
                }
                let mut set_focused_date = |new_date: Option<Date>| {
                    if let Some(date) = new_date {
                        let min_date = (base_ctx.view_date)().replace_day(1).unwrap();
                        if date < min_date {
                            let view_date = previous_month(min_date).unwrap_or(min_date);
                            (base_ctx.set_view_date)(view_date);
                        } else {
                            let max_date = nth_month_next(min_date, base_ctx.visible_month_count())
                                .unwrap_or(min_date);
                            if date >= max_date {
                                let view_date = next_month(min_date).unwrap_or(min_date);
                                (base_ctx.set_view_date)(view_date);
                            }
                        }
                    }
                    match new_date {
                        Some(date) => {
                            if base_ctx.enabled_date_range.contains(date) {
                                base_ctx.focused_date.set(new_date);
                                let date = match base_ctx.available_range() {
                                    Some(range) => range.clamp(date),
                                    None => date,
                                };
                                ctx.set_hovered_date(date);
                            }
                        }
                        None => base_ctx.focused_date.set(None),
                    }
                };
                match e.key() {
                    Key::ArrowLeft | Key::ArrowRight => {
                        e.prevent_default();
                        if let Some(step) = horizontal_day_step(&e.key(), direction) {
                            let target = if step > 0 {
                                focused_date.next_day()
                            } else {
                                focused_date.previous_day()
                            };
                            set_focused_date(target);
                        }
                    }
                    Key::ArrowUp => {
                        e.prevent_default();
                        if e.modifiers().shift() {
                            if let Some(date) = previous_month(focused_date) {
                                set_focused_date(Some(date));
                            }
                        } else {
                            set_focused_date(Some(focused_date.saturating_sub(7.days())));
                        }
                    }
                    Key::ArrowDown => {
                        e.prevent_default();
                        if e.modifiers().shift() {
                            if let Some(date) = next_month(focused_date) {
                                set_focused_date(Some(date));
                            }
                        } else {
                            set_focused_date(Some(focused_date.saturating_add(7.days())));
                        }
                    }
                    Key::Escape => {
                        ctx.reset_selection((props.selected_range)());
                    }
                    _ => {}
                }
            },
            ..merged,
            {props.children}
        }
    }
}

/// The props for the [`CalendarView`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarViewProps {
    /// An offset from the beginning of the view date that this should display
    #[props(default)]
    pub offset: Option<u8>,

    /// Additional attributes to apply to the view element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the calendar element
    pub children: Element,
}

#[derive(Copy, Clone, PartialEq)]
struct CalendarViewContext {
    offset: u8,
}

impl CalendarViewContext {
    fn offset_view_date(&self) -> Date {
        let base_ctx: BaseCalendarContext = consume_context();
        let view_date = (base_ctx.view_date)();

        nth_month_next(view_date, self.offset).unwrap_or(view_date)
    }

    fn set_offset_view_date(&self, date: Date) {
        let base_ctx: BaseCalendarContext = consume_context();
        let view_date = base_ctx.view_date();
        // The date is currently relative to the offset, so we need to adjust it back
        let date = nth_month_previous(date, self.offset).unwrap_or(view_date);
        base_ctx.set_view_date(date);
    }
}

/// A calendar view for one visible month.
///
/// Render one [`CalendarView`] for each month you want visible. The calendar derives the
/// visible month count from the registered views and uses each view's render order as
/// its month offset unless `offset` is provided.
#[component]
pub fn CalendarView(props: CalendarViewProps) -> Element {
    let base_ctx: BaseCalendarContext = use_context();
    let view_id = current_scope_id();

    use_hook(move || {
        base_ctx.register_calendar_view(view_id, props.offset);
    });

    use_effect(move || {
        base_ctx.register_calendar_view(view_id, props.offset);
    });

    use_effect_cleanup(move || {
        base_ctx.unregister_calendar_view(view_id);
    });

    let offset = base_ctx.calendar_view_offset(view_id, props.offset);

    use_context_provider(|| CalendarViewContext { offset });

    rsx! {
        div { ..props.attributes,
            {props.children}
        }
    }
}

/// The props for the [`CalendarHeader`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarHeaderProps {
    /// Optional ID for the header
    #[props(default)]
    pub id: Option<String>,

    /// Additional attributes to extend the header element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the header element
    pub children: Element,
}

/// # CalendarHeader
///
/// The [`CalendarHeader`] component displays the header for the calendar. It typically contains the [`CalendarNavigation`] component
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::calendar::{
///     Calendar, CalendarGrid, CalendarHeader, CalendarMonthTitle, CalendarNavigation, CalendarNextMonthButton, CalendarPreviousMonthButton
/// };
/// use time::{Date, Month, UtcDateTime};
/// #[component]
/// fn Demo() -> Element {
///     let mut selected_date = use_signal(|| None::<Date>);
///     let mut view_date = use_signal(|| UtcDateTime::now().date());
///     rsx! {
///         Calendar {
///             selected_date: selected_date(),
///             on_date_change: move |date| {
///                 tracing::info!("Selected date: {:?}", date);
///                 selected_date.set(date);
///             },
///             view_date: view_date(),
///             on_view_change: move |new_view: Date| {
///                 tracing::info!("View changed to: {}-{}", new_view.year(), new_view.month());
///                 view_date.set(new_view);
///             },
///             CalendarHeader {
///                 CalendarNavigation {
///                     CalendarPreviousMonthButton {
///                         "<"
///                     }
///                     CalendarMonthTitle {}
///                     CalendarNextMonthButton {
///                         ">"
///                     }
///                 }
///             }
///             CalendarGrid {}
///         }
///     }
/// }
/// ```
#[component]
pub fn CalendarHeader(props: CalendarHeaderProps) -> Element {
    // `role`/`aria-level` are owned (semantics this element must keep);
    // merge with the caller's attributes instead of leaving both a literal
    // and a caller override on the same element (backlog row 93).
    let owned = attributes!(div {
        role: "heading",
        "aria-level": "2",
    });
    let merged = merge_attributes(vec![props.attributes.clone(), owned]);

    rsx! {
        div {
            id: props.id,
            ..merged,

            {props.children}
        }
    }
}

/// The props for the [`CalendarNavigation`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarNavigationProps {
    /// Optional ID for the navigation
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the navigation element
    #[props(default)]
    pub children: Element,
}

/// # CalendarNavigation
///
/// The [`CalendarNavigation`] component provides a container for navigation buttons in the calendar header.
/// It typically contains the [`CalendarPreviousMonthButton`], [`CalendarNextMonthButton`], and [`CalendarMonthTitle`] components.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::calendar::{
///     Calendar, CalendarGrid, CalendarHeader, CalendarMonthTitle, CalendarNavigation, CalendarNextMonthButton, CalendarPreviousMonthButton
/// };
/// use time::{Date, Month, UtcDateTime};
/// #[component]
/// fn Demo() -> Element {
///     let mut selected_date = use_signal(|| None::<Date>);
///     let mut view_date = use_signal(|| UtcDateTime::now().date());
///     rsx! {
///         Calendar {
///             selected_date: selected_date(),
///             on_date_change: move |date| {
///                 tracing::info!("Selected date: {:?}", date);
///                 selected_date.set(date);
///             },
///             view_date: view_date(),
///             on_view_change: move |new_view: Date| {
///                 tracing::info!("View changed to: {}-{}", new_view.year(), new_view.month());
///                 view_date.set(new_view);
///             },
///             CalendarHeader {
///                 CalendarNavigation {
///                     CalendarPreviousMonthButton {
///                         "<"
///                     }
///                     CalendarMonthTitle {}
///                     CalendarNextMonthButton {
///                         ">"
///                     }
///                 }
///             }
///             CalendarGrid {}
///         }
///     }
/// }
/// ```
#[component]
pub fn CalendarNavigation(props: CalendarNavigationProps) -> Element {
    rsx! {
        div { ..props.attributes,
            {props.children}
        }
    }
}

/// The props for the [`CalendarPreviousMonthButton`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarPreviousMonthButtonProps {
    /// Additional attributes to apply to the button
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the button element
    pub children: Element,
}

/// # CalendarPreviousMonthButton
///
/// The [`CalendarPreviousMonthButton`] component provides a button to navigate to the previous month in the calendar.
///
/// This must be used inside a [`Calendar`] component.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::calendar::{
///     Calendar, CalendarGrid, CalendarHeader, CalendarMonthTitle, CalendarNavigation, CalendarNextMonthButton, CalendarPreviousMonthButton
/// };
/// use time::{Date, Month, UtcDateTime};
/// #[component]
/// fn Demo() -> Element {
///     let mut selected_date = use_signal(|| None::<Date>);
///     let mut view_date = use_signal(|| UtcDateTime::now().date());
///     rsx! {
///         Calendar {
///             selected_date: selected_date(),
///             on_date_change: move |date| {
///                 tracing::info!("Selected date: {:?}", date);
///                 selected_date.set(date);
///             },
///             view_date: view_date(),
///             on_view_change: move |new_view: Date| {
///                 tracing::info!("View changed to: {}-{}", new_view.year(), new_view.month());
///                 view_date.set(new_view);
///             },
///             CalendarHeader {
///                 CalendarNavigation {
///                     CalendarPreviousMonthButton {
///                         "<"
///                     }
///                     CalendarMonthTitle {}
///                     CalendarNextMonthButton {
///                         ">"
///                     }
///                 }
///             }
///             CalendarGrid {}
///         }
///     }
/// }
/// ```
#[component]
pub fn CalendarPreviousMonthButton(props: CalendarPreviousMonthButtonProps) -> Element {
    let ctx: BaseCalendarContext = use_context();
    let view_ctx: CalendarViewContext = use_context();

    // disable previous button when we reach the limit
    let button_disabled = use_memo(move || {
        // Get the current view date from context
        let view_date = view_ctx.offset_view_date();
        match previous_month(view_date) {
            Some(date) => ctx.enabled_date_range.start.replace_day(1).unwrap() > date,
            None => true,
        }
    });
    // disable previous button when the current selection range does not include the previous month
    let navigate_disabled = use_memo(move || {
        // Get the current view date from context
        let view_date = view_ctx.offset_view_date();
        ctx.available_range()
            .is_some_and(|range| range.start.month() == view_date.month())
    });

    // Handle navigation to previous month
    let handle_prev_month = move |e: Event<MouseData>| {
        e.prevent_default();
        let current_view = (ctx.view_date)();
        if let Some(date) = previous_month(current_view) {
            ctx.set_view_date.call(date)
        }
    };

    // `aria_label`/`type` are overridable defaults; `disabled` reflects this
    // component's own limit/range logic and is owned. Merge with the
    // caller's attributes instead of leaving both a literal and a caller
    // override on the same element (backlog row 93).
    let defaults = attributes!(button {
        aria_label: "Previous month",
        r#type: "button",
    });
    let owned = attributes!(button {
        disabled: (ctx.disabled)() || button_disabled() || navigate_disabled(),
    });
    let merged = merge_attributes(vec![defaults, props.attributes.clone(), owned]);

    rsx! {
        button {
            onclick: handle_prev_month,
            ..merged,

            {props.children}
        }
    }
}

/// The props for the [`CalendarNextMonthButton`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarNextMonthButtonProps {
    /// Additional attributes to apply to the button
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the button element
    pub children: Element,
}

/// # CalendarNextMonthButton
///
/// The [`CalendarNextMonthButton`] component provides a button to navigate to the next month in the calendar.
///
/// This must be used inside a [`Calendar`] component.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::calendar::{
///     Calendar, CalendarGrid, CalendarHeader, CalendarMonthTitle, CalendarNavigation, CalendarNextMonthButton, CalendarPreviousMonthButton
/// };
/// use time::{Date, Month, UtcDateTime};
/// #[component]
/// fn Demo() -> Element {
///     let mut selected_date = use_signal(|| None::<Date>);
///     let mut view_date = use_signal(|| UtcDateTime::now().date());
///     rsx! {
///         Calendar {
///             selected_date: selected_date(),
///             on_date_change: move |date| {
///                 tracing::info!("Selected date: {:?}", date);
///                 selected_date.set(date);
///             },
///             view_date: view_date(),
///             on_view_change: move |new_view: Date| {
///                 tracing::info!("View changed to: {}-{}", new_view.year(), new_view.month());
///                 view_date.set(new_view);
///             },
///             CalendarHeader {
///                 CalendarNavigation {
///                     CalendarPreviousMonthButton {
///                         "<"
///                     }
///                     CalendarMonthTitle {}
///                     CalendarNextMonthButton {
///                         ">"
///                     }
///                 }
///             }
///             CalendarGrid {}
///         }
///     }
/// }
/// ```
#[component]
pub fn CalendarNextMonthButton(props: CalendarNextMonthButtonProps) -> Element {
    let ctx: BaseCalendarContext = use_context();
    let view_ctx: CalendarViewContext = use_context();

    // disable next button when we reach the limit
    let button_disabled = use_memo(move || {
        // Get the current view date from context
        let view_date = view_ctx.offset_view_date();
        match next_month(view_date) {
            Some(date) => {
                let max = ctx.enabled_date_range.end();
                let last_day = max.month().length(max.year());
                max.replace_day(last_day).unwrap() < date
            }
            None => true,
        }
    });
    // disable next button when the current selection range does not include the next month
    let navigate_disabled = use_memo(move || {
        // Get the current view date from context
        let view_date = view_ctx.offset_view_date();
        ctx.available_range()
            .is_some_and(|range| range.end.month() == view_date.month())
    });

    // Handle navigation to next month
    let handle_next_month = move |e: Event<MouseData>| {
        e.prevent_default();
        let current_view = (ctx.view_date)();
        if let Some(date) = next_month(current_view) {
            ctx.set_view_date.call(date)
        }
    };

    // Same reasoning as `CalendarPreviousMonthButton` above.
    let defaults = attributes!(button {
        aria_label: "Next month",
        r#type: "button",
    });
    let owned = attributes!(button {
        disabled: (ctx.disabled)() || button_disabled() || navigate_disabled(),
    });
    let merged = merge_attributes(vec![defaults, props.attributes.clone(), owned]);

    rsx! {
        button {
            onclick: handle_next_month,
            ..merged,

            {props.children}
        }
    }
}

/// The props for the [`CalendarMonthTitle`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarMonthTitleProps {
    /// Additional attributes to apply to the title element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// # CalendarMonthTitle
///
/// The [`CalendarMonthTitle`] component displays the title of the current month in the calendar. It will contain
/// the month and year information as text in the children.
///
/// This must be used inside a [`Calendar`] component.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::calendar::{
///     Calendar, CalendarGrid, CalendarHeader, CalendarMonthTitle, CalendarNavigation, CalendarNextMonthButton, CalendarPreviousMonthButton
/// };
/// use time::{Date, Month, UtcDateTime};
/// #[component]
/// fn Demo() -> Element {
///     let mut selected_date = use_signal(|| None::<Date>);
///     let mut view_date = use_signal(|| UtcDateTime::now().date());
///     rsx! {
///         Calendar {
///             selected_date: selected_date(),
///             on_date_change: move |date| {
///                 tracing::info!("Selected date: {:?}", date);
///                 selected_date.set(date);
///             },
///             view_date: view_date(),
///             on_view_change: move |new_view: Date| {
///                 tracing::info!("View changed to: {}-{}", new_view.year(), new_view.month());
///                 view_date.set(new_view);
///             },
///             CalendarHeader {
///                 CalendarNavigation {
///                     CalendarPreviousMonthButton {
///                         "<"
///                     }
///                     CalendarMonthTitle {}
///                     CalendarNextMonthButton {
///                         ">"
///                     }
///                 }
///             }
///             CalendarGrid {}
///         }
///     }
/// }
/// ```
#[component]
pub fn CalendarMonthTitle(props: CalendarMonthTitleProps) -> Element {
    let view_ctx: CalendarViewContext = use_context();
    // Format the current month and year
    let month_year = use_memo(move || {
        let view_date = view_ctx.offset_view_date();
        format!("{} {}", view_date.month(), view_date.year())
    });

    rsx! {
        div {
            ..props.attributes,

            {month_year}
        }
    }
}

/// The props for the [`CalendarGrid`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarGridProps {
    /// Optional ID for the grid
    #[props(default)]
    pub id: Option<String>,

    /// Additional attributes to apply to the grid element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// The props for the [`CalendarGridRoot`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarGridRootProps {
    /// Optional ID for the grid
    #[props(default)]
    pub id: Option<String>,

    /// Additional attributes to apply to the grid element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the grid element
    pub children: Element,
}

/// The props for the [`CalendarGridHead`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarGridHeadProps {
    /// Additional attributes to apply to the grid head element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the grid head element
    pub children: Element,
}

/// The props for the [`CalendarGridHeaderRow`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarGridHeaderRowProps {
    /// Additional attributes to apply to the grid header row element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the grid header row element
    pub children: Element,
}

/// The props for the [`CalendarGridDayHeader`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarGridDayHeaderProps {
    /// The weekday represented by this header
    pub weekday: Weekday,

    /// Additional attributes to apply to the weekday header element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the weekday header element
    #[props(default)]
    pub children: Option<Element>,
}

/// The props for the [`CalendarGridBody`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarGridBodyProps {
    /// Additional attributes to apply to the grid body element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the grid body element
    pub children: Element,
}

/// The props for the [`CalendarGridWeek`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarGridWeekProps {
    /// Additional attributes to apply to the week row element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the week row element
    pub children: Element,
}

/// The props for the [`CalendarGridCell`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarGridCellProps {
    /// Additional attributes to apply to the day cell element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the day cell element
    pub children: Element,
}

/// Data for a weekday header in a calendar grid.
#[derive(Clone, PartialEq)]
pub struct CalendarGridWeekday {
    weekday: Weekday,
    label: String,
}

impl CalendarGridWeekday {
    /// The weekday represented by this header.
    pub fn weekday(&self) -> Weekday {
        self.weekday
    }

    /// The formatted weekday label.
    pub fn label(&self) -> &str {
        &self.label
    }
}

/// Data returned by [`use_calendar_grid`].
#[derive(Clone, PartialEq)]
pub struct CalendarGridData {
    view_date: Date,
    weekdays: Vec<CalendarGridWeekday>,
    weeks: Vec<Vec<Date>>,
}

impl CalendarGridData {
    /// The first date in the month currently displayed by the grid.
    pub fn view_date(&self) -> Date {
        self.view_date
    }

    /// Weekday headers in display order.
    pub fn weekdays(&self) -> &[CalendarGridWeekday] {
        &self.weekdays
    }

    /// Weeks in the displayed month, each containing seven dates.
    pub fn weeks(&self) -> &[Vec<Date>] {
        &self.weeks
    }
}

/// Return the weekday headers and week rows for the current calendar grid view.
pub fn use_calendar_grid() -> CalendarGridData {
    let ctx: BaseCalendarContext = use_context();
    let view_ctx: CalendarViewContext = use_context();

    use_memo(move || {
        let view_date = view_ctx.offset_view_date();
        CalendarGridData {
            view_date,
            weekdays: calendar_grid_weekdays(ctx.first_day_of_week, ctx.format_weekday),
            weeks: calendar_grid_weeks(view_date, ctx.first_day_of_week),
        }
    })()
}

fn calendar_grid_weekdays(
    first_day_of_week: Weekday,
    format_weekday: Callback<Weekday, String>,
) -> Vec<CalendarGridWeekday> {
    WeekdaySet(0b111_1111)
        .iter(first_day_of_week)
        .map(|weekday| CalendarGridWeekday {
            weekday,
            label: format_weekday.call(weekday),
        })
        .collect()
}

fn calendar_grid_weeks(view_date: Date, first_day_of_week: Weekday) -> Vec<Vec<Date>> {
    let mut grid = Vec::new();

    let previous_month = view_date
        .replace_day(1)
        .expect("invalid or out-of-range date");
    let num_days = days_since(view_date, first_day_of_week);
    let mut date = previous_month.saturating_sub(num_days.days());
    for _ in 1..=num_days {
        grid.push(date);
        date = date.next_day().expect("invalid or out-of-range date");
    }

    let mut date = view_date;
    let num_days_in_month = view_date.month().length(view_date.year());
    for day in 1..=num_days_in_month {
        date = view_date
            .replace_day(day)
            .expect("invalid or out-of-range date");
        grid.push(date);
    }

    let remainder = grid.len() % 7;
    if remainder > 0 {
        for _ in 1..=(7 - remainder) {
            date = date.next_day().expect("invalid or out-of-range date");
            grid.push(date);
        }
    }

    grid.chunks(7).map(|chunk| chunk.to_vec()).collect()
}

/// # CalendarGrid
///
/// The [`CalendarGrid`] component displays the grid of days for the current month in the calendar.
///
/// This must be used inside a [`Calendar`] component.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::calendar::{
///     Calendar, CalendarGrid, CalendarHeader, CalendarMonthTitle, CalendarNavigation, CalendarNextMonthButton, CalendarPreviousMonthButton
/// };
/// use time::{Date, Month, UtcDateTime};
/// #[component]
/// fn Demo() -> Element {
///     let mut selected_date = use_signal(|| None::<Date>);
///     let mut view_date = use_signal(|| UtcDateTime::now().date());
///     rsx! {
///         Calendar {
///             selected_date: selected_date(),
///             on_date_change: move |date| {
///                 tracing::info!("Selected date: {:?}", date);
///                 selected_date.set(date);
///             },
///             view_date: view_date(),
///             on_view_change: move |new_view: Date| {
///                 tracing::info!("View changed to: {}-{}", new_view.year(), new_view.month());
///                 view_date.set(new_view);
///             },
///             CalendarHeader {
///                 CalendarNavigation {
///                     CalendarPreviousMonthButton {
///                         "<"
///                     }
///                     CalendarMonthTitle {}
///                     CalendarNextMonthButton {
///                         ">"
///                     }
///                 }
///             }
///             CalendarGrid {}
///         }
///     }
/// }
/// ```
///
/// ## Styling
///
/// The [`CalendarGrid`] component renders days in a grid that can be styled using CSS. They define the following data attributes:
/// - `data-today`: If the date is today. Possible values are `true` or `false`
/// - `data-selected`: If the date is selected. Possible values are `true` or `false`
/// - `data-month`: The relative month of the date. Possible values are `last`, `current`, or `next`
#[component]
pub fn CalendarGrid(props: CalendarGridProps) -> Element {
    let grid = use_calendar_grid();

    rsx! {
        CalendarGridRoot {
            id: props.id,
            attributes: props.attributes,
            CalendarGridHead {
                CalendarGridHeaderRow {
                    for weekday in grid.weekdays().iter().cloned() {
                        CalendarGridDayHeader {
                            key: "{weekday.weekday():?}",
                            weekday: weekday.weekday(),
                            {weekday.label().to_string()}
                        }
                    }
                }
            }
            CalendarGridBody {
                for week in grid.weeks() {
                    CalendarGridWeek {
                        for date in week.iter().copied() {
                            CalendarGridCell {
                                key: "{date}",
                                CalendarDay { date }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// The root table element for a calendar grid.
///
/// ## Styling
///
/// - `data-direction`: The resolved text direction. Values are `ltr` or `rtl`.
#[component]
pub fn CalendarGridRoot(props: CalendarGridRootProps) -> Element {
    let base_ctx: BaseCalendarContext = use_context();

    // `role`/`dir`/`data-direction` are all owned semantics/state; merge with
    // the caller's attributes instead of leaving both a literal and a caller
    // override on the same element (backlog row 93).
    let owned = attributes!(table {
        role: "grid",
        dir: base_ctx.direction.as_str(),
        "data-direction": base_ctx.direction.as_str(),
    });
    let merged = merge_attributes(vec![props.attributes.clone(), owned]);

    rsx! {
        table {
            id: props.id,
            ..merged,
            {props.children}
        }
    }
}

/// The header section of a calendar grid.
#[component]
pub fn CalendarGridHead(props: CalendarGridHeadProps) -> Element {
    let owned = attributes!(thead {
        aria_hidden: "true"
    });
    let merged = merge_attributes(vec![props.attributes.clone(), owned]);

    rsx! {
        thead {
            ..merged,
            {props.children}
        }
    }
}

/// The row that contains weekday header cells.
#[component]
pub fn CalendarGridHeaderRow(props: CalendarGridHeaderRowProps) -> Element {
    rsx! {
        tr {
            ..props.attributes,
            {props.children}
        }
    }
}

/// A weekday header cell in a calendar grid.
#[component]
pub fn CalendarGridDayHeader(props: CalendarGridDayHeaderProps) -> Element {
    let ctx: BaseCalendarContext = use_context();
    let children = props.children.unwrap_or_else(|| {
        let label = ctx.format_weekday.call(props.weekday);
        rsx! { {label} }
    });

    rsx! {
        th {
            ..props.attributes,
            {children}
        }
    }
}

/// The body section of a calendar grid.
#[component]
pub fn CalendarGridBody(props: CalendarGridBodyProps) -> Element {
    rsx! {
        tbody {
            ..props.attributes,
            {props.children}
        }
    }
}

/// A week row in a calendar grid.
#[component]
pub fn CalendarGridWeek(props: CalendarGridWeekProps) -> Element {
    let owned = attributes!(tr { role: "row" });
    let merged = merge_attributes(vec![props.attributes.clone(), owned]);

    rsx! {
        tr {
            ..merged,
            {props.children}
        }
    }
}

/// A day cell in a calendar grid.
#[component]
pub fn CalendarGridCell(props: CalendarGridCellProps) -> Element {
    rsx! {
        td {
            ..props.attributes,
            {props.children}
        }
    }
}

/// The props for the [`CalendarSelectMonth`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarSelectMonthProps {
    /// Additional attributes to apply to the month select container element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the month select container element.
    #[props(default)]
    pub children: Element,
}

/// The props for the [`CalendarSelectMonthSelect`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarSelectMonthSelectProps {
    /// Additional attributes to apply to the native month select element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// The props for the [`CalendarSelectMonthOption`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarSelectMonthOptionProps {
    /// The month represented by this option.
    pub month: Month,

    /// Additional attributes to apply to the month option element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the month option element.
    #[props(default)]
    pub children: Option<Element>,
}

/// The props for the [`CalendarSelectMonthValue`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarSelectMonthValueProps {
    /// Additional attributes to apply to the displayed month value element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the displayed month value element.
    #[props(default)]
    pub children: Element,
}

/// # CalendarSelectMonth
///
/// The [`CalendarSelectMonth`] component provides a container for the month select controls.
///
/// This must be used inside a [`Calendar`] component.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::calendar::{
///     Calendar, CalendarGrid, CalendarHeader, CalendarNavigation, CalendarNextMonthButton, CalendarPreviousMonthButton,
///     CalendarSelectMonth, CalendarSelectMonthSelect, CalendarSelectMonthValue
/// };
/// use time::{Date, Month, UtcDateTime};
/// #[component]
/// fn Demo() -> Element {
///     let mut selected_date = use_signal(|| None::<Date>);
///     let mut view_date = use_signal(|| UtcDateTime::now().date());
///     rsx! {
///         Calendar {
///             selected_date: selected_date(),
///             on_date_change: move |date| {
///                 tracing::info!("Selected date: {:?}", date);
///                 selected_date.set(date);
///             },
///             view_date: view_date(),
///             on_view_change: move |new_view: Date| {
///                 tracing::info!("View changed to: {}-{}", new_view.year(), new_view.month());
///                 view_date.set(new_view);
///             },
///             CalendarHeader {
///                 CalendarNavigation {
///                     CalendarPreviousMonthButton {
///                         "<"
///                     }
///                     CalendarSelectMonth {
///                         CalendarSelectMonthSelect {}
///                         CalendarSelectMonthValue {}
///                     }
///                     CalendarNextMonthButton {
///                         ">"
///                     }
///                 }
///             }
///             CalendarGrid {}
///         }
///     }
/// }
/// ```
#[component]
pub fn CalendarSelectMonth(props: CalendarSelectMonthProps) -> Element {
    rsx! {
        span {
            ..props.attributes,
            {props.children}
        }
    }
}

/// The native select element for choosing the visible month.
#[component]
pub fn CalendarSelectMonthSelect(props: CalendarSelectMonthSelectProps) -> Element {
    let base_ctx: BaseCalendarContext = use_context();
    let view_ctx: CalendarViewContext = use_context();

    let months = use_memo(move || {
        // Get the current view date from context
        let view_date = view_ctx.offset_view_date();
        let min_date = base_ctx.enabled_date_range.start();
        let max_date = base_ctx.enabled_date_range.end();
        let mut min_month = Month::January;
        if replace_month(view_date, min_month) < min_date {
            min_month = min_date.month();
        }
        let mut max_month = Month::December;
        if replace_month(view_date, max_month) > max_date {
            max_month = max_date.month();
        }

        let mut month = min_month;
        let mut months = Vec::new();
        loop {
            months.push(month);

            if month == max_month {
                return months;
            }
            month = month.next();
        }
    });

    let defaults = attributes!(select {
        aria_label: "Month"
    });
    let merged = merge_attributes(vec![defaults, props.attributes.clone()]);

    rsx! {
        select {
            onchange: move |e| {
                let mut view_date = view_ctx.offset_view_date();
                let number = e.value().parse().unwrap_or(view_date.month() as u8);
                let cur_month = Month::try_from(number).expect("Month out-of-range");
                view_date = view_date.replace_month(cur_month).unwrap();
                view_ctx.set_offset_view_date(view_date);
            },
            ..merged,
            for month in months() {
                CalendarSelectMonthOption { key: "{month:?}", month }
            }
        }
    }
}

/// An option in the native month select element.
#[component]
pub fn CalendarSelectMonthOption(props: CalendarSelectMonthOptionProps) -> Element {
    let base_ctx: BaseCalendarContext = use_context();
    let view_ctx: CalendarViewContext = use_context();
    let children = props
        .children
        .unwrap_or_else(|| rsx! { {base_ctx.format_month.call(props.month)} });

    // `value`/`selected` are this option's own functional state, owned.
    let owned = attributes!(option {
        value: props.month as u8,
        selected: view_ctx.offset_view_date().month() == props.month,
    });
    let merged = merge_attributes(vec![props.attributes.clone(), owned]);

    rsx! {
        option {
            ..merged,
            {children}
        }
    }
}

/// The displayed month value.
#[component]
pub fn CalendarSelectMonthValue(props: CalendarSelectMonthValueProps) -> Element {
    let base_ctx: BaseCalendarContext = use_context();
    let view_ctx: CalendarViewContext = use_context();
    let month = view_ctx.offset_view_date().month();

    rsx! {
        span {
            ..props.attributes,
            {base_ctx.format_month.call(month)}
            {props.children}
        }
    }
}

/// The props for the [`CalendarSelectYear`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarSelectYearProps {
    /// Additional attributes to apply to the year select container element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the year select container element.
    #[props(default)]
    pub children: Element,
}

/// The props for the [`CalendarSelectYearSelect`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarSelectYearSelectProps {
    /// Additional attributes to apply to the native year select element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
}

/// The props for the [`CalendarSelectYearOption`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarSelectYearOptionProps {
    /// The year represented by this option.
    pub year: i32,

    /// Additional attributes to apply to the year option element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the year option element.
    #[props(default)]
    pub children: Option<Element>,
}

/// The props for the [`CalendarSelectYearValue`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarSelectYearValueProps {
    /// Additional attributes to apply to the displayed year value element.
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,

    /// The children of the displayed year value element.
    #[props(default)]
    pub children: Element,
}

/// # CalendarSelectYear
///
/// The [`CalendarSelectYear`] component provides a container for the year select controls.
///
/// This must be used inside a [`Calendar`] component.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::calendar::{
///     Calendar, CalendarGrid, CalendarHeader, CalendarNavigation, CalendarNextMonthButton, CalendarPreviousMonthButton,
///     CalendarSelectYear, CalendarSelectYearSelect, CalendarSelectYearValue
/// };
/// use time::{Date, Month, UtcDateTime};
/// #[component]
/// fn Demo() -> Element {
///     let mut selected_date = use_signal(|| None::<Date>);
///     let mut view_date = use_signal(|| UtcDateTime::now().date());
///     rsx! {
///         Calendar {
///             selected_date: selected_date(),
///             on_date_change: move |date| {
///                 tracing::info!("Selected date: {:?}", date);
///                 selected_date.set(date);
///             },
///             view_date: view_date(),
///             on_view_change: move |new_view: Date| {
///                 tracing::info!("View changed to: {}-{}", new_view.year(), new_view.month());
///                 view_date.set(new_view);
///             },
///             CalendarHeader {
///                 CalendarNavigation {
///                     CalendarPreviousMonthButton {
///                         "<"
///                     }
///                     CalendarSelectYear {
///                         CalendarSelectYearSelect {}
///                         CalendarSelectYearValue {}
///                     }
///                     CalendarNextMonthButton {
///                         ">"
///                     }
///                 }
///             }
///             CalendarGrid {}
///         }
///     }
/// }
/// ```
#[component]
pub fn CalendarSelectYear(props: CalendarSelectYearProps) -> Element {
    rsx! {
        span {
            ..props.attributes,
            {props.children}
        }
    }
}

/// The native select element for choosing the visible year.
#[component]
pub fn CalendarSelectYearSelect(props: CalendarSelectYearSelectProps) -> Element {
    let base_ctx: BaseCalendarContext = use_context();
    let view_ctx: CalendarViewContext = use_context();

    let years = use_memo(move || {
        // Get the current view date from context
        let view_date = view_ctx.offset_view_date();
        let min_date = base_ctx.enabled_date_range.start();
        let max_date = base_ctx.enabled_date_range.end();
        let month = view_date.month();
        let mut min_year = min_date.year();
        if replace_month(min_date, month) < min_date {
            min_year += 1;
        }
        let mut max_year = max_date.year();
        if replace_month(max_date, month) > max_date {
            max_year -= 1;
        }

        min_year..=max_year
    });

    let defaults = attributes!(select { aria_label: "Year" });
    let merged = merge_attributes(vec![defaults, props.attributes.clone()]);

    rsx! {
        select {
            onchange: move |e| {
                let mut view_date = view_ctx.offset_view_date();
                let year = e.value().parse().unwrap_or(view_date.year());
                view_date = view_date.replace_year(year).unwrap_or(view_date);
                view_ctx.set_offset_view_date(view_date);
            },
            ..merged,
            for year in years() {
                CalendarSelectYearOption { key: "{year}", year }
            }
        }
    }
}

/// An option in the native year select element.
#[component]
pub fn CalendarSelectYearOption(props: CalendarSelectYearOptionProps) -> Element {
    let view_ctx: CalendarViewContext = use_context();
    let children = props.children.unwrap_or_else(|| {
        let year = props.year;
        rsx! { "{year}" }
    });

    // `value`/`selected` are this option's own functional state, owned.
    let owned = attributes!(option {
        value: props.year,
        selected: view_ctx.offset_view_date().year() == props.year,
    });
    let merged = merge_attributes(vec![props.attributes.clone(), owned]);

    rsx! {
        option {
            ..merged,
            {children}
        }
    }
}

/// The displayed year value.
#[component]
pub fn CalendarSelectYearValue(props: CalendarSelectYearValueProps) -> Element {
    let view_ctx: CalendarViewContext = use_context();
    let year = view_ctx.offset_view_date().year();

    rsx! {
        span {
            ..props.attributes,
            "{year}"
            {props.children}
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum RelativeMonth {
    Last,
    Current,
    Next,
}

impl RelativeMonth {
    fn current_month(&self) -> bool {
        *self == RelativeMonth::Current
    }
}

impl Display for RelativeMonth {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelativeMonth::Last => write!(f, "last"),
            RelativeMonth::Current => write!(f, "current"),
            RelativeMonth::Next => write!(f, "next"),
        }
    }
}

/// Get a human-readable ARIA label for input date
fn aria_label(date: &Date) -> String {
    format!(
        "{}, {} {}, {}",
        date.weekday(),
        date.month(),
        date.day(),
        date.year()
    )
}

/// The props for the [`CalendarDay`] component.
#[derive(Props, Clone, PartialEq)]
pub struct CalendarDayProps {
    /// The date for this day cell.
    pub date: Date,
    /// Additional attributes to extend the calendar day element
    #[props(extends = GlobalAttributes)]
    pub attributes: Vec<Attribute>,
    /// The children of the calendar day element
    #[props(default)]
    pub children: Option<Element>,
}

/// # CalendarDay
///
/// The [`CalendarDay`] component provides an accessible calendar interface for a date
///
/// This must be used inside a [`CalendarGrid`] component.
///
/// ## Example
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::calendar::*;
/// use time::{Date, Month, UtcDateTime};
/// #[component]
/// fn Demo() -> Element {
///     let mut selected_range = use_signal(|| None::<DateRange>);
///     let mut view_date = use_signal(|| UtcDateTime::now().date());
///     rsx! {
///         RangeCalendar {
///             selected_range: selected_range(),
///             on_range_change: move |range| {
///                 tracing::info!("Selected range: {:?}", range);
///                 selected_range.set(range);
///             },
///             view_date: view_date(),
///             on_view_change: move |new_view: Date| {
///                 tracing::info!("View changed to: {}-{}", new_view.year(), new_view.month());
///                 view_date.set(new_view);
///             },
///             CalendarHeader {
///                 CalendarNavigation {
///                     CalendarPreviousMonthButton {
///                         "<"
///                     }
///                     CalendarMonthTitle {}
///                     CalendarNextMonthButton {
///                         ">"
///                     }
///                 }
///             }
///             CalendarGrid {}
///         }
///     }
/// }
/// ```
///
/// # Styling
///
/// The [`CalendarDay`] component defines the following data attributes you can use to control styling:
/// - `data-disabled`: Indicates if the calendar is disabled. Possible values are `true` or `false`.
/// - `data-unavailable`: Indicates if the date is unavailable. Possible values are `true` or `false`.
/// - `data-today`: Indicates if the cell is today. Possible values are `true` or `false`.
/// - `data-month`: The relative month of the date. Possible values are `last`,
/// - `data-selected`: Indicates if the cell is selected. Possible values are `true` or `false`.
/// - `data-selection-start`: Indicates if cell is the first date in a range selection. Possible values are `true` or `false`.
/// - `data-selection-between`: Indicates if a date interval contains a cell. Possible values are `true` or `false`.
/// - `data-selection-end`: Indicates if cell is the last date in a range selection. Possible values are `true` or `false`.
///
/// Building an entirely custom day cell instead of using [`CalendarDay`]?
/// [`use_calendar_day_state`] and [`calendar_day_attributes`] expose the
/// same state and attributes this component computes for itself.
#[component]
pub fn CalendarDay(props: CalendarDayProps) -> Element {
    let single_context = try_use_context::<CalendarContext>().is_some();
    let CalendarDayProps {
        date,
        attributes,
        children,
    } = props;

    if single_context {
        rsx! {
            SingleCalendarDay { date, attributes: attributes.clone(), children: children.clone() }
        }
    } else {
        rsx! {
            RangeCalendarDay { date, attributes, children }
        }
    }
}

/// The computed interactive/visual state of one calendar day cell -- exactly
/// what the built-in [`CalendarDay`] cell computes for itself, exposed so a
/// custom day-cell UI built outside this crate can render the same states
/// instead of duplicating this logic. Returned by [`use_calendar_day_state`]
/// and turned into markup attributes by [`calendar_day_attributes`] --
/// upstream `DioxusLabs/components#199` ("the APIs to build each component
/// should be public").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CalendarDayState {
    /// The date this state describes.
    pub date: Date,
    /// Whether this date is selected: the calendar's exact selected date in
    /// a [`Calendar`], or contained in the highlighted range (inclusive of
    /// its start/end) in a [`RangeCalendar`]. Always `false` in a
    /// [`Calendar`] for any date other than the selected one.
    pub selected: bool,
    /// Whether this date is the first date of a range selection. Always
    /// `false` inside a [`Calendar`] (there is no range there).
    pub range_start: bool,
    /// Whether this date is the last date of a range selection. Always
    /// `false` inside a [`Calendar`].
    pub range_end: bool,
    /// Whether this date falls strictly between the start and end of a
    /// range selection, excluding the start/end dates themselves (those are
    /// `range_start`/`range_end` instead). Always `false` inside a
    /// [`Calendar`].
    pub in_range: bool,
    /// Whether this date is the grid's current roving-tabindex/keyboard
    /// focus target.
    pub focused: bool,
    /// Whether this date is today (the calendar's configured `today` prop,
    /// which defaults to the real current date).
    pub today: bool,
    /// Whether this date is disabled: either the whole calendar is
    /// disabled, or this date is `unavailable`.
    pub disabled: bool,
    /// Whether this date falls outside the calendar's configured available
    /// date range (`min_date`/`max_date`/`disabled_ranges`).
    pub unavailable: bool,
    /// Whether this date is outside the month currently shown by the
    /// calendar grid -- a leading or trailing day from an adjacent month.
    pub outside_month: bool,
    relative_month: RelativeMonth,
}

/// Compute one date's full [`CalendarDayState`] the same way the built-in
/// [`CalendarDay`] cell does, for building a custom day-cell UI outside this
/// crate that still matches the built-in cell's behavior exactly -- see
/// upstream `DioxusLabs/components#199` ("the APIs to build each component
/// should be public"). Pair it with [`calendar_day_attributes`] to also
/// reproduce the built-in cell's `data-*`/`aria-*` attributes.
///
/// Must be called from a descendant of a [`Calendar`] or [`RangeCalendar`]
/// that has also rendered a [`CalendarView`] (directly, or via
/// [`CalendarGrid`], which requires one the same way [`use_calendar_grid`]
/// does). Returns `None`, rather than panicking, when called anywhere else,
/// so a misplaced call is a value to handle, not a crash.
///
/// ## Example
///
/// ```rust
/// use dioxus::prelude::*;
/// use dioxus_primitives::calendar::{calendar_day_attributes, use_calendar_day_state};
/// use time::Date;
///
/// #[component]
/// fn CustomDay(date: Date) -> Element {
///     let Some(state) = use_calendar_day_state(date) else {
///         return rsx! {};
///     };
///     rsx! {
///         button { ..calendar_day_attributes(&state), "{date.day()}" }
///     }
/// }
/// ```
pub fn use_calendar_day_state(date: Date) -> Option<CalendarDayState> {
    let base_ctx = try_use_context::<BaseCalendarContext>()?;
    let view_ctx = try_use_context::<CalendarViewContext>()?;
    let view_date = view_ctx.offset_view_date();
    let focused = base_ctx
        .focused_date()
        .is_some_and(|d| d == date && d.month() == view_date.month());

    if let Some(ctx) = try_use_context::<CalendarContext>() {
        let selected = (ctx.selected_date)().is_some_and(|d| d == date);
        Some(day_state(
            date,
            &base_ctx,
            view_date,
            focused,
            DaySelection {
                selected,
                range_start: false,
                range_end: false,
                in_range: false,
            },
        ))
    } else if let Some(ctx) = try_use_context::<RangeCalendarContext>() {
        let range = (ctx.highlighted_range)();
        let selected = range.is_some_and(|r| r.contains(date));
        Some(day_state(
            date,
            &base_ctx,
            view_date,
            focused,
            DaySelection {
                selected,
                range_start: is_start(date, range),
                range_end: is_end(date, range),
                in_range: is_between(date, range),
            },
        ))
    } else {
        None
    }
}

/// Build the `data-*`/`aria-*` attributes the built-in [`CalendarDay`] cell
/// itself renders, from a [`CalendarDayState`] (e.g. from
/// [`use_calendar_day_state`]). Spread the result onto a custom day cell's
/// own element (`..calendar_day_attributes(&state)`) to match the built-in
/// cell's styling hooks and accessible name exactly.
///
/// Does not include `tabindex`: unlike every attribute here, its value
/// depends on every date in the grid (only the single "roving" date gets
/// `tabindex="0"`), not just this one, so it can't be derived from a single
/// date's own [`CalendarDayState`] -- nor any event handler.
pub fn calendar_day_attributes(state: &CalendarDayState) -> Vec<Attribute> {
    fn attr(name: &'static str, value: dioxus_core::AttributeValue) -> Attribute {
        Attribute {
            name,
            namespace: None,
            volatile: false,
            value,
        }
    }

    let mut attrs = vec![attr("aria-label", Text(aria_label(&state.date)))];
    if state.today {
        attrs.push(attr("data-today", Bool(true)));
    }
    attrs.push(attr("data-selected", Bool(state.selected)));
    if state.unavailable {
        attrs.push(attr("data-unavailable", Bool(true)));
    }
    attrs.push(attr("data-disabled", Bool(state.disabled)));
    // backlog row 84 finding 1: an unavailable day (or a day inside a
    // disabled calendar -- `state.disabled` is already `(calendar disabled)
    // || unavailable`, see `day_state`) never told assistive tech it could
    // not be chosen: the cell has no native `disabled` attribute (a
    // genuinely disabled `<button>` would drop out of the tab order, which
    // `SingleCalendarDay`/`RangeCalendarDay` deliberately avoid so the grid
    // keeps a single roving tabstop -- see their own `handle_day_select`
    // early-return instead), and until now nothing else stood in for it.
    // `aria-disabled` (unlike a native `disabled` attribute) does not
    // remove the element from the tab order, so this is the correct
    // "focusable but not operable" signal -- matches this crate's own
    // established convention for exactly that shape of control (e.g.
    // `combobox`/`select`/`command`'s own options: `aria_disabled:
    // (option.disabled)()` alongside a still-focusable role). Mirrors
    // `data-disabled` immediately above: same source value, same
    // unconditional presence (a `data-disabled=false` day contributes
    // `aria-disabled=false`, which is the correct, explicit "not disabled"
    // ARIA state for a role that has no native disabled semantics of its
    // own, not merely "no opinion").
    attrs.push(attr("aria-disabled", Bool(state.disabled)));
    if state.range_start {
        attrs.push(attr("data-selection-start", Bool(true)));
    }
    if state.in_range {
        attrs.push(attr("data-selection-between", Bool(true)));
    }
    if state.range_end {
        attrs.push(attr("data-selection-end", Bool(true)));
    }
    attrs.push(attr("data-month", Text(state.relative_month.to_string())));
    attrs
}

/// The selection-related fields of [`CalendarDayState`], grouped into their
/// own type so [`day_state`] takes one struct instead of four positional
/// bools (clippy's `too_many_arguments`) -- each caller's own selection
/// model differs (`Option<Date>` equality for `Calendar`, `DateRange`
/// containment for `RangeCalendar`), so these are computed by the caller,
/// not by `day_state` itself.
#[derive(Default)]
struct DaySelection {
    selected: bool,
    range_start: bool,
    range_end: bool,
    in_range: bool,
}

/// Shared state computation behind [`use_calendar_day_state`] and the
/// built-in [`SingleCalendarDay`]/[`RangeCalendarDay`] cells -- the one
/// place `today`/`disabled`/`unavailable`/`outside_month` get computed, so
/// the built-in cell and a caller's own custom cell can't drift apart.
/// `focused`/`selection` are taken as already-computed rather than
/// recomputed here because the built-in cells need `focused` as a *live*,
/// signal-reading closure (not this function's one-shot snapshot) to drive
/// `use_day_mounted_ref`'s reactive auto-focus effect -- see those
/// components' own call sites for exactly what each one passes.
fn day_state(
    date: Date,
    base_ctx: &BaseCalendarContext,
    view_date: Date,
    focused: bool,
    selection: DaySelection,
) -> CalendarDayState {
    let relative_month = relative_calendar_month(date, base_ctx, view_date.month());
    let unavailable = base_ctx.is_unavailable(date);
    let disabled = (base_ctx.disabled)() || unavailable;

    CalendarDayState {
        date,
        selected: selection.selected,
        range_start: selection.range_start,
        range_end: selection.range_end,
        in_range: selection.in_range,
        focused,
        today: date == base_ctx.today,
        disabled,
        unavailable,
        outside_month: !relative_month.current_month(),
        relative_month,
    }
}

fn relative_calendar_month(
    date: Date,
    base_ctx: &BaseCalendarContext,
    current_month: Month,
) -> RelativeMonth {
    if date < base_ctx.enabled_date_range.start {
        RelativeMonth::Last
    } else if date > base_ctx.enabled_date_range.end {
        RelativeMonth::Next
    } else {
        match date.month().cmp(&current_month) {
            std::cmp::Ordering::Less => RelativeMonth::Last,
            std::cmp::Ordering::Equal => RelativeMonth::Current,
            std::cmp::Ordering::Greater => RelativeMonth::Next,
        }
    }
}

fn is_between(date: Date, range: Option<DateRange>) -> bool {
    range.is_some_and(|r| r.contained_in_interval(date))
}

fn is_start(date: Date, range: Option<DateRange>) -> bool {
    range.is_some_and(|r| r.start == date && date != r.end)
}

fn is_end(date: Date, range: Option<DateRange>) -> bool {
    range.is_some_and(|r| r.end == date && date != r.start)
}

fn use_day_mounted_ref(
    mut is_focused: impl FnMut() -> bool + 'static,
) -> impl FnMut(MountedEvent) + 'static {
    let mut day_ref: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    use_effect(move || {
        if let Some(day) = day_ref() {
            if is_focused() {
                spawn(async move {
                    _ = day.set_focus(true).await;
                });
            }
        }
    });
    move |e| day_ref.set(Some(e.data()))
}

#[component]
fn SingleCalendarDay(props: CalendarDayProps) -> Element {
    let CalendarDayProps {
        date,
        attributes,
        children,
    } = props;
    let mut base_ctx: BaseCalendarContext = use_context();
    let view_ctx: CalendarViewContext = use_context();
    let day = date.day();
    let content = children.unwrap_or_else(|| rsx! { {day.to_string()} });
    let view_date = view_ctx.offset_view_date();
    let is_focused = move || {
        base_ctx
            .focused_date()
            .is_some_and(|d| d == date && d.month() == view_date.month())
    };
    let is_unavailable = base_ctx.is_unavailable(date);
    let onmounted = use_day_mounted_ref(is_focused);

    let ctx: CalendarContext = use_context();
    let is_selected = move || (ctx.selected_date)().is_some_and(|d| d == date);

    // Single source of truth for this cell's state (styling attributes
    // below) and the built-in `CalendarDayState`/`use_calendar_day_state`
    // public API -- see `day_state`'s own doc for why `focused`/`selected`
    // are passed in already-computed rather than recomputed inside it.
    let state = day_state(
        date,
        &base_ctx,
        view_date,
        is_focused(),
        DaySelection {
            selected: is_selected(),
            range_start: false,
            range_end: false,
            in_range: false,
        },
    );
    let in_current_month = !state.outside_month;

    // Handle day selection
    let mut handle_day_select = move |day: u8| {
        if (base_ctx.disabled)() || is_unavailable {
            return;
        }
        let view_date = view_ctx.offset_view_date();
        let date = view_date.replace_day(day).unwrap();
        ctx.set_selected_date.call((!is_selected()).then_some(date));
        base_ctx.focused_date.set(Some(date));
    };

    let focusable_date = (base_ctx.focused_date)()
        .filter(|d| d.month() == view_date.month())
        .or_else(|| {
            ctx.selected_date
                .cloned()
                .filter(|d| d.month() == view_date.month())
        })
        .unwrap_or(view_date);

    // `type` is an overridable default; the roving-focus `tabindex` and
    // `calendar_day_attributes`'s aria/data state are owned, so they merge
    // in last -- same precedence (owned wins) production already had via
    // the old first-literal-wins SSR/last-wins CSR split (backlog row 93),
    // just without the duplicate attribute a caller override used to
    // silently collide with after hydration.
    let defaults = attributes!(button { r#type: "button" });
    let owned = merge_attributes(vec![
        calendar_day_attributes(&state),
        attributes!(button {
            tabindex: if date == focusable_date { "0" } else { "-1" },
        }),
    ]);
    let merged = merge_attributes(vec![defaults, attributes, owned]);

    rsx! {
        button {
            onclick: move |e| {
                e.prevent_default();
                if in_current_month {
                    handle_day_select(day);
                }
            },
            onfocus: move |_| {
                if in_current_month {
                    base_ctx.focused_date.set(Some(date));
                }
            },
            onmounted,
            ..merged,
            {content}
        }
    }
}

#[component]
fn RangeCalendarDay(props: CalendarDayProps) -> Element {
    let CalendarDayProps {
        date,
        attributes,
        children,
    } = props;
    let mut base_ctx: BaseCalendarContext = use_context();
    let day = date.day();
    let content = children.unwrap_or_else(|| rsx! { {day.to_string()} });
    let view_ctx: CalendarViewContext = use_context();
    let view_date = view_ctx.offset_view_date();
    let is_focused = move || {
        base_ctx
            .focused_date()
            .is_some_and(|d| d == date && d.month() == view_date.month())
    };
    let is_unavailable = base_ctx.is_unavailable(date);
    let onmounted = use_day_mounted_ref(is_focused);

    let mut ctx: RangeCalendarContext = use_context();
    let range = ctx.highlighted_range.cloned();
    let selected = range.is_some_and(|r| r.contains(date));

    // Single source of truth for this cell's state (styling attributes
    // below) and the built-in `CalendarDayState`/`use_calendar_day_state`
    // public API -- see `day_state`'s own doc for why `focused`/`selected`/
    // `range_*` are passed in already-computed rather than recomputed
    // inside it. `selected` uses `DateRange::contains` (inclusive of the
    // range's start/end) while `range_start`/`range_end`/`in_range` use
    // `is_start`/`is_end`/`is_between` (which exclude them) -- these are
    // deliberately different predicates for deliberately different
    // attributes; see `DateRange::contains` vs `contained_in_interval`.
    let state = day_state(
        date,
        &base_ctx,
        view_date,
        is_focused(),
        DaySelection {
            selected,
            range_start: is_start(date, range),
            range_end: is_end(date, range),
            in_range: is_between(date, range),
        },
    );
    let in_current_month = !state.outside_month;

    let clamp_date_to_available_range = move |date| {
        let available_range = base_ctx.available_range();
        available_range.map_or(date, |range| range.clamp(date))
    };

    // Handle day selection
    let mut handle_day_select = move |day: u8| {
        if (base_ctx.disabled)() || is_unavailable {
            return;
        }

        let view_date = view_ctx.offset_view_date();
        let date = view_date
            .replace_day(day)
            .ok()
            .map(clamp_date_to_available_range);
        ctx.set_selected_date(date);
        base_ctx.focused_date.set(date);
    };

    let focusable_date = (base_ctx.focused_date)()
        .filter(|d| d.month() == view_date.month())
        .or_else(|| {
            ctx.anchor_date
                .cloned()
                .filter(|d| d.month() == view_date.month())
        })
        .unwrap_or(view_date);

    // Same reasoning as `SingleCalendarDay` above.
    let defaults = attributes!(button { r#type: "button" });
    let owned = merge_attributes(vec![
        calendar_day_attributes(&state),
        attributes!(button {
            tabindex: if date == focusable_date { "0" } else { "-1" },
        }),
    ]);
    let merged = merge_attributes(vec![defaults, attributes, owned]);

    rsx! {
        button {
            onclick: move |e| {
                e.prevent_default();
                if in_current_month {
                    handle_day_select(day);
                }
            },
            onfocus: move |_| {
                if in_current_month {
                    base_ctx.focused_date.set(Some(date));
                }
            },
            onmouseover: move |_| {
                if in_current_month {
                    ctx.set_hovered_date(clamp_date_to_available_range(date));
                }
            },
            onmounted,
            ..merged,
            {content}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;
    use time::macros::date;

    #[test]
    fn horizontal_day_step_ltr_matches_arrow_direction() {
        assert_eq!(
            horizontal_day_step(&Key::ArrowRight, Direction::Ltr),
            Some(1)
        );
        assert_eq!(
            horizontal_day_step(&Key::ArrowLeft, Direction::Ltr),
            Some(-1)
        );
    }

    #[test]
    fn horizontal_day_step_rtl_swaps_left_and_right() {
        assert_eq!(
            horizontal_day_step(&Key::ArrowRight, Direction::Rtl),
            Some(-1)
        );
        assert_eq!(
            horizontal_day_step(&Key::ArrowLeft, Direction::Rtl),
            Some(1)
        );
    }

    #[test]
    fn horizontal_day_step_ignores_vertical_keys_both_directions() {
        for direction in [Direction::Ltr, Direction::Rtl] {
            assert_eq!(horizontal_day_step(&Key::ArrowUp, direction), None);
            assert_eq!(horizontal_day_step(&Key::ArrowDown, direction), None);
            assert_eq!(horizontal_day_step(&Key::Home, direction), None);
            assert_eq!(horizontal_day_step(&Key::End, direction), None);
        }
    }

    #[component]
    fn ConsecutiveCalendarViews() -> Element {
        rsx! {
            Calendar {
                view_date: date!(2026 - 05 - 15),
                CalendarView {
                    CalendarMonthTitle {}
                }
                CalendarView {
                    CalendarMonthTitle {}
                }
                CalendarView {
                    CalendarMonthTitle {}
                }
            }
        }
    }

    #[component]
    fn CalendarDayWithCustomChild() -> Element {
        rsx! {
            Calendar {
                view_date: date!(2026 - 05 - 15),
                CalendarView {
                    CalendarDay {
                        date: date!(2026 - 05 - 15),
                        "Custom day"
                    }
                }
            }
        }
    }

    #[component]
    fn RangeCalendarDayWithCustomChild() -> Element {
        rsx! {
            RangeCalendar {
                view_date: date!(2026 - 05 - 15),
                CalendarView {
                    CalendarDay {
                        date: date!(2026 - 05 - 15),
                        "Custom range day"
                    }
                }
            }
        }
    }

    #[test]
    fn implicit_calendar_views_render_consecutive_months_on_first_render() {
        let mut dom = VirtualDom::new(ConsecutiveCalendarViews);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        assert!(html.contains("May 2026"));
        assert!(html.contains("June 2026"));
        assert!(html.contains("July 2026"));
    }

    #[test]
    fn calendar_day_forwards_custom_children() {
        let mut dom = VirtualDom::new(CalendarDayWithCustomChild);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        assert!(html.contains("Custom day"));
        assert!(!html.contains(">15</button>"));
    }

    #[test]
    fn range_calendar_day_forwards_custom_children() {
        let mut dom = VirtualDom::new(RangeCalendarDayWithCustomChild);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        assert!(html.contains("Custom range day"));
        assert!(!html.contains(">15</button>"));
    }

    // --- CalendarDayState / use_calendar_day_state / calendar_day_attributes
    // (dev-docs/backlog.md row 12c, upstream DioxusLabs/components#199) ---

    #[test]
    fn day_state_computes_today_disabled_unavailable_and_relative_month() {
        with_runtime(|| {
            let disabled_ranges = [DateRange::new(date!(2024 - 06 - 10), date!(2024 - 06 - 20))];
            let mut base_ctx = make_base_ctx_for_relative_month(DateRange::new(
                date!(2024 - 01 - 01),
                date!(2024 - 12 - 31),
            ));
            base_ctx.available_ranges = use_memo(move || AvailableRanges::new(&disabled_ranges));
            let view_date = date!(2024 - 06 - 01); // base_ctx.today == 2024-06-15

            // A plain, available, current-month day.
            let state = day_state(
                date!(2024 - 06 - 01),
                &base_ctx,
                view_date,
                false,
                DaySelection::default(),
            );
            assert!(!state.today);
            assert!(!state.disabled);
            assert!(!state.unavailable);
            assert!(!state.outside_month);
            assert_eq!(state.relative_month, RelativeMonth::Current);

            // Today.
            let state = day_state(
                date!(2024 - 06 - 15),
                &base_ctx,
                view_date,
                false,
                DaySelection::default(),
            );
            assert!(state.today);

            // Inside the disabled range: unavailable, and therefore disabled
            // too even though the calendar itself is not.
            let state = day_state(
                date!(2024 - 06 - 12),
                &base_ctx,
                view_date,
                false,
                DaySelection::default(),
            );
            assert!(state.unavailable);
            assert!(state.disabled);

            // A day from the previous/next month shown in the grid.
            let state = day_state(
                date!(2024 - 05 - 28),
                &base_ctx,
                view_date,
                false,
                DaySelection::default(),
            );
            assert!(state.outside_month);
            assert_eq!(state.relative_month, RelativeMonth::Last);
            let state = day_state(
                date!(2024 - 07 - 03),
                &base_ctx,
                view_date,
                false,
                DaySelection::default(),
            );
            assert!(state.outside_month);
            assert_eq!(state.relative_month, RelativeMonth::Next);

            // `focused`/`selected`/`range_*` pass through unchanged.
            let state = day_state(
                date!(2024 - 06 - 01),
                &base_ctx,
                view_date,
                true,
                DaySelection {
                    selected: true,
                    range_start: true,
                    range_end: true,
                    in_range: true,
                },
            );
            assert!(state.focused);
            assert!(state.selected);
            assert!(state.range_start);
            assert!(state.range_end);
            assert!(state.in_range);
        });
    }

    fn day_state_fixture(overrides: impl FnOnce(&mut CalendarDayState)) -> CalendarDayState {
        let mut state = CalendarDayState {
            date: date!(2024 - 06 - 15),
            selected: false,
            range_start: false,
            range_end: false,
            in_range: false,
            focused: false,
            today: false,
            disabled: false,
            unavailable: false,
            outside_month: false,
            relative_month: RelativeMonth::Current,
        };
        overrides(&mut state);
        state
    }

    fn find_attr<'a>(attrs: &'a [Attribute], name: &str) -> Option<&'a Attribute> {
        attrs.iter().find(|a| a.name == name)
    }

    #[test]
    fn calendar_day_attributes_always_includes_aria_label_selected_and_disabled() {
        let state = day_state_fixture(|_| {});
        let attrs = calendar_day_attributes(&state);

        assert_eq!(
            find_attr(&attrs, "aria-label").unwrap().value,
            Text(aria_label(&state.date))
        );
        assert_eq!(
            find_attr(&attrs, "data-selected").unwrap().value,
            Bool(false)
        );
        assert_eq!(
            find_attr(&attrs, "data-disabled").unwrap().value,
            Bool(false)
        );
        // backlog row 84 finding 1: `aria-disabled` mirrors `data-disabled`
        // exactly (same source value, same unconditional presence) -- see
        // the fix's own comment in `calendar_day_attributes`.
        assert_eq!(
            find_attr(&attrs, "aria-disabled").unwrap().value,
            Bool(false)
        );
        assert_eq!(
            find_attr(&attrs, "data-month").unwrap().value,
            Text("current".to_string())
        );
        // Booleans that are false today are omitted entirely, matching the
        // built-in cell's `if cond { true }` (no-else) attributes.
        assert!(find_attr(&attrs, "data-today").is_none());
        assert!(find_attr(&attrs, "data-unavailable").is_none());
        assert!(find_attr(&attrs, "data-selection-start").is_none());
        assert!(find_attr(&attrs, "data-selection-between").is_none());
        assert!(find_attr(&attrs, "data-selection-end").is_none());
    }

    #[test]
    fn calendar_day_attributes_renders_true_valued_booleans_unquoted() {
        // `AttributeValue::Bool` (not `Text`) renders unquoted
        // (`dioxus-ssr`'s `write_attribute`) -- these must stay `Bool`, not
        // a `Text("true")` lookalike, to keep the built-in cell's rendered
        // HTML byte-identical.
        let state = day_state_fixture(|s| {
            s.today = true;
            s.selected = true;
            s.unavailable = true;
            s.disabled = true;
            s.range_start = true;
            s.range_end = true;
            s.in_range = true;
        });
        let attrs = calendar_day_attributes(&state);

        for name in [
            "data-today",
            "data-selected",
            "data-unavailable",
            "data-disabled",
            "aria-disabled",
            "data-selection-start",
            "data-selection-between",
            "data-selection-end",
        ] {
            assert_eq!(
                find_attr(&attrs, name)
                    .unwrap_or_else(|| panic!("missing {name}"))
                    .value,
                Bool(true),
                "{name} must be present and true"
            );
        }
    }

    #[test]
    fn calendar_day_attributes_reports_last_and_next_relative_month() {
        let state = day_state_fixture(|s| {
            s.outside_month = true;
            s.relative_month = RelativeMonth::Last;
        });
        assert_eq!(
            find_attr(&calendar_day_attributes(&state), "data-month")
                .unwrap()
                .value,
            Text("last".to_string())
        );

        let state = day_state_fixture(|s| {
            s.outside_month = true;
            s.relative_month = RelativeMonth::Next;
        });
        assert_eq!(
            find_attr(&calendar_day_attributes(&state), "data-month")
                .unwrap()
                .value,
            Text("next".to_string())
        );
    }

    #[component]
    fn DayStateOutsideCalendar() -> Element {
        let state = use_calendar_day_state(date!(2024 - 06 - 15));
        rsx! {
            div { id: "result", "{state.is_none()}" }
        }
    }

    #[test]
    fn use_calendar_day_state_returns_none_outside_a_calendar_context() {
        let mut dom = VirtualDom::new(DayStateOutsideCalendar);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        assert!(html.contains(r#"<div id="result">true</div>"#));
    }

    #[component]
    fn DayStateInsideCalendarView() -> Element {
        // `CalendarGrid` isn't rendered -- proves `use_calendar_day_state`
        // only needs a `CalendarView`, matching `use_calendar_grid`'s own
        // context requirements (see `use_calendar_grid`).
        let state = use_calendar_day_state(date!(2024 - 06 - 15));
        rsx! {
            div { id: "result", "{state.is_none()}" }
        }
    }

    #[test]
    fn use_calendar_day_state_is_some_inside_a_calendar_view() {
        #[component]
        fn Demo() -> Element {
            rsx! {
                Calendar {
                    view_date: date!(2024 - 06 - 01),
                    today: date!(2024 - 06 - 15),
                    CalendarView { DayStateInsideCalendarView {} }
                }
            }
        }

        let mut dom = VirtualDom::new(Demo);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        assert!(html.contains(r#"<div id="result">false</div>"#));
    }

    #[test]
    fn use_calendar_day_state_matches_day_state_for_a_single_calendar() {
        // The hook must compute the *same* `CalendarDayState` `day_state`
        // does for the built-in cell -- that's the "one source of truth"
        // this refactor exists for.
        with_runtime(|| {
            let disabled_ranges = [DateRange::new(date!(2024 - 06 - 18), date!(2024 - 06 - 19))];
            let mut base_ctx = make_base_ctx_for_relative_month(DateRange::new(
                date!(2024 - 01 - 01),
                date!(2024 - 12 - 31),
            ));
            base_ctx.available_ranges = use_memo(move || AvailableRanges::new(&disabled_ranges));
            use_context_provider(|| base_ctx);
            use_context_provider(|| CalendarViewContext { offset: 0 });
            use_context_provider(|| CalendarContext {
                selected_date: Signal::new(Some(date!(2024 - 06 - 15))).into(),
                set_selected_date: Callback::new(|_: Option<Date>| {}),
            });

            let view_date = base_ctx.view_date();

            let state = use_calendar_day_state(date!(2024 - 06 - 15))
                .expect("inside a Calendar + CalendarView context");
            let expected = day_state(
                date!(2024 - 06 - 15),
                &base_ctx,
                view_date,
                state.focused,
                DaySelection {
                    selected: true,
                    ..Default::default()
                },
            );
            assert_eq!(state, expected);
            assert!(state.selected);
            assert!(state.today);
            assert!(!state.range_start && !state.range_end && !state.in_range);

            // An unavailable date outside the selection.
            let state = use_calendar_day_state(date!(2024 - 06 - 18)).unwrap();
            assert!(state.unavailable);
            assert!(state.disabled);
            assert!(!state.selected);
        });
    }

    #[test]
    fn use_calendar_day_state_matches_day_state_for_a_range_calendar() {
        with_runtime(|| {
            let base_ctx = make_base_ctx_for_relative_month(DateRange::new(
                date!(2024 - 01 - 01),
                date!(2024 - 12 - 31),
            ));
            use_context_provider(|| base_ctx);
            use_context_provider(|| CalendarViewContext { offset: 0 });
            let range = DateRange::new(date!(2024 - 06 - 10), date!(2024 - 06 - 20));
            use_context_provider(|| RangeCalendarContext {
                anchor_date: Signal::new(None),
                highlighted_range: Signal::new(Some(range)),
                set_selected_range: Callback::new(|_: Option<DateRange>| {}),
            });

            let start = use_calendar_day_state(date!(2024 - 06 - 10)).unwrap();
            assert!(
                start.selected,
                "range endpoints are inclusive of `selected`"
            );
            assert!(start.range_start);
            assert!(!start.in_range);
            assert!(!start.range_end);

            let middle = use_calendar_day_state(date!(2024 - 06 - 15)).unwrap();
            assert!(middle.selected);
            assert!(middle.in_range);
            assert!(!middle.range_start && !middle.range_end);

            let end = use_calendar_day_state(date!(2024 - 06 - 20)).unwrap();
            assert!(end.selected);
            assert!(end.range_end);
            assert!(!end.in_range && !end.range_start);

            let outside = use_calendar_day_state(date!(2024 - 06 - 25)).unwrap();
            assert!(
                !outside.selected
                    && !outside.range_start
                    && !outside.range_end
                    && !outside.in_range
            );

            let view_date = base_ctx.view_date();
            let expected_middle = day_state(
                date!(2024 - 06 - 15),
                &base_ctx,
                view_date,
                middle.focused,
                DaySelection {
                    selected: true,
                    range_start: false,
                    range_end: false,
                    in_range: true,
                },
            );
            assert_eq!(middle, expected_middle);
        });
    }

    #[test]
    fn single_calendar_day_renders_today_selected_and_month_attributes() {
        #[component]
        fn Demo() -> Element {
            rsx! {
                Calendar {
                    view_date: date!(2024 - 06 - 01),
                    today: date!(2024 - 06 - 15),
                    selected_date: Some(date!(2024 - 06 - 15)),
                    CalendarView {
                        CalendarDay { date: date!(2024 - 06 - 15) }
                    }
                }
            }
        }

        let mut dom = VirtualDom::new(Demo);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        assert!(html.contains(&format!(
            r#"aria-label="{}""#,
            aria_label(&date!(2024 - 06 - 15))
        )));
        assert!(html.contains("data-today=true"));
        assert!(html.contains("data-selected=true"));
        assert!(html.contains("data-disabled=false"));
        assert!(html.contains("aria-disabled=false"));
        assert!(html.contains(r#"data-month="current""#));
        assert!(!html.contains("data-unavailable"));
    }

    #[test]
    fn unavailable_calendar_day_renders_aria_disabled_true_and_available_day_renders_aria_disabled_false(
    ) {
        // backlog row 84 finding 1: "Calendar's unavailable days carry no
        // aria-disabled" -- an end-to-end (full `Calendar`, real
        // `disabled_ranges`, `dioxus_ssr::render`) regression guard rather
        // than only the lower-level `calendar_day_attributes` fixture tests
        // above, so this also exercises `is_unavailable`/`day_state`'s own
        // wiring, not just the attribute-building function in isolation.
        #[component]
        fn Demo() -> Element {
            rsx! {
                Calendar {
                    view_date: date!(2024 - 06 - 01),
                    today: date!(2024 - 06 - 01),
                    disabled_ranges: vec![DateRange::new(date!(2024 - 06 - 15), date!(2024 - 06 - 15))],
                    CalendarView {
                        CalendarDay { date: date!(2024 - 06 - 15) }
                        CalendarDay { date: date!(2024 - 06 - 16) }
                    }
                }
            }
        }

        let mut dom = VirtualDom::new(Demo);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        let unavailable_label = format!(r#"aria-label="{}""#, aria_label(&date!(2024 - 06 - 15)));
        let available_label = format!(r#"aria-label="{}""#, aria_label(&date!(2024 - 06 - 16)));
        let unavailable_label_pos = html
            .find(&unavailable_label)
            .expect("unavailable day must render");
        let available_label_pos = html
            .find(&available_label)
            .expect("available day must render");

        // Anchor each slice on the enclosing `<button`'s start, not the
        // `aria-label` attribute's own position: `merge_attributes` (backlog
        // row 93) renders attributes sorted by name rather than in literal
        // source order, so `aria-disabled` (`d` < `l`) now precedes
        // `aria-label` in the output -- slicing from `aria-label` itself
        // would cut it off into the *previous* cell's slice.
        let unavailable_pos = html[..unavailable_label_pos]
            .rfind("<button")
            .expect("unavailable day's enclosing <button> must be found");
        let available_pos = html[..available_label_pos]
            .rfind("<button")
            .expect("available day's enclosing <button> must be found");

        // Slice each day's own attribute run (up to the next day's button,
        // or the end of the string for the last one) so
        // `aria-disabled=false`/`aria-disabled=true` are each checked
        // against the correct cell, not just "somewhere in the whole grid".
        let unavailable_html = &html[unavailable_pos..available_pos.max(unavailable_pos)];
        let available_html = &html[available_pos..];

        assert!(
            unavailable_html.contains("data-unavailable=true"),
            "sanity: the fixture's disabled_ranges must actually make this day unavailable: {unavailable_html}"
        );
        assert!(
            unavailable_html.contains("aria-disabled=true"),
            "an unavailable day must carry aria-disabled=true: {unavailable_html}"
        );
        assert!(
            !available_html.contains("data-unavailable"),
            "sanity: the following day must be available: {available_html}"
        );
        assert!(
            available_html.contains("aria-disabled=false"),
            "an available day must still carry an explicit aria-disabled=false: {available_html}"
        );
    }

    #[test]
    fn range_calendar_day_renders_selection_start_between_and_end_attributes() {
        #[component]
        fn Demo() -> Element {
            rsx! {
                RangeCalendar {
                    view_date: date!(2024 - 06 - 01),
                    today: date!(2024 - 06 - 01),
                    selected_range: Some(DateRange::new(date!(2024 - 06 - 10), date!(2024 - 06 - 20))),
                    CalendarView {
                        CalendarDay { date: date!(2024 - 06 - 10) }
                        CalendarDay { date: date!(2024 - 06 - 15) }
                        CalendarDay { date: date!(2024 - 06 - 20) }
                    }
                }
            }
        }

        let mut dom = VirtualDom::new(Demo);
        dom.rebuild_in_place();
        let html = dioxus_ssr::render(&dom);

        assert!(html.contains("data-selection-start=true"));
        assert!(html.contains("data-selection-between=true"));
        assert!(html.contains("data-selection-end=true"));
        assert_eq!(
            html.matches("data-selected=true").count(),
            3,
            "all three dates are within the inclusive selected range"
        );
    }

    #[test]
    fn test_weekday_set() {
        let mut weekdays = WeekdaySet::single(Weekday::Monday);
        // Test contains
        assert!(weekdays.contains(Weekday::Monday));
        assert!(!weekdays.contains(Weekday::Tuesday));

        // Test remove
        assert!(weekdays.remove(Weekday::Monday));
        assert!(!weekdays.contains(Weekday::Monday));
        assert!(!weekdays.remove(Weekday::Monday)); // Already removed

        let all_days = WeekdaySet(0b111_1111); // All days
        let empty_set = WeekdaySet(0b000_0000); // Empty
        let single_set = WeekdaySet::single(Weekday::Friday); // Single day
        let part_size_set = WeekdaySet(0b010_1010); // Tu, Th, Sa

        // Test iterator
        let days: Vec<_> = all_days.iter(Weekday::Sunday).collect();
        assert_eq!(days.len(), 7);
        assert_eq!(days[0], Weekday::Sunday);

        let mut iter = all_days.iter(Weekday::Wednesday);
        assert_eq!(iter.next(), Some(Weekday::Wednesday));
        assert_eq!(iter.next(), Some(Weekday::Thursday));

        // Test first
        assert_eq!(empty_set.first(), None);
        assert_eq!(single_set.first(), Some(Weekday::Friday));
        assert_eq!(part_size_set.first(), Some(Weekday::Tuesday));
        assert_eq!(all_days.first(), Some(Weekday::Monday));

        // Test is_empty
        assert!(empty_set.is_empty());
        assert!(!part_size_set.is_empty());
        assert!(!single_set.is_empty());
        assert!(!all_days.is_empty());
    }

    #[test]
    fn test_days_since() {
        // Test days since calculation
        let date = date!(2024 - 01 - 01); // Monday
        assert_eq!(days_since(date, Weekday::Monday), 0);
        assert_eq!(days_since(date, Weekday::Sunday), 1);
        assert_eq!(days_since(date, Weekday::Tuesday), 6);
    }

    #[test]
    fn test_month_navigation() {
        let date = date!(2024 - 01 - 15);

        // Test next month
        let next = next_month(date);
        assert!(next.is_some());
        assert_eq!(next.unwrap().month(), Month::February);
        assert_eq!(next.unwrap().year(), 2024);
        assert_eq!(next.unwrap().day(), 15);

        // Test previous month
        let prev = previous_month(date);
        assert!(prev.is_some());
        assert_eq!(prev.unwrap().month(), Month::December);
        assert_eq!(prev.unwrap().year(), 2023);
        assert_eq!(prev.unwrap().day(), 15);
    }

    #[test]
    fn test_calendar_grid_weeks() {
        // Calls the real `calendar_grid_weeks` (the function `CalendarGrid`/
        // `use_calendar_grid` actually use to build the rendered month grid) --
        // not a hand-copied reimplementation. See docs/mutants-baseline.md fix #1.

        // Test February 2021: starts on Monday, 28 days
        // When first day of week is Monday, should fit in exactly 4 weeks
        let feb_2021 = date!(2021 - 02 - 15);
        let feb_grid = calendar_grid_weeks(feb_2021, Weekday::Monday);
        assert_eq!(
            feb_grid.len(),
            4,
            "February 2021 should have exactly 4 weeks"
        );

        // Count days from February in the grid
        let feb_days: Vec<_> = feb_grid
            .iter()
            .flatten()
            .filter(|d| d.month() == Month::February && d.year() == 2021)
            .collect();
        assert_eq!(
            feb_days.len(),
            28,
            "Should have all 28 days of February 2021"
        );

        // Test May 2024: starts on Wednesday, 31 days
        // When first day of week is Sunday, should need 5 weeks
        let may_2024 = date!(2024 - 05 - 15);
        let may_grid = calendar_grid_weeks(may_2024, Weekday::Sunday);
        assert_eq!(
            may_grid.len(),
            5,
            "May 2024 should have exactly 5 weeks when starting from Sunday"
        );

        // Count days from May in the grid
        let may_days: Vec<_> = may_grid
            .iter()
            .flatten()
            .filter(|d| d.month() == Month::May && d.year() == 2024)
            .collect();
        assert_eq!(may_days.len(), 31, "Should have all 31 days of May 2024");

        // Test that we don't generate empty trailing weeks
        // December 2018: starts on Saturday, 31 days (when week starts on Sunday)
        // Should need exactly 6 weeks (30 days in November + 31 in December + 5 in January = 66/7 = 6)
        let dec_2018 = date!(2018 - 12 - 15);
        let dec_grid = calendar_grid_weeks(dec_2018, Weekday::Sunday);
        assert_eq!(
            dec_grid.len(),
            6,
            "December 2018 should have exactly 6 weeks"
        );

        // Verify no week is completely empty, and every week is contiguous
        // and every week's 7 dates are consecutive calendar days
        let mut prev: Option<Date> = None;
        for week in &dec_grid {
            assert!(!week.is_empty(), "No week should be empty");
            assert_eq!(week.len(), 7, "Each week should have exactly 7 days");
            for &d in week {
                if let Some(p) = prev {
                    assert_eq!(p.next_day().unwrap(), d, "grid dates must be consecutive");
                }
                prev = Some(d);
            }
        }

        // The grid's first date is the correct number of days before the 1st
        // of the month for the given first_day_of_week (i.e. the leading
        // days from the previous month line up under the right weekday
        // column), and the grid's last date completes the final week.
        let first_of_dec = dec_2018.replace_day(1).unwrap();
        assert_eq!(dec_grid[0][0].weekday(), Weekday::Sunday);
        assert_eq!(
            first_of_dec - dec_grid[0][0],
            time::Duration::days(days_since(dec_2018, Weekday::Sunday))
        );
    }

    // -----------------------------------------------------------------
    // Pure predicates (docs/mutants-baseline.md fix #8)
    // -----------------------------------------------------------------

    #[test]
    fn test_nth_month_next_and_previous() {
        // NOTE: `nth_month_next`/`nth_month_previous` compute the target month via
        // `Month::nth_next`/`nth_prev`, which cycle *within* a 12-month wheel, and only
        // bump the year by at most 1 to account for that single wrap. They are only ever
        // called in this codebase with small offsets (multi-month calendar view offsets),
        // so this is exercised here with n in a realistic small range rather than n >= 12,
        // where the single `+1`/`-1` year adjustment would under/overshoot the real
        // number of year boundaries crossed.
        let date = date!(2024 - 01 - 15);

        assert_eq!(nth_month_next(date, 0), Some(date), "n=0 is a no-op");
        assert_eq!(nth_month_next(date, 1), Some(date!(2024 - 02 - 15)));

        // Crosses a year boundary within a single 12-month cycle.
        let nov = date!(2024 - 11 - 15);
        assert_eq!(
            nth_month_next(nov, 2),
            Some(date!(2025 - 01 - 15)),
            "wraps into the next year"
        );

        // Clamps the day when the target month is shorter.
        let jan_31 = date!(2024 - 01 - 31);
        assert_eq!(nth_month_next(jan_31, 1), Some(date!(2024 - 02 - 29))); // 2024 is a leap year

        assert_eq!(nth_month_previous(date, 0), Some(date), "n=0 is a no-op");
        assert_eq!(nth_month_previous(date, 1), Some(date!(2023 - 12 - 15)));
        assert_eq!(
            nth_month_previous(nov, 2),
            Some(date!(2024 - 09 - 15)),
            "no wrap needed for a same-year previous"
        );

        let mar_31 = date!(2024 - 03 - 31);
        assert_eq!(nth_month_previous(mar_31, 1), Some(date!(2024 - 02 - 29)));
    }

    #[test]
    fn test_date_range_contains_and_contained_in_interval() {
        let range = DateRange::new(date!(2024 - 01 - 10), date!(2024 - 01 - 20));

        // `contains` is inclusive of both endpoints.
        assert!(range.contains(date!(2024 - 01 - 10)));
        assert!(range.contains(date!(2024 - 01 - 20)));
        assert!(range.contains(date!(2024 - 01 - 15)));
        assert!(!range.contains(date!(2024 - 01 - 09)));
        assert!(!range.contains(date!(2024 - 01 - 21)));

        // `contained_in_interval` is exclusive of both endpoints.
        assert!(!range.contained_in_interval(date!(2024 - 01 - 10)));
        assert!(!range.contained_in_interval(date!(2024 - 01 - 20)));
        assert!(range.contained_in_interval(date!(2024 - 01 - 15)));

        // `DateRange::new` normalizes a reversed (start, end) pair.
        let reversed = DateRange::new(date!(2024 - 01 - 20), date!(2024 - 01 - 10));
        assert_eq!(reversed.start(), date!(2024 - 01 - 10));
        assert_eq!(reversed.end(), date!(2024 - 01 - 20));
    }

    #[test]
    fn test_date_range_display() {
        let range = DateRange::new(date!(2024 - 01 - 10), date!(2024 - 01 - 20));
        assert_eq!(
            range.to_string(),
            format!("{} - {}", date!(2024 - 01 - 10), date!(2024 - 01 - 20))
        );
        assert!(!range.to_string().is_empty());
    }

    #[test]
    fn test_available_ranges_valid_interval_and_to_disabled_ranges() {
        let disabled = [
            DateRange::new(date!(2024 - 01 - 10), date!(2024 - 01 - 15)),
            DateRange::new(date!(2024 - 02 - 01), date!(2024 - 02 - 05)),
        ];
        let ranges = AvailableRanges::new(&disabled);

        // Inside a disabled range (inclusive endpoints) is invalid.
        assert!(!ranges.valid_interval(date!(2024 - 01 - 10)));
        assert!(!ranges.valid_interval(date!(2024 - 01 - 12)));
        assert!(!ranges.valid_interval(date!(2024 - 01 - 15)));
        // Outside any disabled range is valid.
        assert!(ranges.valid_interval(date!(2024 - 01 - 09)));
        assert!(ranges.valid_interval(date!(2024 - 01 - 16)));
        assert!(ranges.valid_interval(date!(2024 - 01 - 31)));
        assert!(!ranges.valid_interval(date!(2024 - 02 - 03)));

        // Round trips back out as the same (sorted, merged) disabled ranges.
        assert_eq!(ranges.to_disabled_ranges(), disabled.to_vec());
    }

    #[test]
    fn test_available_ranges_merges_overlapping_disabled_ranges() {
        // Two overlapping disabled ranges collapse into one.
        let disabled = [
            DateRange::new(date!(2024 - 01 - 10), date!(2024 - 01 - 20)),
            DateRange::new(date!(2024 - 01 - 15), date!(2024 - 01 - 25)),
        ];
        let ranges = AvailableRanges::new(&disabled);

        assert_eq!(
            ranges.to_disabled_ranges(),
            vec![DateRange::new(date!(2024 - 01 - 10), date!(2024 - 01 - 25))]
        );
        assert!(!ranges.valid_interval(date!(2024 - 01 - 18)));
    }

    #[test]
    fn test_available_ranges_available_range() {
        let disabled = [DateRange::new(date!(2024 - 01 - 10), date!(2024 - 01 - 20))];
        let ranges = AvailableRanges::new(&disabled);
        let bounds = DateRange::new(date!(2024 - 01 - 01), date!(2024 - 01 - 31));

        // A date between two disabled boundaries gets the gap between them.
        let available = ranges
            .available_range(date!(2024 - 01 - 05), bounds)
            .expect("date is not disabled");
        assert_eq!(available.start(), date!(2024 - 01 - 01));
        assert_eq!(available.end(), date!(2024 - 01 - 09));

        let available = ranges
            .available_range(date!(2024 - 01 - 25), bounds)
            .expect("date is not disabled");
        assert_eq!(available.start(), date!(2024 - 01 - 21));
        assert_eq!(available.end(), date!(2024 - 01 - 31));

        // A disabled date itself has no available range.
        assert_eq!(ranges.available_range(date!(2024 - 01 - 15), bounds), None);
    }

    #[test]
    fn test_is_between_is_start_is_end() {
        let range = Some(DateRange::new(date!(2024 - 01 - 10), date!(2024 - 01 - 20)));

        assert!(is_start(date!(2024 - 01 - 10), range));
        assert!(!is_start(date!(2024 - 01 - 20), range));
        assert!(!is_start(date!(2024 - 01 - 15), range));

        assert!(is_end(date!(2024 - 01 - 20), range));
        assert!(!is_end(date!(2024 - 01 - 10), range));
        assert!(!is_end(date!(2024 - 01 - 15), range));

        assert!(is_between(date!(2024 - 01 - 15), range));
        assert!(
            !is_between(date!(2024 - 01 - 10), range),
            "endpoints are not 'between'"
        );
        assert!(
            !is_between(date!(2024 - 01 - 20), range),
            "endpoints are not 'between'"
        );

        // A single-day range (start == end) is neither a start nor an end per
        // the `date != r.end` / `date != r.start` guards.
        let single_day = Some(DateRange::new(date!(2024 - 01 - 10), date!(2024 - 01 - 10)));
        assert!(!is_start(date!(2024 - 01 - 10), single_day));
        assert!(!is_end(date!(2024 - 01 - 10), single_day));

        assert!(!is_start(date!(2024 - 01 - 10), None));
        assert!(!is_end(date!(2024 - 01 - 10), None));
        assert!(!is_between(date!(2024 - 01 - 10), None));
    }

    #[test]
    fn test_aria_label() {
        let d = date!(2024 - 03 - 04); // A Monday
        assert_eq!(aria_label(&d), "Monday, March 4, 2024");
    }

    #[test]
    fn test_relative_calendar_month() {
        with_runtime(|| {
            let enabled_range = DateRange::new(date!(2024 - 01 - 01), date!(2024 - 12 - 31));
            let base_ctx = make_base_ctx_for_relative_month(enabled_range);
            let current_month = Month::June;

            // Outside the enabled range entirely.
            assert_eq!(
                relative_calendar_month(date!(2023 - 12 - 15), &base_ctx, current_month),
                RelativeMonth::Last
            );
            assert_eq!(
                relative_calendar_month(date!(2025 - 01 - 15), &base_ctx, current_month),
                RelativeMonth::Next
            );

            // Inside the enabled range: compared against `current_month`.
            assert_eq!(
                relative_calendar_month(date!(2024 - 05 - 15), &base_ctx, current_month),
                RelativeMonth::Last
            );
            assert_eq!(
                relative_calendar_month(date!(2024 - 06 - 15), &base_ctx, current_month),
                RelativeMonth::Current
            );
            assert_eq!(
                relative_calendar_month(date!(2024 - 07 - 15), &base_ctx, current_month),
                RelativeMonth::Next
            );
        });
    }

    #[test]
    fn test_relative_month_current_month() {
        assert!(RelativeMonth::Current.current_month());
        assert!(!RelativeMonth::Last.current_month());
        assert!(!RelativeMonth::Next.current_month());
    }

    #[test]
    fn test_relative_month_display() {
        assert_eq!(RelativeMonth::Last.to_string(), "last");
        assert_eq!(RelativeMonth::Current.to_string(), "current");
        assert_eq!(RelativeMonth::Next.to_string(), "next");
    }

    // -----------------------------------------------------------------
    // Context accessors (docs/mutants-baseline.md fix #8)
    // -----------------------------------------------------------------

    /// Run a closure inside a Dioxus runtime context so that Signal/Memo/Callback
    /// APIs are available (mirrors `virtual/virtualizer.rs`'s `with_runtime`).
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

    /// Build a minimal but real `BaseCalendarContext` just to exercise
    /// `relative_calendar_month`'s `enabled_date_range` read; must run inside `with_runtime`.
    fn make_base_ctx_for_relative_month(enabled_date_range: DateRange) -> BaseCalendarContext {
        BaseCalendarContext {
            focused_date: Signal::new(None),
            view_date: use_signal(|| date!(2024 - 06 - 01)).into(),
            available_ranges: use_memo(|| AvailableRanges::new(&[])),
            set_view_date: Callback::new(|_: Date| {}),
            format_weekday: Callback::new(|w: Weekday| format!("{w:?}")),
            format_month: Callback::new(|m: Month| m.to_string()),
            disabled: use_signal(|| false).into(),
            today: date!(2024 - 06 - 15),
            first_day_of_week: Weekday::Monday,
            enabled_date_range,
            view_registrations: use_signal(Vec::new),
            direction: Direction::Ltr,
        }
    }

    /// Build a real, fully wired `BaseCalendarContext` (not a stand-in) for exercising the
    /// signal-wiring accessor methods; must run inside `with_runtime`.
    fn make_base_ctx(enabled_date_range: DateRange) -> BaseCalendarContext {
        let view_date_sig = use_signal(|| enabled_date_range.start());
        BaseCalendarContext {
            focused_date: Signal::new(None),
            view_date: view_date_sig.into(),
            available_ranges: use_memo(|| AvailableRanges::new(&[])),
            set_view_date: Callback::new(move |d: Date| {
                let mut view_date_sig = view_date_sig;
                view_date_sig.set(d);
            }),
            format_weekday: Callback::new(|w: Weekday| format!("{w:?}")),
            format_month: Callback::new(|m: Month| m.to_string()),
            disabled: use_signal(|| false).into(),
            today: enabled_date_range.start(),
            first_day_of_week: Weekday::Monday,
            enabled_date_range,
            view_registrations: use_signal(Vec::new),
            direction: Direction::Ltr,
        }
    }

    #[test]
    fn test_base_calendar_context_focused_date() {
        with_runtime(|| {
            let mut ctx =
                make_base_ctx(DateRange::new(date!(2024 - 01 - 01), date!(2024 - 12 - 31)));

            assert_eq!(ctx.focused_date(), None);
            ctx.set_focused_date(Some(date!(2024 - 06 - 15)));
            assert_eq!(ctx.focused_date(), Some(date!(2024 - 06 - 15)));

            assert!(ctx.is_focused(date!(2024 - 06 - 15)));
            assert!(!ctx.is_focused(date!(2024 - 06 - 16)));
        });
    }

    #[test]
    fn test_base_calendar_context_set_view_date_clamps_to_enabled_range() {
        with_runtime(|| {
            let ctx = make_base_ctx(DateRange::new(date!(2024 - 01 - 01), date!(2024 - 12 - 31)));

            ctx.set_view_date(date!(2024 - 06 - 15));
            assert_eq!(ctx.view_date(), date!(2024 - 06 - 15));

            // Out-of-range requests are clamped into the enabled range.
            ctx.set_view_date(date!(2025 - 03 - 01));
            assert_eq!(ctx.view_date(), date!(2024 - 12 - 31));

            ctx.set_view_date(date!(2020 - 01 - 01));
            assert_eq!(ctx.view_date(), date!(2024 - 01 - 01));
        });
    }

    #[test]
    fn test_base_calendar_context_is_disabled_and_is_unavailable() {
        with_runtime(|| {
            let mut disabled_sig = use_signal(|| false);
            let mut ctx =
                make_base_ctx(DateRange::new(date!(2024 - 01 - 01), date!(2024 - 12 - 31)));
            ctx.disabled = disabled_sig.into();
            assert!(!ctx.is_disabled(), "false signal must read back as false");
            disabled_sig.set(true);
            assert!(ctx.is_disabled(), "true signal must read back as true");

            let disabled_ranges = [DateRange::new(date!(2024 - 06 - 10), date!(2024 - 06 - 20))];
            ctx.available_ranges = use_memo(move || AvailableRanges::new(&disabled_ranges));
            assert!(ctx.is_unavailable(date!(2024 - 06 - 15)));
            assert!(!ctx.is_unavailable(date!(2024 - 06 - 01)));
        });
    }

    #[test]
    fn test_base_calendar_context_available_range_with_and_without_range_calendar() {
        with_runtime(|| {
            let ctx = make_base_ctx(DateRange::new(date!(2024 - 01 - 01), date!(2024 - 12 - 31)));
            // No `RangeCalendarContext` has been provided in this scope.
            assert_eq!(ctx.available_range(), None);

            // With a `RangeCalendarContext` whose anchor date is set, the base
            // context's `available_range` looks up the available span around it.
            let anchor_date = Signal::new(Some(date!(2024 - 06 - 15)));
            use_context_provider(|| RangeCalendarContext {
                anchor_date,
                highlighted_range: Signal::new(None),
                set_selected_range: Callback::new(|_: Option<DateRange>| {}),
            });
            assert_eq!(
                ctx.available_range(),
                Some(DateRange::new(date!(2024 - 01 - 01), date!(2024 - 12 - 31))),
                "with no disabled ranges, the full enabled range is available"
            );
        });
    }

    #[test]
    fn test_base_calendar_context_visible_month_count_and_registrations() {
        with_runtime(|| {
            let ctx = make_base_ctx(DateRange::new(date!(2024 - 01 - 01), date!(2024 - 12 - 31)));
            let a = ScopeId(101);
            let b = ScopeId(102);

            assert_eq!(ctx.visible_month_count(), 1, "no views registered yet");
            assert_eq!(ctx.view_registrations.read().len(), 0);

            ctx.register_calendar_view(a, None);
            assert_eq!(
                ctx.calendar_view_offset(a, None),
                0,
                "first registration order"
            );
            assert_eq!(ctx.visible_month_count(), 1);
            assert_eq!(ctx.view_registrations.read().len(), 1);

            // Re-registering the exact same (id, offset) must be a no-op, not a duplicate push.
            ctx.register_calendar_view(a, None);
            assert_eq!(
                ctx.view_registrations.read().len(),
                1,
                "re-registering an identical (id, offset) must not add a duplicate entry"
            );

            ctx.register_calendar_view(b, Some(3));
            assert_eq!(ctx.calendar_view_offset(b, Some(3)), 3);
            assert_eq!(
                ctx.visible_month_count(),
                4,
                "highest offset (3) + 1 determines the visible month count"
            );

            ctx.unregister_calendar_view(b);
            assert_eq!(
                ctx.visible_month_count(),
                1,
                "removing the highest-offset view shrinks the count back down"
            );
            assert_eq!(ctx.view_registrations.read().len(), 1);

            // Unregistering the *only* remaining view must still remove it (regression
            // guard for the "nothing to remove" short-circuit misfiring when every
            // registered view happens to share the target id).
            ctx.unregister_calendar_view(a);
            assert!(
                ctx.view_registrations.read().is_empty(),
                "unregistering the last remaining view must empty the registrations"
            );

            // Unregistering an id that was never registered is a safe no-op.
            ctx.unregister_calendar_view(a);
            assert!(ctx.view_registrations.read().is_empty());
        });
    }

    #[test]
    fn test_calendar_context_accessors() {
        with_runtime(|| {
            let mut selected = use_signal(|| None::<Date>);
            let last_set = use_signal(|| None::<Option<Date>>);
            let ctx = CalendarContext {
                selected_date: selected.into(),
                set_selected_date: Callback::new(move |d| {
                    let mut last_set = last_set;
                    last_set.set(Some(d));
                }),
            };

            assert_eq!(ctx.selected_date(), None);
            selected.set(Some(date!(2024 - 06 - 15)));
            assert_eq!(
                ctx.selected_date(),
                Some(date!(2024 - 06 - 15)),
                "selected_date must reflect the underlying signal, not a fixed value"
            );

            ctx.set_selected_date(Some(date!(2024 - 07 - 01)));
            assert_eq!(last_set(), Some(Some(date!(2024 - 07 - 01))));
        });
    }

    fn make_range_ctx() -> (RangeCalendarContext, Signal<Option<DateRange>>) {
        let anchor_date = Signal::new(None);
        let highlighted_range = Signal::new(None);
        let emitted = Signal::new(None::<DateRange>);
        let ctx = RangeCalendarContext {
            anchor_date,
            highlighted_range,
            set_selected_range: Callback::new(move |r: Option<DateRange>| {
                let mut emitted = emitted;
                emitted.set(r);
            }),
        };
        (ctx, highlighted_range)
    }

    #[test]
    fn test_range_calendar_context_set_selected_date_two_click_flow() {
        with_runtime(|| {
            let (mut ctx, highlighted_range) = make_range_ctx();

            // First click sets the anchor and a same-day highlighted range.
            ctx.set_selected_date(Some(date!(2024 - 06 - 10)));
            assert_eq!(
                highlighted_range(),
                Some(DateRange::new(date!(2024 - 06 - 10), date!(2024 - 06 - 10)))
            );

            // Second click completes the range from the anchor.
            ctx.set_selected_date(Some(date!(2024 - 06 - 20)));
            assert_eq!(
                highlighted_range(),
                Some(DateRange::new(date!(2024 - 06 - 10), date!(2024 - 06 - 20)))
            );
        });
    }

    #[test]
    fn test_range_calendar_context_set_hovered_date() {
        with_runtime(|| {
            let (mut ctx, highlighted_range) = make_range_ctx();

            // No anchor yet: hovering does nothing.
            ctx.set_hovered_date(date!(2024 - 06 - 15));
            assert_eq!(highlighted_range(), None);

            ctx.set_selected_date(Some(date!(2024 - 06 - 10)));
            ctx.set_hovered_date(date!(2024 - 06 - 25));
            assert_eq!(
                highlighted_range(),
                Some(DateRange::new(date!(2024 - 06 - 10), date!(2024 - 06 - 25)))
            );
        });
    }

    #[test]
    fn test_range_calendar_context_reset_selection() {
        with_runtime(|| {
            let (mut ctx, highlighted_range) = make_range_ctx();

            ctx.set_selected_date(Some(date!(2024 - 06 - 10)));
            let reset_to = Some(DateRange::new(date!(2024 - 01 - 01), date!(2024 - 01 - 02)));
            ctx.reset_selection(reset_to);

            assert_eq!(highlighted_range(), reset_to);
            // The anchor was cleared, so a subsequent click starts a new anchor
            // rather than completing a range against the old one.
            ctx.set_selected_date(Some(date!(2024 - 03 - 01)));
            assert_eq!(
                highlighted_range(),
                Some(DateRange::new(date!(2024 - 03 - 01), date!(2024 - 03 - 01)))
            );
        });
    }

    #[test]
    fn test_calendar_view_context_offset_view_date_round_trip() {
        with_runtime(|| {
            let base_ctx =
                make_base_ctx(DateRange::new(date!(2024 - 01 - 01), date!(2024 - 12 - 31)));
            use_context_provider(|| base_ctx);
            base_ctx.set_view_date(date!(2024 - 06 - 01));

            let view_ctx = CalendarViewContext { offset: 2 };
            assert_eq!(
                view_ctx.offset_view_date(),
                date!(2024 - 08 - 01),
                "offset view date is n months after the base view date"
            );

            // Setting a date on the offset view adjusts the base view date back by the offset.
            view_ctx.set_offset_view_date(date!(2024 - 09 - 15));
            assert_eq!(base_ctx.view_date(), date!(2024 - 07 - 15));
        });
    }
}

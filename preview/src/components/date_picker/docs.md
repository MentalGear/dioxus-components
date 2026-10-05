The DatePicker component is used to display a date input and a Calendar popover, allowing users to enter or select a date value.

## Component Structure

```rust
DatePicker {
    // The currently selected date in the date picker (if any).
    selected_date,
    on_value_change: move |v: Option<Date>| {
        // This callback is triggered when a date is selected in the
        // calendar or the user entered it from the keyboard.
        // The date parameter contains the selected date.
    },
    // Optional number of pre-composed calendar months to show in the popover.
    month_count: 1,
    // Optional: keep the popover open after a date is picked (default `true` closes it).
    close_on_select: true,
    // Optional: how long the popover stays open after a selection (default 300 ms).
    close_delay: Duration::from_millis(300),
    // Optional placeholder formatters for the input fields.
    on_format_day_placeholder: || "D",
    on_format_month_placeholder: || "M",
    on_format_year_placeholder: || "Y",
}
```

The styled `DatePicker` and `DateRangePicker` render the input, trigger, popover content, and calendar by default.

## Closing after a selection

Picking a date closes the popover, but not on the same frame: the selected day stays painted for
`close_delay` first, so the click visibly registered, and then the popover plays its normal exit
animation and focus returns to the trigger.

| Prop | Default | Effect |
| --- | --- | --- |
| `close_on_select: bool` | `true` | `false` keeps the popover open after a selection. Escape, an outside click and the trigger still close it. |
| `close_delay: Duration` | `300 ms` | How long to hold the popover open after a selection. `Duration::ZERO` closes immediately. Ignored when `close_on_select` is `false`. |

The default is 300 ms: long enough for the selected day's fill transition (100-200 ms) to finish
and be noticed, short enough that the popover does not look stuck.

- Keyboard selection (`Enter` / `Space` on a day) behaves exactly like a click.
- A pending close is cancelled when the user presses a key inside the picker, navigates to another
  month, or opens or closes the popover again before it elapses (a range picker also cancels on any
  pointer press, so starting a new range is never cut short). Selecting another date restarts the
  wait, and clicking the day that was just selected again (a double-click) keeps the selection
  instead of clearing it. The timer belongs to the picker, so unmounting it cancels the close.
- `DateRangePicker` closes only once the range is complete (its end date is picked), with the same
  dwell; the first click of a range never closes it.

## Multiple months

`month_count: 2` renders two months side by side inside one popover panel, which grows to fit them
(the previous-month button sits on the first month, the next-month button on the last). On a narrow
screen the months stack vertically and the panel stays inside the viewport.

## Unavailable dates

`disabled_ranges` marks ranges of dates as unavailable. They are muted and struck through, cannot be
selected, and (in a `DateRangePicker`) cannot be spanned by a range. The `unavailable_dates` demo
computes its ranges from today, so they are always inside the month the picker opens on.

## Popover overlay

`DatePickerPopover` (what both pickers render their calendar in) takes `overlay: bool`, default
`false`, forwarded to the popover primitive: it only matters for a **modal** popover
(`is_modal: true`), where `overlay: true` dims the page behind it with the shared scrim and
`false` leaves it transparent. The styled pickers are non-modal, so it is a no-op for them.

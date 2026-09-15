The Command component is a filterable command palette: a search input paired with an
always-visible, filtered list of actions. It is typically opened inside a `CommandDialog`, but
`Command` itself has no dialog logic and can be composed anywhere a filterable action list is
useful.

Unlike `Combobox`, `Command`'s list is never a popover -- it has no open/closed state of its own.
The surrounding `CommandDialog` (or whatever the caller wraps it in) owns visibility.

## Component Structure

```rust
let mut open = use_signal(|| false);
let mut query = use_signal(String::new);

CommandDialog {
    open: open(),
    on_open_change: move |v| open.set(v),
    Command::<String> {
        query: Some(query()),
        on_query_change: move |next| query.set(next),
        on_value_change: move |value: Option<String>| {
            // run the chosen command, then close
            open.set(false);
        },
        input_aria_label: "Search commands",
        list_aria_label: "Commands",
        CommandEmpty { "No results found." }
        CommandGroup {
            CommandGroupLabel { "File" }
            CommandItem::<String> {
                index: 0usize,
                value: "new-file".to_string(),
                text_value: "New File",
                shortcut: rsx! {
                    KbdGroup { Kbd { "⌘" } Kbd { "N" } }
                },
                "New File"
            }
        }
    }
}
```

`CommandItem`'s `shortcut` slot is presentation-only -- it renders whatever element you pass (a
`Kbd`/`KbdGroup` pair, above) but does not bind a keyboard shortcut for you. Wire up a global
keyboard listener yourself if you want the displayed shortcut to actually fire the action.

The Toast component is used to display brief messages to the user, typically for notifications or alerts. The toast messages can be focused with the keyboard with the `f6` key.

## Component Structure

```rust
// The Toast provider provides the toast context to its children and handler rendering any toasts that are sent.
ToastProvider {
    // Any child component can consume the toast context and send a toast to be rendered.
    button {
        onclick: |event: MouseEvent| {
            // Consume the toast context to send a toast.
            let toast_api = consume_toast();
            toast_api
                .error(
                    "Critical Error".to_string(),
                    ToastOptions::new()
                        .description("Some info you need")
                        .duration(Duration::from_secs(60))
                        .permanent(false),
                );
        },
        "Show Toast"
    }
}
```

## Motion and visibility

A toast's auto-dismiss is a timer the user is waiting on, so it follows the tab and nothing else. While the tab is hidden (another tab, a minimized window) the countdown is dropped, and when you come back it starts over with the full duration, so a toast you could not see is still there. Scrolling the page never changes how long a toast stays. The visibility-gated timers for decoration (spinners, tickers, autoplay) are the opposite: they stop when merely scrolled away. See `dioxus_primitives::activity::use_document_visible`.

The slide-in and restack transitions use `--dx-motion-duration-slow`; `duration` and `permanent` on the toast control how long it stays. See [Motion](/docs#motion) for the tokens, reduced motion and the per-component table.

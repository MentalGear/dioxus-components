The Attachment component displays a file or image attachment: its media, name and metadata, with optional actions and an upload state. Use it for files and images in chat composers, message threads and upload lists.

It is styled composition (there is no ARIA widget pattern, so no primitive underneath) and composes the existing `Button` for its actions. `AttachmentGroup` and the shimmering title use the `dx-scroll-fade-x` and `dx-shimmer` effects from `assets/dx-effects.css`, which the component's `component.json` ships as a global asset next to the theme.

## Component Structure

```rust
AttachmentGroup {
    Attachment {
        // Available states: Idle, Uploading, Processing, Error, Done
        state: AttachmentState::Uploading,
        // Available sizes: Default, Sm, Xs
        size: AttachmentSize::Default,
        // Available orientations: Horizontal, Vertical
        orientation: AttachmentOrientation::Horizontal,

        AttachmentMedia {
            // Media variants: Icon, Image
            variant: AttachmentMediaVariant::Icon,
            FileText {}
        }
        AttachmentContent {
            AttachmentTitle { "sales-dashboard.pdf" }
            AttachmentDescription { "PDF · 2.4 MB" }
        }
        AttachmentActions {
            AttachmentAction { aria_label: "Remove sales-dashboard.pdf",
                X {}
            }
        }
        // A full-card overlay behind the actions: opens a link or dialog
        AttachmentTrigger { aria_label: "Preview sales-dashboard.pdf" }
    }
}
```

`Uploading` and `Processing` shimmer the title, `Error` switches to a destructive treatment, `Idle` draws a dashed border. `AttachmentAction` is a themed `Button` defaulting to the ghost variant and the `IconXs` size.

## Accessibility

- `AttachmentAction` is usually icon-only: give each one an `aria-label` naming the action and its target ("Remove sales-dashboard.pdf").
- `AttachmentTrigger` covers the card and has no text of its own: give it an `aria-label` for what activating it does. It sits behind the actions in the stacking order, so the trigger and the actions are separately focusable and clickable. Pass `as` to render a link instead of a button.
- An `AttachmentGroup` scrolls horizontally. When its attachments are interactive, keyboard users reach the off-screen ones by tabbing. For a row of presentational attachments, make the group itself focusable with `tabindex: "0"`, `role: "group"` and an `aria-label`.
- The `Error` state uses a destructive colour. Keep the failure reason in `AttachmentDescription`, so the state is not conveyed by colour alone.

## Motion

While an attachment is `Uploading` or `Processing` its title shimmers with the `dx-shimmer` effect from `dx-effects.css` (tune with `--dx-shimmer-duration`, `--dx-shimmer-color`, `--dx-shimmer-spread` and `--dx-shimmer-angle`; `prefers-reduced-motion` turns it off). The sweep is paused whenever the title is off-screen or the tab is hidden, and an attachment that is not busy subscribes to nothing. See [Motion](/docs#motion) for the tokens, reduced motion and the per-component table.

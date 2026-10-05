The Bubble component displays framed conversational content: chat text, short structured output, quoted replies, suggestions and reactions. It sizes to its content, up to 80% of the row; the `Ghost` variant spans the full row so assistant text can stay unframed.

`Bubble` is only the message surface. Put avatars, names, timestamps and message-level actions in `Message`.

## Component Structure

```rust
BubbleGroup {
    Bubble {
        // Available variants: Default, Secondary, Muted, Tinted, Outline, Ghost, Destructive
        variant: BubbleVariant::Secondary,

        // Available alignments: Start, End
        align: BubbleAlign::Start,

        BubbleContent { "I checked the registry output and removed the stale route." }

        BubbleReactions {
            // Available sides: Top, Bottom. Available alignments: Start, End
            side: BubbleSide::Bottom,
            align: BubbleAlign::End,
            role: "img",
            aria_label: "Reaction: thumbs up",
            span { "👍" }
        }
    }
}
```

Group consecutive bubbles from the same sender with `BubbleGroup`. Set `align` on each `Bubble`, not on the group; inside a `Message` the row's own alignment takes care of it.

To turn a bubble into a link or button, pass `as` to `BubbleContent` and render the element yourself:

```rust
BubbleContent {
    r#as: move |attrs: Vec<Attribute>| rsx! {
        button { r#type: "button", onclick: move |_| reply(), ..attrs, "Click here" }
    },
}
```

## Accessibility

- A bubble is presentational; keep the conversation's semantics on the surrounding container.
- A static row of reactions needs `role: "img"` and an `aria-label` naming them (otherwise each emoji is read with no context and `+8` is read as "plus eight"). Interactive reactions are real buttons, each with an `aria-label` when icon-only.
- A clickable bubble must be a real `button` or `a` (through `as`), which also gives it a visible focus ring. Its accessible name is the bubble text.
- Variants signal tone with colour. Keep the meaning in the text too, especially for `Destructive`: put the error context in the message, not only in the colour.

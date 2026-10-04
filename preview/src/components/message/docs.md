The Message component lays out a single message in a conversation: it owns the row layout, which means the avatar, the alignment, the header and the footer. Render the visible message surface inside it with `Bubble`.

For AI apps it also hosts reasoning steps, tool calls and assistant messages: a `Marker` with `role: "status"` inside a `Message` announces the in-progress state.

## Component Structure

```rust
MessageGroup {
    Message {
        // Available alignments: Start, End
        align: MessageAlign::Start,

        MessageAvatar {
            Avatar { AvatarFallback { "CN" } }
        }
        MessageContent {
            MessageHeader { "Shadcn" }
            Bubble { variant: BubbleVariant::Muted,
                BubbleContent { "How can I help you today?" }
            }
            MessageFooter { "Delivered" }
        }
    }
}
```

- `align: MessageAlign::End` mirrors the row for the sender's own messages; the footer (and its actions) stay at the end.
- `MessageAvatar` anchors to the bottom of the row and shifts up to stay level with the bubble when there is a footer. Leave it empty on the earlier messages of a `MessageGroup` to keep them aligned with the avatar on the last one.
- `MessageHeader` and `MessageFooter` lose their inset next to an unframed (`Ghost`) bubble.

## Accessibility

`Message` is a presentational layout wrapper; accessibility comes from the content you place in it.

- Give every icon-only footer action (copy, retry, feedback) an `aria-label`.
- For an in-progress message, use a `Marker` with `role: "status"` so the update is announced as it appears.

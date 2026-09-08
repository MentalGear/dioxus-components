The Empty component is a placeholder shown in place of content that hasn't loaded, doesn't exist yet, or was filtered away to nothing.

## Component Structure

```rust
Empty {
    EmptyHeader {
        EmptyMedia { variant: EmptyMediaVariant::Icon,
            Inbox {}
        }
        EmptyTitle { "No messages yet" }
        EmptyDescription { "Start a conversation to see it here." }
    }
    EmptyContent {
        Button { "New message" }
    }
}
```

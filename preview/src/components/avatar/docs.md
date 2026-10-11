The Avatar components display a user's profile picture or fallback initials. Use the composable `Avatar`, `AvatarImage`, and `AvatarFallback` primitives when you need full control, or `ImageAvatar` for the common image-with-fallback case.

## Component Structure

```rust
Avatar {
    aria_label: "Jane Doe",
    AvatarImage {
        src: "https://example.com/avatar.png",
        alt: "Jane Doe",
    }
    AvatarFallback { "JD" }
}
```

```rust
ImageAvatar {
    src: "https://example.com/avatar.png",
    alt: "Jane Doe",
    on_state_change: |state: AvatarState| { /* image is loading/loaded/failed */ },
    "JD"
}
```

## Motion

While the image is loading the placeholder pulses (`opacity`, 1.5 s, forever). The pulse is paused whenever the avatar is off-screen or the tab is hidden (`dioxus_primitives::activity::use_motion_when`), and an avatar that is not loading subscribes to nothing. Override `animation` on `.dx-avatar[data-state="loading"]` to change or remove it. See [Motion](/docs#motion) for the tokens, reduced motion and the per-component table.

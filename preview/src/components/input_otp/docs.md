The `InputOtp` component is a one-time-passcode entry control: a row of single-character boxes backed by one real, focusable `<input>`. Typing, Backspace/Delete, the arrow keys, and pasting a full code all work through that one real input; `InputOtpSlot` boxes are a purely visual, `aria-hidden` overlay that mirrors its value and caret position.

## Component Structure

```rust
InputOtp {
    max_length: 6,
    aria_label: "One-time passcode",
    InputOtpGroup {
        InputOtpSlot { index: 0 }
        InputOtpSlot { index: 1 }
        InputOtpSlot { index: 2 }
    }
    InputOtpSeparator {}
    InputOtpGroup {
        InputOtpSlot { index: 3 }
        InputOtpSlot { index: 4 }
        InputOtpSlot { index: 5 }
    }
}
```

## Motion

The caret blinks (`opacity`, 1 s, forever) on the one focused slot only, i.e. while the user is typing into the field; it is visible by definition, so it is not gated (it is on `check-motion-gating.sh`'s allowlist). Override `animation` on `.dx-input-otp-slot[data-active="true"]::after` to change or remove it; reduced motion swaps in `--dx-motion-duration-reduced`. See [Motion](/docs#motion) for the tokens, reduced motion and the per-component table.

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

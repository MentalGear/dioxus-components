The sheet component is a panel that slides in from the edge of the screen. It can be used to display additional content, forms, or navigation menus without leaving the current page.

## Component Structure

```rust
Sheet {
    open: open(),
    // Which edge to slide in from. Available sides: Top, Right (default), Bottom, Left.
    "data-side": SheetSide::Right.as_str(),
    SheetContentClose {}
    SheetHeader {
        SheetTitle { "Edit profile" }
        SheetDescription { "Make changes to your profile here. Click save when you're done." }
    }
    SheetFooter {
        SheetClose { "Close" }
    }
}
```

## SheetClose with `as` prop

The `as` prop allows you to render a custom element while preserving the close behavior, similar to shadcn/ui's `asChild` pattern.

```rust
// Default: renders as <button>
SheetClose { "Close" }

// Custom element: attributes include the preset onclick handler
SheetClose {
    as: |attributes| rsx! {
        a { href: "#", ..attributes, "Go back" }
    }
}
```

## Alignment with shadcn/ui v4

This component's `style.css` was checked line-by-line against shadcn/ui v4's
`Sheet` (`SheetOverlay`/`SheetContent`/`SheetHeader`/`SheetFooter`/
`SheetTitle`/`SheetDescription`/`SheetPrimitive.Close`) and brought in line
with it, translated onto this repo's own design tokens rather than copying
Tailwind classes verbatim:

- **Overlay**: `fixed inset-0 bg-black/50`. On the web build, `.dx-sheet` is
  a real `<dialog>` (`primitives/src/dialog.rs`'s modal web arm), so the
  visible tint moved to its native `::backdrop` pseudo-element there; the
  non-web (Blitz) arm has no such element, so `.dx-sheet-root` keeps the
  tint for that arm. Both are driven by the same `bg-black/50` value and the
  same fade keyframes.
- **Content**: `fixed z-50 flex flex-col gap-4 bg-background shadow-lg`,
  sliding in from its `data-side` with `transition ease-in-out`, a 500ms
  open / 300ms close duration (this repo's motion scale has no exact 500ms
  step, so that one value is a literal; 300ms matches
  `--dx-motion-duration-slower` exactly). `right`/`left` are
  `inset-y-0 h-full w-3/4 border-l|border-r`, capped at `sm:max-w-sm`
  (384px) only from a 640px viewport up, matching shadcn's own
  breakpoint-gated cap rather than applying it unconditionally.
  `top`/`bottom` are `inset-x-0 h-auto border-b|border-t`.
- **Header/Footer/Title/Description**: `flex flex-col gap-1.5 p-4` /
  `mt-auto flex flex-col gap-2 p-4` / `text-foreground font-semibold` /
  `text-muted-foreground text-sm`. Title has no explicit text-size class in
  shadcn's own source (unlike `DialogTitle`'s `text-lg`); since this repo
  has no global heading-reset the way Tailwind's preflight does, that
  translates to an explicit `--dx-text-base` here rather than an omitted
  font-size, which would otherwise fall back to the browser's own, much
  larger, default `<h2>` size.
- **Close button**: `absolute top-4 right-4 rounded-xs opacity-70
  hover:opacity-100`, a `--dx-ring` focus-visible ring, and a `size-4`
  (16px) icon with an `aria-label` (this repo's equivalent of shadcn's
  visually-hidden "Close" span). The button's own hit target stays a fixed
  24x24px box around that icon, larger than shadcn's own (icon-sized, no
  padding) -- a deliberate, pre-existing choice in this codebase favoring
  WCAG 2.5.8's 24px target-size guidance over byte-for-byte fidelity here.
- **Native `<dialog>` centering defect (shared with Drawer)**: Chromium's
  `dialog:modal` UA stylesheet ships `inset: 0; margin: auto; width/height:
  fit-content`. Overriding only the one edge a given side needs (e.g.
  `right: 0`) used to leave the UA's own `inset: 0` still supplying the
  *opposite* edge uncontested, over-constraining the box against a definite
  `width`/`height` -- the UA resolved that by centering the panel into its
  own auto margins rather than anchoring it (confirmed live: a right/left
  sheet rendered as a floating centered card, and a top/bottom sheet
  stretched to the full viewport height instead of sizing to its content).
  Fixed by resetting `margin`/`inset`/`width`/`height` to a known, non-auto
  baseline on `.dx-sheet` itself and letting each `[data-side]` rule reopen
  (set back to `auto`) exactly the one edge it doesn't pin.

See `dev-docs/backlog.md` for the session this alignment pass landed in.

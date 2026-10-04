The message scroller is the scroll container for a chat transcript. It owns *scroll* and nothing else: where the thread opens, whether it follows a reply that is still arriving, where a new turn lands, keeping your place while older messages load above you, and a button to get back to the latest message. It does not own messages, models, transport or persistence; the rows are whatever you put inside a `MessageScrollerItem` (here, `Message`, `Bubble` and `Marker`).

It is a port of shadcn/ui's `message-scroller`. The one rule behind every behavior: **never move the reader against their intent.**

## Component Structure

```rust
MessageScrollerProvider { auto_scroll: true,
    MessageScroller {
        MessageScrollerViewport {
            MessageScrollerContent {
                for message in messages() {
                    MessageScrollerItem {
                        key: "{message.id}",
                        message_id: "{message.id}",
                        scroll_anchor: message.is_user,
                        Message { /* ... */ }
                    }
                }
            }
        }
        MessageScrollerButton {}
    }
}
```

`MessageScroller` fills its parent, so place it in a container with a fixed or constrained height.

- `MessageScrollerProvider` is the headless root. It owns the scroll controller and the behavior props, and renders no element.
- `MessageScroller` is the frame. It lays out the viewport and the jump button.
- `MessageScrollerViewport` is the scrolling element. It is `role="region"` with `aria-label="Messages"` and `tabindex="0"`, so keyboard users can scroll it.
- `MessageScrollerContent` holds the rows. It is `role="log"` with `aria-relevant="additions"`: a new row is announced, token-by-token text growth inside a row is not.
- `MessageScrollerItem` wraps every direct child of the content. A row can be a message, a marker, a typing indicator, a separator, or a "load earlier" row.
- `MessageScrollerButton` scrolls to the end (or, with `direction: MessageScrollerDirection::Start`, the start).

## Following the live edge

With `auto_scroll`, the transcript follows new content while the reader is at the live edge. Scrolling away releases it, whether by the wheel, touch, the scroll keys (arrows, Page Up/Down, Home/End, Space) or by dragging the scrollbar, and so does an explicit jump to a message. New chunks then arrive without moving the reader. The jump button, or `scroll_to_end`, re-engages following.

Following is off by default: nothing moves unless you ask it to.

## Anchoring a turn

Mark the row that starts a turn with `scroll_anchor` (usually the user's message). When a new anchor is appended, the viewport scrolls it near the top, keeping `scroll_previous_item_peek` pixels of the previous row visible above it, and the reply streams into the room below. Once the reply fills the viewport the reader is back at the live edge and `auto_scroll` takes over. Anchoring is not tied to a role: any row can be an anchor.

## Opening a saved thread

`default_scroll_position` picks where the transcript opens on its first non-empty render: `End` (the default), `Start`, or `LastAnchor`: the user's latest message with its reply below it, falling back to the end when that turn already fits.

### Avoiding a flash on load

A scroll container always opens at the top, so a server-rendered transcript would show its oldest messages and then jump. When the position is `End` or `LastAnchor`, the root and viewport render `data-pending-scroll` (identically on the server and on the hydrating client) and the styled viewport stays hidden until the position is applied. The attribute is removed for an empty transcript too, and a few seconds after load as a failsafe, so a transcript can never stay hidden. It is applied once and never hides the viewport again.

## Loading earlier messages

Older messages prepended above the current ones do not move what you are reading: the viewport keeps the first visible row where it is (`preserve_scroll_on_prepend`, on by default). Give rows a stable `message_id`, so the scroller has a specific row to hold.

## Virtualization

A long thread should not lay out and paint thousands of rows that nobody can see. `virtualize` on the provider picks how the scroller keeps off-screen rows cheap:

```rust
MessageScrollerProvider { virtualize: Virtualization::ContentVisibility, /* ... */ }
```

- `Virtualization::None`: every row is fully rendered all the time. Use it for short threads, or when a row's content cannot tolerate being skipped.
- `Virtualization::ContentVisibility` (the default): rows stay in the DOM and the browser skips layout and paint for the ones that are off-screen (`content-visibility: auto`, with `contain-intrinsic-size: auto` so it remembers each row's last rendered height). Because the rows are real DOM, **the accessibility tree and text selection keep reaching every message**, including the ones scrolled far out of view, and so does the browser's find-in-page where the engine supports it (see below). The scroller never skips three groups of rows: the rows at the live edge (so the end geometry is exact while a reply streams), the row that holds focus, and the rows that hold the ends of the current selection. It marks them with `data-keep-rendered`, which the stylesheet excludes from skipping. Tune the first-render height guess with the `--dx-message-scroller-row-estimate` custom property (default `10rem`).

**Find-in-page support.** Skipped content stays findable in Chromium and in Firefox 125 and later, and in Safari from Safari 26. In Safari 18 through 25 the browser's find-in-page does **not** find text inside a skipped (off-screen) row (WebKit bug 283846): there the text is still in the DOM, the accessibility tree and any selection, but Cmd+F will not match it until the row has been scrolled into view. If your users depend on find-in-page across a long thread on those versions, use `Virtualization::None`, or give them a search of your own built on `scroll_to_message`.

Mounting only a window of rows is a possible future option, but it is not part of this API and not planned for now: unmounted rows are not in the DOM, so native find-in-page, screen-reader history and a selection that spans messages could not reach them, and an app would have to provide its own search. `ContentVisibility` keeps the DOM, the accessibility tree and selection whole (and find-in-page where supported, as above), and is the right size for hundreds to low thousands of turns.

The `long` example below keeps 2,000 rows in the DOM: only the rows in view are laid out, and following a reply still lands exactly at the end. In browsers that support it, find-in-page still locates text in a row far off-screen.

## Commands

`use_message_scroller()` returns commands you can call from any descendant of the provider:

```rust
let scroller = use_message_scroller();
scroller.scroll_to_end(ScrollBehavior::Smooth);
scroller.scroll_to_start(ScrollBehavior::Auto);
scroller.scroll_to_message("m42", ScrollToMessageOptions::default());
```

`use_message_scroller_scrollable()` is a signal of which edges still have content (`Scrollable { start, end }`), fed by a message only when the value changes, never per scroll event or per token. While you are following the stream, `end` stays `false`, so a "new messages" affordance does not flicker on every chunk.

## Styling

The root and viewport carry these attributes, written by the controller rather than by Rust, so a later render never overwrites them:

- `data-scrollable`: `"start"`, `"end"`, `"start end"`, or absent.
- `data-autoscrolling`: present while a programmatic scroll to the end runs.
- `data-pending-scroll`: present until the opening position is applied.

The button carries `data-active` (`"true"` when there is content in its direction) and is `inert` otherwise.

## Notes

- Rows are always real elements (see Virtualization), so the accessibility tree sees the whole transcript, and find-in-page does too in browsers that search skipped content.
- The styled viewport has a bottom edge fade from `dx-effects.css`. Without that file the fade is simply absent.
- On a renderer without `document::eval` (native/Blitz) the shell renders as plain overflow: no auto-follow, no flash guard, and an inert button.

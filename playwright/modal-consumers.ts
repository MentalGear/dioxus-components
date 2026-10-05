/**
 * The modal overlays that open a native `<dialog>` through the shared driver
 * (`use_dialog_open_driver`, `primitives/src/lib.rs`), DERIVED from the source rather
 * than listed by hand: `modal-exit.spec.ts` runs the per-frame exit assertions and the
 * shared scrim assertions over every one of them, so a new modal component cannot ship
 * without them.
 *
 * Two derivations, both by reading the repo (the grep a reviewer would run):
 *   - PRIMITIVE CALLERS: every `primitives/src/*.rs` that calls `use_dialog_open_driver(`
 *     (today `dialog.rs`, `alert_dialog.rs` and `popover.rs`, whose modal arm used to carry a
 *     deferred-close driver of its own and is covered by `popover.spec.ts`) and every one that
 *     calls `.showModal()` itself (today only `lib.rs` -- the driver). A new file on either
 *     list fails `modal-exit.spec.ts` until it is accounted for here.
 *   - THEMED CONSUMERS: every `preview/src/components/<name>/component.rs` that composes one
 *     of those primitives (`DialogRoot`, `AlertDialogRoot`, `Drawer`, `CommandDialog`) -- today
 *     dialog, alert_dialog, sheet, drawer, command (plus the unstyled `top_layer` oracle
 *     fixture). Every other call site (sidebar's `Sheet`, the theme picker's and charts
 *     gallery's `Drawer`, the email client's compose `Dialog`) goes through one of those
 *     wrappers, so it is covered by the wrapper's entry.
 * The derived set and `CONSUMERS` below must match in both directions (the spec checks it);
 * `CONSUMERS` only adds what cannot be derived from source: how each demo is opened and closed.
 */
import * as fs from "fs";
import * as path from "path";

const REPO = path.resolve(__dirname, "..");
const PRIMITIVES = path.join(REPO, "primitives", "src");
const COMPONENTS = path.join(REPO, "preview", "src", "components");

const read = (file: string) =>
  fs
    .readFileSync(file, "utf8")
    .split("\n")
    .filter((line) => !/^\s*\/\//.test(line))
    .join("\n");

/** Primitive modules that open a native `<dialog>`, as `{ viaDriver, ownShowModal }` file-name sets. */
export function derivePrimitiveOpeners(): { viaDriver: string[]; ownShowModal: string[] } {
  const viaDriver: string[] = [];
  const ownShowModal: string[] = [];
  for (const f of fs.readdirSync(PRIMITIVES).filter((n) => n.endsWith(".rs")).sort()) {
    const src = read(path.join(PRIMITIVES, f));
    if (/use_dialog_open_driver\(/.test(src.replace(/fn use_dialog_open_driver\(/g, ""))) viaDriver.push(f);
    if (/\.showModal\(\)/.test(src)) ownShowModal.push(f);
  }
  return { viaDriver, ownShowModal };
}

/** Themed components whose `component.rs` composes a driver-backed primitive. */
export function deriveThemedConsumers(): string[] {
  const out: string[] = [];
  const compose = /(?:^|[^\w])(?:dialog::|alert_dialog::|drawer::|command::)?(?:DialogRoot|AlertDialogRoot|Drawer|CommandDialog)\s*\{/m;
  for (const name of fs.readdirSync(COMPONENTS).sort()) {
    const file = path.join(COMPONENTS, name, "component.rs");
    if (fs.existsSync(file) && compose.test(read(file))) out.push(name);
  }
  return out;
}

export type Closer = "escape" | "backdrop" | { button: string };

export interface ModalVariant {
  /** Names the test. */
  name: string;
  /** The demo button that opens it. */
  trigger: string;
  closers: Closer[];
}

export interface ModalConsumer {
  /** The `preview/src/components/<component>` directory, also the `/component/?name=` slug. */
  component: string;
  /** Matches the `<dialog>` the component renders. */
  dialogSelector: string;
  /** The `overlay` variant's button that opens the same modal with `overlay: false`. */
  overlayOffTrigger: string;
  variants: ModalVariant[];
}

/** Components that compose a driver-backed primitive but have no styled modal exit to measure, with why. */
export const EXEMPT: Record<string, string> = {
  top_layer: "oracle fixture: unstyled DialogRoot/AlertDialogRoot demos for oracle/tier2-html/top-layer*.spec.ts, no exit animation",
};

export const CONSUMERS: ModalConsumer[] = [
  {
    component: "dialog",
    overlayOffTrigger: "Dialog without overlay",
    dialogSelector: "dialog.dx-dialog",
    variants: [
      { name: "Show Dialog", trigger: "Show Dialog", closers: ["escape", { button: "Close" }, "backdrop"] },
    ],
  },
  {
    component: "alert_dialog",
    overlayOffTrigger: "Alert without overlay",
    dialogSelector: "dialog.dx-alert-dialog",
    variants: [{ name: "Show Alert Dialog", trigger: "Show Alert Dialog", closers: ["escape", { button: "Cancel" }] }],
  },
  {
    component: "sheet",
    overlayOffTrigger: "Sheet without overlay",
    dialogSelector: "dialog.dx-sheet",
    variants: [
      { name: "right", trigger: "Right", closers: ["escape", { button: "Close" }] },
      { name: "left", trigger: "Left", closers: ["escape"] },
      { name: "top", trigger: "Top", closers: ["escape"] },
      { name: "bottom", trigger: "Bottom", closers: ["escape"] },
    ],
  },
  {
    component: "drawer",
    overlayOffTrigger: "Drawer without overlay",
    dialogSelector: "dialog.dx-drawer",
    variants: [
      { name: "bottom", trigger: "Move Goal", closers: ["escape", { button: "Cancel" }] },
      { name: "top", trigger: "Open from Top", closers: ["escape"] },
    ],
  },
  {
    component: "command",
    overlayOffTrigger: "Palette without overlay",
    dialogSelector: "dialog.dx-command-dialog",
    variants: [{ name: "Open Command Palette", trigger: "Open Command Palette", closers: ["escape"] }],
  },
];

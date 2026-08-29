# Tier 3 — Radix-parity (labelled opinion)

Source: [`../../../docs/conformance-harness.md`](../../../docs/conformance-harness.md), "Tier 3 — Radix-parity, labelled as opinion", and the tier table at the top of that document.

## Rule-source policy

**Status: opinion, not standard.** Radix UI behaviour is upstream never committed to; it is useful only where the standards (tier 1, tier 2) are silent. Every rule file here must:

1. Be labelled as opinion in its file header, not as "conformance".
2. Name the specific Radix source file/behaviour it is drawn from (e.g. `@radix-ui/react-dialog`'s `onCloseAutoFocus` handling), pinned to a commit where practical.
3. Read, in any PR, as a proposal — not as a claim that the library is non-conformant.

Keeping this tier separate is not pedantry: this project's own `README.md` names shadcn for styling and APG for behaviour, and never mentions Radix. Filing a Radix behaviour as "conformance" invites an upstream maintainer to fairly reject the whole suite; filed here, it is a proposal.

Examples of behaviours that belong here because no standard specifies them: body scroll lock while a modal is open, `onCloseAutoFocus` semantics (restore focus to the trigger unless dismissal came from outside interaction), `aria-hidden` on background content while an overlay is open, collision-aware repositioning.

## Calibration

Reference subject: a pinned Radix demo. Per conformance-harness.md, avoid running this tier's reference calibration in CI (it depends on a third party); pin the Radix version/commit used and re-verify manually when a rule is added or revisited.

## Status

Per conformance-harness.md: not implemented.

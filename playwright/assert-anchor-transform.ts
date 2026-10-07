/**
 * Shared check that an overlay's open/close animation leaves `transform` alone (2026-10-05).
 *
 * THE CLASS: an anchored overlay centres itself on its anchor with `transform: translateX(-50%)`
 * (`primitives/src/top_layer.rs`'s engine stylesheet, keyed on `[data-side]`/`[data-align]`). A
 * `@keyframes` that sets `transform` REPLACES that value for as long as the animation runs, so the
 * panel plays its fade at its un-centred spot (the date picker 144px off, the colour picker 133px)
 * and snaps to the centre on the last frame. The fade animates `translate`/`scale` instead, which are
 * separate properties and compose with `transform`. `scripts/check-anchored-keyframes.sh` bans the
 * source pattern; this proves the rendered result, per component.
 *
 * WHY A PROBE, not a position sample: the menus (dropdown, menubar, navbar, navigation menu, context
 * menu) set no `data-side` today, so they carry no centring transform to lose and a frame-sampled
 * centre-x would pass on the broken keyframes too. So the probe supplies the centring itself: it sets an
 * inline `transform: translateX(-37px)` on the content (an inline declaration sits BELOW animations in
 * the cascade, exactly as the engine's rule does), seeks the element's own CSS keyframes to their start,
 * midpoint and end, and reads the computed `transform` at each. Keyframes that set `transform`
 * make it read as the animated matrix instead of the probe; keyframes that animate `translate`/`scale`
 * leave it at the probe. The seek is deterministic -- no rAF sampling, no dependence on a starved
 * main thread -- runs on a clone of the animation's keyframes (the real CSS animation is never paused or
 * finished), and the probe is removed again before it returns.
 */
import { expect, type Locator } from "@playwright/test";

const PROBE = "translateX(-37px)";
const PROBE_MATRIX = "matrix(1, 0, 0, 1, -37, 0)";

interface PhaseValues {
  transform: string;
  scale: string;
  translate: string;
}

export interface AnimationProbe {
  name: string;
  /** Every property any keyframe of this animation sets (`opacity`, `scale`, `translate`, ...). */
  keyframeProps: string[];
  start: PhaseValues;
  mid: PhaseValues;
  end: PhaseValues;
}

/**
 * Probes every CSS animation attached to `content` (a running one or a `forwards`-filled one), with `attrs`
 * set on it for the duration of the probe -- `{ "data-state": "closed" }` selects the CLOSE animation of an
 * open panel without closing it (nothing re-renders in between: the attributes go on and come off inside one
 * synchronous evaluate, so the component never sees them).
 */
export async function probeAnimations(content: Locator, attrs: Record<string, string> = {}): Promise<AnimationProbe[]> {
  return content.evaluate(async (el, { probe, attrs }) => {
    // Two frames first: on an element that mounted a moment ago, flipping `data-state` before its first
    // style resolution finds no CSS animation to probe (observed: a close-state probe straight after
    // `toBeVisible()` failed ~6 runs in 8). Everything after this line is synchronous -- the attributes go on
    // and come off inside one task, so the component never sees them.
    await new Promise<void>((resolve) => requestAnimationFrame(() => requestAnimationFrame(() => resolve())));
    const html = el as HTMLElement;
    const prev = html.style.getPropertyValue("transform");
    const prevPriority = html.style.getPropertyPriority("transform");
    const prevAttrs = Object.keys(attrs).map((k) => [k, el.getAttribute(k)] as const);
    for (const [k, v] of Object.entries(attrs)) el.setAttribute(k, v);
    const anims = el.getAnimations().filter((a): a is CSSAnimation => "animationName" in a);
    html.style.setProperty("transform", probe);
    const read = (): { transform: string; scale: string; translate: string } => {
      const cs = getComputedStyle(el);
      return { transform: cs.transform, scale: cs.scale, translate: cs.translate };
    };
    try {
      return anims.map((a) => {
        const keyframeProps = new Set<string>();
        const keyframes = a.effect!.getKeyframes();
        for (const kf of keyframes) {
          for (const k of Object.keys(kf)) {
            if (!["offset", "computedOffset", "easing", "composite"].includes(k)) keyframeProps.add(k);
          }
        }
        // Seek a CLONE of the keyframes, not the real animation: a script-driven pause()/finish() leaves the
        // CSS animation behind when `data-state` flips later (the next probe then finds a stale one). The
        // clone is a script animation, composited ABOVE the element's CSS animations, over the very same
        // properties -- whatever the keyframes set is what it shows.
        const clone = new Animation(new KeyframeEffect(el, keyframes, { duration: 1000, fill: "both" }), document.timeline);
        clone.pause();
        const at = (fraction: number) => {
          clone.currentTime = fraction * 1000;
          return read();
        };
        const start = at(0);
        const mid = at(0.5);
        const end = at(1);
        clone.cancel();
        return { name: a.animationName, keyframeProps: [...keyframeProps], start, mid, end };
      });
    } finally {
      if (prev) html.style.setProperty("transform", prev, prevPriority);
      else html.style.removeProperty("transform");
      for (const [k, v] of prevAttrs) {
        if (v === null) el.removeAttribute(k);
        else el.setAttribute(k, v);
      }
    }
  }, { probe: PROBE, attrs });
}

/**
 * Asserts the content's open/close fade (the animation that animates `opacity`) never sets `transform`
 * -- its keyframes carry no `transform`, and the probed centring survives at the start, midpoint and end
 * -- and that it really does move something (`scale` or `translate` differs between start and end), so a
 * fade that lost its motion cannot pass as "composes fine".
 */
export async function expectFadeKeepsTransform(content: Locator, label: string, attrs: Record<string, string> = {}): Promise<void> {
  const fades = (await probeAnimations(content, attrs)).filter((a) => a.keyframeProps.includes("opacity"));
  expect(fades.length, `${label}: expected a CSS fade (an animation with opacity keyframes) on the content`).toBeGreaterThan(0);
  for (const a of fades) {
    expect(a.keyframeProps, `${label}: @keyframes ${a.name} sets \`transform\` (replaces the anchor's centring transform)`).not.toContain("transform");
    for (const phase of ["start", "mid", "end"] as const) {
      expect(
        a[phase].transform,
        `${label}: @keyframes ${a.name} replaced the element's transform at its ${phase} (an anchored overlay's centring translateX(-50%) would be lost)`,
      ).toBe(PROBE_MATRIX);
    }
    const moved = a.start.scale !== a.end.scale || a.start.translate !== a.end.translate;
    expect(moved, `${label}: @keyframes ${a.name} moves nothing (start ${JSON.stringify(a.start)}, end ${JSON.stringify(a.end)})`).toBe(true);
  }
}

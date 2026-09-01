/**
 * Shared assertion logic for the accordion close-animation regression check.
 *
 * Used by `accordion-animation.spec.ts` in both of its modes (real app route
 * and standalone reproduction page) -- same function, so "it passes on the
 * repro" and "it passes on the app" are the same claim.
 *
 * A sample is `{ t: number (ms since close started), h: number (px),
 * exists: boolean }`. The last sample where `exists` is true is the final
 * on-screen height right before `use_animated_open` unmounts the element;
 * everything after that has `exists: false`.
 */
export interface Sample {
  t: number;
  h: number;
  exists: boolean;
  unmounted?: boolean;
}

export interface CloseAnimationReport {
  mountedSamples: Sample[];
  finalMountedHeight: number;
  maxPlateauRun: number;
  plateauHeights: number[];
}

/** Frame-to-frame delta (in px) below which two samples count as "the same". */
const PLATEAU_EPSILON_PX = 1;

/** Longest allowed run of consecutive near-identical-height frames before unmount. */
const MAX_ALLOWED_PLATEAU_FRAMES = 2;

/** Height (px) the content must reach before it is unmounted. */
const MAX_FINAL_HEIGHT_PX = 1;

export function analyzeCloseSamples(samples: Sample[]): CloseAnimationReport {
  const mountedSamples = samples.filter((s) => s.exists);
  if (mountedSamples.length === 0) {
    throw new Error("no samples captured while the content element was mounted");
  }

  const finalMountedHeight = mountedSamples[mountedSamples.length - 1].h;

  // Once the height has actually reached the target (~0px), holding there
  // for the rest of the close-animation-plus-hold window is correct
  // behaviour, not jank -- so plateau detection only looks at the frames
  // BEFORE the panel first reaches that target. A plateau found there means
  // the animation stalled away from 0 (e.g. on a residual padding box)
  // rather than continuing to shrink toward it.
  let doneIndex = mountedSamples.findIndex((s) => s.h < MAX_FINAL_HEIGHT_PX);
  if (doneIndex === -1) doneIndex = mountedSamples.length - 1;

  // Sampling starts the moment the close click resolves, a few frames
  // before the framework has re-rendered `data-open="false"` and the close
  // animation has begun; those leading frames sit at the fully-open height
  // and are event/render latency, not animation jank. Plateau detection
  // therefore begins at the first frame the height actually moves.
  let startIndex = 1;
  while (
    startIndex <= doneIndex &&
    Math.abs(mountedSamples[startIndex].h - mountedSamples[startIndex - 1].h) <= PLATEAU_EPSILON_PX
  ) {
    startIndex++;
  }

  let maxPlateauRun = 0;
  let currentRun = 1;
  const plateauHeights: number[] = [];
  for (let i = startIndex + 1; i <= doneIndex; i++) {
    const delta = Math.abs(mountedSamples[i].h - mountedSamples[i - 1].h);
    if (delta <= PLATEAU_EPSILON_PX) {
      currentRun++;
      if (currentRun > maxPlateauRun) {
        maxPlateauRun = currentRun;
        plateauHeights.length = 0;
        plateauHeights.push(mountedSamples[i - 1].h, mountedSamples[i].h);
      }
    } else {
      currentRun = 1;
    }
  }

  return { mountedSamples, finalMountedHeight, maxPlateauRun, plateauHeights };
}

export function assertCloseAnimationReachesZero(samples: Sample[]) {
  const report = analyzeCloseSamples(samples);

  if (report.finalMountedHeight >= MAX_FINAL_HEIGHT_PX) {
    throw new Error(
      `content height did not reach ~0 before unmount: last on-screen height was ` +
        `${report.finalMountedHeight.toFixed(2)}px (must be < ${MAX_FINAL_HEIGHT_PX}px). ` +
        `This is the "residual box that snaps to 0 on unmount" jank.`
    );
  }

  // Allow a small extra frame or two for rAF/animation-frame slop.
  if (report.maxPlateauRun > MAX_ALLOWED_PLATEAU_FRAMES + 1) {
    throw new Error(
      `content height plateaued at ~${report.plateauHeights[0]?.toFixed(2)}px for ` +
        `${report.maxPlateauRun} consecutive frames (allowed: ${MAX_ALLOWED_PLATEAU_FRAMES}). ` +
        `This is the "holds on for a second near the end" jank.`
    );
  }

  return report;
}

import { defineConfig, devices } from "@playwright/test";
import { perTestTimeoutMs, timingSpecIgnore } from "./base-url";
// Local-only, forced-classic-scrollbar variant of oracle.local.config.ts.
//
// This repo's default headless Chromium renders 0-width overlay scrollbars
// even for a genuinely tall, scrollable page (see
// docs/phase4-spike-findings.md, "Round 2 -- solved by construction",
// Construction A/C) -- so the scroll-lock scrollbar-gap regression is
// invisible to a normal headless run. Running headed Chromium (`headless:
// false`) under `xvfb-run` (a virtual X server) makes Chromium render a real,
// space-reserving classic scrollbar (confirmed: a 15px gap on this image),
// exactly like most users' actual Windows/Linux/other-engine browsers.
//
// Invoke with:
//   xvfb-run -a npx playwright test --config=xvfb.local.config.ts <spec>
export default defineConfig({
  testDir: ".",
  fullyParallel: false,
  workers: 1,
  reporter: "list",
  timeout: perTestTimeoutMs(90 * 1000), // override: PW_TEST_TIMEOUT=<ms>
  testIgnore: timingSpecIgnore(), // main-thread.spec.ts is skipped on a known-debug build (base-url.ts)
  projects: [{
    name: "chromium",
    // Same gap as baseline.local.config.ts (dev-docs/backlog.md row 105): no mobile-emulation
    // device here, so the `... (mobile)` `.tap()` specs fail rather than being excluded.
    grepInvert: /mobile/,
    use: {
      ...devices["Desktop Chrome"],
      launchOptions: {
        executablePath: "/opt/pw-browsers/chromium-1194/chrome-linux/chrome",
        headless: false,
      },
    },
  }],
});

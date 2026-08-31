import { defineConfig, devices } from "@playwright/test";
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
  timeout: 90 * 1000,
  projects: [{
    name: "chromium",
    use: {
      ...devices["Desktop Chrome"],
      launchOptions: {
        executablePath: "/opt/pw-browsers/chromium-1194/chrome-linux/chrome",
        headless: false,
      },
    },
  }],
});

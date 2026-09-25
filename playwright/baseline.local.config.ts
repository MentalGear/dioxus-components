import { defineConfig, devices } from "@playwright/test";
// Local-only: use the image's preinstalled Chromium and the already-running dx server.
// Modeled on oracle.local.config.ts, but parallelized for a full-suite baseline run.
export default defineConfig({
  testDir: ".",
  fullyParallel: true,
  workers: 4,
  reporter: "list",
  timeout: 90 * 1000,
  projects: [{
    name: "chromium",
    // Mirrors playwright.config.ts's own CI project: without this, navbar.spec.ts:26,
    // select.spec.ts:132 and sidebar.spec.ts:151 (each named `... (mobile)`) run here too and
    // fail on `.tap()`, which needs `hasTouch` -- this launch has no mobile emulation. Reproduces
    // identically on f103bbd; dev-docs/backlog.md row 105.
    grepInvert: /mobile/,
    use: {
      ...devices["Desktop Chrome"],
      launchOptions: { executablePath: "/opt/pw-browsers/chromium-1194/chrome-linux/chrome", args: ["--no-sandbox"] },
    },
  }],
});

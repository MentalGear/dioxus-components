import { defineConfig, devices } from "@playwright/test";
// Local-only: use the image's preinstalled Chromium and the already-running dx server.
export default defineConfig({
  testDir: ".",
  fullyParallel: false,
  workers: 1,
  reporter: "list",
  timeout: 90 * 1000,
  projects: [{
    name: "chromium",
    // Same gap as baseline.local.config.ts (dev-docs/backlog.md row 105): no mobile-emulation
    // device here, so the `... (mobile)` `.tap()` specs fail rather than being excluded.
    grepInvert: /mobile/,
    use: {
      ...devices["Desktop Chrome"],
      launchOptions: { executablePath: "/opt/pw-browsers/chromium-1194/chrome-linux/chrome" },
    },
  }],
});

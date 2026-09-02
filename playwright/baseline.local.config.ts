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
    use: {
      ...devices["Desktop Chrome"],
      launchOptions: { executablePath: "/opt/pw-browsers/chromium-1194/chrome-linux/chrome", args: ["--no-sandbox"] },
    },
  }],
});

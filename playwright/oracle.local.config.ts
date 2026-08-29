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
    use: {
      ...devices["Desktop Chrome"],
      launchOptions: { executablePath: "/opt/pw-browsers/chromium-1194/chrome-linux/chrome" },
    },
  }],
});

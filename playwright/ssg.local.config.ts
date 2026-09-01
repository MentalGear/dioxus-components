import { defineConfig, devices } from "@playwright/test";
// Local-only: the SSG lane. Points at a plain static file server serving
// the fullstack-SSG-prerendered site (`dx build --ssg --features
// fullstack --platform web`'s output directory,
// `preview/target/dx/preview/debug/web/public` locally), NOT the `dx serve`
// dev server baseline.local.config.ts/playwright.config.ts depend on -- no
// webServer entry here, so this config never tries to start (or wait on) a
// dx dev server. Start the static server yourself first, e.g.:
//
//   python3 -m http.server 8090 -d <site-dir>
//
// Existing specs across this repo hardcode `http://127.0.0.1:8080` as their
// base URL, so when running any of *those* against this lane, also serve
// the same SSG site dir on port 8080 (two static-server processes over the
// same directory is fine -- it's just files). The new oracle spec this
// config was added for (`oracle/hydration-parity.spec.ts`) hardcodes 8090
// itself. See docs/conformance-harness.md, "SSG lane", for the full
// build+serve recipe this mirrors.
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
      launchOptions: { executablePath: "/opt/pw-browsers/chromium-1194/chrome-linux/chrome" },
    },
  }],
});

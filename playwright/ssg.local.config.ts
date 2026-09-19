import { defineConfig, devices } from "@playwright/test";
import fs from "node:fs";
// The SSG lane. Points at a plain static file server serving the
// fullstack-SSG-prerendered site (`dx build --ssg --features fullstack
// --platform web`'s output directory,
// `target/dx/preview/release/web/public` -- `debug` instead of `release` if
// built without `--release`), NOT the `dx serve` dev server
// baseline.local.config.ts/playwright.config.ts depend on -- no webServer
// entry here, so this config never tries to start (or wait on) a dx dev
// server. Start the static server yourself first, e.g.:
//
//   python3 -m http.server 8090 -d <site-dir>
//
// Existing specs across this repo hardcode `http://127.0.0.1:8080` as their
// base URL, so when running any of *those* against this lane, also serve
// the same SSG site dir on port 8080 (two static-server processes over the
// same directory is fine -- it's just files). The oracle spec this config
// was added for (`oracle/hydration-parity.spec.ts`) hardcodes 8090 itself,
// and reads `SSG_SITE_DIR` (the same site dir this config's own comment
// above names) to enumerate every prerendered component page for its row
// 46 rules -- see that file's own header. See dev-docs/conformance-
// harness.md, "SSG lane", for the full build+serve recipe this mirrors, and
// `.github/workflows/playwright.yml`'s `ssg-hydration-parity` job for how
// CI drives this same config (dev-docs/backlog.md row 22).
//
// `executablePath` below is this AGENT SANDBOX's own local Chromium
// install path (`dx`/`npx playwright install` land browsers somewhere this
// sandbox's own tooling doesn't always find automatically) -- committed
// because this file itself is committed and used by the CI job above, but
// only ever applied when that exact path exists; a real CI runner (which
// installs its own browser via `npx playwright install --with-deps
// chromium` into Playwright's normal cache dir) hits neither branch and
// gets Playwright's own default resolution, same as every other config in
// this repo that never sets `executablePath` at all.
const SANDBOX_CHROMIUM = "/opt/pw-browsers/chromium-1194/chrome-linux/chrome";
const launchOptions = fs.existsSync(SANDBOX_CHROMIUM)
  ? { executablePath: SANDBOX_CHROMIUM }
  : {};

export default defineConfig({
  testDir: ".",
  fullyParallel: true,
  workers: 4,
  reporter: process.env.CI ? [["list"], ["html", { open: "never" }]] : "list",
  timeout: 90 * 1000,
  projects: [{
    name: "chromium",
    use: {
      ...devices["Desktop Chrome"],
      launchOptions,
    },
  }],
});

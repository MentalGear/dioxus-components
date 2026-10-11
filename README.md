<div align="center">
  <h1>shadcn-dioxus</h1>
  <p><strong>Accessible, customizable components for Dioxus.</strong></p>
</div>

<div align="center">
  <!-- Crates version -->
  <a href="https://crates.io/crates/dioxus-primitives">
    <img src="https://img.shields.io/crates/v/dioxus-primitives.svg?style=flat-square"
    alt="Crates.io version" />
  </a>
  <!-- Downloads -->
  <a href="https://crates.io/crates/dioxus-primitives">
    <img src="https://img.shields.io/crates/d/dioxus-primitives.svg?style=flat-square"
      alt="Download" />
  </a>
  <!-- docs -->
  <a href="https://docs.rs/dioxus-primitives">
    <img src="https://img.shields.io/badge/docs-latest-blue.svg?style=flat-square"
      alt="docs.rs docs" />
  </a>
</div>

---

<br/>

shadcn-dioxus is a shadcn style component library for Dioxus built on top of the unstyled [Dioxus primitives](https://crates.io/crates/dioxus-primitives) library. The unstyled primitives serve as the foundation for building accessible and customizable UI components in Dioxus applications. The styled versions serve as a starting point to develop your own design system.

**Live preview:** browse the component gallery at [mentalgear.github.io/shadcn-dioxus](https://mentalgear.github.io/shadcn-dioxus/) — individual component pages live at `https://mentalgear.github.io/shadcn-dioxus/component/<name>/`.

## Getting started

First, explore the [component gallery](https://mentalgear.github.io/shadcn-dioxus/) to find the components you want to use.

Install the `shadcn-dioxus` command-line tool once. It copies a component's source and stylesheet into your app, adds the crates it needs to `Cargo.toml`, installs the components it depends on, and reads **this** registry by default, so no command below needs a `--git` flag:

```sh
cargo install --git https://github.com/MentalGear/shadcn-dioxus shadcn-dioxus
```

```sh
shadcn-dioxus new myapp      # new project, from the starter template (no dx needed)
shadcn-dioxus init           # existing project (writes the registry into Dioxus.toml)
shadcn-dioxus add button
shadcn-dioxus update         # never overwrites files you edited (--force does)
```

`shadcn-dioxus list` shows everything available and `shadcn-dioxus remove <name>` uninstalls a component. The full reference (what `update` keeps and replaces, which registry is read, `--path`/`--git`/`--rev`) is in [`cli/README.md`](./cli/README.md).

### New project

```sh
shadcn-dioxus new myapp
cd myapp
shadcn-dioxus add button
```

The starter declares `mod components;` in `src/main.rs`, depends on `dioxus-primitives` from this repository and names this registry in its `Dioxus.toml`. After your first add, link the shared theme once in `App` with `document::Link { rel: "stylesheet", href: asset!("/assets/dx-components-theme.css") }`.

### Existing project

Run `shadcn-dioxus init` once from the project root. It writes `[components.registry]` into `Dioxus.toml` (keeping your comments), creates `src/components`, declares `mod components;` in `src/main.rs`, copies the shared theme to `assets/dx-components-theme.css` and links it from `App`. It is safe to run again. Then `shadcn-dioxus add button`.

`shadcn-dioxus update` brings installed components up to the registry's latest. It records a hash of every file it writes, so a component with a file you edited is left unchanged and listed (the command exits non-zero); `--force` replaces it anyway.

### Without the CLI

The registry is also a plain `dx components` registry, if you would rather not install another tool. If you do not already have `dx`:

```
cargo install dioxus-cli
```

Any one of these works:

- **A new project from the starter template** (the same project `shadcn-dioxus new` makes):

  ```
  dx new myapp --template https://github.com/MentalGear/shadcn-dioxus --subtemplate templates/starter
  cd myapp
  dx components add button
  ```

  (Add `--yes` to skip the prompts.) The starter's `Dioxus.toml` names this registry, so the plain `dx components add button` reads it.

- **Name the registry once in your app's `Dioxus.toml`**, then use plain `dx components add <name>`:

  ```toml
  [components.registry]
  git = "https://github.com/MentalGear/shadcn-dioxus"
  ```

- **Pass the registry on every command:**

  ```
  dx components add button --git https://github.com/MentalGear/shadcn-dioxus
  ```

  `dx components list --git https://github.com/MentalGear/shadcn-dioxus` shows everything available. Keep the URL exactly as written, since the components that depend on each other (and on `dioxus-primitives`) name it too.

**Warning:** plain `dx components add <name>` without one of those setups reads dx's default registry (upstream `DioxusLabs/components`) and installs *upstream's* version of the component, not this one. The `--git` flag or the `Dioxus.toml` setting is what points it here.

Either way the first add creates a `components` folder in your project (if it doesn't already exist), adds the component files to it, adds the `dioxus-primitives` dependency to your `Cargo.toml`, and copies the shared theme to `assets/dx-components-theme.css`. With `dx`, declare the module with `mod components;` in your `main.rs` and link the theme once in your root component with `document::Link { rel: "stylesheet", href: asset!("/assets/dx-components-theme.css") }`; `shadcn-dioxus init` does both for you.

## Motion

Animation speed comes from `--dx-motion-duration-*` and `--dx-motion-ease*` tokens, `prefers-reduced-motion` is answered by the stylesheets, an infinite animation may only animate compositor properties, and motion nobody can see is not run: loops pause, timers stop and entrances wait to be seen (`use_motion`, `use_interval_while`, `use_entered_view_when`, `use_document_visible`). The tokens, the hooks, the gates, how to write your own animated component and a per-component table of what moves and what each does while unseen are in [`dev-docs/motion.md`](./dev-docs/motion.md), and on the site under [Docs, "Motion"](https://mentalgear.github.io/shadcn-dioxus/docs/#motion).

## Contributing

### Project structure

This repository contains two main crates:
- `dioxus-primitives`: The core unstyled component library.
- `preview`: A Dioxus application that showcases the components from `dioxus-primitives` with shadcn-styled versions.

### Adding new components

If you want to add a new component, you should:
1. If there is any new interaction logic or accessibility features required, implement an unstyled component in the `dioxus-primitives` crate. When adding components to the primitives library, ensure:
    - It adheres to the [WAI-ARIA Authoring Practices for accessibility](https://www.w3.org/WAI/standards-guidelines/aria/).
    - All styling can be modified via props. Every element should spread attributes and children from the props
2. In the `preview` crate, create a styled version of the component using shadcn styles. This will serve as an example of how to use the unstyled component and serve as the styled version `dx components` will add to projects.
3. Add tests in `playwright` to ensure the component behaves as expected.

### Testing changes

The components use a combination of unit tests with cargo, css linting, and end-to-end tests with Playwright.

To run the unit tests for the `dioxus-primitives` crate, use:

```sh
cargo test -p dioxus-primitives
```

To run the CSS linting, use:

```sh
cd preview
npm install
npx stylelint "src/**/*.css"
```

To run the Playwright end-to-end tests, use:

```sh
cd playwright
npm install
npx playwright test
```

Most specs also run an [axe-core](https://github.com/dequelabs/axe-core) static accessibility scan (valid ARIA, accessible names, unique landmarks, contrast, heading order, …) via the shared `playwright/axe.ts` helper — see [`dev-docs/conformance-harness.md`](./dev-docs/conformance-harness.md), "axe (static rules)", for what it covers versus the behaviour oracles and its exclusion policy.

Local-only Playwright configs, for driving the suite against an
already-running `dx serve`/`dx run` server instead of letting Playwright's
own `webServer` block manage one: `baseline.local.config.ts` (full-suite
runs), `xvfb.local.config.ts` (headed Chromium under a virtual X server, for
tests that need a real, space-reserving scrollbar rather than headless
Chromium's 0-width one), and `ssg.local.config.ts` (points at a plain static
file server serving the fullstack-SSG-prerendered build rather than the dev
server — see [`dev-docs/conformance-harness.md`](./dev-docs/conformance-harness.md),
"Hydration/deployment parity", for the full build-and-serve recipe this
covers, including `oracle/hydration-parity.spec.ts`). When running any of
these under `root` (as in a container), the touch/mobile-emulation oracle
specs need Chromium launched with `--no-sandbox` — see
`baseline.local.config.ts`'s `launchOptions` for the pattern.

Several source-level guard scripts enforce conventions that would
otherwise regress one component or change at a time. Some run in CI
(`.github/workflows/main.yml`: `check-cfg-axis.sh`,
`check-hooks-in-closures.sh`, `check-self-subscribing-effects.sh`,
`check-css-logical-properties.sh`), and
all of them are cheap enough to run on every relevant change locally,
CI job or not:

```sh
# preview/ markup composes only themed wrappers (crate::components::*),
# never a raw dioxus_primitives:: component directly -- see
# dev-docs/preview-composition.md for why this matters.
scripts/check-preview-composition.sh

# rendered markup/component structure/attribute choice splits on the `web`
# Cargo feature, never on `target_family = "wasm"` -- see
# dev-docs/recommended-implementations.md, Caveat 1, for the production
# incident this guards against.
scripts/check-cfg-axis.sh

# every themed component's shipped classes are namespaced
# dx-<component>[-...], the collision-safety property #[css_module]
# hashing used to provide -- see dev-docs/backlog.md row 32.
scripts/check-dx-class-prefix.sh

# a themed stylesheet may not hard-code a value that exactly matches a
# design token -- see dev-docs/backlog.md row 31b.
scripts/check-css-literals.sh

# no Dioxus hook is called inside the closure passed to another hook
# (use_context_provider, use_hook, use_memo, use_effect, use_callback,
# use_signal, use_resource, use_future) -- that panics at runtime
# ("hook list is already borrowed") with nothing at compile time to catch
# it. See dev-docs/backlog.md row 74 for the Navigation Menu incident this
# guards against.
scripts/check-hooks-in-closures.sh

# inside a use_effect closure, no signal is read with tracked syntax
# (x() / x.read()) and also written (x.set(...) / x.write()) -- that
# re-subscribes the effect to a value it just changed itself. See
# dev-docs/backlog.md row 73 for the Drawer drag-hang incident this
# guards against.
scripts/check-self-subscribing-effects.sh

# a themed stylesheet may not hard-code a physical inline-axis CSS value
# (margin-left/right, left/right, border-*-left/right*, text-align: left/
# right, a non-zero translateX(), ...) where a logical property would
# express the same rule and mirror correctly under dir="rtl" -- see
# dev-docs/backlog.md row 13. Two escape hatches: any rule selector
# mentioning `data-side=` (a screen-geometry fact, not a reading-direction
# one), or a `/* rtl-physical: <reason> */` comment -- see the script's own
# header for the full allowlist.
scripts/check-css-logical-properties.sh
```

### Running the preview

To test your changes, you can run the preview application. For a desktop build, use:

```sh
dx serve -p preview --desktop
```

or for the web build:

```sh
dx serve -p preview --web
```

### Deploying the preview / docs site

GitHub Pages is configured to serve straight from `main`'s `/docs` folder
— there is no CI build step. To publish a new preview build:

```sh
scripts/deploy-preview.sh
```

This builds the `preview` app in release mode and overwrites `/docs` with
the output. Review the result (`git status`, `git diff --stat -- docs`),
then commit and push to `main`.

## License

This project is dual licensed under the [MIT](./LICENSE-MIT) and [Apache 2.0](./LICENSE-APACHE) licenses.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this repository, by you, shall be licensed as MIT or Apache 2.0, without any additional terms or conditions.

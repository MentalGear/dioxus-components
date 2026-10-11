# shadcn-dioxus (CLI)

The installer for the [shadcn-dioxus](https://github.com/MentalGear/shadcn-dioxus) component registry. It copies a
component's source and stylesheet into your Dioxus app, adds the crates it needs to `Cargo.toml`, and installs
the components it depends on, **from this registry**.

This is the CLI reference. For the install walkthrough (CLI first, then the `dx` alternatives and the warning about plain `dx components add`) see the repository's [README](../README.md#getting-started) or the site's [Docs](https://mentalgear.github.io/shadcn-dioxus/docs/); for what each component animates and how, see [`dev-docs/motion.md`](../dev-docs/motion.md).

```sh
cargo install --git https://github.com/MentalGear/shadcn-dioxus shadcn-dioxus
```

```sh
shadcn-dioxus new myapp              # a new app from the registry's starter template (no dx needed)
shadcn-dioxus init                   # once per existing app: Dioxus.toml registry, src/components, mod components;, theme
shadcn-dioxus list                   # every component in the registry
shadcn-dioxus add button date_picker # also installs calendar and popover, which date_picker needs
shadcn-dioxus remove button
shadcn-dioxus update                 # bring installed components up to the registry's latest, keeping your edits
shadcn-dioxus clean                  # delete the cached clones
```

## `new`

`shadcn-dioxus new <name> [--path <dir>] [--git <url>] [--rev <rev>] [--vcs git|none]` creates `<name>` from
`templates/starter` of the registry and prints the next steps (`cd <name>`, `shadcn-dioxus add button`,
`dx serve`). It produces the same project as
`dx new <name> --template https://github.com/MentalGear/shadcn-dioxus --subtemplate templates/starter`, without
dx or cargo-generate:

- `<name>` is a directory (a path works: the last component is the project name). The package name is
  sanitised like cargo-generate does (`My App` becomes `my-app`, `my_app` and `my-app` are kept); a name cargo
  rejects (starts with a digit, a Rust keyword) is refused before anything is written.
- `Cargo.toml.liquid` becomes `Cargo.toml`; `{{project-name}}`, `{{authors}}` (`git config user.name <user.email>`,
  else `$USER`), `{{crate_name}}`, `{{username}}` and `{{os-arch}}` are replaced in file contents and file names.
  Anything else in the template (a filter, a `{% tag %}`, an unknown variable, a `[hooks]` or `[placeholders]`
  table) is an error naming the file, not a silent copy, so the template and this renderer cannot drift apart.
- The target must not exist or must be an empty directory; otherwise nothing is touched. The project is
  rendered in memory first, so a failure leaves no half-written directory.
- `--vcs git` (default) runs `git init` without a commit; `--vcs none` skips it.
- `--path`, `--git` and `--rev` choose where the *template* comes from, like they choose the registry for `add`.
  They do not change the registry URL written into the new project's `Dioxus.toml`; that stays the default one.
  A cached clone that predates the template is refreshed once automatically.

## `update`

`shadcn-dioxus update [components...] [--force] [--skip-cargo-add]` refreshes the registry (a cached git clone
is fetched; a pinned `--rev` branch follows the branch, a tag or commit stays put; `--path` is read as it is)
and then updates what is already installed. Without names it covers every installed component the registry
still has; with names, those plus the components they depend on. Outside a Dioxus project and without names it
only refreshes the cached clone.

What it overwrites and what it keeps. `add` and `update` record a hash of every file they write in
`src/components/.shadcn-dioxus.json` (components) and, by file name, of the global assets such as the theme.
That record is what separates "the registry changed this file" from "you edited it":

| File state | Result |
| --- | --- |
| identical to the registry's | nothing to do |
| identical to what was last installed (you never touched it), registry has a newer version | replaced |
| differs from what was installed (you edited it) | the whole component is left unchanged and listed; the command exits 1 |
| installed without this tool (`dx components add`, or before the record existed), differs from the registry | treated as your edit, same as above (a component identical to the registry is adopted into the record) |
| missing locally, still in the registry | restored |
| not in the registry any more, untouched | deleted |
| not in the registry any more, you edited it | kept |

- A component is updated all-or-nothing: one edited file keeps all its files as they are, so you never get a
  mix of old and new. The other components still update; the exit status is non-zero if any was skipped.
- The theme (`assets/dx-components-theme.css`) follows the same rule: an unedited one moves with the registry,
  an edited one is kept and reported once.
- A dependency the registry newly added to a component you have is installed (with its `mod.rs` line and
  crates). A component of yours that the registry does not know is left alone.
- `--force` replaces each differing component directory and the theme outright, discarding your edits (like
  `add --force`); it removes files you added inside those directories.
- `cargo add` runs for the crate dependencies of components that changed or were installed; `--skip-cargo-add`
  turns that off. Your own edits to `Cargo.toml` are not touched otherwise.
- `.shadcn-dioxus.json` is safe to commit; delete it and `update` falls back to the "installed without this
  tool" row.

`init` is idempotent and prints what it changed (`created`, `updated`, `unchanged`, `left alone`). It writes
`[components.registry]` into `Dioxus.toml` (creating or merging, keeping your comments), creates
`src/components/mod.rs`, declares `mod components;` in `src/main.rs`, copies the theme to
`assets/dx-components-theme.css` and links it from `App`. After it, plain `dx components add <name>` reads this
registry too.

## Which registry

First match wins: `--path <dir>` (a local checkout); `--git <url>` / `--rev <rev>`; the app's `Dioxus.toml`
`[components.registry]` (`path`, or `git` + `rev`); the default, `https://github.com/MentalGear/shadcn-dioxus`.
Clones are cached under `$SHADCN_DIOXUS_HOME/registries` (default: the platform cache directory).

## Other options

`add --all`, `--force` (replace installed components and a modified theme), `--skip-cargo-add`,
`--module-path`, `--global-assets-path`. `Dioxus.toml` keys `[components] components_dir` and
`[application] asset_dir` are honoured as in `dx`.

## Fork of `dx components`

This is a fork of the `dx components` subcommand of dioxus-cli 0.7.9 (`src/cli/component.rs`,
`src/config/component.rs`) and of the `dioxus-component-manifest` crate,
<https://github.com/DioxusLabs/dioxus>, licensed `MIT OR Apache-2.0` like this repository (`LICENSE-MIT`,
`LICENSE-APACHE` at the repository root); the Dioxus authors' copyright and licence apply to the copied code
(Jonathan Kelley, Evan Almloff and contributors). Each forked file says so in its header. It is not proposed back
upstream.

Changed from `dx components`:

- the default registry is this repository, not `DioxusLabs/components`;
- a bare-name `componentDependencies` entry resolves in the registry of the component that names it (dx
  resolves it in *its* default registry, which is why `date_picker` needed `{ "name", "git" }` objects); the
  object form still works, and our own entries naming the default URL follow the copy being installed
  (`--path`, `file://`, `--rev`) instead of the network;
- every component copies its global assets from its own registry;
- destinations are checked before anything is written, so "already installed" leaves the project untouched;
- a modified global asset (your theme) is kept unless `--force`;
- a clone goes to a temporary directory and is renamed into the cache only when complete;
- `init`, `new` and `update` are new; `schema` is dropped; git runs through the `git` binary instead of libgit2, and there is no
  async runtime or `cargo metadata` call, so the crate has six dependencies.

`component.json` is parsed exactly as dx parses it (same fields, `deny_unknown_fields`), so a manifest this tool
accepts is one `dx components add` accepts.

## Developing

`cargo test -p shadcn-dioxus`. The default registry URL lives in `src/registry.rs`
(`DEFAULT_REGISTRY_GIT_URL`); `scripts/check-registry-url.sh` fails if it differs from `REGISTRY_GIT_URL` in
`preview/src/main.rs`.

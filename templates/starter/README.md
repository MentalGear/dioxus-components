# {{project-name}}

A Dioxus app set up for the shadcn-dioxus component registry
(https://github.com/MentalGear/shadcn-dioxus; see `[components.registry]` in `Dioxus.toml`).

## Add components with the CLI

Install the `shadcn-dioxus` command-line tool once:

```
cargo install --git https://github.com/MentalGear/shadcn-dioxus shadcn-dioxus
```

Then, from this directory:

```
shadcn-dioxus list              # everything available
shadcn-dioxus add button        # copies src/components/button, its dependencies and the shared theme
shadcn-dioxus update            # bring installed components up to the registry's latest (never overwrites files you edited)
dx serve                        # run the app (web by default)
```

`shadcn-dioxus remove <name>` uninstalls a component. The full reference is in the registry's
`cli/README.md`.

## Alternative: `dx components`

If you would rather not install another tool, the registry is also a plain `dx components` registry, and
`Dioxus.toml` already points `dx` at it, so no `--git` flag is needed here:

```
dx components list
dx components add button
```

(Outside a project that sets `[components.registry]`, plain `dx components add` reads dx's default registry and
installs upstream's version of the component, not this one.)

## After the first add

`mod components;` is already declared in `src/main.rs`; link the theme once in `App`
with `document::Link { rel: "stylesheet", href: asset!("/assets/dx-components-theme.css") }`.

//! `init`: make an existing Dioxus app ready for `add` (and for plain `dx components add`).
//!
//! New code, not part of dx. It does four things, each idempotent and each reported as `created`,
//! `updated`, `unchanged` or `left alone`:
//!   1. `Dioxus.toml`: the `[components.registry]` table, created or merged, so that a plain
//!      `dx components add <name>` (and this tool without flags) reads this registry;
//!   2. `src/components/mod.rs`;
//!   3. `mod components;` in `src/main.rs`;
//!   4. the shared theme: copied to `assets/dx-components-theme.css` and linked once from `App` with
//!      `document::Link { rel: "stylesheet", href: asset!("/assets/dx-components-theme.css") }`
//!      (the line the docs page tells users to add; `asset!` refuses to compile for a missing file, so
//!      the file is copied first).
//!
//! Every decision is a pure function over file contents ([`merge_registry`], [`with_mod_decl`],
//! [`with_theme_link`]); the network (reading the registry for the theme) happens before the first
//! write, so a failure leaves the project untouched.

use crate::add::{copy_global_assets, ensure_components_module, AssetOutcome};
use crate::project::{parse_config, Project, RegistryConfig};
use crate::registry::{resolve_source, Registries, RegistryArgs, DEFAULT_REGISTRY_GIT_URL};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};
use toml_edit::{value, DocumentMut, Item, Table};

/// The shared stylesheet every styled component lists in its `globalAssets`.
pub const THEME_FILE: &str = "dx-components-theme.css";

/// What `init` did to `Dioxus.toml`.
#[derive(Debug, PartialEq, Eq)]
pub enum RegistryChange {
    /// There was no `Dioxus.toml`.
    Created,
    /// There was one without a registry; the table was added.
    Added,
    /// It already names exactly this registry.
    Unchanged,
    /// It names a different registry and was not touched (no `--force`).
    Differs(RegistryConfig),
    /// It named a different registry and `--force` replaced it.
    Replaced,
}

/// The `[components.registry]` this run wants: the flags, else this repository.
pub fn desired_registry(args: &RegistryArgs) -> RegistryConfig {
    if let Some(path) = &args.path {
        return RegistryConfig {
            path: Some(path.to_string_lossy().into_owned()),
            ..Default::default()
        };
    }
    RegistryConfig {
        git: Some(
            args.git
                .clone()
                .unwrap_or_else(|| DEFAULT_REGISTRY_GIT_URL.to_owned()),
        ),
        rev: args.rev.clone(),
        path: None,
    }
}

/// `Dioxus.toml` content with `[components.registry]` set to `desired`. Comments, ordering and every
/// other key are preserved. Returns the new content (equal to the input when nothing changes).
pub fn merge_registry(
    existing: Option<&str>,
    desired: &RegistryConfig,
    force: bool,
) -> Result<(String, RegistryChange)> {
    let text = existing.unwrap_or("");
    let current = parse_config(text)?.registry;
    if current == *desired {
        return Ok((text.to_owned(), RegistryChange::Unchanged));
    }
    if !current.is_empty() && !force {
        return Ok((text.to_owned(), RegistryChange::Differs(current)));
    }
    let mut doc: DocumentMut = text.parse()?;
    if !doc.contains_key("components") {
        let mut components = Table::new();
        components.set_implicit(true);
        doc["components"] = Item::Table(components);
    }
    if !doc["components"]
        .as_table_like()
        .is_some_and(|t| t.contains_key("registry"))
    {
        doc["components"]["registry"] = Item::Table(Table::new());
    }
    let registry = &mut doc["components"]["registry"];
    for key in ["git", "rev", "path"] {
        if let Some(table) = registry.as_table_like_mut() {
            table.remove(key);
        }
    }
    for (key, v) in [
        ("git", &desired.git),
        ("rev", &desired.rev),
        ("path", &desired.path),
    ] {
        if let Some(v) = v {
            registry[key] = value(v.as_str());
        }
    }
    let change = match (existing, current.is_empty()) {
        (None, _) => RegistryChange::Created,
        (Some(_), true) => RegistryChange::Added,
        (Some(_), false) => RegistryChange::Replaced,
    };
    Ok((doc.to_string(), change))
}

fn is_decl(line: &str, name: &str) -> bool {
    let line = line.trim();
    let line = line
        .strip_prefix("pub ")
        .map(str::trim_start)
        .unwrap_or(line);
    line == format!("mod {name};")
}

/// `main.rs` content with `mod <name>;` declared, or `None` if it already is. The declaration goes
/// after the last top-level `use` statement (multi-line ones included), else at the top.
pub fn with_mod_decl(source: &str, name: &str) -> Option<String> {
    if source.lines().any(|line| is_decl(line, name)) {
        return None;
    }
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let mut last_use_end = None;
    let mut in_use = false;
    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_end();
        if in_use {
            if trimmed.ends_with(';') {
                in_use = false;
                last_use_end = Some(i);
            }
        } else if line.starts_with("use ") || line.starts_with("pub use ") {
            if trimmed.ends_with(';') {
                last_use_end = Some(i);
            } else {
                in_use = true;
            }
        }
    }
    let decl = format!("mod {name};\n");
    let mut out = String::new();
    match last_use_end {
        Some(end) => {
            for (i, line) in lines.iter().enumerate() {
                out.push_str(line);
                if i == end {
                    if !line.ends_with('\n') {
                        out.push('\n');
                    }
                    out.push('\n');
                    out.push_str(&decl);
                    if lines.get(i + 1).is_some_and(|next| !next.trim().is_empty()) {
                        out.push('\n');
                    }
                }
            }
        }
        None => {
            out.push_str(&decl);
            out.push('\n');
            out.push_str(source);
        }
    }
    Some(out)
}

/// What linking the theme did to `main.rs`.
#[derive(Debug, PartialEq, Eq)]
pub enum LinkEdit {
    /// The theme is already referenced.
    Present,
    Inserted(String),
    /// No `rsx! {` block that ends its line was found to put the link in.
    NotFound,
}

/// The line the docs tell users to add.
pub fn theme_link_line(href: &str) -> String {
    format!(r#"document::Link {{ rel: "stylesheet", href: asset!("{href}") }}"#)
}

/// `main.rs` content with the theme `document::Link` as the first node of the first `rsx! {` block
/// at or after `fn App` (else the first `rsx! {` in the file).
pub fn with_theme_link(source: &str, href: &str) -> LinkEdit {
    if source.contains(THEME_FILE) {
        return LinkEdit::Present;
    }
    let lines: Vec<&str> = source.split_inclusive('\n').collect();
    let start = lines
        .iter()
        .position(|l| l.trim_start().starts_with("fn App") || l.contains(" fn App"))
        .unwrap_or(0);
    let is_open = |line: &str| {
        let t = line.trim_end();
        t.trim_start().contains("rsx!")
            && t.ends_with('{')
            && t.trim_end_matches('{').trim_end().ends_with("rsx!")
    };
    let Some(at) = (start..lines.len()).find(|&i| is_open(lines[i])) else {
        return LinkEdit::NotFound;
    };
    let indent: String = lines[at]
        .chars()
        .take_while(|c| c.is_whitespace() && *c != '\n')
        .collect();
    let mut out = String::new();
    for (i, line) in lines.iter().enumerate() {
        out.push_str(line);
        if i == at {
            if !line.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&format!("{indent}    {}\n", theme_link_line(href)));
        }
    }
    LinkEdit::Inserted(out)
}

/// The `asset!` path of `<assets_root>/dx-components-theme.css`, if the assets directory is inside the crate.
pub fn theme_href(crate_root: &Path, assets_root: &Path) -> Option<String> {
    let rel = assets_root.strip_prefix(crate_root).ok()?;
    let mut href = String::from("/");
    for part in rel.components() {
        href.push_str(&part.as_os_str().to_string_lossy());
        href.push('/');
    }
    href.push_str(THEME_FILE);
    Some(href)
}

/// Options of `init`.
pub struct InitOptions {
    pub registry: RegistryArgs,
    pub force: bool,
    pub no_theme: bool,
}

fn write_if_changed(path: &Path, old: Option<&str>, new: &str) -> Result<bool> {
    if old == Some(new) {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, new).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(true)
}

/// Run `init` against the crate containing `start`. Returns the report lines (also printed).
pub fn run(
    start: &Path,
    registries: &mut Registries,
    options: &InitOptions,
) -> Result<Vec<String>> {
    let project = Project::discover(start)?;
    let mut report = Vec::new();
    let root = &project.crate_root;

    // 1. Decide the Dioxus.toml content (pure), so the effective registry is known before any write.
    let toml_path: PathBuf = project
        .config_path
        .clone()
        .unwrap_or_else(|| root.join("Dioxus.toml"));
    let old_toml = project
        .config_path
        .as_ref()
        .map(std::fs::read_to_string)
        .transpose()?;
    let (new_toml, change) = merge_registry(
        old_toml.as_deref(),
        &desired_registry(&options.registry),
        options.force,
    )?;
    let effective = parse_config(&new_toml)?;

    // 2. Read the theme from the effective registry before writing anything.
    let theme = if options.no_theme {
        None
    } else {
        let source = resolve_source(
            &RegistryArgs::default(),
            &effective.registry,
            toml_path.parent(),
        );
        let registry = registries.get(&source)?;
        registry.components.iter().find_map(|c| {
            let asset = c
                .global_assets
                .iter()
                .find(|a| Path::new(a).file_name().is_some_and(|n| n == THEME_FILE))?;
            let mut only_theme = c.clone();
            only_theme.component.global_assets = vec![asset.clone()];
            Some(only_theme)
        })
    };

    // 3. Write.
    let toml_shown = toml_path
        .strip_prefix(root)
        .unwrap_or(&toml_path)
        .display()
        .to_string();
    match change {
        RegistryChange::Unchanged => report.push(format!("unchanged  {toml_shown}: [components.registry] already set")),
        RegistryChange::Differs(current) => report.push(format!(
            "left alone {toml_shown}: [components.registry] is already {current:?}; pass --force to replace it"
        )),
        RegistryChange::Created | RegistryChange::Added | RegistryChange::Replaced => {
            write_if_changed(&toml_path, old_toml.as_deref(), &new_toml)?;
            let verb = match change {
                RegistryChange::Created => "created",
                RegistryChange::Added => "updated",
                _ => "replaced",
            };
            report.push(format!("{verb:<10} {toml_shown}: [components.registry]"));
        }
    }

    // Re-read so components_dir / asset_dir from the (possibly new) Dioxus.toml apply.
    let project = Project::discover(start)?;
    let components_root = project.components_root(None);
    let shown = |p: &Path| p.strip_prefix(root).unwrap_or(p).display().to_string();
    report.push(if ensure_components_module(&components_root)? {
        format!("created    {}", shown(&components_root.join("mod.rs")))
    } else {
        format!("unchanged  {}", shown(&components_root.join("mod.rs")))
    });

    let main_rs = root.join("src").join("main.rs");
    let module_name = components_root
        .strip_prefix(root.join("src"))
        .ok()
        .filter(|rel| rel.components().count() == 1)
        .map(|rel| rel.to_string_lossy().into_owned());
    let mut main_src = std::fs::read_to_string(&main_rs).ok();
    match (&main_src, &module_name) {
        (None, _) => report.push("left alone src/main.rs: not found; declare the components module in your crate root yourself".into()),
        (_, None) => report.push(format!(
            "left alone src/main.rs: {} is not directly under src/; declare the module with #[path] yourself",
            shown(&components_root)
        )),
        (Some(src), Some(name)) => match with_mod_decl(src, name) {
            None => report.push(format!("unchanged  src/main.rs: `mod {name};` already declared")),
            Some(updated) => {
                write_if_changed(&main_rs, Some(src), &updated)?;
                main_src = Some(updated);
                report.push(format!("updated    src/main.rs: added `mod {name};`"));
            }
        },
    }

    if let Some(theme) = theme {
        let assets_root = project.assets_root(None);
        for outcome in copy_global_assets(&assets_root, &theme, options.force, None)? {
            report.push(match outcome {
                AssetOutcome::Copied(p) => format!("created    {}", shown(&p)),
                AssetOutcome::Unchanged(p) => format!("unchanged  {}", shown(&p)),
                AssetOutcome::Kept(p) => format!("left alone {}: differs from the registry's copy (your edits); --force replaces it", shown(&p)),
            });
        }
        let href = theme_href(root, &assets_root);
        match (&main_src, href) {
            (Some(src), Some(href)) => match with_theme_link(src, &href) {
                LinkEdit::Present => report.push("unchanged  src/main.rs: theme already linked".into()),
                LinkEdit::Inserted(updated) => {
                    write_if_changed(&main_rs, Some(src), &updated)?;
                    report.push(format!("updated    src/main.rs: linked the theme stylesheet ({href})"));
                }
                LinkEdit::NotFound => report.push(format!(
                    "left alone src/main.rs: no `rsx! {{` block to put the link in; add `{}` to your root component",
                    theme_link_line(&href)
                )),
            },
            (None, _) => {}
            (_, None) => report.push(format!(
                "left alone src/main.rs: the assets directory {} is outside the crate; link the stylesheet yourself",
                assets_root.display()
            )),
        }
    }

    for line in &report {
        println!("{line}");
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::test_support::write_component;

    fn git(url: &str) -> RegistryConfig {
        RegistryConfig {
            git: Some(url.into()),
            ..Default::default()
        }
    }

    #[test]
    fn creates_dioxus_toml_with_exactly_the_registry_table() {
        let (text, change) = merge_registry(None, &git("https://x/y"), false).unwrap();
        assert_eq!(change, RegistryChange::Created);
        assert_eq!(text, "[components.registry]\ngit = \"https://x/y\"\n");
    }

    #[test]
    fn merges_into_an_existing_file_keeping_everything_else() {
        let before = "# my app\n[application]\nname = \"a\" # keep\n\n[web.app]\ntitle = \"T\"\n";
        let (text, change) = merge_registry(Some(before), &git("https://x/y"), false).unwrap();
        assert_eq!(change, RegistryChange::Added);
        assert!(text.starts_with(before), "{text}");
        assert!(
            text.ends_with("[components.registry]\ngit = \"https://x/y\"\n"),
            "{text}"
        );
        // Idempotent.
        let (again, change) = merge_registry(Some(&text), &git("https://x/y"), false).unwrap();
        assert_eq!(change, RegistryChange::Unchanged);
        assert_eq!(again, text);
    }

    #[test]
    fn merges_into_an_existing_components_table() {
        let before = "[components]\ncomponents_dir = \"src/ui\"\n";
        let (text, _) = merge_registry(Some(before), &git("https://x/y"), false).unwrap();
        let cfg = parse_config(&text).unwrap();
        assert_eq!(cfg.registry.git.as_deref(), Some("https://x/y"));
        assert_eq!(cfg.components_dir, Some(PathBuf::from("src/ui")));
    }

    #[test]
    fn a_different_registry_is_kept_unless_forced() {
        let before = "[components.registry]\ngit = \"https://other/o\"\nrev = \"v1\"\n";
        let (text, change) = merge_registry(Some(before), &git("https://x/y"), false).unwrap();
        assert!(matches!(change, RegistryChange::Differs(_)));
        assert_eq!(text, before);
        let (text, change) = merge_registry(Some(before), &git("https://x/y"), true).unwrap();
        assert_eq!(change, RegistryChange::Replaced);
        let cfg = parse_config(&text).unwrap();
        assert_eq!(
            cfg.registry,
            git("https://x/y"),
            "the old rev must not survive: {text}"
        );
    }

    #[test]
    fn mod_decl_goes_after_the_last_use_and_only_once() {
        let src = "use dioxus::prelude::*;\n\nfn main() {}\n";
        let out = with_mod_decl(src, "components").unwrap();
        assert_eq!(
            out,
            "use dioxus::prelude::*;\n\nmod components;\n\nfn main() {}\n"
        );
        assert_eq!(with_mod_decl(&out, "components"), None);
        // A multi-line use statement is not split.
        let multi = "use a::{\n    b,\n    c,\n};\nfn main() {}\n";
        assert_eq!(
            with_mod_decl(multi, "components").unwrap(),
            "use a::{\n    b,\n    c,\n};\n\nmod components;\n\nfn main() {}\n"
        );
        // No use at all, and an existing `pub mod`.
        assert_eq!(
            with_mod_decl("fn main() {}\n", "components").unwrap(),
            "mod components;\n\nfn main() {}\n"
        );
        assert_eq!(with_mod_decl("pub mod components;\n", "components"), None);
        // The starter template shape: the comment above the declaration is left where it is.
        let starter = "use dioxus::prelude::*;\n\n// c\nmod components;\n";
        assert_eq!(with_mod_decl(starter, "components"), None);
    }

    #[test]
    fn theme_link_is_the_first_node_of_app_rsx_and_only_once() {
        let src = "fn main() { dioxus::launch(App); }\n\n#[component]\nfn App() -> Element {\n    rsx! {\n        main { h1 { \"x\" } }\n    }\n}\n";
        let LinkEdit::Inserted(out) = with_theme_link(src, "/assets/dx-components-theme.css")
        else {
            panic!()
        };
        assert!(out.contains("    rsx! {\n        document::Link { rel: \"stylesheet\", href: asset!(\"/assets/dx-components-theme.css\") }\n        main"), "{out}");
        assert_eq!(
            with_theme_link(&out, "/assets/dx-components-theme.css"),
            LinkEdit::Present
        );
        // A one-line rsx is not rewritten: the user is told instead.
        assert_eq!(
            with_theme_link("fn App() -> Element { rsx! { div {} } }\n", "/assets/t.css"),
            LinkEdit::NotFound
        );
    }

    #[test]
    fn theme_href_follows_the_assets_dir() {
        assert_eq!(
            theme_href(Path::new("/a"), Path::new("/a/assets")).unwrap(),
            "/assets/dx-components-theme.css"
        );
        assert_eq!(
            theme_href(Path::new("/a"), Path::new("/a/static/x")).unwrap(),
            "/static/x/dx-components-theme.css"
        );
        assert_eq!(theme_href(Path::new("/a"), Path::new("/b/assets")), None);
    }

    /// A tiny Dioxus app and a local registry carrying a theme.
    fn fixtures(tmp: &Path) -> (PathBuf, RegistryArgs) {
        let reg = tmp.join("reg");
        write_component(&reg, ".", r#"{ "name": "r", "members": ["button"] }"#);
        write_component(
            &reg,
            "button",
            r#"{ "name": "button", "globalAssets": ["../dx-components-theme.css"] }"#,
        );
        std::fs::write(reg.join("dx-components-theme.css"), "/* theme */").unwrap();
        let app = tmp.join("app");
        std::fs::create_dir_all(app.join("src")).unwrap();
        std::fs::write(app.join("Cargo.toml"), "[package]\nname = \"app\"\n").unwrap();
        std::fs::write(
            app.join("src/main.rs"),
            "use dioxus::prelude::*;\n\nfn main() {\n    dioxus::launch(App);\n}\n\n#[component]\nfn App() -> Element {\n    rsx! {\n        h1 { \"hi\" }\n    }\n}\n",
        )
        .unwrap();
        let args = RegistryArgs {
            path: Some(reg),
            ..Default::default()
        };
        (app, args)
    }

    fn snapshot(app: &Path) -> Vec<(String, String)> {
        let mut files = Vec::new();
        let mut stack = vec![app.to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else {
                    files.push((
                        path.strip_prefix(app)
                            .unwrap()
                            .to_string_lossy()
                            .into_owned(),
                        std::fs::read_to_string(&path).unwrap(),
                    ));
                }
            }
        }
        files.sort();
        files
    }

    #[test]
    fn init_sets_the_app_up_and_is_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let (app, registry) = fixtures(tmp.path());
        let mut registries = Registries::new(tmp.path().join("cache"));
        let options = InitOptions {
            registry,
            force: false,
            no_theme: false,
        };

        let first = run(&app, &mut registries, &options).unwrap();
        assert!(
            first.iter().all(|l| !l.starts_with("left alone")),
            "{first:?}"
        );
        let after_first = snapshot(&app);
        let names: Vec<_> = after_first.iter().map(|(n, _)| n.as_str()).collect();
        assert_eq!(
            names,
            [
                "Cargo.toml",
                "Dioxus.toml",
                "assets/dx-components-theme.css",
                "src/components/mod.rs",
                "src/main.rs"
            ]
        );
        let main = &after_first
            .iter()
            .find(|(n, _)| n == "src/main.rs")
            .unwrap()
            .1;
        assert!(main.contains("mod components;"));
        assert!(main.contains("asset!(\"/assets/dx-components-theme.css\")"));
        let dioxus = &after_first
            .iter()
            .find(|(n, _)| n == "Dioxus.toml")
            .unwrap()
            .1;
        assert!(
            dioxus.starts_with("[components.registry]\npath = "),
            "{dioxus}"
        );

        let second = run(&app, &mut registries, &options).unwrap();
        assert!(
            second.iter().all(|l| l.starts_with("unchanged")),
            "{second:?}"
        );
        assert_eq!(
            snapshot(&app),
            after_first,
            "a second init must change nothing"
        );
    }

    #[test]
    fn init_with_no_theme_only_does_the_local_steps() {
        let tmp = tempfile::tempdir().unwrap();
        let (app, registry) = fixtures(tmp.path());
        // The registry is never read: pointing at a directory that does not exist must not matter.
        let registry = RegistryArgs {
            path: Some(tmp.path().join("nope")),
            ..registry
        };
        let mut registries = Registries::new(tmp.path().join("cache"));
        run(
            &app,
            &mut registries,
            &InitOptions {
                registry,
                force: false,
                no_theme: true,
            },
        )
        .unwrap();
        assert!(!app.join("assets").exists());
        assert!(std::fs::read_to_string(app.join("src/main.rs"))
            .unwrap()
            .contains("mod components;"));
    }
}

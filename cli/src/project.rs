//! Locating the user's crate and reading its `Dioxus.toml`.
//!
//! Replaces dx's `Workspace` (cargo-metadata based `find_main_package`, `load_dioxus_config`,
//! `crate_root_from_path`) from dioxus-cli 0.7.9 (`MIT OR Apache-2.0`) with a plain directory walk:
//! the crate root is the nearest ancestor holding a `Cargo.toml`, and `Dioxus.toml` / `dioxus.toml`
//! is searched from there up to the workspace root, as dx does.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use toml_edit::{DocumentMut, Item};

/// How many parent folders are searched for a `Cargo.toml` (dx uses the same bound).
const MAX_ANCESTORS: usize = 10;

/// The registry named by `[components.registry]` in `Dioxus.toml`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegistryConfig {
    pub git: Option<String>,
    pub rev: Option<String>,
    pub path: Option<String>,
}

impl RegistryConfig {
    pub fn is_empty(&self) -> bool {
        self.git.is_none() && self.rev.is_none() && self.path.is_none()
    }
}

/// The parts of `Dioxus.toml` this tool reads.
#[derive(Clone, Debug, Default)]
pub struct DioxusConfig {
    /// `[components.registry]`.
    pub registry: RegistryConfig,
    /// `[components] components_dir`.
    pub components_dir: Option<PathBuf>,
    /// `[application] asset_dir`.
    pub asset_dir: Option<PathBuf>,
}

/// A user's crate plus its (possibly absent) `Dioxus.toml`.
#[derive(Clone, Debug)]
pub struct Project {
    pub crate_root: PathBuf,
    /// The `Dioxus.toml` that applies, if one exists between the crate and the workspace root.
    pub config_path: Option<PathBuf>,
    pub config: DioxusConfig,
}

impl Project {
    /// Find the crate containing `start` and load its configuration.
    pub fn discover(start: &Path) -> Result<Self> {
        let crate_root = crate_root_from(start)?;
        let config_path = find_dioxus_toml(&crate_root);
        let config = match &config_path {
            Some(path) => {
                let text = std::fs::read_to_string(path)
                    .with_context(|| format!("Failed to read {}", path.display()))?;
                parse_config(&text)
                    .with_context(|| format!("Failed to parse {}", path.display()))?
            }
            None => DioxusConfig::default(),
        };
        Ok(Self {
            crate_root,
            config_path,
            config,
        })
    }

    /// The components module directory: `--module-path`, else `[components] components_dir`, else
    /// `src/components`.
    pub fn components_root(&self, module_path: Option<&Path>) -> PathBuf {
        if let Some(module_path) = module_path {
            return module_path.to_path_buf();
        }
        match &self.config.components_dir {
            Some(dir) => self.crate_root.join(dir),
            None => self.crate_root.join("src").join("components"),
        }
    }

    /// The global assets directory: `--global-assets-path`, else `[application] asset_dir`, else `assets`.
    pub fn assets_root(&self, assets_path: Option<&Path>) -> PathBuf {
        if let Some(assets_path) = assets_path {
            return assets_path.to_path_buf();
        }
        match &self.config.asset_dir {
            Some(dir) => self.crate_root.join(dir),
            None => self.crate_root.join("assets"),
        }
    }
}

/// The nearest ancestor of `start` (inclusive) with a `Cargo.toml` that declares a `[package]`.
pub fn crate_root_from(start: &Path) -> Result<PathBuf> {
    let start = canonical(start)?;
    let mut dir = start.as_path();
    for _ in 0..MAX_ANCESTORS {
        let manifest = dir.join("Cargo.toml");
        if manifest.is_file() {
            let text = std::fs::read_to_string(&manifest)
                .with_context(|| format!("Failed to read {}", manifest.display()))?;
            if text
                .parse::<DocumentMut>()
                .is_ok_and(|doc| doc.contains_key("package"))
            {
                return Ok(dir.to_path_buf());
            }
            // A virtual workspace manifest: the user is above their crate.
            if text.contains("[workspace]") {
                bail!(
                    "{} is a workspace manifest without a [package]; run this from inside the app's crate directory",
                    manifest.display()
                );
            }
        }
        match dir.parent() {
            Some(parent) => dir = parent,
            None => break,
        }
    }
    bail!(
        "No Cargo.toml found in {} or its {} parent directories; run this from inside a Dioxus app",
        start.display(),
        MAX_ANCESTORS
    )
}

/// `Dioxus.toml` (or `dioxus.toml`) in the crate root or any ancestor up to and including the
/// workspace root.
pub fn find_dioxus_toml(crate_root: &Path) -> Option<PathBuf> {
    let mut dir = crate_root;
    for _ in 0..MAX_ANCESTORS {
        if let Some(found) = ["Dioxus.toml", "dioxus.toml"]
            .into_iter()
            .map(|file| dir.join(file))
            .find(|path| path.is_file())
        {
            return Some(found);
        }
        // The workspace root is the last place dx looks.
        let manifest = dir.join("Cargo.toml");
        if std::fs::read_to_string(&manifest).is_ok_and(|text| text.contains("[workspace]")) {
            return None;
        }
        dir = dir.parent()?;
    }
    None
}

fn canonical(path: &Path) -> Result<PathBuf> {
    std::fs::canonicalize(path).with_context(|| format!("Failed to resolve {}", path.display()))
}

/// Read the `[components]` and `[application]` keys this tool uses.
pub fn parse_config(text: &str) -> Result<DioxusConfig> {
    let doc: DocumentMut = text.parse()?;
    let str_at = |table: &str, sub: Option<&str>, key: &str| -> Option<String> {
        let mut item: &Item = doc.get(table)?;
        if let Some(sub) = sub {
            item = item.get(sub)?;
        }
        item.get(key)?.as_str().map(str::to_owned)
    };
    Ok(DioxusConfig {
        registry: RegistryConfig {
            git: str_at("components", Some("registry"), "git"),
            rev: str_at("components", Some("registry"), "rev"),
            path: str_at("components", Some("registry"), "path"),
        },
        components_dir: str_at("components", None, "components_dir").map(PathBuf::from),
        asset_dir: str_at("application", None, "asset_dir").map(PathBuf::from),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_registry_and_dirs() {
        let cfg = parse_config(
            "[application]\nasset_dir = \"static\"\n[components]\ncomponents_dir = \"src/ui\"\n[components.registry]\ngit = \"https://x/y\"\nrev = \"v1\"\n",
        )
        .unwrap();
        assert_eq!(cfg.registry.git.as_deref(), Some("https://x/y"));
        assert_eq!(cfg.registry.rev.as_deref(), Some("v1"));
        assert_eq!(cfg.components_dir, Some(PathBuf::from("src/ui")));
        assert_eq!(cfg.asset_dir, Some(PathBuf::from("static")));
    }

    #[test]
    fn missing_keys_are_none() {
        let cfg = parse_config("[web.app]\ntitle = \"x\"\n").unwrap();
        assert!(cfg.registry.is_empty());
        assert!(cfg.components_dir.is_none());
    }

    #[test]
    fn dioxus_toml_is_found_in_workspace_ancestor_only_up_to_the_root() {
        let tmp = tempfile::tempdir().unwrap();
        let ws = tmp.path().join("ws");
        let app = ws.join("app");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::write(ws.join("Cargo.toml"), "[workspace]\nmembers = [\"app\"]\n").unwrap();
        std::fs::write(app.join("Cargo.toml"), "[package]\nname = \"app\"\n").unwrap();
        // A Dioxus.toml ABOVE the workspace root must not apply.
        std::fs::write(tmp.path().join("Dioxus.toml"), "").unwrap();
        assert_eq!(find_dioxus_toml(&app), None);
        std::fs::write(ws.join("Dioxus.toml"), "").unwrap();
        assert_eq!(find_dioxus_toml(&app), Some(ws.join("Dioxus.toml")));
    }

    #[test]
    fn crate_root_walks_up_from_a_subdirectory() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(tmp.path().join("src/deep")).unwrap();
        std::fs::write(tmp.path().join("Cargo.toml"), "[package]\nname = \"app\"\n").unwrap();
        let root = crate_root_from(&tmp.path().join("src/deep")).unwrap();
        assert_eq!(root, std::fs::canonicalize(tmp.path()).unwrap());
    }
}

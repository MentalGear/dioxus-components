//! The record of what `add` and `update` installed: a hash per installed file.
//!
//! New code, not part of dx. `update` must tell "the registry changed this file" from "you edited this
//! file". Without a baseline both look the same (the file differs from the registry), so `add` and
//! `update` record the hash of every file as they wrote it in `<components dir>/.shadcn-dioxus.json`
//! (components) and, for global assets such as the theme, by file name. A file whose current hash equals
//! the recorded one is untouched by you and may be replaced; any other difference is a local edit and is
//! never overwritten without `--force`. Components installed without this tool (`dx components add`) or
//! before this file existed have no baseline: their files are compared with the registry only, and a
//! difference is treated as an edit.
//!
//! The hash is FNV-1a (64 bit): it detects edits, it is not a security boundary.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The file name inside the components directory.
pub const LOCK_FILE: &str = ".shadcn-dioxus.json";

pub fn hash_bytes(bytes: &[u8]) -> String {
    format!("{:016x}", crate::registry::fnv1a(bytes))
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Lock {
    /// component name -> (path relative to the component directory -> hash)
    #[serde(default)]
    pub components: BTreeMap<String, BTreeMap<String, String>>,
    /// global asset file name -> hash
    #[serde(default)]
    pub assets: BTreeMap<String, String>,
}

impl Lock {
    pub fn path(components_root: &Path) -> PathBuf {
        components_root.join(LOCK_FILE)
    }

    /// A missing file is an empty lock; an unreadable one is an error rather than a silent reset (which
    /// would turn every edit into "unknown").
    pub fn load(components_root: &Path) -> Result<Self> {
        let path = Self::path(components_root);
        match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text)
                .with_context(|| format!("Failed to parse {}", path.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(error) => Err(error).with_context(|| format!("Failed to read {}", path.display())),
        }
    }

    pub fn save(&self, components_root: &Path) -> Result<()> {
        let path = Self::path(components_root);
        if self.components.is_empty() && self.assets.is_empty() && !path.exists() {
            return Ok(());
        }
        std::fs::create_dir_all(components_root)?;
        let mut text = serde_json::to_string_pretty(self)?;
        text.push('\n');
        std::fs::write(&path, text).with_context(|| format!("Failed to write {}", path.display()))
    }

    pub fn component(&self, name: &str) -> Option<&BTreeMap<String, String>> {
        self.components.get(name)
    }

    pub fn record_component(&mut self, name: &str, files: &BTreeMap<String, Vec<u8>>) {
        self.components.insert(
            name.to_owned(),
            files
                .iter()
                .map(|(rel, bytes)| (rel.clone(), hash_bytes(bytes)))
                .collect(),
        );
    }

    pub fn record_asset(&mut self, file_name: &str, bytes: &[u8]) {
        self.assets.insert(file_name.to_owned(), hash_bytes(bytes));
    }

    /// Is `current` exactly what this tool wrote for the asset?
    pub fn asset_untouched(&self, file_name: &str, current: &[u8]) -> bool {
        self.assets
            .get(file_name)
            .is_some_and(|h| *h == hash_bytes(current))
    }
}

/// Every file under `dir` as `relative/path -> bytes` (paths use `/`).
pub fn read_tree(dir: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    fn walk(root: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) -> Result<()> {
        for entry in
            std::fs::read_dir(dir).with_context(|| format!("Failed to read {}", dir.display()))?
        {
            let path = entry?.path();
            if path.is_dir() {
                walk(root, &path, out)?;
            } else {
                let rel = path
                    .strip_prefix(root)?
                    .components()
                    .map(|c| c.as_os_str().to_string_lossy().into_owned())
                    .collect::<Vec<_>>()
                    .join("/");
                out.insert(
                    rel,
                    std::fs::read(&path)
                        .with_context(|| format!("Failed to read {}", path.display()))?,
                );
            }
        }
        Ok(())
    }
    let mut out = BTreeMap::new();
    if dir.is_dir() {
        walk(dir, dir, &mut out)?;
    }
    Ok(out)
}

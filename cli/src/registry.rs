//! Registry resolution, the git clone cache and component discovery.
//!
//! Forked from dioxus-cli 0.7.9 `src/cli/component.rs` (`RemoteComponentRegistry`,
//! `ComponentRegistry`, `read_component`, `discover_components`;
//! <https://github.com/DioxusLabs/dioxus>, `MIT OR Apache-2.0`). Differences from dx:
//!   * the default registry is [`DEFAULT_REGISTRY_GIT_URL`] (this repository), not DioxusLabs/components;
//!   * git runs through the `git` binary (cargo needs it for git dependencies anyway) instead of libgit2;
//!   * a clone goes to a temporary directory and is renamed into the cache only once complete, so a
//!     failed or interrupted clone can never leave an empty cache directory that later runs mistake for
//!     a finished registry (dx creates the directory first and has that hole);
//!   * the cache lives in this tool's own directory, not dx's, and is keyed by a stable hash;
//!   * everything is synchronous (no tokio).

use crate::manifest::Component;
use anyhow::{bail, Context, Result};
use clap::Args;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    process::Command,
    rc::Rc,
};

/// The registry installed from when nothing else is named.
///
/// This is the same string as `REGISTRY_GIT_URL` in `preview/src/main.rs` (the docs and copy buttons)
/// and as every `git` in the `component.json` files and the starter template;
/// `scripts/check-registry-url.sh` fails when this constant drifts from it.
pub const DEFAULT_REGISTRY_GIT_URL: &str = "https://github.com/MentalGear/shadcn-dioxus";

/// The `name` of this registry's root `component.json`. A registry with this name that refers to
/// [`DEFAULT_REGISTRY_GIT_URL`] is referring to itself (see `add::dependency_source`).
pub const REGISTRY_NAME: &str = "shadcn-dioxus";

/// Registry options shared by every subcommand that reads a registry.
#[derive(Clone, Debug, Default, Args)]
pub struct RegistryArgs {
    /// Git URL of the component registry (default: the project's `Dioxus.toml` `[components.registry]`,
    /// else https://github.com/MentalGear/shadcn-dioxus)
    #[arg(long)]
    pub git: Option<String>,

    /// Revision (branch, tag or commit) of the registry
    #[arg(long)]
    pub rev: Option<String>,

    /// A local registry directory (a checkout), used instead of a git clone
    #[arg(long)]
    pub path: Option<PathBuf>,
}

/// Where a registry's files come from.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Source {
    /// A local directory.
    Path(PathBuf),
    /// A git repository, cloned into the cache.
    Git { url: String, rev: Option<String> },
}

impl std::fmt::Display for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::Path(path) => write!(f, "{}", path.display()),
            Source::Git { url, rev: None } => write!(f, "{url}"),
            Source::Git {
                url,
                rev: Some(rev),
            } => write!(f, "{url}#{rev}"),
        }
    }
}

/// Pick the registry to use. First match wins:
///   1. `--path`
///   2. `--git` and/or `--rev` (a lone `--rev` applies to the `Dioxus.toml` git URL, else the default)
///   3. `Dioxus.toml` `[components.registry]` (`path`, else `git` + `rev`; relative paths are relative
///      to the directory holding that `Dioxus.toml`)
///   4. [`DEFAULT_REGISTRY_GIT_URL`]
pub fn resolve_source(
    args: &RegistryArgs,
    config: &crate::project::RegistryConfig,
    config_dir: Option<&Path>,
) -> Source {
    if let Some(path) = &args.path {
        return Source::Path(path.clone());
    }
    if args.git.is_some() || args.rev.is_some() {
        let url = args
            .git
            .clone()
            .or_else(|| config.git.clone())
            .unwrap_or_else(|| DEFAULT_REGISTRY_GIT_URL.to_owned());
        return Source::Git {
            url,
            rev: args.rev.clone(),
        };
    }
    if let Some(path) = &config.path {
        let path = PathBuf::from(path);
        return Source::Path(match config_dir {
            Some(dir) if path.is_relative() => dir.join(path),
            _ => path,
        });
    }
    Source::Git {
        url: config
            .git
            .clone()
            .unwrap_or_else(|| DEFAULT_REGISTRY_GIT_URL.to_owned()),
        rev: config.rev.clone(),
    }
}

/// The directory registries are cloned into: `$SHADCN_DIOXUS_HOME/registries`, else the platform cache
/// directory.
pub fn default_cache_root() -> PathBuf {
    if let Some(home) = std::env::var_os("SHADCN_DIOXUS_HOME") {
        return PathBuf::from(home).join("registries");
    }
    dirs::cache_dir()
        .or_else(dirs::home_dir)
        .unwrap_or_else(std::env::temp_dir)
        .join("shadcn-dioxus")
        .join("registries")
}

/// FNV-1a: stable across Rust versions, unlike `DefaultHasher` (which dx keys its cache on).
pub(crate) fn fnv1a(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// The cache directory of one `(url, rev)`. The exact URL text is part of the key, so two spellings of
/// one repository are two clones (as in dx).
pub fn cache_path(cache_root: &Path, url: &str, rev: Option<&str>) -> PathBuf {
    let key = format!("{url}\0{}", rev.unwrap_or(""));
    let slug: String = url
        .trim_end_matches('/')
        .rsplit(['/', ':'])
        .next()
        .unwrap_or("registry")
        .trim_end_matches(".git")
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();
    cache_root.join(format!("{slug}-{:016x}", fnv1a(key.as_bytes())))
}

fn git(dir: Option<&Path>, args: &[&str]) -> Result<String> {
    let mut cmd = Command::new("git");
    if let Some(dir) = dir {
        cmd.arg("-C").arg(dir);
    }
    // Never block on a credential prompt: a private or misspelt URL must fail, not hang.
    let output = cmd
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .context("Failed to run `git` (it must be installed and on PATH)")?;
    if !output.status.success() {
        bail!(
            "`git {}` failed: {}",
            args.join(" "),
            String::from_utf8_lossy(&output.stderr).trim()
        );
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

impl Source {
    /// The directory holding the registry's files, cloning it into `cache_root` first if needed.
    pub fn resolve_in(&self, cache_root: &Path) -> Result<PathBuf> {
        match self {
            Source::Path(path) => {
                if !path.is_dir() {
                    bail!(
                        "Component registry path '{}' is not a directory",
                        path.display()
                    );
                }
                Ok(path.clone())
            }
            Source::Git { url, rev } => {
                let dir = cache_path(cache_root, url, rev.as_deref());
                if !dir.exists() {
                    clone_registry(url, rev.as_deref(), &dir)?;
                }
                Ok(dir)
            }
        }
    }

    /// Fetch the latest changes of a cloned registry (a no-op for a local path).
    pub fn update_in(&self, cache_root: &Path) -> Result<()> {
        let Source::Git { url, rev } = self else {
            return Ok(());
        };
        let dir = self.resolve_in(cache_root)?;
        git(
            Some(&dir),
            &["fetch", "--quiet", "--tags", "--force", "origin"],
        )
        .with_context(|| format!("Failed to update component registry '{url}'"))?;
        match rev {
            Some(rev) => checkout_rev(&dir, url, rev)?,
            None => {
                git(Some(&dir), &["reset", "--quiet", "--hard", "@{upstream}"])?;
            }
        }
        Ok(())
    }
}

fn clone_registry(url: &str, rev: Option<&str>, dest: &Path) -> Result<()> {
    let parent = dest
        .parent()
        .context("registry cache path has no parent directory")?;
    std::fs::create_dir_all(parent)?;
    let tmp = parent.join(format!(
        ".clone-{}-{}",
        std::process::id(),
        dest.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("registry")
    ));
    let _ = std::fs::remove_dir_all(&tmp);
    eprintln!("Downloading {url}...");
    let tmp_str = tmp.to_string_lossy().into_owned();
    let result = git(None, &["clone", "--quiet", url, &tmp_str])
        .with_context(|| format!("Failed to download component registry '{url}'"))
        .and_then(|_| match rev {
            Some(rev) => checkout_rev(&tmp, url, rev),
            None => Ok(()),
        })
        .and_then(|_| {
            std::fs::rename(&tmp, dest).with_context(|| {
                format!(
                    "Failed to move the downloaded registry to {}",
                    dest.display()
                )
            })
        });
    if result.is_err() {
        let _ = std::fs::remove_dir_all(&tmp);
    }
    result
}

fn checkout_rev(dir: &Path, url: &str, rev: &str) -> Result<()> {
    // `origin/<rev>` first: after a fetch a local branch of the same name is the STALE one that the clone
    // created (it only moves on `reset`), so trying the bare name first pinned `update` to old commits.
    // Tags and commit ids have no `origin/` form and fall through to the bare name.
    git(
        Some(dir),
        &["checkout", "--quiet", "--detach", &format!("origin/{rev}")],
    )
    .or_else(|_| git(Some(dir), &["checkout", "--quiet", "--detach", rev]))
    .map(|_| ())
    .with_context(|| format!("Failed to find revision '{rev}' in '{url}'"))
}

/// A component read from a registry, together with the registry it came from.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ResolvedComponent {
    /// The component's directory (canonical).
    pub path: PathBuf,
    /// The registry root (canonical); a component may only copy global assets from inside it.
    pub registry_root: PathBuf,
    pub component: Component,
}

impl std::ops::Deref for ResolvedComponent {
    type Target = Component;
    fn deref(&self) -> &Component {
        &self.component
    }
}

/// Every installable component of one registry.
#[derive(Debug)]
pub struct Registry {
    pub source: Source,
    pub root: PathBuf,
    /// The root `component.json`'s `name`.
    pub name: String,
    pub components: Vec<ResolvedComponent>,
}

impl Registry {
    /// Read the registry at `root`: the root `component.json` and, recursively, its `members`.
    /// Virtual components (those with members) are not installable and are left out.
    pub fn read(source: Source, root: &Path) -> Result<Self> {
        let canonical_root = std::fs::canonicalize(root)
            .with_context(|| format!("Failed to resolve registry {}", root.display()))?;
        let top = read_component(&canonical_root, &canonical_root)?;
        let name = top.component.name.clone();
        let mut queue = member_paths(&top);
        let mut components = vec![top];
        while let Some(path) = queue.pop() {
            let component = read_component(&path, &canonical_root)?;
            queue.extend(member_paths(&component));
            components.push(component);
        }
        components.retain(|c| c.members.is_empty());
        components.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(Self {
            source,
            root: canonical_root,
            name,
            components,
        })
    }

    pub fn find(&self, name: &str) -> Result<&ResolvedComponent> {
        self.components
            .iter()
            .find(|c| c.name == name)
            .with_context(|| format!("Component '{name}' not found in registry {}", self.source))
    }
}

fn member_paths(component: &ResolvedComponent) -> Vec<PathBuf> {
    component
        .members
        .iter()
        .map(|m| component.path.join(m))
        .collect()
}

fn read_component(path: &Path, registry_root: &Path) -> Result<ResolvedComponent> {
    let json_path = path.join("component.json");
    let bytes = std::fs::read(&json_path).with_context(|| {
        format!(
            "Failed to open component manifest at {}",
            json_path.display()
        )
    })?;
    let component = serde_json::from_slice(&bytes)
        .with_context(|| format!("Invalid component manifest {}", json_path.display()))?;
    Ok(ResolvedComponent {
        path: std::fs::canonicalize(path)?,
        registry_root: registry_root.to_path_buf(),
        component,
    })
}

/// Registries already read in this run, so a registry named by many dependencies is read (and, for
/// git, cloned) once.
pub struct Registries {
    cache_root: PathBuf,
    loaded: HashMap<Source, Rc<Registry>>,
}

impl Registries {
    pub fn new(cache_root: PathBuf) -> Self {
        Self {
            cache_root,
            loaded: HashMap::new(),
        }
    }

    pub fn cache_root(&self) -> &Path {
        &self.cache_root
    }

    pub fn get(&mut self, source: &Source) -> Result<Rc<Registry>> {
        if let Some(registry) = self.loaded.get(source) {
            return Ok(registry.clone());
        }
        let root = source.resolve_in(&self.cache_root)?;
        let registry = Rc::new(Registry::read(source.clone(), &root)?);
        self.loaded.insert(source.clone(), registry.clone());
        Ok(registry)
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    use std::path::Path;

    /// Write `<root>/<rel>/component.json` (and an empty `component.rs`) for a test registry.
    pub fn write_component(root: &Path, rel: &str, json: &str) {
        let dir = if rel == "." {
            root.to_path_buf()
        } else {
            root.join(rel)
        };
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("component.json"), json).unwrap();
        std::fs::write(dir.join("component.rs"), format!("// {rel}\n")).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::project::RegistryConfig;

    fn cfg(git: Option<&str>, rev: Option<&str>, path: Option<&str>) -> RegistryConfig {
        RegistryConfig {
            git: git.map(Into::into),
            rev: rev.map(Into::into),
            path: path.map(Into::into),
        }
    }

    fn git_src(url: &str, rev: Option<&str>) -> Source {
        Source::Git {
            url: url.into(),
            rev: rev.map(Into::into),
        }
    }

    #[test]
    fn resolution_order_default_config_cli() {
        let none = RegistryArgs::default();
        // 4. nothing named: ours.
        assert_eq!(
            resolve_source(&none, &cfg(None, None, None), None),
            git_src(DEFAULT_REGISTRY_GIT_URL, None)
        );
        // 3. Dioxus.toml git + rev.
        assert_eq!(
            resolve_source(&none, &cfg(Some("https://c/c"), Some("v1"), None), None),
            git_src("https://c/c", Some("v1"))
        );
        // 3. Dioxus.toml path wins over its git, relative to the config's directory.
        assert_eq!(
            resolve_source(
                &none,
                &cfg(Some("https://c/c"), None, Some("reg")),
                Some(Path::new("/app"))
            ),
            Source::Path(PathBuf::from("/app/reg"))
        );
        // 2. --git replaces the config; the config's rev is not inherited by a different repository.
        let git = RegistryArgs {
            git: Some("https://cli/cli".into()),
            ..Default::default()
        };
        assert_eq!(
            resolve_source(
                &git,
                &cfg(Some("https://c/c"), Some("v1"), Some("reg")),
                None
            ),
            git_src("https://cli/cli", None)
        );
        // 2. a lone --rev pins the config's (else the default) repository.
        let rev = RegistryArgs {
            rev: Some("abc".into()),
            ..Default::default()
        };
        assert_eq!(
            resolve_source(&rev, &cfg(Some("https://c/c"), None, None), None),
            git_src("https://c/c", Some("abc"))
        );
        assert_eq!(
            resolve_source(&rev, &cfg(None, None, None), None),
            git_src(DEFAULT_REGISTRY_GIT_URL, Some("abc"))
        );
        // 1. --path beats everything.
        let path = RegistryArgs {
            path: Some("/x".into()),
            git: Some("https://cli/cli".into()),
            ..Default::default()
        };
        assert_eq!(
            resolve_source(&path, &cfg(Some("https://c/c"), None, None), None),
            Source::Path(PathBuf::from("/x"))
        );
    }

    #[test]
    fn cache_path_is_stable_and_keyed_on_url_and_rev() {
        let root = Path::new("/c");
        let a = cache_path(root, "https://github.com/MentalGear/shadcn-dioxus", None);
        assert_eq!(
            a,
            cache_path(root, "https://github.com/MentalGear/shadcn-dioxus", None)
        );
        assert_ne!(
            a,
            cache_path(
                root,
                "https://github.com/MentalGear/shadcn-dioxus",
                Some("v1")
            )
        );
        assert_ne!(
            a,
            cache_path(
                root,
                "https://github.com/MentalGear/shadcn-dioxus.git",
                None
            )
        );
        assert!(a
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .starts_with("shadcn-dioxus-"));
        // Pinned so an upgrade never silently orphans every user's cache.
        assert_eq!(fnv1a(b"abc"), 0xe71fa2190541574b);
    }

    #[test]
    fn reads_members_and_skips_virtual_components() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path();
        test_support::write_component(root, ".", r#"{ "name": "reg", "members": ["c/a", "c/b"] }"#);
        test_support::write_component(root, "c/a", r#"{ "name": "a", "description": "A" }"#);
        test_support::write_component(root, "c/b", r#"{ "name": "b" }"#);
        let registry = Registry::read(Source::Path(root.into()), root).unwrap();
        assert_eq!(registry.name, "reg");
        let names: Vec<_> = registry
            .components
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(names, ["a", "b"]);
        assert!(registry.find("zzz").is_err());
    }

    fn git_ok(dir: &Path, args: &[&str]) {
        let out = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args([
                "-c",
                "user.email=t@t",
                "-c",
                "user.name=t",
                "-c",
                "commit.gpgsign=false",
            ])
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    #[test]
    fn git_clone_is_cached_pinned_and_never_left_half_done() {
        let tmp = tempfile::tempdir().unwrap();
        let upstream = tmp.path().join("up");
        test_support::write_component(&upstream, ".", r#"{ "name": "reg", "members": ["a"] }"#);
        test_support::write_component(&upstream, "a", r#"{ "name": "a" }"#);
        git_ok(&upstream, &["init", "--quiet"]);
        git_ok(&upstream, &["add", "."]);
        git_ok(&upstream, &["commit", "--quiet", "-m", "one"]);
        git_ok(&upstream, &["tag", "v1"]);
        test_support::write_component(
            &upstream,
            "a",
            r#"{ "name": "a", "description": "second" }"#,
        );
        git_ok(&upstream, &["commit", "--quiet", "-am", "two"]);

        let cache = tmp.path().join("cache");
        let url = upstream.to_string_lossy().into_owned();
        let mut registries = Registries::new(cache.clone());
        let latest = registries.get(&git_src(&url, None)).unwrap();
        assert_eq!(latest.find("a").unwrap().description, "second");
        let pinned = registries.get(&git_src(&url, Some("v1"))).unwrap();
        assert_eq!(pinned.find("a").unwrap().description, "");

        // A failed download leaves nothing in the cache that a later run could mistake for a clone.
        let bad = git_src(&tmp.path().join("missing").to_string_lossy(), None);
        assert!(registries.get(&bad).is_err());
        let bad_rev = git_src(&url, Some("no-such-rev"));
        assert!(registries.get(&bad_rev).is_err());
        let leftovers: Vec<_> = std::fs::read_dir(&cache)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        assert_eq!(leftovers.len(), 2, "{leftovers:?}");
        assert!(leftovers.iter().all(|n| !n.starts_with(".clone-")));

        // `update` moves an unpinned clone forward.
        test_support::write_component(&upstream, "a", r#"{ "name": "a", "description": "third" }"#);
        git_ok(&upstream, &["commit", "--quiet", "-am", "three"]);
        git_src(&url, None).update_in(&cache).unwrap();
        let fresh = Registries::new(cache).get(&git_src(&url, None)).unwrap();
        assert_eq!(fresh.find("a").unwrap().description, "third");
    }

    #[test]
    fn update_moves_a_branch_pin_forward_and_leaves_tags_and_commits_alone() {
        let tmp = tempfile::tempdir().unwrap();
        let upstream = tmp.path().join("up");
        let desc = |text: &str| format!(r#"{{ "name": "a", "description": "{text}" }}"#);
        test_support::write_component(&upstream, ".", r#"{ "name": "reg", "members": ["a"] }"#);
        test_support::write_component(&upstream, "a", &desc("one"));
        git_ok(&upstream, &["init", "--quiet"]);
        git_ok(&upstream, &["add", "."]);
        git_ok(&upstream, &["commit", "--quiet", "-m", "one"]);
        git_ok(&upstream, &["tag", "v1"]);
        // The default branch is the one a clone also creates locally: the stale-name trap.
        let branch = git(Some(&upstream), &["symbolic-ref", "--short", "HEAD"]).unwrap();
        let sha = git(Some(&upstream), &["rev-parse", "HEAD"]).unwrap();

        let cache = tmp.path().join("cache");
        let url = upstream.to_string_lossy().into_owned();
        let description = |rev: &str| {
            Registries::new(cache.clone())
                .get(&git_src(&url, Some(rev)))
                .unwrap()
                .find("a")
                .unwrap()
                .description
                .clone()
        };
        for rev in [branch.as_str(), "v1", sha.as_str()] {
            assert_eq!(description(rev), "one", "{rev}");
        }

        // Upstream's branch moves on; a stale cache must not mask it after `update`.
        test_support::write_component(&upstream, "a", &desc("two"));
        git_ok(&upstream, &["commit", "--quiet", "-am", "two"]);
        for rev in [branch.as_str(), "v1", sha.as_str()] {
            git_src(&url, Some(rev)).update_in(&cache).unwrap();
        }
        assert_eq!(
            description(&branch),
            "two",
            "the branch pin follows its branch"
        );
        assert_eq!(description("v1"), "one", "a tag stays put");
        assert_eq!(description(&sha), "one", "a commit stays put");
    }
}

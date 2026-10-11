//! `update`: bring installed components up to the registry's current version without losing your edits.
//!
//! New code, not part of dx (`dx components` has no update). What it does, in order:
//!   1. refreshes the registry's cached clone (`git fetch`, then the branch tip or the pinned `--rev`);
//!      a local `--path` registry is read as it is;
//!   2. picks the components: the ones named, or every installed component the registry still has; their
//!      `componentDependencies` come along (installed ones are updated, missing ones are installed);
//!   3. for each installed component, compares every file with the registry's current one and with what
//!      this tool last wrote (`lock.rs`):
//!        - same as the registry: nothing to do;
//!        - same as what was last written (you did not touch it): replaced;
//!        - anything else (you edited it, or there is no record of what was installed): the whole
//!          component is left alone, listed, and the command fails at the end, unless `--force`;
//!        - missing locally: restored; absent from the registry and untouched: deleted; absent from the
//!          registry and yours: kept;
//!   4. global assets (the theme) follow the same rule, so an unedited theme moves with the registry and an
//!      edited one is kept unless `--force`;
//!   5. `cargo add` for the crate dependencies of components that changed or were installed.
//!
//! `--force` replaces each component's directory outright, as `add --force` does.

use crate::add::{
    copy_global_assets, ensure_components_module, plan, with_mod_line, AssetOutcome, ExistsBehavior,
};
use crate::lock::{hash_bytes, read_tree, Lock};
use crate::manifest::CargoDependency;
use crate::project::Project;
use crate::registry::{Registries, ResolvedComponent, Source};
use anyhow::{bail, Context, Result};
use std::collections::BTreeMap;
use std::path::Path;

pub struct UpdateOptions<'a> {
    /// Components to update; empty means every installed component the registry has.
    pub requested: &'a [String],
    pub module_path: Option<&'a Path>,
    pub global_assets_path: Option<&'a Path>,
    pub force: bool,
    pub skip_cargo_add: bool,
}

type Files = BTreeMap<String, Vec<u8>>;

/// What updating one installed component would do to its files.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct FilePlan {
    /// Files to write (new, restored or replaced), `(relative path, bytes)`.
    pub writes: Vec<(String, Vec<u8>)>,
    /// Files the registry dropped and you never touched.
    pub deletes: Vec<String>,
    /// Files you changed (or that cannot be told from a change) and that differ from the registry's.
    pub conflicts: Vec<(String, Conflict)>,
    /// Files that are not in the registry any more but that you edited; kept.
    pub kept: Vec<String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Conflict {
    /// It differs from what this tool installed.
    EditedLocally,
    /// There is no record of what was installed, so a difference may be your edit.
    NoBaseline,
}

impl FilePlan {
    pub fn is_noop(&self) -> bool {
        self.writes.is_empty() && self.deletes.is_empty() && self.conflicts.is_empty()
    }
}

/// The decision for one component, as a pure function of the three versions of its files.
///   * `registry`: what the registry has now;
///   * `current`: what is in the project;
///   * `baseline`: hashes this tool recorded when it last wrote the component, if it ever did.
pub fn plan_files(
    registry: &Files,
    current: &Files,
    baseline: Option<&BTreeMap<String, String>>,
) -> FilePlan {
    let untouched = |rel: &str, bytes: &[u8]| {
        baseline
            .and_then(|b| b.get(rel))
            .is_some_and(|h| *h == hash_bytes(bytes))
    };
    let mut plan = FilePlan::default();
    for (rel, new) in registry {
        match current.get(rel) {
            Some(cur) if cur == new => {}
            Some(cur) if untouched(rel, cur) => plan.writes.push((rel.clone(), new.clone())),
            Some(_) => plan.conflicts.push((
                rel.clone(),
                if baseline.is_some_and(|b| b.contains_key(rel)) {
                    Conflict::EditedLocally
                } else {
                    Conflict::NoBaseline
                },
            )),
            None => plan.writes.push((rel.clone(), new.clone())),
        }
    }
    for (rel, cur) in current {
        if registry.contains_key(rel) {
            continue;
        }
        if untouched(rel, cur) {
            plan.deletes.push(rel.clone());
        } else {
            plan.kept.push(rel.clone());
        }
    }
    plan
}

/// The files `add` would copy for `component` (its directory minus `exclude`).
fn registry_files(component: &ResolvedComponent) -> Result<Files> {
    let mut files = read_tree(&component.path)?;
    files.retain(|rel, _| {
        !component
            .exclude
            .iter()
            .any(|excluded| Path::new(rel).starts_with(excluded))
    });
    Ok(files)
}

fn write_files(dir: &Path, files: &[(String, Vec<u8>)]) -> Result<()> {
    for (rel, bytes) in files {
        let path = dir.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, bytes)
            .with_context(|| format!("Failed to write {}", path.display()))?;
    }
    Ok(())
}

fn print_assets(
    outcomes: Vec<AssetOutcome>,
    reported: &mut std::collections::HashSet<std::path::PathBuf>,
) {
    for outcome in outcomes {
        match outcome {
            AssetOutcome::Copied(path) => {
                if reported.insert(path.clone()) {
                    println!("  updated {}", path.display())
                }
            }
            AssetOutcome::Unchanged(_) => {}
            AssetOutcome::Kept(path) => {
                // Every component lists the shared theme; say so once.
                if reported.insert(path.clone()) {
                    println!(
                        "  kept your modified {} (it differs from the registry's; --force replaces it)",
                        path.display()
                    )
                }
            }
        }
    }
}

/// Is any component installed under this name?
fn installed(components_root: &Path, name: &str) -> bool {
    components_root.join(name).is_dir()
}

pub fn update(
    project: &Project,
    registries: &mut Registries,
    source: &Source,
    options: &UpdateOptions,
) -> Result<()> {
    // The registry's clone first: reading it before this would mask the new commit.
    source.update_in(registries.cache_root())?;
    let registry = registries.get(source)?;
    let components_root = project.components_root(options.module_path);
    let assets_root = project.assets_root(options.global_assets_path);

    let requested: Vec<ResolvedComponent> = if options.requested.is_empty() {
        registry
            .components
            .iter()
            .filter(|c| installed(&components_root, &c.name))
            .cloned()
            .collect()
    } else {
        let mut found = Vec::new();
        for name in options.requested {
            let component = registry.find(name)?;
            if !installed(&components_root, name) {
                bail!(
                    "Component '{name}' is not installed in {} (`shadcn-dioxus add {name}` installs it)",
                    components_root.display()
                );
            }
            found.push(component.clone());
        }
        found
    };
    if requested.is_empty() {
        println!(
            "No installed components of {source} found in {}; nothing to update",
            components_root.display()
        );
        return Ok(());
    }
    let planned = plan(registries, &registry, &requested, ExistsBehavior::Return)?;

    let mut lock = Lock::load(&components_root)?;
    let mut skipped: Vec<String> = Vec::new();
    let mut changed: Vec<&ResolvedComponent> = Vec::new();
    let mut module_checked = false;
    let mut reported_assets = std::collections::HashSet::new();

    for planned in &planned {
        let component = &planned.component;
        let name = component.name.as_str();
        let dest = components_root.join(name);
        let new_files = registry_files(component)?;
        let is_new = !dest.is_dir();
        let mut did_change = false;

        if is_new {
            ensure_components_module(&components_root)?;
            write_files(
                &dest,
                &new_files
                    .iter()
                    .map(|(r, b)| (r.clone(), b.clone()))
                    .collect::<Vec<_>>(),
            )?;
            lock.record_component(name, &new_files);
            println!("Added component '{name}' (needed by a component you have)");
            did_change = true;
        } else {
            let current = read_tree(&dest)?;
            if options.force {
                if current != new_files {
                    std::fs::remove_dir_all(&dest)?;
                    let all: Vec<_> = new_files
                        .iter()
                        .map(|(r, b)| (r.clone(), b.clone()))
                        .collect();
                    write_files(&dest, &all)?;
                    println!("Updated component '{name}' (--force: replaced the whole directory)");
                    did_change = true;
                }
                lock.record_component(name, &new_files);
            } else {
                let file_plan = plan_files(&new_files, &current, lock.component(name));
                if !file_plan.conflicts.is_empty() {
                    println!("Left component '{name}' unchanged: these files differ from the registry's and are yours or unknown:");
                    for (rel, why) in &file_plan.conflicts {
                        let why = match why {
                            Conflict::EditedLocally => "edited since it was installed",
                            Conflict::NoBaseline => {
                                "no record of what was installed, so it may be your edit"
                            }
                        };
                        println!("  {}: {why}", dest.join(rel).display());
                    }
                    skipped.push(name.to_owned());
                    continue;
                }
                if !file_plan.is_noop() {
                    write_files(&dest, &file_plan.writes)?;
                    for rel in &file_plan.deletes {
                        let _ = std::fs::remove_file(dest.join(rel));
                    }
                    let mut touched: Vec<&str> = file_plan
                        .writes
                        .iter()
                        .map(|(r, _)| r.as_str())
                        .chain(file_plan.deletes.iter().map(String::as_str))
                        .collect();
                    touched.sort();
                    println!("Updated component '{name}': {}", touched.join(", "));
                    did_change = true;
                }
                for rel in &file_plan.kept {
                    println!(
                        "  kept {} (not in the registry any more, but you edited it)",
                        dest.join(rel).display()
                    );
                }
                lock.record_component(name, &new_files);
            }
        }

        print_assets(
            copy_global_assets(&assets_root, component, options.force, Some(&mut lock))?,
            &mut reported_assets,
        );

        let mod_rs = components_root.join("mod.rs");
        if !module_checked {
            ensure_components_module(&components_root)?;
            module_checked = true;
        }
        let content = std::fs::read_to_string(&mod_rs)
            .with_context(|| format!("Failed to read {}", mod_rs.display()))?;
        if let Some(updated) = with_mod_line(&content, name) {
            std::fs::write(&mod_rs, updated)
                .with_context(|| format!("Failed to write {}", mod_rs.display()))?;
        }
        if did_change {
            changed.push(component);
        } else if !is_new {
            println!("Component '{name}' is up to date");
        }
    }
    lock.save(&components_root)?;

    if !options.skip_cargo_add {
        let mut deps: Vec<&CargoDependency> = Vec::new();
        for component in &changed {
            for dep in &component.cargo_dependencies {
                if !deps.iter().any(|d| d.add_args() == dep.add_args()) {
                    deps.push(dep);
                }
            }
        }
        crate::add::add_rust_dependencies(&project.crate_root, &deps)?;
    }

    if !skipped.is_empty() {
        bail!(
            "{} component(s) left unchanged because of local edits: {} (--force replaces them)",
            skipped.len(),
            skipped.join(", ")
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::test_support::write_component;

    fn files(entries: &[(&str, &str)]) -> Files {
        entries
            .iter()
            .map(|(r, b)| ((*r).to_owned(), b.as_bytes().to_vec()))
            .collect()
    }

    fn baseline_of(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
        files(entries)
            .iter()
            .map(|(r, b)| (r.clone(), hash_bytes(b)))
            .collect()
    }

    #[test]
    fn untouched_files_follow_the_registry() {
        let old = [("a.rs", "v1"), ("b.css", "v1")];
        let plan = plan_files(
            &files(&[("a.rs", "v2"), ("b.css", "v1")]),
            &files(&old),
            Some(&baseline_of(&old)),
        );
        assert_eq!(plan.writes, vec![("a.rs".to_owned(), b"v2".to_vec())]);
        assert!(plan.conflicts.is_empty() && plan.deletes.is_empty());
    }

    #[test]
    fn an_edited_file_is_a_conflict_not_an_overwrite() {
        let installed = [("a.rs", "v1")];
        let plan = plan_files(
            &files(&[("a.rs", "v2")]),
            &files(&[("a.rs", "mine")]),
            Some(&baseline_of(&installed)),
        );
        assert!(plan.writes.is_empty());
        assert_eq!(
            plan.conflicts,
            vec![("a.rs".to_owned(), Conflict::EditedLocally)]
        );
    }

    #[test]
    fn without_a_baseline_a_difference_is_a_conflict_and_equality_is_not() {
        let plan = plan_files(
            &files(&[("a.rs", "v2"), ("b.rs", "same")]),
            &files(&[("a.rs", "v1"), ("b.rs", "same")]),
            None,
        );
        assert_eq!(
            plan.conflicts,
            vec![("a.rs".to_owned(), Conflict::NoBaseline)]
        );
        assert!(plan.writes.is_empty());
    }

    #[test]
    fn added_removed_and_deleted_files() {
        let old = [("a.rs", "v1"), ("gone.rs", "v1"), ("mine.rs", "v1")];
        let current = files(&[("a.rs", "v1"), ("gone.rs", "v1"), ("mine.rs", "edited")]);
        let plan = plan_files(
            &files(&[("a.rs", "v1"), ("new.rs", "n")]),
            &current,
            Some(&baseline_of(&old)),
        );
        assert_eq!(plan.writes, vec![("new.rs".to_owned(), b"n".to_vec())]);
        assert_eq!(plan.deletes, vec!["gone.rs".to_owned()]);
        assert_eq!(plan.kept, vec!["mine.rs".to_owned()]);
        assert!(plan.conflicts.is_empty());
        // A file you deleted but the registry still ships comes back.
        let restored = plan_files(
            &files(&[("a.rs", "v1")]),
            &files(&[]),
            Some(&baseline_of(&old)),
        );
        assert_eq!(restored.writes.len(), 1);
    }

    #[test]
    fn everything_equal_is_a_noop() {
        let same = [("a.rs", "v1")];
        assert!(plan_files(&files(&same), &files(&same), None).is_noop());
    }

    // ---- end to end over a --path registry -------------------------------------------------------

    fn registry(root: &Path, button_css: &str, theme: &str) {
        write_component(
            root,
            ".",
            r#"{ "name": "r", "members": ["button", "calendar"] }"#,
        );
        write_component(
            root,
            "button",
            r#"{ "name": "button", "exclude": ["component.json"], "componentDependencies": ["calendar"], "globalAssets": ["../theme.css"] }"#,
        );
        write_component(
            root,
            "calendar",
            r#"{ "name": "calendar", "exclude": ["component.json"] }"#,
        );
        std::fs::write(root.join("button/mod.rs"), "mod component;\n").unwrap();
        std::fs::write(root.join("calendar/mod.rs"), "mod component;\n").unwrap();
        std::fs::write(root.join("button/style.css"), button_css).unwrap();
        std::fs::write(root.join("theme.css"), theme).unwrap();
    }

    fn project(root: &Path) -> Project {
        std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"app\"\n").unwrap();
        Project::discover(root).unwrap()
    }

    fn run_add(project: &Project, reg: &Path, cache: &Path, names: &[&str]) {
        let names: Vec<String> = names.iter().map(|s| (*s).to_owned()).collect();
        crate::add::add(
            project,
            &mut Registries::new(cache.to_path_buf()),
            &Source::Path(reg.to_path_buf()),
            &crate::add::AddOptions {
                requested: &names,
                all: false,
                module_path: None,
                global_assets_path: None,
                force: false,
                skip_cargo_add: true,
            },
        )
        .unwrap();
    }

    fn run_update(
        project: &Project,
        reg: &Path,
        cache: &Path,
        names: &[&str],
        force: bool,
    ) -> Result<()> {
        let names: Vec<String> = names.iter().map(|s| (*s).to_owned()).collect();
        update(
            project,
            &mut Registries::new(cache.to_path_buf()),
            &Source::Path(reg.to_path_buf()),
            &UpdateOptions {
                requested: &names,
                module_path: None,
                global_assets_path: None,
                force,
                skip_cargo_add: true,
            },
        )
    }

    fn read(path: &Path) -> String {
        std::fs::read_to_string(path).unwrap()
    }

    #[test]
    fn update_moves_untouched_components_and_their_dependencies() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = tmp.path().join("reg");
        let app = tmp.path().join("app");
        std::fs::create_dir_all(&app).unwrap();
        let cache = tmp.path().join("cache");
        registry(&reg, "v1", "theme v1");
        let project = project(&app);
        run_add(&project, &reg, &cache, &["button"]);
        assert_eq!(read(&app.join("src/components/button/style.css")), "v1");
        assert!(app.join("src/components/.shadcn-dioxus.json").is_file());

        registry(&reg, "v2", "theme v2");
        std::fs::write(reg.join("calendar/component.rs"), "// calendar v2\n").unwrap();
        run_update(&project, &reg, &cache, &["button"], false).unwrap();
        assert_eq!(read(&app.join("src/components/button/style.css")), "v2");
        assert_eq!(
            read(&app.join("src/components/calendar/component.rs")),
            "// calendar v2\n"
        );
        // The theme was never edited, so it moved with the registry.
        assert_eq!(read(&app.join("assets/theme.css")), "theme v2");
        // A second run changes nothing.
        run_update(&project, &reg, &cache, &[], false).unwrap();
    }

    #[test]
    fn update_refuses_edited_components_and_keeps_an_edited_theme() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = tmp.path().join("reg");
        let app = tmp.path().join("app");
        std::fs::create_dir_all(&app).unwrap();
        let cache = tmp.path().join("cache");
        registry(&reg, "v1", "theme v1");
        let project = project(&app);
        run_add(&project, &reg, &cache, &["button"]);

        std::fs::write(app.join("src/components/button/style.css"), "mine").unwrap();
        std::fs::write(app.join("assets/theme.css"), "my theme").unwrap();
        registry(&reg, "v2", "theme v2");
        std::fs::write(reg.join("calendar/component.rs"), "// calendar v2\n").unwrap();

        let error = run_update(&project, &reg, &cache, &[], false)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("button") && error.contains("local edits"),
            "{error}"
        );
        // The edited component is untouched, the clean dependency did update, the theme is yours.
        assert_eq!(read(&app.join("src/components/button/style.css")), "mine");
        assert_eq!(
            read(&app.join("src/components/calendar/component.rs")),
            "// calendar v2\n"
        );
        assert_eq!(read(&app.join("assets/theme.css")), "my theme");

        // --force replaces both.
        run_update(&project, &reg, &cache, &["button"], true).unwrap();
        assert_eq!(read(&app.join("src/components/button/style.css")), "v2");
        assert_eq!(read(&app.join("assets/theme.css")), "theme v2");
    }

    #[test]
    fn update_without_a_record_treats_differences_as_edits() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = tmp.path().join("reg");
        let app = tmp.path().join("app");
        std::fs::create_dir_all(&app).unwrap();
        let cache = tmp.path().join("cache");
        registry(&reg, "v1", "theme v1");
        let project = project(&app);
        run_add(&project, &reg, &cache, &["button"]);
        // As if `dx components add` had installed it: no lock file.
        std::fs::remove_file(app.join("src/components/.shadcn-dioxus.json")).unwrap();
        registry(&reg, "v2", "theme v2");
        assert!(run_update(&project, &reg, &cache, &["button"], false).is_err());
        assert_eq!(read(&app.join("src/components/button/style.css")), "v1");
    }

    #[test]
    fn update_names_must_be_installed_and_known() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = tmp.path().join("reg");
        let app = tmp.path().join("app");
        std::fs::create_dir_all(&app).unwrap();
        let cache = tmp.path().join("cache");
        registry(&reg, "v1", "theme v1");
        let project = project(&app);
        let not_installed = run_update(&project, &reg, &cache, &["button"], false)
            .unwrap_err()
            .to_string();
        assert!(not_installed.contains("not installed"), "{not_installed}");
        let unknown = run_update(&project, &reg, &cache, &["zzz"], false)
            .unwrap_err()
            .to_string();
        assert!(unknown.contains("not found"), "{unknown}");
        // No names and nothing installed is not an error.
        run_update(&project, &reg, &cache, &[], false).unwrap();
    }

    #[test]
    fn update_installs_a_dependency_the_registry_added() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = tmp.path().join("reg");
        let app = tmp.path().join("app");
        std::fs::create_dir_all(&app).unwrap();
        let cache = tmp.path().join("cache");
        registry(&reg, "v1", "theme v1");
        // v1: button has no dependency
        write_component(
            &reg,
            "button",
            r#"{ "name": "button", "exclude": ["component.json"] }"#,
        );
        let project = project(&app);
        run_add(&project, &reg, &cache, &["button"]);
        assert!(!app.join("src/components/calendar").exists());
        // v2: it needs calendar now
        registry(&reg, "v1", "theme v1");
        run_update(&project, &reg, &cache, &["button"], false).unwrap();
        assert!(app.join("src/components/calendar/mod.rs").is_file());
        assert!(read(&app.join("src/components/mod.rs")).contains("pub mod calendar;"));
    }
}

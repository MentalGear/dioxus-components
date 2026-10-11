//! `new`: create an app from the registry's `templates/starter`, without dx or cargo-generate.
//!
//! New code, not part of dx. The template is the one `dx new --template <registry> --subtemplate
//! templates/starter` renders with cargo-generate; this module re-implements the small part of
//! cargo-generate that the template uses, so the two paths produce the same project:
//!   * `*.liquid` files lose the `.liquid` suffix (`Cargo.toml.liquid` -> `Cargo.toml`);
//!   * `{{project-name}}` and `{{authors}}` (plus `crate_name`, `username` and `os-arch`) are replaced in
//!     file contents and in file names; anything else (a filter, a `{% tag %}`, an unknown variable) is an
//!     error naming the file, never a silent copy-through, so the template cannot drift from this renderer;
//!   * the project name is sanitised like cargo-generate does (`My App` -> `my-app`), the directory keeps
//!     the name as typed;
//!   * `authors` is `git config user.name <user.email>`, else `$USER`;
//!   * `cargo-generate.toml` is read for `[template] exclude` and is not copied; a `[hooks]` or
//!     `[placeholders]` table, or `include`/`ignore`, is refused because they are not implemented here;
//!   * `--vcs git` (the default) runs `git init` with no commit, `--vcs none` skips it.
//!
//! The project is rendered completely in memory before the first byte is written, and a directory this
//! command created is removed again if writing fails.

use crate::registry::{Registries, RegistryArgs, Source};
use anyhow::{bail, Context, Result};
use clap::ValueEnum;
use std::path::{Path, PathBuf};
use std::process::Command;
use toml_edit::DocumentMut;

/// Where the starter lives inside a registry.
pub const TEMPLATE_DIR: &str = "templates/starter";

/// The template's cargo-generate configuration; read, never copied.
const TEMPLATE_CONFIG: &str = "cargo-generate.toml";

/// Version control for the new project (the values `dx new --vcs` takes).
#[derive(Clone, Copy, Debug, PartialEq, Eq, ValueEnum)]
pub enum Vcs {
    /// `git init` (no commit)
    Git,
    /// Leave the directory without a repository
    None,
}

pub struct NewOptions<'a> {
    /// The directory to create; its last component is the project name.
    pub target: &'a Path,
    pub registry: &'a RegistryArgs,
    pub vcs: Vcs,
}

// ---------------------------------------------------------------------------------------------------
// Names
// ---------------------------------------------------------------------------------------------------

/// Split into words the way heck 0.5 (cargo-generate's case converter) does: on every non-alphanumeric
/// character, after a lowercase letter that is followed by an uppercase one, and before the last capital
/// of an acronym (`HTTPServer` -> `HTTP`, `Server`).
fn words(input: &str) -> Vec<String> {
    #[derive(Clone, Copy, PartialEq)]
    enum Mode {
        Boundary,
        Lower,
        Upper,
    }
    let mut out = Vec::new();
    for word in input.split(|c: char| !c.is_alphanumeric()) {
        if word.is_empty() {
            continue;
        }
        let mut chars = word.char_indices().peekable();
        let mut init = 0;
        let mut mode = Mode::Boundary;
        while let Some((i, c)) = chars.next() {
            if let Some(&(next_i, next)) = chars.peek() {
                let next_mode = if c.is_lowercase() {
                    Mode::Lower
                } else if c.is_uppercase() {
                    Mode::Upper
                } else {
                    mode
                };
                if next_mode == Mode::Lower && next.is_uppercase() {
                    out.push(word[init..next_i].to_lowercase());
                    init = next_i;
                    mode = Mode::Boundary;
                } else if mode == Mode::Upper && c.is_uppercase() && next.is_lowercase() {
                    out.push(word[init..i].to_lowercase());
                    init = i;
                    mode = Mode::Boundary;
                } else {
                    mode = next_mode;
                }
            } else {
                out.push(word[init..].to_lowercase());
                break;
            }
        }
    }
    out
}

/// cargo-generate's project name: kept when it already is kebab-case or snake_case, else kebab-cased.
pub fn sanitize_project_name(raw: &str) -> String {
    let words = words(raw);
    let kebab = words.join("-");
    let snake = words.join("_");
    if raw == kebab || raw == snake {
        raw.to_owned()
    } else {
        kebab
    }
}

/// cargo-generate's `crate_name`: the project name in snake_case.
pub fn crate_name(project_name: &str) -> String {
    words(project_name).join("_")
}

const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "crate", "else", "enum", "extern", "false", "fn", "for",
    "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
    "self", "Self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where",
    "while", "async", "await", "dyn", "abstract", "become", "box", "do", "final", "macro",
    "override", "priv", "typeof", "unsized", "virtual", "yield", "try",
];

/// Refuse a name cargo would reject, before anything is written (cargo-generate only finds out when it
/// runs `cargo fmt`, after the files exist).
pub fn validate_package_name(name: &str) -> Result<()> {
    let Some(first) = name.chars().next() else {
        bail!("the project name is empty");
    };
    if !(first.is_ascii_alphabetic() || first == '_') {
        bail!(
            "'{name}' is not a valid package name: it must start with a letter (or '_'), not '{first}'"
        );
    }
    if let Some(bad) = name
        .chars()
        .find(|c| !(c.is_ascii_alphanumeric() || *c == '-' || *c == '_'))
    {
        bail!("'{name}' is not a valid package name: '{bad}' is not allowed (use letters, digits, '-' and '_')");
    }
    if KEYWORDS.contains(&name) {
        bail!("'{name}' is not a valid package name: it is a Rust keyword");
    }
    Ok(())
}

/// What cargo-generate puts in `{{authors}}`: git's `user.name`, else `$USER`, then ` <email>` if git has
/// a `user.email`.
pub fn format_authors(name: Option<&str>, email: Option<&str>, user: Option<&str>) -> String {
    let non_empty = |s: Option<&str>| {
        s.map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_owned)
    };
    let name = non_empty(name)
        .or_else(|| non_empty(user))
        .unwrap_or_default();
    match non_empty(email) {
        Some(email) => format!("{name} <{email}>").trim_start().to_owned(),
        None => name,
    }
}

fn git_config(key: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["config", "--get", key])
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

fn current_authors() -> String {
    let user = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .ok();
    format_authors(
        git_config("user.name").as_deref(),
        git_config("user.email").as_deref(),
        user.as_deref(),
    )
}

// ---------------------------------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------------------------------

/// The placeholders a template may use.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Variables {
    pub project_name: String,
    pub crate_name: String,
    pub authors: String,
    pub username: String,
    pub os_arch: String,
}

impl Variables {
    pub fn new(project_name: &str, authors: String) -> Self {
        let username = std::env::var("USER")
            .or_else(|_| std::env::var("USERNAME"))
            .unwrap_or_default();
        Self {
            project_name: project_name.to_owned(),
            crate_name: crate_name(project_name),
            authors,
            username,
            os_arch: format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH),
        }
    }

    fn get(&self, name: &str) -> Option<&str> {
        match name {
            "project-name" => Some(&self.project_name),
            "crate_name" => Some(&self.crate_name),
            "authors" => Some(&self.authors),
            "username" => Some(&self.username),
            "os-arch" => Some(&self.os_arch),
            _ => None,
        }
    }
}

/// Replace every `{{ variable }}` in `text`. A filter, an unknown variable, an unterminated `{{` or a
/// `{% tag %}` is an error: the template is then using more of liquid than this renderer implements.
pub fn render(text: &str, vars: &Variables) -> Result<String> {
    if text.contains("{%") {
        bail!("it uses a liquid `{{% tag %}}`, which `shadcn-dioxus new` does not implement (use `dx new`)");
    }
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after
            .find("}}")
            .context("it has a `{{` without a closing `}}`")?;
        let name = after[..end].trim();
        let value = vars.get(name).with_context(|| {
            format!(
                "it uses the placeholder `{{{{{name}}}}}`, which `shadcn-dioxus new` does not know \
                 (known: project-name, crate_name, authors, username, os-arch)"
            )
        })?;
        out.push_str(value);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Ok(out)
}

// ---------------------------------------------------------------------------------------------------
// The template
// ---------------------------------------------------------------------------------------------------

/// `[template] exclude` globs of `cargo-generate.toml`. Anything the renderer does not implement is an
/// error rather than a silently different project.
fn read_excludes(template: &Path) -> Result<Vec<String>> {
    let path = template.join(TEMPLATE_CONFIG);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let doc: DocumentMut = text
        .parse()
        .with_context(|| format!("Failed to parse {}", path.display()))?;
    for table in ["hooks", "placeholders", "conditional"] {
        if doc.contains_key(table) {
            bail!(
                "{} has a [{table}] table, which `shadcn-dioxus new` does not implement (use `dx new`)",
                path.display()
            );
        }
    }
    let mut excludes = Vec::new();
    if let Some(template_table) = doc.get("template").and_then(|t| t.as_table()) {
        for (key, item) in template_table {
            match key {
                "exclude" => {
                    let array = item.as_array().with_context(|| {
                        format!("{}: `exclude` must be an array", path.display())
                    })?;
                    excludes.extend(array.iter().filter_map(|v| v.as_str()).map(str::to_owned));
                }
                "include" | "ignore" => bail!(
                    "{} sets `{key}`, which `shadcn-dioxus new` does not implement (use `dx new`)",
                    path.display()
                ),
                _ => {}
            }
        }
    }
    Ok(excludes)
}

/// gitignore-style glob over a `/`-separated relative path: `*` and `?` stay inside one segment, `**` spans
/// segments, a pattern without `/` matches any single path component.
fn glob_match(pattern: &str, path: &str) -> bool {
    fn segment(pat: &[char], text: &[char]) -> bool {
        match pat.split_first() {
            None => text.is_empty(),
            Some(('*', rest)) => (0..=text.len()).any(|i| segment(rest, &text[i..])),
            Some(('?', rest)) => !text.is_empty() && segment(rest, &text[1..]),
            Some((c, rest)) => text.first() == Some(c) && segment(rest, &text[1..]),
        }
    }
    fn segments(pat: &[&str], path: &[&str]) -> bool {
        match pat.split_first() {
            None => path.is_empty(),
            Some((&"**", rest)) => (0..=path.len()).any(|i| segments(rest, &path[i..])),
            Some((p, rest)) => match path.split_first() {
                Some((first, tail)) => {
                    let p: Vec<char> = p.chars().collect();
                    let f: Vec<char> = first.chars().collect();
                    segment(&p, &f) && segments(rest, tail)
                }
                None => false,
            },
        }
    }
    let pattern = pattern.trim_start_matches("./").trim_end_matches('/');
    let parts: Vec<&str> = path.split('/').collect();
    if pattern.contains('/') {
        let pat: Vec<&str> = pattern.trim_start_matches('/').split('/').collect();
        segments(&pat, &parts)
    } else {
        parts.iter().any(|part| {
            let p: Vec<char> = pattern.chars().collect();
            let t: Vec<char> = part.chars().collect();
            segment(&p, &t)
        })
    }
}

/// Excluded when the path, or any directory above it, matches a pattern.
fn is_excluded(excludes: &[String], rel: &str) -> bool {
    let mut prefix = String::new();
    for part in rel.split('/') {
        if !prefix.is_empty() {
            prefix.push('/');
        }
        prefix.push_str(part);
        if excludes.iter().any(|pattern| glob_match(pattern, &prefix)) {
            return true;
        }
    }
    false
}

/// One rendered file: where it goes in the project and what it holds.
#[derive(Debug)]
pub struct RenderedFile {
    pub rel: PathBuf,
    pub contents: Vec<u8>,
    /// The template file, to copy its permissions (an executable script stays executable).
    pub source: PathBuf,
}

/// Render the template directory into memory.
pub fn render_template(template: &Path, vars: &Variables) -> Result<Vec<RenderedFile>> {
    let excludes = read_excludes(template)?;
    let mut files = Vec::new();
    let mut stack = vec![(template.to_path_buf(), String::new())];
    while let Some((dir, rel_dir)) = stack.pop() {
        let mut entries = std::fs::read_dir(&dir)
            .with_context(|| format!("Failed to read {}", dir.display()))?
            .collect::<std::io::Result<Vec<_>>>()?;
        entries.sort_by_key(|e| e.file_name());
        for entry in entries {
            let name = entry.file_name().to_string_lossy().into_owned();
            let rel = if rel_dir.is_empty() {
                name.clone()
            } else {
                format!("{rel_dir}/{name}")
            };
            if name == ".git" || (rel_dir.is_empty() && name == TEMPLATE_CONFIG) {
                continue;
            }
            if is_excluded(&excludes, &rel) {
                continue;
            }
            let path = entry.path();
            let file_type = entry.file_type()?;
            if file_type.is_dir() {
                stack.push((path, rel));
            } else if file_type.is_file() {
                files.push((path, rel));
            } else {
                bail!(
                    "{} is a symbolic link or special file; the template may only hold files and directories",
                    path.display()
                );
            }
        }
    }
    files.sort_by(|a, b| a.1.cmp(&b.1));
    let mut rendered: Vec<RenderedFile> = files
        .into_iter()
        .map(|(source, rel)| {
            let rendered_rel = render(&rel, vars)
                .with_context(|| format!("Failed to render the name of template file '{rel}'"))?;
            let rendered_rel = rendered_rel
                .strip_suffix(".liquid")
                .unwrap_or(&rendered_rel)
                .to_owned();
            let bytes = std::fs::read(&source)
                .with_context(|| format!("Failed to read {}", source.display()))?;
            let contents = match String::from_utf8(bytes) {
                Ok(text) => render(&text, vars)
                    .with_context(|| format!("Failed to render template file '{rel}'"))?
                    .into_bytes(),
                // Not text (an image, a font): copied as is, like cargo-generate does.
                Err(not_text) => not_text.into_bytes(),
            };
            Ok(RenderedFile {
                rel: PathBuf::from(rendered_rel),
                contents,
                source,
            })
        })
        .collect::<Result<_>>()?;
    rendered.sort_by(|a, b| a.rel.cmp(&b.rel));
    Ok(rendered)
}

// ---------------------------------------------------------------------------------------------------
// The command
// ---------------------------------------------------------------------------------------------------

/// `Ok(true)` when `dir` exists and holds at least one entry; an error when it is not a directory.
fn dir_is_occupied(dir: &Path) -> Result<bool> {
    match std::fs::metadata(dir) {
        Ok(meta) if meta.is_dir() => Ok(std::fs::read_dir(dir)
            .with_context(|| format!("Failed to read {}", dir.display()))?
            .next()
            .is_some()),
        Ok(_) => bail!("'{}' already exists and is not a directory", dir.display()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error).with_context(|| format!("Failed to inspect {}", dir.display())),
    }
}

/// The template directory of `source`, refreshing a cached clone once if it predates the template.
fn find_template(registries: &mut Registries, source: &Source) -> Result<PathBuf> {
    let cache_root = registries.cache_root().to_path_buf();
    let root = source.resolve_in(&cache_root)?;
    let template = root.join(TEMPLATE_DIR);
    if template.is_dir() {
        return Ok(template);
    }
    if matches!(source, Source::Git { .. }) {
        eprintln!(
            "{source} has no {TEMPLATE_DIR} in the cached clone; fetching the latest changes..."
        );
        source.update_in(&cache_root)?;
        if template.is_dir() {
            return Ok(template);
        }
    }
    bail!(
        "Registry {source} has no {TEMPLATE_DIR} directory (it may predate the starter template; try \
         --rev, --git or --path)"
    )
}

fn write_project(target: &Path, files: &[RenderedFile]) -> Result<()> {
    for file in files {
        let dest = target.join(&file.rel);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create {}", parent.display()))?;
        }
        std::fs::write(&dest, &file.contents)
            .with_context(|| format!("Failed to write {}", dest.display()))?;
        if let Ok(meta) = std::fs::metadata(&file.source) {
            let _ = std::fs::set_permissions(&dest, meta.permissions());
        }
    }
    Ok(())
}

pub fn run(registries: &mut Registries, options: &NewOptions<'_>) -> Result<PathBuf> {
    let absolute = std::path::absolute(options.target)
        .with_context(|| format!("Failed to resolve {}", options.target.display()))?;
    // `.` and `foo/..` have no usable name of their own: normalise like a shell would.
    let target = absolute
        .components()
        .fold(PathBuf::new(), |mut acc, component| {
            match component {
                std::path::Component::ParentDir => {
                    acc.pop();
                }
                std::path::Component::CurDir => {}
                other => acc.push(other),
            }
            acc
        });
    let raw_name = target
        .file_name()
        .and_then(|n| n.to_str())
        .context("cannot derive a project name from the target path")?
        .to_owned();
    let project_name = sanitize_project_name(&raw_name);
    validate_package_name(&project_name)?;
    if project_name != raw_name {
        eprintln!(
            "Renaming project called `{raw_name}` to `{project_name}` to be a valid package name"
        );
    }

    if dir_is_occupied(&target)? {
        bail!(
            "'{}' already exists and is not empty; choose another name or remove it",
            options.target.display()
        );
    }

    let source = crate::registry::resolve_source(
        options.registry,
        &crate::project::RegistryConfig::default(),
        None,
    );
    let template = find_template(registries, &source)?;
    let vars = Variables::new(&project_name, current_authors());
    let files = render_template(&template, &vars)?;

    let created_dir = !target.exists();
    let written = write_project(&target, &files);
    if let Err(error) = written {
        if created_dir {
            let _ = std::fs::remove_dir_all(&target);
        }
        return Err(error);
    }

    if options.vcs == Vcs::Git {
        match Command::new("git")
            .arg("init")
            .arg("--quiet")
            .arg(&target)
            .output()
        {
            Ok(out) if out.status.success() => {}
            Ok(out) => eprintln!(
                "warning: `git init` failed: {}",
                String::from_utf8_lossy(&out.stderr).trim()
            ),
            Err(error) => eprintln!("warning: could not run `git init`: {error}"),
        }
    }

    println!("Created {project_name} at {} ({source})", target.display());
    println!();
    println!("Next steps:");
    println!("  cd {}", options.target.display());
    println!("  shadcn-dioxus add button");
    println!("  dx serve");
    Ok(target)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn vars() -> Variables {
        Variables {
            project_name: "my-app".into(),
            crate_name: "my_app".into(),
            authors: "Ada <ada@example.com>".into(),
            username: "ada".into(),
            os_arch: "linux-x86_64".into(),
        }
    }

    #[test]
    fn sanitises_like_cargo_generate() {
        for (raw, want) in [
            ("myapp", "myapp"),
            ("my-app", "my-app"),
            ("my_app", "my_app"),
            ("My App", "my-app"),
            ("MyApp", "my-app"),
            ("Foo_Bar-baz", "foo-bar-baz"),
            ("my-app_x", "my-app-x"),
            ("HTTPServer", "http-server"),
            ("app2", "app2"),
            ("123abc", "123abc"),
            ("a  b", "a-b"),
        ] {
            assert_eq!(sanitize_project_name(raw), want, "{raw}");
        }
    }

    #[test]
    fn crate_name_is_snake_case() {
        assert_eq!(crate_name("my-app"), "my_app");
        assert_eq!(crate_name("my_app"), "my_app");
        assert_eq!(crate_name("app"), "app");
    }

    #[test]
    fn rejects_names_cargo_rejects() {
        for bad in ["", "123abc", "-x", "fn", "my.app", "näme", "a/b"] {
            assert!(validate_package_name(bad).is_err(), "{bad}");
        }
        for good in ["a", "_x", "my-app", "my_app2", "App"] {
            assert!(validate_package_name(good).is_ok(), "{good}");
        }
    }

    #[test]
    fn authors_follow_cargo_generate() {
        assert_eq!(
            format_authors(Some("Ada"), Some("ada@x.y"), Some("bob")),
            "Ada <ada@x.y>"
        );
        assert_eq!(format_authors(Some("Ada"), None, Some("bob")), "Ada");
        assert_eq!(
            format_authors(None, Some("e@x.y"), Some("bob")),
            "bob <e@x.y>"
        );
        assert_eq!(format_authors(None, None, Some("bob")), "bob");
        assert_eq!(format_authors(None, None, None), "");
        assert_eq!(format_authors(Some(" "), None, None), "");
    }

    #[test]
    fn renders_placeholders_with_or_without_spaces() {
        let text = "name = \"{{project-name}}\" by {{ authors }} ({{crate_name}}, {{username}}, {{os-arch}})";
        assert_eq!(
            render(text, &vars()).unwrap(),
            "name = \"my-app\" by Ada <ada@example.com> (my_app, ada, linux-x86_64)"
        );
    }

    #[test]
    fn leaves_text_without_placeholders_alone() {
        let text = "fn main() { let a = {1}; } // {x} }}";
        assert_eq!(render(text, &vars()).unwrap(), text);
    }

    #[test]
    fn refuses_what_it_cannot_render() {
        let unknown = render("{{nope}}", &vars()).unwrap_err().to_string();
        assert!(unknown.contains("`{{nope}}`"), "{unknown}");
        assert!(render("{{project-name | upcase}}", &vars()).is_err());
        assert!(render("{{project-name", &vars()).is_err());
        assert!(render("{% if x %}y{% endif %}", &vars()).is_err());
    }

    #[test]
    fn globs() {
        assert!(glob_match("**/.DS_Store", ".DS_Store"));
        assert!(glob_match("**/.DS_Store", "a/b/.DS_Store"));
        assert!(!glob_match("**/.DS_Store", "a/b/DS_Store"));
        assert!(glob_match("*.bak", "src/x.bak"));
        assert!(glob_match("target", "target"));
        assert!(glob_match("src/*.tmp", "src/a.tmp"));
        assert!(!glob_match("src/*.tmp", "src/x/a.tmp"));
        assert!(is_excluded(&["target".to_owned()], "target/debug/x"));
        assert!(!is_excluded(&["target".to_owned()], "src/main.rs"));
    }

    fn write(root: &Path, rel: &str, text: &str) {
        let path = root.join(rel);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn renders_a_template_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let t = tmp.path();
        write(t, "Cargo.toml.liquid", "name = \"{{project-name}}\"\n");
        write(t, "src/main.rs", "// {{project-name}}\n");
        write(t, "src/components/mod.rs", "");
        write(t, "src/.DS_Store", "junk");
        write(t, "{{crate_name}}.txt", "x");
        write(
            t,
            TEMPLATE_CONFIG,
            "[template]\nexclude = [\"**/.DS_Store\"]\n",
        );
        let files = render_template(t, &vars()).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.rel.to_str().unwrap()).collect();
        assert_eq!(
            names,
            [
                "Cargo.toml",
                "my_app.txt",
                "src/components/mod.rs",
                "src/main.rs"
            ]
        );
        assert_eq!(files[0].contents, b"name = \"my-app\"\n");
        assert_eq!(files[3].contents, b"// my-app\n");
        assert!(files[2].contents.is_empty());
    }

    #[test]
    fn refuses_template_features_it_does_not_implement() {
        for config in [
            "[hooks]\npost = [\"x.rhai\"]\n",
            "[placeholders]\nfoo = { type = \"string\", prompt = \"?\" }\n",
            "[template]\ninclude = [\"a\"]\n",
        ] {
            let tmp = tempfile::tempdir().unwrap();
            write(tmp.path(), TEMPLATE_CONFIG, config);
            assert!(render_template(tmp.path(), &vars()).is_err(), "{config}");
        }
    }

    #[test]
    fn a_bad_placeholder_names_its_file() {
        let tmp = tempfile::tempdir().unwrap();
        write(tmp.path(), "src/main.rs", "{{oops}}");
        let error = format!("{:#}", render_template(tmp.path(), &vars()).unwrap_err());
        assert!(
            error.contains("src/main.rs") && error.contains("oops"),
            "{error}"
        );
    }

    /// The real starter: every placeholder it uses must be one this renderer knows, and the output must be
    /// the project `dx new` makes.
    #[test]
    fn the_real_starter_renders() {
        let template = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join(TEMPLATE_DIR);
        let files = render_template(&template, &vars()).unwrap();
        let names: Vec<_> = files.iter().map(|f| f.rel.to_str().unwrap()).collect();
        for want in [
            ".gitignore",
            "Cargo.toml",
            "Dioxus.toml",
            "README.md",
            "src/components/mod.rs",
            "src/main.rs",
        ] {
            assert!(names.contains(&want), "{want} missing from {names:?}");
        }
        assert!(!names.contains(&TEMPLATE_CONFIG) && !names.contains(&"Cargo.toml.liquid"));
        for file in &files {
            let text = String::from_utf8_lossy(&file.contents);
            assert!(
                !text.contains("{{"),
                "{} still has a placeholder",
                file.rel.display()
            );
        }
        let cargo = files
            .iter()
            .find(|f| f.rel == Path::new("Cargo.toml"))
            .unwrap();
        let cargo = String::from_utf8_lossy(&cargo.contents);
        assert!(
            cargo.contains("name = \"my-app\"")
                && cargo.contains("authors = [\"Ada <ada@example.com>\"]")
        );
    }

    fn registry_with_template(root: &Path) {
        let t = root.join(TEMPLATE_DIR);
        write(&t, "Cargo.toml.liquid", "name = \"{{project-name}}\"\n");
        write(&t, "src/main.rs", "fn main() {}\n");
    }

    fn options<'a>(target: &'a Path, registry: &'a RegistryArgs, vcs: Vcs) -> NewOptions<'a> {
        NewOptions {
            target,
            registry,
            vcs,
        }
    }

    #[test]
    fn creates_a_project_and_a_git_repository() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = tmp.path().join("registry");
        registry_with_template(&reg);
        let args = RegistryArgs {
            path: Some(reg),
            ..Default::default()
        };
        let mut registries = Registries::new(tmp.path().join("cache"));
        let target = tmp.path().join("work").join("My App");
        let made = run(&mut registries, &options(&target, &args, Vcs::Git)).unwrap();
        assert_eq!(made, target);
        assert_eq!(
            std::fs::read_to_string(target.join("Cargo.toml")).unwrap(),
            "name = \"my-app\"\n"
        );
        assert!(target.join("src/main.rs").is_file());
        assert!(!target.join("Cargo.toml.liquid").exists());
        // `git init` ran (git is required by the registry code anyway).
        assert!(target.join(".git").is_dir());
    }

    #[test]
    fn vcs_none_makes_no_repository() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = tmp.path().join("registry");
        registry_with_template(&reg);
        let args = RegistryArgs {
            path: Some(reg),
            ..Default::default()
        };
        let mut registries = Registries::new(tmp.path().join("cache"));
        let target = tmp.path().join("app");
        run(&mut registries, &options(&target, &args, Vcs::None)).unwrap();
        assert!(target.join("Cargo.toml").is_file());
        assert!(!target.join(".git").exists());
    }

    #[test]
    fn refuses_a_non_empty_directory_and_touches_nothing() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = tmp.path().join("registry");
        registry_with_template(&reg);
        let args = RegistryArgs {
            path: Some(reg),
            ..Default::default()
        };
        let mut registries = Registries::new(tmp.path().join("cache"));
        let target = tmp.path().join("app");
        write(&target, "keep.txt", "mine");
        let error = run(&mut registries, &options(&target, &args, Vcs::None))
            .unwrap_err()
            .to_string();
        assert!(error.contains("not empty"), "{error}");
        assert_eq!(
            std::fs::read_to_string(target.join("keep.txt")).unwrap(),
            "mine"
        );
        assert!(!target.join("Cargo.toml").exists());
        // An existing file is refused too.
        let file = tmp.path().join("file");
        std::fs::write(&file, "x").unwrap();
        assert!(run(&mut registries, &options(&file, &args, Vcs::None)).is_err());
    }

    #[test]
    fn an_empty_existing_directory_is_fine() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = tmp.path().join("registry");
        registry_with_template(&reg);
        let args = RegistryArgs {
            path: Some(reg),
            ..Default::default()
        };
        let mut registries = Registries::new(tmp.path().join("cache"));
        let target = tmp.path().join("app");
        std::fs::create_dir(&target).unwrap();
        run(&mut registries, &options(&target, &args, Vcs::None)).unwrap();
        assert!(target.join("Cargo.toml").is_file());
    }

    #[test]
    fn a_registry_without_the_template_is_reported() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = tmp.path().join("registry");
        std::fs::create_dir_all(&reg).unwrap();
        let args = RegistryArgs {
            path: Some(reg),
            ..Default::default()
        };
        let mut registries = Registries::new(tmp.path().join("cache"));
        let target = tmp.path().join("app");
        let error = run(&mut registries, &options(&target, &args, Vcs::None))
            .unwrap_err()
            .to_string();
        assert!(error.contains(TEMPLATE_DIR), "{error}");
        assert!(!target.exists());
    }

    #[test]
    fn a_bad_template_leaves_no_directory_behind() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = tmp.path().join("registry");
        write(&reg.join(TEMPLATE_DIR), "src/main.rs", "{{unknown}}");
        let args = RegistryArgs {
            path: Some(reg),
            ..Default::default()
        };
        let mut registries = Registries::new(tmp.path().join("cache"));
        let target = tmp.path().join("app");
        assert!(run(&mut registries, &options(&target, &args, Vcs::None)).is_err());
        assert!(!target.exists());
    }

    #[test]
    fn an_invalid_name_is_refused_before_anything_is_written() {
        let tmp = tempfile::tempdir().unwrap();
        let reg = tmp.path().join("registry");
        registry_with_template(&reg);
        let args = RegistryArgs {
            path: Some(reg),
            ..Default::default()
        };
        let mut registries = Registries::new(tmp.path().join("cache"));
        let target = tmp.path().join("123abc");
        assert!(run(&mut registries, &options(&target, &args, Vcs::None)).is_err());
        assert!(!target.exists());
    }
}

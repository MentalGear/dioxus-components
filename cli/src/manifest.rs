//! The `component.json` wire format.
//!
//! Forked from the `dioxus-component-manifest` crate, version 0.7.9
//! (<https://github.com/DioxusLabs/dioxus>, Evan Almloff, `MIT OR Apache-2.0`), with `schemars`
//! removed. The shapes, the camelCase keys and `deny_unknown_fields` are kept byte-for-byte on purpose:
//! this registry is installed by both `dx components add` and this tool, so a manifest this parser
//! accepts must be one dx accepts too (a field only we understand would break the dx path).

use serde::{Deserialize, Serialize};
use std::process::Command;

/// A component compatible with the dioxus components system.
/// This may be a "virtual" component which is empty except for a list of members.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Component {
    pub name: String,

    #[serde(default)]
    pub description: String,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub authors: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub component_dependencies: Vec<ComponentDependency>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cargo_dependencies: Vec<CargoDependency>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub members: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub exclude: Vec<String>,

    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub global_assets: Vec<String>,
}

/// A dependency on another component.
///
/// `Builtin` is the bare-name form (`"calendar"`). dx resolves it in ITS default registry; this tool
/// resolves it in the registry of the component that names it (see `add::dependency_source`).
/// `ThirdParty` is the `{ "name", "git", "rev" }` object form.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum ComponentDependency {
    Builtin(String),
    ThirdParty {
        name: String,
        git: String,
        #[serde(default)]
        rev: Option<String>,
    },
}

impl ComponentDependency {
    /// The name of the component depended on.
    pub fn name(&self) -> &str {
        match self {
            Self::Builtin(name) => name,
            Self::ThirdParty { name, .. } => name,
        }
    }
}

/// A dependency on a cargo crate required for a component.
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum CargoDependency {
    Simple(String),
    Detailed {
        name: String,
        #[serde(default)]
        version: Option<String>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        features: Vec<String>,
        #[serde(default, skip_serializing_if = "std::ops::Not::not")]
        default_features: bool,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        git: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        rev: Option<String>,
    },
}

impl CargoDependency {
    /// The `cargo add` arguments (after `cargo add`) for this dependency. Kept identical to dx's
    /// `add_command`, quirks included (`--no-default-features` unless `default_features` is true), so
    /// both installers leave the same `Cargo.toml` behind.
    pub fn add_args(&self) -> Vec<String> {
        match self {
            CargoDependency::Simple(name) => vec![name.clone()],
            CargoDependency::Detailed {
                name,
                version,
                features,
                default_features,
                git,
                rev,
            } => {
                let mut args = vec![format!(
                    "{name}{}",
                    version
                        .as_ref()
                        .map(|version| format!("@{version}"))
                        .unwrap_or_default()
                )];
                if !features.is_empty() {
                    args.push("--features".into());
                    args.push(features.join(","));
                }
                if !*default_features {
                    args.push("--no-default-features".into());
                }
                if let Some(git) = git {
                    args.push("--git".into());
                    args.push(git.clone());
                }
                if let Some(rev) = rev {
                    args.push("--rev".into());
                    args.push(rev.clone());
                }
                args
            }
        }
    }

    /// The `cargo add` command for this dependency.
    pub fn add_command(&self) -> Command {
        let mut cmd = Command::new("cargo");
        cmd.arg("add").args(self.add_args());
        cmd
    }

    /// The name of the dependency.
    pub fn name(&self) -> &str {
        match self {
            CargoDependency::Simple(name) => name,
            CargoDependency::Detailed { name, .. } => name,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_both_dependency_forms() {
        let c: Component = serde_json::from_str(
            r#"{ "name": "date_picker",
                 "componentDependencies": ["calendar", { "name": "popover", "git": "https://x/y", "rev": "abc" }],
                 "cargoDependencies": ["time", { "name": "dioxus-icons", "version": "0.1.0" }],
                 "globalAssets": ["../../../assets/t.css"] }"#,
        )
        .unwrap();
        assert_eq!(
            c.component_dependencies[0],
            ComponentDependency::Builtin("calendar".into())
        );
        assert_eq!(
            c.component_dependencies[1],
            ComponentDependency::ThirdParty {
                name: "popover".into(),
                git: "https://x/y".into(),
                rev: Some("abc".into())
            }
        );
        assert_eq!(
            c.cargo_dependencies[1].add_args(),
            ["dioxus-icons@0.1.0", "--no-default-features"]
        );
    }

    #[test]
    fn rejects_unknown_fields_like_dx() {
        assert!(serde_json::from_str::<Component>(r#"{ "name": "a", "bogus": 1 }"#).is_err());
    }
}

//! `shadcn-dioxus`: the installer for the shadcn-dioxus component registry.
//!
//! A fork of the `dx components` subcommand of dioxus-cli 0.7.9 (<https://github.com/DioxusLabs/dioxus>,
//! `MIT OR Apache-2.0`, (c) the Dioxus authors), reduced to what a single registry needs and changed
//! where `dx components` fell short for it. See `cli/README.md` for the differences.

mod add;
mod init;
mod lock;
mod manifest;
mod new;
mod project;
mod registry;
mod update;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use project::Project;
use registry::{default_cache_root, resolve_source, Registries, RegistryArgs, Source};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "shadcn-dioxus",
    version,
    about = "Add shadcn-dioxus components to a Dioxus app"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Add components (and the components and crates they need) to this project
    Add {
        #[command(flatten)]
        components: ComponentArgs,
        #[command(flatten)]
        registry: RegistryArgs,
        /// Overwrite components that already exist, and a modified theme
        #[arg(long)]
        force: bool,
        /// Do not run `cargo add` for the components' crate dependencies
        #[arg(long)]
        skip_cargo_add: bool,
    },
    /// Remove components from this project
    Remove {
        #[command(flatten)]
        components: ComponentArgs,
        #[command(flatten)]
        registry: RegistryArgs,
    },
    /// List the components of the registry
    List {
        #[command(flatten)]
        registry: RegistryArgs,
    },
    /// Set an existing Dioxus app up for components: Dioxus.toml registry, components module, theme
    Init {
        #[command(flatten)]
        registry: RegistryArgs,
        /// Replace a different `[components.registry]`, and a modified theme
        #[arg(long)]
        force: bool,
        /// Only write Dioxus.toml, the components module and `mod components;`; skip the theme
        #[arg(long)]
        no_theme: bool,
    },
    /// Create a new app from the registry's starter template (what `dx new --template ... --subtemplate
    /// templates/starter` makes, without dx or cargo-generate)
    New {
        /// The directory to create; its last component is the project name
        name: PathBuf,
        /// Where the starter comes from: --path (a checkout), --git / --rev, else the default registry
        #[command(flatten)]
        registry: RegistryArgs,
        /// Version control for the new project (`git` runs `git init`, without a commit)
        #[arg(long, value_enum, default_value = "git")]
        vcs: new::Vcs,
    },
    /// Update installed components to the registry's latest version, keeping files you edited
    ///
    /// Refreshes the registry's cached clone, then replaces each installed component's files (and the
    /// theme) that you have not edited since they were installed. A component with a file you edited is
    /// left unchanged and listed, and the command fails; --force replaces it anyway. Outside a project,
    /// and with no component names, it only refreshes the cached clone.
    Update {
        /// The components to update, comma separated (default: every installed component)
        #[arg(value_delimiter = ',')]
        components: Vec<String>,
        #[command(flatten)]
        registry: RegistryArgs,
        /// Replace components and the theme even if you edited them
        #[arg(long)]
        force: bool,
        /// Do not run `cargo add` for the crate dependencies of updated components
        #[arg(long)]
        skip_cargo_add: bool,
        /// The location of the component module in your project (default: src/components)
        #[arg(long)]
        module_path: Option<PathBuf>,
        /// The location of the global assets in your project (default: assets)
        #[arg(long)]
        global_assets_path: Option<PathBuf>,
    },
    /// Delete every cached registry clone
    Clean,
}

#[derive(Args)]
struct ComponentArgs {
    /// The components to add or remove, comma separated
    #[arg(required_unless_present = "all", value_delimiter = ',')]
    components: Vec<String>,
    /// The location of the component module in your project (default: src/components)
    #[arg(long)]
    module_path: Option<PathBuf>,
    /// The location of the global assets in your project (default: assets)
    #[arg(long)]
    global_assets_path: Option<PathBuf>,
    /// Every component in the registry
    #[arg(long)]
    all: bool,
}

/// The registry for this run: flags, else the project's `Dioxus.toml`, else the default.
fn source_for(project: Option<&Project>, args: &RegistryArgs) -> Source {
    let config = project
        .map(|p| p.config.registry.clone())
        .unwrap_or_default();
    let config_dir = project
        .and_then(|p| p.config_path.as_deref())
        .and_then(|p| p.parent());
    resolve_source(args, &config, config_dir)
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let cwd = std::env::current_dir()?;
    let mut registries = Registries::new(default_cache_root());
    match cli.command {
        Command::Add {
            components,
            registry,
            force,
            skip_cargo_add,
        } => {
            let project = Project::discover(&cwd)?;
            let source = source_for(Some(&project), &registry);
            add::add(
                &project,
                &mut registries,
                &source,
                &add::AddOptions {
                    requested: &components.components,
                    all: components.all,
                    module_path: components.module_path.as_deref(),
                    global_assets_path: components.global_assets_path.as_deref(),
                    force,
                    skip_cargo_add,
                },
            )
        }
        Command::Remove {
            components,
            registry,
        } => {
            let project = Project::discover(&cwd)?;
            let source = source_for(Some(&project), &registry);
            add::remove(
                &project,
                &mut registries,
                &source,
                &components.components,
                components.all,
                components.module_path.as_deref(),
            )
        }
        Command::List { registry } => {
            // `list` works outside a project too: the registry then comes from the flags or the default.
            let project = Project::discover(&cwd).ok();
            add::list(&mut registries, &source_for(project.as_ref(), &registry))
        }
        Command::Init {
            registry,
            force,
            no_theme,
        } => init::run(
            &cwd,
            &mut registries,
            &init::InitOptions {
                registry,
                force,
                no_theme,
            },
        )
        .map(|_| ()),
        Command::New {
            name,
            registry,
            vcs,
        } => new::run(
            &mut registries,
            &new::NewOptions {
                target: &name,
                registry: &registry,
                vcs,
            },
        )
        .map(|_| ()),
        Command::Update {
            components,
            registry,
            force,
            skip_cargo_add,
            module_path,
            global_assets_path,
        } => match Project::discover(&cwd) {
            Ok(project) => {
                let source = source_for(Some(&project), &registry);
                update::update(
                    &project,
                    &mut registries,
                    &source,
                    &update::UpdateOptions {
                        requested: &components,
                        module_path: module_path.as_deref(),
                        global_assets_path: global_assets_path.as_deref(),
                        force,
                        skip_cargo_add,
                    },
                )
            }
            Err(error) if !components.is_empty() => Err(error),
            Err(_) => {
                let source = source_for(None, &registry);
                source.update_in(registries.cache_root())?;
                println!("Updated {source}");
                Ok(())
            }
        },
        Command::Clean => {
            let _ = std::fs::remove_dir_all(registries.cache_root());
            println!("Removed {}", registries.cache_root().display());
            Ok(())
        }
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error:#}");
        std::process::exit(1);
    }
}

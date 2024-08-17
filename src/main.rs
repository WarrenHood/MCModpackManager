mod mod_meta;
mod modpack;
mod providers;
mod resolver;

use clap::{Parser, Subcommand};
use mod_meta::{ModMeta, ModProvider};
use modpack::ModpackMeta;
use std::{error::Error, path::PathBuf};

/// A Minecraft Modpack Manager
#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialise a new mcmpmgr project in the specified directory (or current dir if not specified)
    Init {
        /// The root modpack project directory
        directory: Option<PathBuf>,
        /// Name of the modpack project
        #[arg(long)]
        name: Option<String>,
        /// The modpack's Minecraft version
        #[arg(long, default_value_t = String::from("1.20.1"))]
        mc_version: String,
        /// The modpack's modloader
        #[arg(long, default_value_t = modpack::ModLoader::Fabric)]
        modloader: modpack::ModLoader,
        /// Default providers to download the mods from for the modpack (can be overridden on a per-mod basis)
        #[arg(long)]
        providers: Vec<ModProvider>,
    },
    /// Create and initialise a new mcmpmgr project in the current directory
    New {
        /// Name of the new modpack project
        name: String,
        /// The modpack's Minecraft version
        #[arg(long, default_value_t = String::from("1.20.1"))]
        mc_version: String,
        /// The modpack's modloader
        #[arg(long, default_value_t = modpack::ModLoader::Fabric)]
        modloader: modpack::ModLoader,
        /// Default providers to download the mods from for the modpack (can be overridden on a per-mod basis)
        #[arg(long)]
        providers: Vec<ModProvider>,
    },
    /// Add a new mod to the modpack
    Add {
        /// Name of the mod to add to the project, optionally including a version
        name: String,
        /// Providers to download the mods from
        #[arg(long)]
        providers: Vec<ModProvider>,
        /// URL to download the mod from
        #[arg(long)]
        url: Option<String>,
        /// Whether to ignore exact transitive mod versions
        #[arg(long, short, action)]
        ignore_transitive_versions: bool,
    },
    /// Remove a mod from the modpack
    Remove {
        /// Name of the mod to remove from the modpack
        name: String,
    },
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::Init {
                directory,
                mc_version,
                modloader,
                name,
                providers,
            } => {
                let dir = directory.unwrap_or(std::env::current_dir()?);
                let pack_name = if let Some(name) = name {
                    name
                } else {
                    dir.file_name()
                        .ok_or(format!(
                            "Cannot find pack name based on directory '{}'",
                            dir.display()
                        ))?
                        .to_string_lossy()
                        .into()
                };
                println!(
                    "Initializing project '{}' at '{}'...",
                    &pack_name,
                    dir.display()
                );
                let mut mc_modpack_meta: ModpackMeta =
                    ModpackMeta::new(&pack_name, &mc_version, modloader);
                for provider in providers.into_iter() {
                    mc_modpack_meta = mc_modpack_meta.provider(provider);
                }
                mc_modpack_meta.init_project(&dir)?;
                let modpack_lock = resolver::PinnedPackMeta::load_from_directory(&dir, false).await?;
                modpack_lock.save_to_dir(&dir)?;
            }
            Commands::New {
                name,
                mc_version,
                modloader,
                providers,
            } => {
                let dir = std::env::current_dir()?.join(PathBuf::from(&name));
                println!(
                    "Creating new modpack project '{}' at '{}'...",
                    &name,
                    dir.display()
                );
                std::fs::create_dir_all(&dir)?;
                let mut mc_modpack_meta: ModpackMeta =
                    ModpackMeta::new(&name, &mc_version, modloader);
                for provider in providers.into_iter() {
                    mc_modpack_meta = mc_modpack_meta.provider(provider);
                }
                mc_modpack_meta.init_project(&dir)?;

                let modpack_lock = resolver::PinnedPackMeta::load_from_directory(&dir, false).await?;
                modpack_lock.save_to_dir(&dir)?;
            }
            Commands::Add {
                name,
                providers,
                url,
                ignore_transitive_versions,
            } => {
                let mut modpack_meta = ModpackMeta::load_from_current_directory()?;
                let old_modpack_meta = modpack_meta.clone();

                let mut mod_meta = ModMeta::new(&name)?;
                if let Some(url) = url {
                    mod_meta = mod_meta.url(&url);
                }
                for provider in providers.into_iter() {
                    mod_meta = mod_meta.provider(provider);
                }
                modpack_meta = modpack_meta.add_mod(&mod_meta);
                modpack_meta.save_current_dir_project()?;

                let revert_modpack_meta = |e| -> ! {
                    let revert_result = old_modpack_meta.save_current_dir_project();
                    if let Err(result) = revert_result {
                        panic!("Failed to revert modpack meta: {}", result);
                    }
                    panic!("Reverted modpack meta:\n{}", e);
                };

                match resolver::PinnedPackMeta::load_from_current_directory(ignore_transitive_versions).await {
                    Ok(mut modpack_lock) => {
                        let pin_result = modpack_lock
                            .pin_mod_and_deps(&mod_meta, &modpack_meta, ignore_transitive_versions)
                            .await;
                        if let Err(e) = pin_result {
                            revert_modpack_meta(e);
                        }

                        if let Err(e) = modpack_lock.save_current_dir_lock() {
                            revert_modpack_meta(e);
                        }
                    }
                    Err(e) => {
                        revert_modpack_meta(e);
                    }
                };
            }
            Commands::Remove { name } => {
                let mut modpack_meta = ModpackMeta::load_from_current_directory()?;
                let old_modpack_meta = modpack_meta.clone();

                modpack_meta = modpack_meta.remove_mod(&name);
                modpack_meta.save_current_dir_project()?;

                let revert_modpack_meta = |e| -> ! {
                    let revert_result = old_modpack_meta.save_current_dir_project();
                    if let Err(result) = revert_result {
                        panic!("Failed to revert modpack meta: {}", result);
                    }
                    panic!("Reverted modpack meta:\n{}", e);
                };

                match resolver::PinnedPackMeta::load_from_current_directory(false).await {
                    Ok(mut modpack_lock) => {
                        let remove_result = modpack_lock.remove_mod(&name, &modpack_meta);
                        if let Err(e) = remove_result {
                            revert_modpack_meta(e);
                        }

                        if let Err(e) = modpack_lock.save_current_dir_lock() {
                            revert_modpack_meta(e);
                        }
                    }
                    Err(e) => {
                        revert_modpack_meta(e);
                    }
                };
            }
        }
    };

    Ok(())
}

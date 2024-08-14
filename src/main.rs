mod modpack;

use clap::{Parser, Subcommand};
use modpack::{ModLoader, ModMeta, ModProvider, ModpackMeta};
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
    /// Initialises a new mcmpmgr project in the specified directory (or current dir if not specified)
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
    },
    /// Creates and initialises a new mcmpmgr project in the current directory
    New {
        /// Name of the new modpack project
        name: String,
        /// The modpack's Minecraft version
        #[arg(long, default_value_t = String::from("1.20.1"))]
        mc_version: String,
        /// The modpack's modloader
        #[arg(long, default_value_t = modpack::ModLoader::Fabric)]
        modloader: modpack::ModLoader,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::Init {
                directory,
                mc_version,
                modloader,
                name,
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
                let mc_modpack_meta: ModpackMeta =
                    ModpackMeta::new(&pack_name, &mc_version, modloader);
                mc_modpack_meta.init_project(&dir)?;
            }
            Commands::New {
                name,
                mc_version,
                modloader,
            } => {
                let dir = std::env::current_dir()?.join(PathBuf::from(&name));
                println!(
                    "Creating new modpack project '{}' at '{}'...",
                    &name,
                    dir.display()
                );
                std::fs::create_dir_all(&dir)?;
                let mc_modpack_meta: ModpackMeta = ModpackMeta::new(&name, &mc_version, modloader);
                mc_modpack_meta.init_project(&dir)?;
            }
        }
    };

    Ok(())
}

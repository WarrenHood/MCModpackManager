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
        directory: Option<PathBuf>,
        #[arg(long, default_value_t = String::from("1.20.1"))]
        mc_version: String,
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
            } => {
                let dir = directory.unwrap_or(std::env::current_dir()?);
                let mc_modpack_meta = ModpackMeta::new(&mc_version, modloader);
                let modpack_meta_file_path = dir.clone().join(PathBuf::from("mcmodpack.toml"));
                if modpack_meta_file_path.exists() {
                    return Err(format!(
                        "mcmodpack.toml already exists at {}",
                        modpack_meta_file_path.display()
                    )
                    .into());
                }

                std::fs::write(
                    modpack_meta_file_path,
                    toml::to_string(&mc_modpack_meta)
                        .expect("MC Modpack Meta should be serializable"),
                )?;

                println!("MC modpack project initialized at {}", dir.display());
            }
        }
    };

    Ok(())
}

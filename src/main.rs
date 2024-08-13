mod modpack;

use clap::{Parser, Subcommand};
use std::path::PathBuf;

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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    if let Some(command) = cli.command {
        match command {
            Commands::Init {
                directory,
                mc_version,
                modloader,
            } => {
                let dir = if let Some(directory) = directory {
                    directory
                } else {
                    std::env::current_dir()
                        .expect("You should have permissions to access the current directory")
                };

                let mc_modpack_meta = modpack::ModpackMeta {
                    mc_version: mc_version,
                    modloader: modloader,
                    mods: vec![],
                };

                let modpack_meta_file_path = dir.clone().join(PathBuf::from("mcmodpack.toml"));

                if modpack_meta_file_path.exists() {
                    anyhow::bail!(
                        "mcmodpack.toml already exists at {}",
                        modpack_meta_file_path.display()
                    );
                }

                if let Err(e) = std::fs::write(
                    modpack_meta_file_path,
                    toml::to_string(&mc_modpack_meta)
                        .expect("MC Modpack Meta should be serializable"),
                ) {
                    anyhow::bail!("Unable to initialize new MC modpack project at {}:\n{}", dir.display(), e);
                };

                println!("MC modpack project initialized at {}", dir.display());
            }
        }
    };

    Ok(())
}

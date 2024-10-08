mod file_meta;
mod mod_meta;
mod modpack;
mod profiles;
mod providers;
mod resolver;

use anyhow::{Error, Result};
use clap::{Args, Parser, Subcommand};
use file_meta::{get_normalized_relative_path, FileApplyPolicy, FileMeta};
use mod_meta::{ModMeta, ModProvider};
use modpack::ModpackMeta;
use profiles::{PackSource, Profile};
use providers::DownloadSide;
use std::path::PathBuf;

/// A Minecraft Modpack Manager
#[derive(Parser)]
#[command(author, version, about, long_about = None)]
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
        /// Use exact transitive mod dependency versions
        #[arg(long, short, action)]
        locked: bool,
        /// Minecraft version override
        #[arg(long)]
        mc_version: Option<String>,
        /// Modloader override
        #[arg(long, short)]
        modloader: Option<modpack::ModLoader>,
        /// Side override
        #[arg(long, short)]
        side: Option<DownloadSide>,
    },
    /// Remove a mod from the modpack
    Remove {
        /// Name of the mod to remove from the modpack
        name: String,
        /// Forcefully remove the mod without checking if anything depends on it
        #[arg(long, short, action)]
        force: bool,
    },
    /// Forbid a mod from the modpack
    Forbid {
        /// Name of the mod to remove and forbid from the modpack
        name: String,
    },
    /// Download the mods in the pack to a specified folder
    Download {
        /// Mods directory
        mods_dir: PathBuf,
        /// Side to download for
        #[arg(long, default_value_t = DownloadSide::Server)]
        side: DownloadSide,
        /// Download mods from a remote modpack in a git repo
        #[arg(long)]
        git: Option<String>,
        /// Download mods from a local modpack
        #[arg(long)]
        path: Option<PathBuf>,
    },
    /// Update all mods to the latest possible version
    Update {
        /// Use exact transitive mod dependency versions
        #[arg(long, short, action)]
        locked: bool,
    },
    /// Manage local files in the modpack
    File(FileArgs),
    /// Manage mcmpmgr profiles
    Profile(ProfileArgs),
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
struct FileArgs {
    #[command(subcommand)]
    command: Option<FileCommands>,
}

#[derive(Debug, Subcommand)]
enum FileCommands {
    /// List all files/folders in the pack
    List,
    /// Add new files/folder to the pack
    Add {
        /// Local path to file/folder to include in the pack (must be in the pack root)
        local_path: PathBuf,
        /// Target path to copy the file/folder to relative to the MC instance directory
        #[arg(short, long)]
        target_path: Option<String>,
        /// Side to copy the file/folder to
        #[arg(long, default_value_t = DownloadSide::Server)]
        side: DownloadSide,
        /// File apply policy - whether to always apply the file or just apply it once (if the file doesn't exist)
        #[arg(long, default_value_t = FileApplyPolicy::Always)]
        apply_policy: FileApplyPolicy,
    },
    /// Show metadata about a file in the pack
    Show {
        /// Local path of the file/folder to show
        local_path: String,
    },
    /// Remove a file/folder from the pack
    Remove {
        /// local path to file/folder to remove
        local_path: PathBuf,
    },
}

#[derive(Debug, Args)]
#[command(args_conflicts_with_subcommands = true)]
struct ProfileArgs {
    #[command(subcommand)]
    command: Option<ProfileCommands>,
}

#[derive(Debug, Subcommand)]
enum ProfileCommands {
    /// List all profiles
    List,
    /// Add or overwrite a profile
    Add {
        /// Name of the profile
        name: String,
        /// Side to download the profile for. (Client, Server, or Both)
        #[arg(long, default_value_t = DownloadSide::Server)]
        side: DownloadSide,
        /// A local file path to a modpack directory or a git repo url prefixed with 'git+'
        #[arg(long, short)]
        pack_source: PackSource,
        /// Instance directory (containing a mods folder)
        #[arg(long, short)]
        instance_directory: PathBuf,
    },
    /// Install a profile
    Install {
        /// Name of the profile to install
        name: String,
    },
    /// Show information about a profile
    Show {
        /// Name of the profile to show
        name: String,
    },
    /// Delete a profile
    Remove {
        /// Profile to remove
        name: String,
    },
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> anyhow::Result<()> {
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
                    let dir_name = dir.file_name();
                    if let Some(dir_name) = dir_name {
                        dir_name.to_string_lossy().to_string()
                    } else {
                        anyhow::bail!(
                            "Cannot find pack name based on directory '{}'",
                            dir.display()
                        )
                    }
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
                let modpack_lock =
                    resolver::PinnedPackMeta::load_from_directory(&dir, true).await?;
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

                let modpack_lock =
                    resolver::PinnedPackMeta::load_from_directory(&dir, true).await?;
                modpack_lock.save_to_dir(&dir)?;
            }
            Commands::Add {
                name,
                providers,
                url,
                locked,
                mc_version,
                modloader,
                side,
            } => {
                let mut modpack_meta = ModpackMeta::load_from_current_directory()?;
                let old_modpack_meta = modpack_meta.clone();

                let mut mod_meta = ModMeta::new(&name)?;

                if let Some(modloader) = modloader {
                    mod_meta = mod_meta.modloader(modloader);
                }

                if let Some(mc_version) = mc_version {
                    mod_meta = mod_meta.mc_version(&mc_version);
                }

                if let Some(url) = url {
                    mod_meta = mod_meta.url(&url);
                }
                if let Some(side) = side {
                    match side {
                        DownloadSide::Both => {
                            mod_meta.server_side = Some(true);
                            mod_meta.client_side = Some(true);
                        }
                        DownloadSide::Server => {
                            mod_meta.server_side = Some(true);
                            mod_meta.client_side = Some(false);
                        }
                        DownloadSide::Client => {
                            mod_meta.server_side = Some(false);
                            mod_meta.client_side = Some(true);
                        }
                    }
                }
                for provider in providers.into_iter() {
                    mod_meta = mod_meta.provider(provider);
                }
                modpack_meta = modpack_meta.add_mod(&mod_meta)?;
                modpack_meta.save_current_dir_project()?;

                let revert_modpack_meta = |e| -> ! {
                    let revert_result = old_modpack_meta.save_current_dir_project();
                    if let Err(result) = revert_result {
                        panic!("Failed to revert modpack meta: {}", result);
                    }
                    panic!("Reverted modpack meta:\n{}", e);
                };

                match resolver::PinnedPackMeta::load_from_current_directory(!locked).await {
                    Ok(mut modpack_lock) => {
                        let remove_result =
                            modpack_lock.remove_mod(&mod_meta.name, &modpack_meta, true);
                        if let Err(e) = remove_result {
                            revert_modpack_meta(e);
                        }

                        let pin_result = modpack_lock
                            .pin_mod_and_deps(&mod_meta, &modpack_meta, !locked)
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
            Commands::Remove { name, force } => {
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

                match resolver::PinnedPackMeta::load_from_current_directory(true).await {
                    Ok(mut modpack_lock) => {
                        let remove_result = modpack_lock.remove_mod(&name, &modpack_meta, force);
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
            Commands::Forbid { name } => {
                let mut modpack_meta = ModpackMeta::load_from_current_directory()?;
                let old_modpack_meta = modpack_meta.clone();

                modpack_meta.forbid_mod(&name);
                modpack_meta = modpack_meta.remove_mod(&name);
                modpack_meta.save_current_dir_project()?;

                let revert_modpack_meta = |e| -> ! {
                    let revert_result = old_modpack_meta.save_current_dir_project();
                    if let Err(result) = revert_result {
                        panic!("Failed to revert modpack meta: {}", result);
                    }
                    panic!("Reverted modpack meta:\n{}", e);
                };

                match resolver::PinnedPackMeta::load_from_current_directory(true).await {
                    Ok(mut modpack_lock) => {
                        let remove_result = modpack_lock.remove_mod(&name, &modpack_meta, true);
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
            Commands::Download {
                mods_dir,
                side,
                git,
                path,
            } => {
                let mut pack_dir: Option<tempfile::TempDir> = None;
                let pack_lock = if let Some(git_url) = git {
                    let (lock_meta, repo_dir) =
                        resolver::PinnedPackMeta::load_from_git_repo(&git_url, true).await?;
                    // Hold on to the repo directory until pack_dir is dropped
                    let _ = pack_dir.insert(repo_dir);
                    lock_meta
                } else if let Some(local_path) = path {
                    resolver::PinnedPackMeta::load_from_directory(&local_path, true).await?
                } else {
                    resolver::PinnedPackMeta::load_from_current_directory(true).await?
                };

                pack_lock.download_mods(&mods_dir, side).await?;
                println!("Mods updated");
            }
            Commands::Update { locked } => {
                let mut pack_lock = resolver::PinnedPackMeta::new();
                let modpack_meta = ModpackMeta::load_from_current_directory()?;
                pack_lock.init(&modpack_meta, !locked).await?;
                pack_lock.save_current_dir_lock()?;
            }
            Commands::File(FileArgs { command }) => {
                if let Some(command) = command {
                    match command {
                        FileCommands::List => todo!(),
                        FileCommands::Add {
                            local_path,
                            target_path,
                            side,
                            apply_policy,
                        } => {
                            let mut modpack_meta = ModpackMeta::load_from_current_directory()?;
                            let current_dir = &std::env::current_dir()?;
                            let target_path = if let Some(target_path) = target_path {
                                target_path
                            } else {
                                get_normalized_relative_path(&local_path, &current_dir)?
                            };
                            let file_meta = FileMeta {
                                target_path,
                                side,
                                apply_policy,
                            };

                            modpack_meta.add_file(&local_path, &file_meta, current_dir)?;
                            modpack_meta.save_current_dir_project()?;
                        }
                        FileCommands::Show { local_path } => todo!(),
                        FileCommands::Remove { local_path } => {
                            let mut modpack_meta = ModpackMeta::load_from_current_directory()?;
                            modpack_meta.remove_file(&local_path, &std::env::current_dir()?)?;
                            modpack_meta.save_current_dir_project()?;
                        }
                    }
                }
            }
            Commands::Profile(ProfileArgs { command }) => {
                if let Some(command) = command {
                    match command {
                        ProfileCommands::List => {
                            let userdata = profiles::Data::load()?;
                            println!("Profiles:");
                            for profile in userdata.get_profile_names().iter() {
                                println!("- {profile}");
                            }
                        }
                        ProfileCommands::Add {
                            name,
                            side,
                            pack_source,
                            instance_directory,
                        } => {
                            let mut userdata = profiles::Data::load()?;
                            let profile = Profile::new(&instance_directory, pack_source, side)?;
                            userdata.add_profile(&name, profile);
                            userdata.save()?;
                            println!("Saved profile '{name}'");
                        }
                        ProfileCommands::Install { name } => {
                            let userdata = profiles::Data::load()?;
                            let profile = userdata.get_profile(&name);

                            let profile = if let Some(profile) = profile {
                                profile
                            } else {
                                anyhow::bail!("Profile '{name}' does not exist")
                            };

                            println!("Installing profile '{name}'...");
                            profile.install().await?;
                            println!("Installed profile '{name}' successfully");
                        }
                        ProfileCommands::Remove { name } => {
                            let mut userdata = profiles::Data::load()?;
                            userdata.remove_profile(&name);
                            println!("Removed profile '{name}'");
                        }
                        ProfileCommands::Show { name } => {
                            let userdata = profiles::Data::load()?;
                            let profile = userdata.get_profile(&name);

                            let profile = if let Some(profile) = profile {
                                profile
                            } else {
                                anyhow::bail!("Profile '{name}' does not exist")
                            };
                            println!("Profile name      : {name}");
                            println!("Instance folder   : {}", profile.instance_folder.display());
                            println!("Modpack source    : {}", profile.pack_source);
                            println!("Side              : {}", profile.side);
                        }
                    }
                }
            }
        }
    };

    Ok(())
}

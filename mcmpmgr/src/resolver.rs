use anyhow::Result;
use reqwest::{header::CONTENT_DISPOSITION, Url};
use serde::{Deserialize, Serialize};
use sha1::Sha1;
use sha2::{Digest, Sha512};
use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{OsStr, OsString},
    path::{Path, PathBuf},
};

use crate::{
    mod_meta::{ModMeta, ModProvider},
    modpack::ModpackMeta,
    providers::{modrinth::Modrinth, DownloadSide, FileSource, PinnedMod},
};

const MODPACK_LOCK_FILENAME: &str = "modpack.lock";

#[derive(Serialize, Deserialize)]
pub struct PinnedPackMeta {
    mods: BTreeMap<String, PinnedMod>,
    #[serde(skip_serializing, skip_deserializing)]
    modrinth: Modrinth,
}

impl PinnedPackMeta {
    pub fn new() -> Self {
        Self {
            mods: Default::default(),
            modrinth: Modrinth::new(),
        }
    }

    /// Clears out anything not in the mods list, and then downloads anything in the mods list not present
    pub async fn download_mods(
        &self,
        mods_dir: &PathBuf,
        download_side: DownloadSide,
    ) -> Result<()> {
        let _ = std::fs::create_dir_all(mods_dir);
        let files = std::fs::read_dir(mods_dir)?;
        let mut pinned_files_cache = BTreeSet::new();
        for file in files.into_iter() {
            let file = file?;
            if file.file_type()?.is_file() {
                let filename = file.file_name();
                if !self.file_is_pinned(&filename, download_side, &mut pinned_files_cache) {
                    println!(
                        "Deleting file {:#?} as it is not in the pinned mods",
                        filename
                    );
                    tokio::fs::remove_file(file.path()).await?;
                }
            }
        }

        for (_, pinned_mod) in self.mods.iter().filter(|m| {
            download_side == DownloadSide::Both
                || download_side == DownloadSide::Client && m.1.client_side
                || download_side == DownloadSide::Server && m.1.server_side
        }) {
            for filesource in pinned_mod.source.iter() {
                match filesource {
                    crate::providers::FileSource::Download {
                        url,
                        sha1: _,
                        sha512,
                        filename,
                    } => {
                        if mods_dir.join(PathBuf::from(filename)).exists() {
                            println!("Found existing mod {}", filename);
                            continue;
                        }
                        println!("Downloading {} from {}", filename, url);
                        let file_contents = reqwest::get(url).await?.bytes().await?;
                        let mut hasher = Sha512::new();
                        hasher.update(&file_contents);
                        let sha512_hash = format!("{:X}", hasher.finalize()).to_ascii_lowercase();
                        let sha512 = sha512.to_ascii_lowercase();
                        if sha512_hash != *sha512 {
                            eprintln!(
                                "Sha512 hash mismatch for file {}\nExpected:\n{}\nGot:\n{}",
                                filename, sha512, sha512_hash
                            );
                            anyhow::bail!(
                                "Sha512 hash mismatch for file {}\nExpected:\n{}\nGot:\n{}",
                                filename,
                                sha512,
                                sha512_hash
                            )
                        }

                        tokio::fs::write(mods_dir.join(filename), file_contents).await?;
                    }
                    crate::providers::FileSource::Local {
                        path: _,
                        sha1: _,
                        sha512: _,
                        filename: _,
                    } => unimplemented!(),
                }
            }
        }

        Ok(())
    }

    pub fn file_is_pinned(
        &self,
        file_name: &OsStr,
        mod_side: DownloadSide,
        cache: &mut BTreeSet<OsString>,
    ) -> bool {
        if cache.contains(file_name) {
            return true;
        }
        for (_, pinned_mod) in self.mods.iter().filter(|m| {
            mod_side == DownloadSide::Both
                || mod_side == DownloadSide::Client && m.1.client_side
                || mod_side == DownloadSide::Server && m.1.server_side
        }) {
            for filesource in pinned_mod.source.iter() {
                match filesource {
                    crate::providers::FileSource::Download {
                        url: _,
                        sha1: _,
                        sha512: _,
                        filename,
                    } => {
                        let pinned_filename = OsStr::new(filename);
                        cache.insert(pinned_filename.into());
                        if pinned_filename == file_name {
                            return true;
                        }
                    }
                    crate::providers::FileSource::Local {
                        path: _,
                        sha1: _,
                        sha512: _,
                        filename,
                    } => {
                        let pinned_filename = OsStr::new(filename);
                        cache.insert(pinned_filename.into());
                        if pinned_filename == file_name {
                            return true;
                        }
                    }
                }
            }
        }

        return false;
    }

    pub async fn pin_mod_and_deps(
        &mut self,
        mod_metadata: &ModMeta,
        pack_metadata: &ModpackMeta,
        ignore_transitive_versions: bool,
    ) -> Result<()> {
        if let Some(mod_meta) = self.mods.get(&mod_metadata.name) {
            if mod_metadata.version != "*" && mod_metadata.version == mod_meta.version {
                // Skip already pinned mods
                // TODO: Replace * with the current mod version in the modpack meta so this doesn't get called twice for the first mod created
                return Ok(());
            }
        }
        let mut deps =
            BTreeSet::from_iter(self.pin_mod(mod_metadata, pack_metadata).await?.into_iter());

        if ignore_transitive_versions {
            // Ignore transitive dep versions
            deps = deps.iter().map(|d| d.clone().version("*")).collect();
        }

        let pinned_version = self
            .mods
            .get(&mod_metadata.name)
            .expect("should be in pinned mods")
            .version
            .clone();

        while !deps.is_empty() {
            let mut next_deps = BTreeSet::new();
            for dep in deps.iter() {
                println!(
                    "Adding mod {}@{} (dependency of {}@{})",
                    dep.name, dep.version, mod_metadata.name, pinned_version
                );
                next_deps.extend(self.pin_mod(dep, &pack_metadata).await?);
            }
            deps = next_deps;
        }

        Ok(())
    }

    /// Pin a mod version
    ///
    /// A list of dependencies to pin is included
    pub async fn pin_mod(
        &mut self,
        mod_metadata: &ModMeta,
        pack_metadata: &ModpackMeta,
    ) -> Result<Vec<ModMeta>> {
        if pack_metadata.forbidden_mods.contains(&mod_metadata.name) {
            println!("Skipping adding forbidden mod {}...", mod_metadata.name);
            return Ok(vec![]);
        }

        let mod_providers = if let Some(mod_providers) = &mod_metadata.providers {
            mod_providers
        } else {
            &vec![]
        };
        let mut checked_providers: BTreeSet<ModProvider> = BTreeSet::new();
        for mod_provider in mod_providers
            .iter()
            .chain(pack_metadata.default_providers.iter())
        {
            if checked_providers.contains(&mod_provider) {
                // No need to repeat a check for a provider if we have already checked it
                continue;
            }
            checked_providers.insert(mod_provider.clone());
            match mod_provider {
                crate::mod_meta::ModProvider::CurseForge => unimplemented!(),
                crate::mod_meta::ModProvider::Modrinth => {
                    let pinned_mod = self.modrinth.resolve(&mod_metadata, pack_metadata).await;
                    if let Ok(pinned_mod) = pinned_mod {
                        self.mods
                            .insert(mod_metadata.name.clone(), pinned_mod.clone());
                        println!("Pinned {}@{}", mod_metadata.name, pinned_mod.version);
                        if let Some(deps) = &pinned_mod.deps {
                            return Ok(deps
                                .iter()
                                .filter(|d| !self.mods.contains_key(&d.name))
                                .cloned()
                                .collect());
                        }
                        return Ok(vec![]);
                    } else if let Err(e) = pinned_mod {
                        eprintln!(
                            "Failed to resolve {}@{} with provider {:#?}: {}",
                            mod_metadata.name, mod_metadata.version, mod_provider, e
                        );
                    }
                }
                crate::mod_meta::ModProvider::Raw => {
                    let url = mod_metadata
                        .download_url
                        .clone()
                        .ok_or(anyhow::format_err!(
                            "A download url is required to pin {}",
                            mod_metadata.name
                        ))?;
                    let file_response = reqwest::get(&url).await?;

                    // TODO: Get filename from content disposition
                    let _content_disposition = file_response.headers().get(CONTENT_DISPOSITION);
                    let url_parsed = Url::parse(&url)?;
                    let filename = url_parsed
                        .path_segments()
                        .ok_or(anyhow::format_err!(
                            "Cannot get path segments from url {}",
                            url
                        ))?
                        .last()
                        .ok_or(anyhow::format_err!("Cannot get filename from url {}", url))?;

                    let file_contents = file_response.bytes().await?;
                    let mut sha1_hasher = Sha1::new();
                    let mut sha512_hasher = Sha512::new();
                    sha1_hasher.update(&file_contents);
                    sha512_hasher.update(&file_contents);
                    let sha1_hash = format!("{:X}", sha1_hasher.finalize()).to_ascii_lowercase();
                    let sha512_hash =
                        format!("{:X}", sha512_hasher.finalize()).to_ascii_lowercase();

                    let pinned_mod = PinnedMod {
                        source: vec![FileSource::Download {
                            url: url.into(),
                            sha1: sha1_hash,
                            sha512: sha512_hash,
                            filename: filename.into(),
                        }],
                        version: "Unknown".into(),
                        deps: None,
                        server_side: mod_metadata.server_side.unwrap_or(true),
                        client_side: mod_metadata.client_side.unwrap_or(true),
                    };
                    self.mods
                        .insert(mod_metadata.name.clone(), pinned_mod.clone());
                    println!("Pinned {}@{}", mod_metadata.name, pinned_mod.version);
                    return Ok(vec![]);
                }
            };
        }

        anyhow::bail!(
            "Failed to pin mod '{}' (providers={:#?}) with constraint {} and all its deps",
            mod_metadata.name,
            mod_metadata.providers,
            mod_metadata.version
        )
    }

    fn get_dependent_mods(&self, mod_name: &str) -> BTreeSet<String> {
        let mut dependent_mods = BTreeSet::new();

        for (pinned_mod_name, pinned_mod) in self.mods.iter() {
            if let Some(deps) = &pinned_mod.deps {
                for dep in deps.iter() {
                    if dep.name == mod_name {
                        dependent_mods.insert(pinned_mod_name.clone());
                    }
                }
            }
        }
        dependent_mods
    }

    pub fn remove_mod(
        &mut self,
        mod_name: &str,
        pack_metadata: &ModpackMeta,
        force: bool,
    ) -> Result<()> {
        if !self.mods.contains_key(mod_name) {
            eprintln!(
                "Skipping removing non-existent mod {} from modpack",
                mod_name
            );
            return Ok(());
        }
        let dependent_mods = self.get_dependent_mods(mod_name);

        if dependent_mods.len() > 0 {
            if force {
                println!("Forcefully removing mod {} even though it is depended on by the following mods:\n{:#?}", mod_name, dependent_mods);
            } else {
                anyhow::bail!(
                    "Cannot remove mod {}.The following mods depend on it:\n{:#?}",
                    mod_name,
                    dependent_mods
                )
            }
        }
        let removed_mod = self.mods.remove(mod_name);
        if let Some(removed_mod) = removed_mod {
            println!("Removed mod {}@{}", mod_name, removed_mod.version);
        }
        self.prune_mods(pack_metadata)?;
        Ok(())
    }

    /// Remove all mods from lockfile that aren't in the pack metadata or depended on by another mod
    fn prune_mods(&mut self, pack_metadata: &ModpackMeta) -> Result<()> {
        let mods_to_remove: BTreeSet<String> = self
            .mods
            .keys()
            .filter(|mod_name| {
                !pack_metadata.mods.contains_key(*mod_name)
                    && self.get_dependent_mods(mod_name).len() == 0
            })
            .map(|mod_name| mod_name.into())
            .collect();

        for mod_name in mods_to_remove {
            let removed_mod = self.mods.remove(&mod_name);
            if let Some(removed_mod) = removed_mod {
                println!("Pruned mod {}@{}", mod_name, removed_mod.version);
            }
        }

        Ok(())
    }

    pub async fn init(
        &mut self,
        modpack_meta: &ModpackMeta,
        ignore_transitive_versions: bool,
    ) -> Result<()> {
        for mod_meta in modpack_meta.iter_mods() {
            self.pin_mod_and_deps(mod_meta, modpack_meta, ignore_transitive_versions)
                .await?;
        }
        Ok(())
    }

    pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        std::fs::write(
            path,
            toml::to_string(self).expect("Pinned pack meta should be serializable"),
        )?;
        // println!("Saved modpack.lock to {}", path.display());
        Ok(())
    }

    pub fn save_current_dir_lock(&self) -> Result<()> {
        self.save_to_dir(&std::env::current_dir()?)?;
        Ok(())
    }

    pub fn save_to_dir(&self, dir: &PathBuf) -> Result<()> {
        let modpack_lock_file_path = dir.join(PathBuf::from(MODPACK_LOCK_FILENAME));
        self.save_to_file(&modpack_lock_file_path)?;
        Ok(())
    }

    pub async fn load_from_directory(
        directory: &Path,
        ignore_transitive_versions: bool,
    ) -> Result<Self> {
        let modpack_lock_file_path = directory.join(PathBuf::from(MODPACK_LOCK_FILENAME));
        if !modpack_lock_file_path.exists() {
            let mut new_modpack_lock = Self::new();
            new_modpack_lock
                .init(
                    &ModpackMeta::load_from_directory(directory)?,
                    ignore_transitive_versions,
                )
                .await?;
            return Ok(new_modpack_lock);
        };
        let modpack_lock_contents = std::fs::read_to_string(modpack_lock_file_path)?;
        Ok(toml::from_str(&modpack_lock_contents)?)
    }

    pub async fn load_from_current_directory(ignore_transitive_versions: bool) -> Result<Self> {
        Self::load_from_directory(&std::env::current_dir()?, ignore_transitive_versions).await
    }

    /// Load a pack from a git repo cloned to a temporary directory
    pub async fn load_from_git_repo(
        git_url: &str,
        ignore_transitive_versions: bool,
    ) -> Result<(Self, tempfile::TempDir)> {
        let pack_dir = tempfile::tempdir()?;
        println!(
            "Cloning modpack from git repo {} to {:#?}...",
            git_url,
            pack_dir.path()
        );
        let _repo = git2::Repository::clone(git_url, pack_dir.path())?;

        let modpack_meta = ModpackMeta::load_from_directory(pack_dir.path())?;
        let pinned_pack_meta =
            PinnedPackMeta::load_from_directory(pack_dir.path(), ignore_transitive_versions)
                .await?;

        println!(
            "Loaded modpack '{}' (MC {} - {}) from git",
            modpack_meta.pack_name,
            modpack_meta.mc_version,
            modpack_meta.modloader.to_string()
        );

        Ok((pinned_pack_meta, pack_dir))
    }
}

use crate::{
    file_meta::{get_normalized_relative_path, FileApplyPolicy, FileMeta},
    mod_meta::{ModMeta, ModProvider},
    providers::DownloadSide,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

const MODPACK_FILENAME: &str = "modpack.toml";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum ModLoader {
    Forge,
    Fabric,
}

impl ToString for ModLoader {
    fn to_string(&self) -> String {
        match self {
            ModLoader::Forge => "Forge",
            ModLoader::Fabric => "Fabric",
        }
        .into()
    }
}

impl std::str::FromStr for ModLoader {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Fabric" => Ok(Self::Fabric),
            "Forge" => Ok(Self::Forge),
            _ => anyhow::bail!("Invalid mod launcher: {}", s),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModpackMeta {
    /// The name of the modpack
    pub pack_name: String,
    /// The intended minecraft version on which this pack should run
    pub mc_version: String,
    /// The default modloader for the modpack
    pub modloader: ModLoader,
    /// Map of mod name -> mod metadata
    pub mods: BTreeMap<String, ModMeta>,
    /// Mapping of relative paths to files to copy over from the modpack
    pub files: Option<BTreeMap<String, FileMeta>>,
    /// Default provider for newly added mods in the modpack
    pub default_providers: Vec<ModProvider>,
    /// A set of forbidden mods in the modpack
    pub forbidden_mods: BTreeSet<String>,
}

impl ModpackMeta {
    pub fn new(pack_name: &str, mc_version: &str, modloader: ModLoader) -> Self {
        Self {
            pack_name: pack_name.into(),
            mc_version: mc_version.into(),
            modloader: modloader,
            ..Default::default()
        }
    }

    pub fn iter_mods(&self) -> std::collections::btree_map::Values<String, ModMeta> {
        self.mods.values().into_iter()
    }

    pub fn load_from_directory(directory: &Path) -> Result<Self> {
        let modpack_meta_file_path = directory.join(PathBuf::from(MODPACK_FILENAME));
        if !modpack_meta_file_path.exists() {
            anyhow::bail!(
                "Directory '{}' does not seem to be a valid modpack project directory.",
                directory.display()
            )
        };
        let modpack_contents = std::fs::read_to_string(modpack_meta_file_path)?;
        Ok(toml::from_str(&modpack_contents)?)
    }

    pub fn load_from_current_directory() -> Result<Self> {
        Self::load_from_directory(&std::env::current_dir()?)
    }

    pub fn provider(mut self, provider: ModProvider) -> Self {
        if !self.default_providers.contains(&provider) {
            self.default_providers.push(provider);
        }
        self
    }

    pub fn add_mod(mut self, mod_meta: &ModMeta) -> Result<Self> {
        if self.forbidden_mods.contains(&mod_meta.name) {
            anyhow::bail!("Cannot add forbidden mod {} to modpack", mod_meta.name)
        } else {
            self.mods
                .insert(mod_meta.name.to_string(), mod_meta.clone());
        }
        Ok(self)
    }

    pub fn forbid_mod(&mut self, mod_name: &str) {
        self.forbidden_mods.insert(mod_name.into());
        println!("Mod {} has been forbidden from the modpack", mod_name);
    }

    pub fn remove_mod(mut self, mod_name: &str) -> Self {
        self.mods.remove(mod_name);
        self
    }

    /// Add local files or folders to the pack. These should be committed to version control
    pub fn add_file(
        &mut self,
        file_path: &Path,
        file_meta: &FileMeta,
        pack_root: &Path,
    ) -> Result<&mut Self> {
        let relative_path = if file_path.is_relative() {
            file_path
        } else {
            &pathdiff::diff_paths(file_path, pack_root).ok_or(anyhow::format_err!(
                "Cannot get relative path of {} in {}",
                file_path.display(),
                pack_root.display()
            ))?
        };
        let full_path = pack_root.join(relative_path);

        // Make sure this path is consistent across platforms
        let relative_path = get_normalized_relative_path(relative_path, &pack_root)?;

        if !full_path
            .canonicalize()?
            .starts_with(pack_root.canonicalize()?)
        {
            anyhow::bail!(
                "You cannot add local files to the modpack from outside the pack source directory. {} is not contained in {}",
                full_path.canonicalize()?.display(),
                pack_root.canonicalize()?.display()
            );
        }

        match &mut self.files {
            Some(files) => {
                files.insert(relative_path.clone(), file_meta.clone());
            }
            None => {
                self.files
                    .insert(BTreeMap::new())
                    .insert(relative_path.clone(), file_meta.clone());
            }
        }

        println!(
            "Added file '{relative_path}' -> '{}' to modpack...",
            file_meta.target_path
        );

        Ok(self)
    }

    pub fn remove_file(&mut self, file_path: &PathBuf, pack_root: &Path) -> Result<&mut Self> {
        let relative_path = get_normalized_relative_path(&file_path, pack_root)?;
        if let Some(files) = &mut self.files {
            let removed = files.remove(&relative_path);
            if let Some(removed) = removed {
                println!(
                    "Removed file '{relative_path}' -> '{}' from modpack...",
                    removed.target_path
                );
            }
        }
        Ok(self)
    }

    /// Installs all the manual files from the pack into the specified directory
    ///
    /// Files/Folders are added if they don't exist if the policy is set to `FileApplyPolicy::Once`.
    /// Otherwise, files/folders are always overwritten.
    ///
    /// Files/Folders, when applied, will ensure that the exact contents of that file or folder match in the instance folder
    /// Ie. If a folder is being applied, any files in that folder not in the modpack will be removed
    pub fn install_files(
        &self,
        pack_dir: &Path,
        instance_dir: &Path,
        side: DownloadSide,
    ) -> Result<()> {
        println!(
            "Applying modpack files: {} -> {}...",
            pack_dir.display(),
            instance_dir.display()
        );
        if let Some(files) = &self.files {
            for (rel_path, file_meta) in files {
                let source_path = pack_dir.join(rel_path);
                let target_path = instance_dir.join(&file_meta.target_path);
                if !side.contains(file_meta.side) {
                    println!(
                        "Skipping apply of {} -> {}. (Applies for side={}, current side={})",
                        source_path.display(),
                        target_path.display(),
                        file_meta.side.to_string(),
                        side.to_string()
                    );
                    continue;
                }
                if target_path.exists() && file_meta.apply_policy == FileApplyPolicy::Once {
                    println!(
                        "Skipping apply of {} -> {}. (Already applied once)",
                        source_path.display(),
                        target_path.display(),
                    );
                    continue;
                }

                // Otherwise, this file/folder needs to be applied
                if source_path.is_dir() {
                    // Sync a folder
                    if target_path.exists() {
                        println!(
                            "Syncing and overwriting existing directory {} -> {}",
                            source_path.display(),
                            target_path.display(),
                        );
                        std::fs::remove_dir_all(&target_path)?;
                    }
                }
                self.copy_files(&source_path, &target_path)?;
            }
        }
        Ok(())
    }

    fn copy_files(&self, src: &Path, dst: &Path) -> Result<()> {
        if src.is_dir() {
            std::fs::create_dir_all(dst)?;
            for entry in std::fs::read_dir(src)? {
                let entry = entry?;
                let src_path = entry.path();
                let dst_path = dst.join(entry.file_name());
                self.copy_files(&src_path, &dst_path)?;
            }
        } else {
            let parent_dir = dst.parent();
            if let Some(parent_dir) = parent_dir {
                std::fs::create_dir_all(parent_dir)?;
            }
            println!("Syncing file {} -> {}", src.display(), dst.display());
            std::fs::copy(src, dst)?;
        }

        Ok(())
    }

    pub fn init_project(&self, directory: &Path) -> Result<()> {
        let modpack_meta_file_path = directory.join(PathBuf::from(MODPACK_FILENAME));
        if modpack_meta_file_path.exists() {
            anyhow::bail!(
                "{MODPACK_FILENAME} already exists at {}",
                modpack_meta_file_path.display()
            )
        }

        self.save_to_file(&modpack_meta_file_path)?;
        println!("MC modpack project initialized at {}", directory.display());
        Ok(())
    }

    pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        std::fs::write(
            path,
            toml::to_string(self).expect("MC Modpack Meta should be serializable"),
        )?;
        // println!("Saved modpack metadata to {}", path.display());
        Ok(())
    }

    pub fn save_current_dir_project(&self) -> Result<()> {
        let modpack_meta_file_path = std::env::current_dir()?.join(PathBuf::from(MODPACK_FILENAME));
        self.save_to_file(&modpack_meta_file_path)?;
        Ok(())
    }
}

impl std::default::Default for ModpackMeta {
    fn default() -> Self {
        Self {
            pack_name: "my_modpack".into(),
            mc_version: "1.20.1".into(),
            modloader: ModLoader::Forge,
            mods: Default::default(),
            files: Default::default(),
            default_providers: vec![ModProvider::Modrinth],
            forbidden_mods: Default::default(),
        }
    }
}

use crate::{
    file_meta::{get_normalized_relative_path, FileMeta},
    mod_meta::{ModMeta, ModProvider},
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

        let target_path = PathBuf::from(&file_meta.target_path);
        if !target_path.is_relative() {
            anyhow::bail!(
                "Target path {} for file {} is not relative!",
                file_meta.target_path,
                file_path.display()
            );
        }

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

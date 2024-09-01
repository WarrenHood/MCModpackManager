use crate::mod_meta::{ModMeta, ModProvider};
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
    pub pack_name: String,
    pub mc_version: String,
    pub modloader: ModLoader,
    pub mods: BTreeMap<String, ModMeta>,
    pub default_providers: Vec<ModProvider>,
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
            default_providers: vec![ModProvider::Modrinth],
            forbidden_mods: Default::default(),
        }
    }
}

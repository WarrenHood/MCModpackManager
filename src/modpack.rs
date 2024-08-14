use std::{borrow::BorrowMut, error::Error, path::PathBuf};

use serde::{Deserialize, Serialize};

const MODPACK_FILENAME: &str = "modpack.toml";

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum ModProvider {
    /// Get mods from CurseForge
    CurseForge,
    /// Get mods from Modrinth
    Modrinth,
    /// Get mods from anywhere on the internet. Note: A download url is needed for this
    Raw,
}

impl std::str::FromStr for ModProvider {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "curseforge" => Ok(ModProvider::CurseForge),
            "modrinth" => Ok(ModProvider::Modrinth),
            "raw" => Ok(ModProvider::Raw),
            _ => Err(format!("Invalid mod launcher: {}", s)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ModMeta {
    mod_name: String,
    version: String,
    providers: Option<Vec<ModProvider>>,
    download_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq)]
struct ModMetaBuilder {
    mod_name: String,
    version: String,
    providers: Option<Vec<ModProvider>>,
    download_url: Option<String>,
}

impl ModMeta {
    pub fn new(mod_name: &str) -> Result<Self, Box<dyn Error>> {
        if mod_name.contains("@") {
            let mod_name_and_version: Vec<&str> = mod_name.split("@").collect();
            if mod_name_and_version.len() != 2 {
                return Err(format!("Invalid mod with version constraint: '{}'", &mod_name).into());
            }
            return Ok(Self {
                mod_name: mod_name_and_version[0].into(),
                version: mod_name_and_version[1].into(),
                ..Default::default()
            });
        }
        Ok(Self {
            mod_name: mod_name.into(),
            ..Default::default()
        })
    }

    pub fn provider(mut self, provider: ModProvider) -> Self {
        if let Some(providers) = self.providers.borrow_mut() {
            if !providers.contains(&provider) {
                providers.push(provider)
            }
        } else {
            self.providers = Some(vec![provider]);
        }
        self
    }

    pub fn url(mut self, download_url: &str) -> Self {
        self.download_url = Some(download_url.into());
        self
    }

    pub fn version(mut self, version_constraint: &str) -> Self {
        self.version = version_constraint.into();
        self
    }
}

impl Default for ModMeta {
    fn default() -> Self {
        Self {
            mod_name: Default::default(),
            version: "*".into(),
            providers: None,
            download_url: Default::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Fabric" => Ok(Self::Fabric),
            "Forge" => Ok(Self::Forge),
            _ => Err(format!("Invalid mod launcher: {}", s)),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ModpackMeta {
    pack_name: String,
    mc_version: String,
    modloader: ModLoader,
    mods: Vec<ModMeta>,
    default_providers: Vec<ModProvider>,
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

    pub fn load_from_directory(directory: &PathBuf) -> Result<Self, Box<dyn Error>> {
        let modpack_meta_file_path = directory.clone().join(PathBuf::from(MODPACK_FILENAME));
        if !modpack_meta_file_path.exists() {
            return Err(format!(
                "Directory '{}' does not seem to be a valid modpack project directory.",
                directory.display()
            )
            .into());
        };
        let modpack_contents = std::fs::read_to_string(modpack_meta_file_path)?;
        Ok(toml::from_str(&modpack_contents)?)
    }

    pub fn load_from_current_directory() -> Result<Self, Box<dyn Error>> {
        Self::load_from_directory(&std::env::current_dir()?)
    }

    pub fn provider(mut self, provider: ModProvider) -> Self {
        if !self.default_providers.contains(&provider) {
            self.default_providers.push(provider);
        }
        self
    }

    pub fn add_mod(mut self, mod_meta: ModMeta) -> Self {
        if !self.mods.contains(&mod_meta) {
            self.mods = self
                .mods
                .into_iter()
                .filter(|m| m.mod_name != mod_meta.mod_name)
                .collect();
            println!(
                "Adding {}@{} to modpack '{}'...",
                mod_meta.mod_name, mod_meta.version, self.pack_name
            );
            self.mods.push(mod_meta);
        }
        self
    }

    pub fn init_project(&self, directory: &PathBuf) -> Result<(), Box<dyn Error>> {
        let modpack_meta_file_path = directory.clone().join(PathBuf::from(MODPACK_FILENAME));
        if modpack_meta_file_path.exists() {
            return Err(format!(
                "mcmodpack.toml already exists at {}",
                modpack_meta_file_path.display()
            )
            .into());
        }

        self.save_to_file(&modpack_meta_file_path)?;
        println!("MC modpack project initialized at {}", directory.display());
        Ok(())
    }

    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn Error>> {
        std::fs::write(
            path,
            toml::to_string(self).expect("MC Modpack Meta should be serializable"),
        )?;
        println!("Saved modpack metadata to {}", path.display());
        Ok(())
    }

    pub fn save_current_dir_project(&self) -> Result<(), Box<dyn Error>> {
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
        }
    }
}

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{borrow::BorrowMut, error::Error};

use crate::modpack::ModLoader;

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq, Hash, Clone)]
pub enum ModProvider {
    /// Get mods from CurseForge
    CurseForge,
    /// Get mods from Modrinth
    Modrinth,
    /// Get mods from anywhere on the internet. Note: A download url is needed for this
    Raw,
}

impl std::str::FromStr for ModProvider {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "curseforge" => Ok(ModProvider::CurseForge),
            "modrinth" => Ok(ModProvider::Modrinth),
            "raw" => Ok(ModProvider::Raw),
            _ => anyhow::bail!("Invalid mod provider: {}", s),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct ModMeta {
    pub name: String,
    pub version: String,
    pub providers: Option<Vec<ModProvider>>,
    pub mc_version: Option<String>,
    pub loader: Option<ModLoader>,
    pub download_url: Option<String>,
    pub server_side: Option<bool>,
    pub client_side: Option<bool>
}

impl PartialEq for ModMeta {
    fn eq(&self, other: &Self) -> bool {
        // Only compare mod metadata by name and version
        self.name == other.name && self.version == other.version
    }
}

impl Eq for ModMeta {}

impl ModMeta {
    pub fn new(mod_name: &str) -> Result<Self> {
        if mod_name.contains("@") {
            let mod_name_and_version: Vec<&str> = mod_name.split("@").collect();
            if mod_name_and_version.len() != 2 {
                anyhow::bail!("Invalid mod with version constraint: '{}'", &mod_name)
            }
            return Ok(Self {
                name: mod_name_and_version[0].into(),
                version: mod_name_and_version[1].into(),
                ..Default::default()
            });
        }
        Ok(Self {
            name: mod_name.into(),
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

    pub fn modloader(mut self, modloader: ModLoader) -> Self {
        self.loader = Some(modloader);
        self
    }

    pub fn mc_version(mut self, mc_version: &str) -> Self {
        self.mc_version = Some(mc_version.into());
        self
    }
}

impl Default for ModMeta {
    fn default() -> Self {
        Self {
            name: Default::default(),
            version: "*".into(),
            providers: None,
            download_url: Default::default(),
            mc_version: None,
            loader: None,
            server_side: None,
            client_side: None,
        }
    }
}

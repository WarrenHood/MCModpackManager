use std::{borrow::BorrowMut, error::Error};

use serde::{Deserialize, Serialize};

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModMeta {
    pub name: String,
    pub version: String,
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
}

impl Default for ModMeta {
    fn default() -> Self {
        Self {
            name: Default::default(),
            version: "*".into(),
            providers: None,
            download_url: Default::default(),
        }
    }
}
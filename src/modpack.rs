use std::borrow::BorrowMut;

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub enum ModProvider {
    /// Get mods from CurseForge
    CurseForge,
    /// Get mods from Modrinth
    Modrinth,
    /// Get mods from anywhere on the internet. Note: A download url is needed for this
    Raw,
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
    pub fn new(mod_name: &str) -> Self {
        Self {
            mod_name: mod_name.into(),
            ..Default::default()
        }
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
    mc_version: String,
    modloader: ModLoader,
    mods: Vec<ModMeta>,
    default_providers: Vec<ModProvider>,
}

impl ModpackMeta {
    pub fn new(mc_version: &str, modloader: ModLoader) -> Self {
        Self {
            mc_version: mc_version.into(),
            modloader: modloader,
            ..Default::default()
        }
    }

    pub fn provider(mut self, provider: ModProvider) -> Self {
        if !self.default_providers.contains(&provider) {
            self.default_providers.push(provider);
        }
        self
    }

    pub fn add_mod(mut self, mod_meta: ModMeta) -> Self {
        if !self.mods.contains(&mod_meta) {
            self.mods.push(mod_meta);
        }
        self
    }
}

impl std::default::Default for ModpackMeta {
    fn default() -> Self {
        Self {
            mc_version: "1.20.1".into(),
            modloader: ModLoader::Forge,
            mods: Default::default(),
            default_providers: vec![ModProvider::Modrinth],
        }
    }
}

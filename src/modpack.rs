use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ModMeta {
    download_url: String,
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
    pub mc_version: String,
    pub modloader: ModLoader,
    pub mods: Vec<ModMeta>,
}

impl std::default::Default for ModpackMeta {
    fn default() -> Self {
        Self {
            mc_version: "1.20.1".into(),
            modloader: ModLoader::Forge,
            mods: Default::default(),
        }
    }
}

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    error::Error,
    fmt::Display,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{providers::DownloadSide, resolver::PinnedPackMeta};

const CONFIG_DIR_NAME: &str = "mcmpmgr";
const DATA_FILENAME: &str = "data.toml";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PackSource {
    Git { url: String },
    Local { path: PathBuf },
}

impl FromStr for PackSource {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        if s.starts_with("git+") {
            let url = s.trim_start_matches("git+").to_string();
            Ok(PackSource::Git { url })
        } else {
            let path = PathBuf::from(s);
            Ok(PackSource::Local { path })
        }
    }
}

impl Display for PackSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PackSource::Git { url } => write!(f, "git+{url}"),
            PackSource::Local { path } => write!(f, "{}", path.display()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub mods_folder: PathBuf,
    pub pack_source: PackSource,
    pub side: DownloadSide,
}

impl Profile {
    pub fn new(mods_folder: &Path, pack_source: PackSource, side: DownloadSide) -> Self {
        Self {
            mods_folder: mods_folder.into(),
            pack_source,
            side,
        }
    }

    pub async fn install(&self) -> Result<(), Box<dyn Error>> {
        let (pack_lock, temp_dir) = match &self.pack_source {
            PackSource::Git { url } => {
                let (pack_lock, packdir) = PinnedPackMeta::load_from_git_repo(&url, true).await?;
                (pack_lock, Some(packdir))
            }
            PackSource::Local { path } => (
                PinnedPackMeta::load_from_directory(&path, true).await?,
                None,
            ),
        };

        pack_lock
            .download_mods(&self.mods_folder, self.side)
            .await?;
        Ok(())
    }
}

/// User data and configs for the modpack manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data {
    profiles: HashMap<String, Profile>,
}

impl Default for Data {
    fn default() -> Self {
        Self {
            profiles: Default::default(),
        }
    }
}

impl Data {
    pub fn get_profile_names(&self) -> Vec<String> {
        self.profiles.keys().cloned().collect()
    }

    // Add or update a profile
    pub fn add_profile(&mut self, profile_name: &str, profile: Profile) {
        self.profiles.insert(profile_name.into(), profile);
    }

    pub fn get_profile(&self, profile_name: &str) -> Option<&Profile> {
        self.profiles.get(profile_name)
    }

    pub fn get_profile_mut(&mut self, profile_name: &str) -> Option<&mut Profile> {
        self.profiles.get_mut(profile_name)
    }

    pub fn remove_profile(&mut self, profile_name: &str) {
        self.profiles.remove(profile_name);
    }

    fn get_config_folder_path() -> Result<PathBuf, Box<dyn Error>> {
        home::home_dir()
            .and_then(|home_dir| Some(home_dir.join(format!(".config/{CONFIG_DIR_NAME}"))))
            .ok_or("Unable to locate home directory".into())
    }

    pub fn load() -> Result<Self, Box<dyn Error>> {
        let config_dir = Self::get_config_folder_path()?;
        if !config_dir.exists() {
            println!("Creating config directory {config_dir:#?}...");
            std::fs::create_dir_all(&config_dir)?;
        }

        let datafile = config_dir.join(DATA_FILENAME);

        Ok(if !datafile.exists() {
            Self::default()
        } else {
            let data_string = std::fs::read_to_string(datafile)?;
            toml::from_str(&data_string)?
        })
    }

    pub fn save(&self) -> Result<(), Box<dyn Error>> {
        let config_dir = Self::get_config_folder_path()?;
        if !config_dir.exists() {
            println!("Creating config directory {config_dir:#?}...");
            std::fs::create_dir_all(&config_dir)?;
        }

        let datafile = config_dir.join(DATA_FILENAME);
        std::fs::write(datafile, toml::to_string(self)?)?;
        println!("Saved user profiles configuration");
        Ok(())
    }
}

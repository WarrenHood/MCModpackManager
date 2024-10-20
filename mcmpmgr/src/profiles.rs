use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt::Display,
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::{modpack::ModpackMeta, providers::DownloadSide, resolver::PinnedPackMeta};

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
            let path = PathBuf::from(s).canonicalize();
            match path {
                Ok(path) => Ok(PackSource::Local { path }),
                Err(e) => Err(e.to_string())
            }
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
    pub instance_folder: PathBuf,
    pub pack_source: PackSource,
    pub side: DownloadSide,
}

impl Profile {
    pub fn new(
        instance_folder: &Path,
        pack_source: PackSource,
        side: DownloadSide,
    ) -> Result<Self> {
        Ok(Self {
            instance_folder: instance_folder.canonicalize()?,
            pack_source,
            side,
        })
    }

    pub async fn install(&self) -> Result<()> {
        let (pack_lock, pack_directory, _temp_dir) = match &self.pack_source {
            PackSource::Git { url } => {
                let (pack_lock, packdir) = PinnedPackMeta::load_from_git_repo(&url, true).await?;
                let pack_path = packdir.path().to_path_buf();
                (pack_lock, pack_path, Some(packdir))
            }
            PackSource::Local { path } => (
                PinnedPackMeta::load_from_directory(&path, true).await?,
                path.to_path_buf(),
                None,
            ),
        };
        let modpack_meta = ModpackMeta::load_from_directory(&pack_directory)?;
        modpack_meta.install_files(&pack_directory, &self.instance_folder, self.side)?;

        pack_lock
            .download_mods(&self.instance_folder.join("mods"), self.side)
            .await?;
        Ok(())
    }
}

/// User data and configs for the modpack manager
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Data {
    profiles: BTreeMap<String, Profile>,
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

    fn get_config_folder_path() -> Result<PathBuf> {
        let home_dir = home::home_dir()
            .and_then(|home_dir| Some(home_dir.join(format!(".config/{CONFIG_DIR_NAME}"))));

        if let Some(home_dir) = home_dir {
            Ok(home_dir)
        } else {
            anyhow::bail!("Unable to locate home directory")
        }
    }

    pub fn load() -> Result<Self> {
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

    pub fn save(&self) -> Result<()> {
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

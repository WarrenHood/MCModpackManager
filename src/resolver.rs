use serde::{Deserialize, Serialize};
use std::{collections::HashMap, error::Error, path::PathBuf};

use crate::{mod_meta::ModMeta, modpack::ModpackMeta};

const MODPACK_LOCK_FILENAME: &str = "modpack.lock";

#[derive(Serialize, Deserialize)]
enum FileSource {
    Download { url: String },
    Local { path: PathBuf },
}

#[derive(Serialize, Deserialize)]
struct PinnedMod {
    /// Source of the file
    source: FileSource,
    /// SHA1 Hash
    sha1: String,
    /// SHA512 Hash
    sha512: String,
    /// Version of mod
    version: String,
}

impl PinnedMod {
    pub fn resolve_mod(mod_metadata: &ModMeta) -> Self {
        // TODO: Actually implement this
        Self {
            source: FileSource::Download {
                url: format!("https://fake.url/mods/{}", mod_metadata.name),
            },
            sha1: "FakeSha1".into(),
            sha512: "FakeSha512".into(),
            version: if mod_metadata.version == "*" {
                "1.0.0-fake-latest-version".into()
            } else {
                mod_metadata.version.clone()
            },
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct PinnedPackMeta {
    mods: HashMap<String, PinnedMod>,
}

impl PinnedPackMeta {
    pub fn new() -> Self {
        Self {
            mods: Default::default(),
        }
    }

    pub fn pin_mod(&mut self, mod_metadata: &ModMeta) -> &mut Self {
        let pinned_mod = PinnedMod::resolve_mod(mod_metadata);
        self.mods.insert(mod_metadata.name.clone(), pinned_mod);
        self
    }

    pub fn init(&mut self, modpack_meta: &ModpackMeta) {
        modpack_meta.iter_mods().for_each(|m| {
            self.pin_mod(m);
        });
    }

    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn Error>> {
        std::fs::write(
            path,
            toml::to_string(self).expect("Pinned pack meta should be serializable"),
        )?;
        println!("Saved modpack.lock to {}", path.display());
        Ok(())
    }

    pub fn save_current_dir_lock(&self) -> Result<(), Box<dyn Error>> {
        let modpack_lock_file_path =
            std::env::current_dir()?.join(PathBuf::from(MODPACK_LOCK_FILENAME));
        self.save_to_file(&modpack_lock_file_path)?;
        Ok(())
    }

    pub fn load_from_directory(directory: &PathBuf) -> Result<Self, Box<dyn Error>> {
        let modpack_lock_file_path = directory.clone().join(PathBuf::from(MODPACK_LOCK_FILENAME));
        if !modpack_lock_file_path.exists() {
            let mut new_modpack_lock = Self::new();
            new_modpack_lock.init(&ModpackMeta::load_from_directory(directory)?);
            return Ok(new_modpack_lock);
        };
        let modpack_lock_contents = std::fs::read_to_string(modpack_lock_file_path)?;
        Ok(toml::from_str(&modpack_lock_contents)?)
    }

    pub fn load_from_current_directory() -> Result<Self, Box<dyn Error>> {
        Self::load_from_directory(&std::env::current_dir()?)
    }
}

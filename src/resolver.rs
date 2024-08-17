use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    error::Error,
    path::PathBuf,
};

use crate::{
    mod_meta::{ModMeta, ModProvider},
    modpack::ModpackMeta,
    providers::{modrinth::Modrinth, PinnedMod},
};

const MODPACK_LOCK_FILENAME: &str = "modpack.lock";

#[derive(Serialize, Deserialize)]
pub struct PinnedPackMeta {
    mods: HashMap<String, PinnedMod>,
    #[serde(skip_serializing, skip_deserializing)]
    modrinth: Modrinth,
}

impl PinnedPackMeta {
    pub fn new() -> Self {
        Self {
            mods: Default::default(),
            modrinth: Modrinth::new(),
        }
    }

    pub async fn pin_mod_and_deps(
        &mut self,
        mod_metadata: &ModMeta,
        pack_metadata: &ModpackMeta,
    ) -> Result<(), Box<dyn Error>> {
        if let Some(mod_meta) = self.mods.get(&mod_metadata.name) {
            if mod_metadata.version != "*"
                && semver::Version::parse(&mod_metadata.version)? == mod_meta.version
            {
                // Skip already pinned mods
                // TODO: Replace * with the current mod version in the modpack meta so this doesn't get called twice for the first mod created
                return Ok(());
            }
        }
        let mut deps =
            HashSet::from_iter(self.pin_mod(mod_metadata, pack_metadata).await?.into_iter());

        let pinned_version = self
            .mods
            .get(&mod_metadata.name)
            .expect("should be in pinned mods")
            .version
            .clone();

        while !deps.is_empty() {
            let mut next_deps = HashSet::new();
            for dep in deps.iter() {
                println!(
                    "Adding mod {}@{} (dependency of {}@{})",
                    dep.name, dep.version, mod_metadata.name, pinned_version
                );
                next_deps.extend(self.pin_mod(dep, &pack_metadata).await?);
            }
            deps = next_deps;
        }

        Ok(())
    }

    /// Pin a mod version
    ///
    /// A list of dependencies to pin is included
    pub async fn pin_mod(
        &mut self,
        mod_metadata: &ModMeta,
        pack_metadata: &ModpackMeta,
    ) -> Result<Vec<ModMeta>, Box<dyn Error>> {
        let mod_providers = if let Some(mod_providers) = &mod_metadata.providers {
            mod_providers
        } else {
            &vec![]
        };
        let mut checked_providers: HashSet<ModProvider> = HashSet::new();
        for mod_provider in mod_providers
            .iter()
            .chain(pack_metadata.default_providers.iter())
        {
            if checked_providers.contains(&mod_provider) {
                // No need to repeat a check for a provider if we have already checked it
                continue;
            }
            checked_providers.insert(mod_provider.clone());
            match mod_provider {
                crate::mod_meta::ModProvider::CurseForge => unimplemented!(),
                crate::mod_meta::ModProvider::Modrinth => {
                    let pinned_mod = self.modrinth.resolve(&mod_metadata, pack_metadata).await;
                    if let Ok(pinned_mod) = pinned_mod {
                        self.mods
                            .insert(mod_metadata.name.clone(), pinned_mod.clone());
                        println!("Pinned {}@{}", mod_metadata.name, pinned_mod.version);
                        if let Some(deps) = &pinned_mod.deps {
                            return Ok(deps
                                .iter()
                                .filter(|d| !self.mods.contains_key(&d.name))
                                .cloned()
                                .collect());
                        }
                        return Ok(vec![]);
                    } else if let Err(e) = pinned_mod {
                        eprintln!(
                            "Failed to resolve {}@{} with provider {:#?}: {}",
                            mod_metadata.name, mod_metadata.version, mod_provider, e
                        );
                    }
                }
                crate::mod_meta::ModProvider::Raw => unimplemented!(),
            };
        }

        Err(format!(
            "Failed to pin mod '{}' (providers={:#?}) with constraint {} and all its deps",
            mod_metadata.name, mod_metadata.providers, mod_metadata.version
        )
        .into())
    }

    fn get_dependent_mods(&self, mod_name: &str) -> HashSet<String> {
        let mut dependent_mods = HashSet::new();

        for (pinned_mod_name, pinned_mod) in self.mods.iter() {
            if let Some(deps) = &pinned_mod.deps {
                for dep in deps.iter() {
                    if dep.name == mod_name {
                        dependent_mods.insert(pinned_mod_name.clone());
                    }
                }
            }
        }
        dependent_mods
    }

    pub fn remove_mod(
        &mut self,
        mod_name: &str,
        pack_metadata: &ModpackMeta,
    ) -> Result<(), Box<dyn Error>> {
        let dependent_mods = self.get_dependent_mods(mod_name);

        if dependent_mods.len() > 0 {
            return Err(format!(
                "Cannot remove mod {}.The following mods depend on it:\n{:#?}",
                mod_name, dependent_mods
            )
            .into());
        }
        let removed_mod = self.mods.remove(mod_name);
        if let Some(removed_mod) = removed_mod {
            println!("Removed mod {}@{}", mod_name, removed_mod.version);
        }
        self.prune_mods(pack_metadata)?;
        Ok(())
    }

    /// Remove all mods from lockfile that aren't in the pack metadata or depended on by another mod
    fn prune_mods(&mut self, pack_metadata: &ModpackMeta) -> Result<(), Box<dyn Error>> {
        let mods_to_remove: HashSet<String> = self
            .mods
            .keys()
            .filter(|mod_name| {
                !pack_metadata.mods.contains_key(*mod_name)
                    && self.get_dependent_mods(mod_name).len() == 0
            })
            .map(|mod_name| mod_name.into())
            .collect();

        for mod_name in mods_to_remove {
            let removed_mod = self.mods.remove(&mod_name);
            if let Some(removed_mod) = removed_mod {
                println!("Pruned mod {}@{}", mod_name, removed_mod.version);
            }
        }

        Ok(())
    }

    pub async fn init(&mut self, modpack_meta: &ModpackMeta) -> Result<(), Box<dyn Error>> {
        for mod_meta in modpack_meta.iter_mods() {
            self.pin_mod_and_deps(mod_meta, modpack_meta).await?;
        }
        Ok(())
    }

    pub fn save_to_file(&self, path: &PathBuf) -> Result<(), Box<dyn Error>> {
        std::fs::write(
            path,
            toml::to_string(self).expect("Pinned pack meta should be serializable"),
        )?;
        // println!("Saved modpack.lock to {}", path.display());
        Ok(())
    }

    pub fn save_current_dir_lock(&self) -> Result<(), Box<dyn Error>> {
        self.save_to_dir(&std::env::current_dir()?)?;
        Ok(())
    }

    pub fn save_to_dir(&self, dir: &PathBuf) -> Result<(), Box<dyn Error>> {
        let modpack_lock_file_path = dir.join(PathBuf::from(MODPACK_LOCK_FILENAME));
        self.save_to_file(&modpack_lock_file_path)?;
        Ok(())
    }

    pub async fn load_from_directory(directory: &PathBuf) -> Result<Self, Box<dyn Error>> {
        let modpack_lock_file_path = directory.clone().join(PathBuf::from(MODPACK_LOCK_FILENAME));
        if !modpack_lock_file_path.exists() {
            let mut new_modpack_lock = Self::new();
            new_modpack_lock
                .init(&ModpackMeta::load_from_directory(directory)?)
                .await?;
            return Ok(new_modpack_lock);
        };
        let modpack_lock_contents = std::fs::read_to_string(modpack_lock_file_path)?;
        Ok(toml::from_str(&modpack_lock_contents)?)
    }

    pub async fn load_from_current_directory() -> Result<Self, Box<dyn Error>> {
        Self::load_from_directory(&std::env::current_dir()?).await
    }
}

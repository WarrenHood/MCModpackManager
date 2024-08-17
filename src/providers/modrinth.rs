use serde::{Deserialize, Serialize};
use std::error::Error;

use super::PinnedMod;
use crate::{mod_meta::ModMeta, modpack::ModpackMeta, providers::FileSource};

pub struct Modrinth {
    client: reqwest::Client,
}

#[derive(Serialize, Deserialize, Debug)]
struct VersionDeps {
    dependency_type: String,
    project_id: String,
    file_name: Option<String>,
    version_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
struct VersionHashes {
    sha1: String,
    sha512: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct VersionFiles {
    // file_type: String,
    filename: String,
    hashes: VersionHashes,
    // primary: bool,
    // size: i64,
    url: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct ModrinthProjectVersion {
    author_id: String,
    date_published: String,
    dependencies: Vec<VersionDeps>,
    // downloads: i64,
    files: Vec<VersionFiles>,
    // loaders: Vec<String>,
    // name: String,
    // project_id: String,
    version_number: semver::Version,
    // version_type: String,
}

impl Modrinth {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    /// Resolve a list of mod candidates in order of newest to oldest
    pub async fn resolve(
        &self,
        mod_meta: &ModMeta,
        pack_meta: &ModpackMeta,
    ) -> Result<PinnedMod, Box<dyn Error>> {
        let mut versions = self.get_project_versions(&mod_meta.name, pack_meta).await?;
        versions.sort_by_key(|v| v.version_number.clone());
        versions.reverse();

        let package = if mod_meta.version == "*" {
            versions.last().ok_or(format!(
                "Cannot find package {} for loader={} and mc version={}",
                mod_meta.name,
                pack_meta.modloader.to_string().to_lowercase(),
                pack_meta.mc_version
            ))?
        } else {
            let expected_version = semver::Version::parse(&mod_meta.version)?;
            versions
                .iter()
                .filter(|v| v.version_number == expected_version)
                .nth(0)
                .ok_or(format!(
                    "Cannot find package {}@{}",
                    mod_meta.name, mod_meta.version
                ))?
        };

        Ok(PinnedMod {
            source: package
                .files
                .iter()
                .map(|f| FileSource::Download {
                    url: f.url.clone(),
                    sha1: f.hashes.sha1.clone(),
                    sha512: f.hashes.sha512.clone(),
                })
                .collect(),
            version: package.version_number.clone(),
            deps: None, // TODO: Get deps
        })
    }

    async fn get_project_versions(
        &self,
        mod_id: &str,
        pack_meta: &ModpackMeta,
    ) -> Result<Vec<ModrinthProjectVersion>, Box<dyn Error>> {
        let project_Versions: Vec<ModrinthProjectVersion> = self
            .client
            .get(format!(
                "https://api.modrinth.com/v2/project/{mod_id}/version"
            ))
            .query(&[
                (
                    "loaders",
                    format!("[\"{}\"]", pack_meta.modloader.to_string().to_lowercase()),
                ),
                ("game_versions", format!("[\"{}\"]", pack_meta.mc_version)),
            ])
            .send()
            .await?
            .json()
            .await?;

        Ok(project_Versions)
    }
}

impl Default for Modrinth {
    fn default() -> Self {
        Self {
            client: Default::default(),
        }
    }
}

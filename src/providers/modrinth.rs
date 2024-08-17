use serde::{Deserialize, Serialize};
use std::{collections::HashSet, error::Error};

use super::PinnedMod;
use crate::{
    mod_meta::{ModMeta, ModProvider},
    modpack::ModpackMeta,
    providers::FileSource,
};

pub struct Modrinth {
    client: reqwest::Client,
}

#[derive(Serialize, Deserialize)]
struct DonationUrls1 {
    id: String,
    platform: String,
    url: String,
}
#[derive(Serialize, Deserialize)]
struct Gallery1 {
    created: String,
    description: String,
    featured: bool,
    ordering: i64,
    title: String,
    url: String,
}
#[derive(Serialize, Deserialize)]
struct License1 {
    id: String,
    name: String,
    url: String,
}
#[derive(Serialize, Deserialize)]
struct ModrinthProject {
    slug: String,
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
    name: String,
    project_id: String,
    id: String,
    version_number: semver::Version,
    // version_type: String,
}

impl Modrinth {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub async fn get_project_slug(&self, project_id: &str) -> Result<String, Box<dyn Error>> {
        let mut project: ModrinthProject = self
            .client
            .get(format!("https://api.modrinth.com/v2/project/{project_id}"))
            .send()
            .await?
            .json()
            .await?;

        Ok(project.slug)
    }

    pub async fn get_mod_meta(
        &self,
        project_id: &str,
        project_version: Option<&str>,
        pack_meta: &ModpackMeta,
    ) -> Result<ModMeta, Box<dyn Error>> {
        let project_versions = self.get_project_versions(project_id, pack_meta).await?;
        let project_slug = self.get_project_slug(project_id).await?;

        for version in project_versions.into_iter() {
            if project_version.is_none() || project_version.unwrap_or("*") == version.id {
                return Ok(ModMeta::new(&project_slug)?
                    .provider(ModProvider::Modrinth)
                    .version(&version.version_number.to_string()));
            }
        }
        Err(format!(
            "Couldn't find project '{}' with version '{}'",
            project_id,
            project_version.unwrap_or("*")
        )
        .into())
    }

    /// Resolve a list of mod candidates in order of newest to oldest
    pub async fn resolve(
        &self,
        mod_meta: &ModMeta,
        pack_meta: &ModpackMeta,
    ) -> Result<PinnedMod, Box<dyn Error>> {
        let versions = self.get_project_versions(&mod_meta.name, pack_meta).await?;

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

        let mut deps_meta = HashSet::new();
        for dep in package
            .dependencies
            .iter()
            .filter(|dep| dep.dependency_type == "required")
        {
            deps_meta.insert(
                self.get_mod_meta(&dep.project_id, dep.version_id.as_deref(), pack_meta)
                    .await?,
            );
        }

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
            deps: if package.dependencies.len() > 0 {
                Some(deps_meta)
            } else {
                None
            },
        })
    }

    async fn get_project_versions(
        &self,
        mod_id: &str,
        pack_meta: &ModpackMeta,
    ) -> Result<Vec<ModrinthProjectVersion>, Box<dyn Error>> {
        let mut project_versions: Vec<ModrinthProjectVersion> = self
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

        project_versions.sort_by_key(|v| v.version_number.clone());
        project_versions.reverse();

        Ok(project_versions)
    }
}

impl Default for Modrinth {
    fn default() -> Self {
        Self {
            client: Default::default(),
        }
    }
}

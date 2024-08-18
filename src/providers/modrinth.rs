use serde::{Deserialize, Serialize};
use std::{collections::HashSet, error::Error};

use super::PinnedMod;
use crate::{
    mod_meta::{ModMeta, ModProvider},
    modpack::{ModLoader, ModpackMeta},
    providers::FileSource,
};

pub struct Modrinth {
    client: reqwest::Client,
}

#[derive(Serialize, Deserialize)]
struct ModrinthProject {
    slug: String,
    client_side: String,
    server_side: String,
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
    // author_id: String,
    date_published: String,
    dependencies: Option<Vec<VersionDeps>>,
    // downloads: i64,
    files: Vec<VersionFiles>,
    // loaders: Vec<String>,
    // name: String,
    // project_id: String,
    id: String,
    version_number: String,
    // version_type: String,
}

impl Modrinth {
    pub fn new() -> Self {
        Self {
            ..Default::default()
        }
    }

    pub async fn get_project(&self, project_id: &str) -> Result<ModrinthProject, Box<dyn Error>> {
        let project: ModrinthProject = self
            .client
            .get(format!("https://api.modrinth.com/v2/project/{project_id}"))
            .send()
            .await?
            .json()
            .await?;

        Ok(project)
    }

    pub async fn get_mod_meta(
        &self,
        project_id: &str,
        project_version: Option<&str>,
        pack_meta: &ModpackMeta,
        loader_override: Option<ModLoader>,
        game_version_override: Option<String>,
    ) -> Result<ModMeta, Box<dyn Error>> {
        let project_versions = self
            .get_project_versions(
                project_id,
                pack_meta,
                false, // TODO: Change this to allow specific versions of mods for the wrong version to be installed
                loader_override.clone(),
                game_version_override.clone(),
            )
            .await?;
        let project_slug = self.get_project(project_id).await?.slug;

        for version in project_versions.iter() {
            if project_version.is_none() || project_version.unwrap_or("*") == version.id {
                let mut mod_meta = ModMeta::new(&project_slug)?
                    .provider(ModProvider::Modrinth)
                    .version(&version.version_number.to_string());

                if let Some(loader) = loader_override {
                    mod_meta.loader = Some(loader.clone());
                }

                if let Some(mc_version) = game_version_override {
                    mod_meta = mod_meta.mc_version(&mc_version);
                }

                return Ok(mod_meta);
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
        let versions = self
            .get_project_versions(
                &mod_meta.name,
                pack_meta,
                false,
                mod_meta.loader.clone(),
                mod_meta.mc_version.clone(),
            )
            .await?;

        let package = if mod_meta.version == "*" {
            versions.first().ok_or(format!(
                "Cannot find package {} for loader={} and mc version={}",
                mod_meta.name,
                pack_meta.modloader.to_string().to_lowercase(),
                pack_meta.mc_version
            ))?
        } else {
            versions
                .iter()
                .filter(|v| v.version_number == mod_meta.version)
                .nth(0)
                .ok_or(format!(
                    "Cannot find package {}@{}",
                    mod_meta.name, mod_meta.version
                ))?
        };

        let mut deps_meta = HashSet::new();
        if let Some(deps) = &package.dependencies {
            for dep in deps.iter().filter(|dep| dep.dependency_type == "required") {
                deps_meta.insert(
                    self.get_mod_meta(
                        &dep.project_id,
                        dep.version_id.as_deref(),
                        pack_meta,
                        mod_meta.loader.clone(),
                        mod_meta.mc_version.clone(),
                    )
                    .await?,
                );
            }
        }

        let project = self.get_project(&mod_meta.name).await?;

        Ok(PinnedMod {
            source: package
                .files
                .iter()
                .map(|f| FileSource::Download {
                    url: f.url.clone(),
                    sha1: f.hashes.sha1.clone(),
                    sha512: f.hashes.sha512.clone(),
                    filename: f.filename.clone(),
                })
                .collect(),
            version: package.version_number.clone(),
            deps: if package
                .dependencies
                .as_ref()
                .is_some_and(|deps| deps.len() > 0)
            {
                Some(deps_meta)
            } else {
                None
            },
            server_side: project.server_side != "unsupported",
            client_side: project.client_side != "unsupported",
        })
    }

    async fn get_project_versions(
        &self,
        mod_id: &str,
        pack_meta: &ModpackMeta,
        ignore_game_version_and_loader: bool, // For deps we might as well let them use anything
        loader_override: Option<ModLoader>,
        game_version_override: Option<String>,
    ) -> Result<Vec<ModrinthProjectVersion>, Box<dyn Error>> {
        let loader = loader_override
            .unwrap_or(pack_meta.modloader.clone())
            .to_string()
            .to_lowercase();
        let game_version = game_version_override.unwrap_or(pack_meta.mc_version.clone());
        let query_vec = if ignore_game_version_and_loader {
            &vec![]
        } else {
            &vec![
                ("loaders", format!("[\"{}\"]", loader)),
                ("game_versions", format!("[\"{}\"]", game_version)),
            ]
        };

        let mut project_versions: Vec<ModrinthProjectVersion> = self
            .client
            .get(format!(
                "https://api.modrinth.com/v2/project/{mod_id}/version"
            ))
            .query(query_vec)
            .send()
            .await?
            .json()
            .await?;
        project_versions.sort_by_key(|v| v.date_published.clone());
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

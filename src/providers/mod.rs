use std::{collections::HashSet, path::PathBuf};
use serde::{Deserialize, Serialize};
use crate::mod_meta::ModMeta;

pub mod modrinth;
pub mod raw;

#[derive(Serialize, Deserialize, Clone)]
enum FileSource {
    Download { url: String, sha1: String, sha512: String},
    Local { path: PathBuf, sha1: String, sha512: String },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PinnedMod {
    /// Source of the files for the mod
    source: Vec<FileSource>,
    /// Version of mod
    pub version: semver::Version,
    /// Pinned dependencies of a pinned mod
    pub deps: Option<HashSet<ModMeta>>
}
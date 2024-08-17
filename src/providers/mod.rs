use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::mod_meta::ModMeta;

pub mod modrinth;
pub mod raw;

#[derive(Serialize, Deserialize)]
enum FileSource {
    Download { url: String, sha1: String, sha512: String},
    Local { path: PathBuf, sha1: String, sha512: String },
}

#[derive(Serialize, Deserialize)]
pub struct PinnedMod {
    /// Source of the files for the mod
    source: Vec<FileSource>,
    /// Version of mod
    version: semver::Version,
    /// Pinned dependencies of a pinned mod
    deps: Option<Vec<PinnedMod>>
}
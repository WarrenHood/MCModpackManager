use std::{collections::HashSet, path::PathBuf};
use serde::{Deserialize, Serialize};
use crate::mod_meta::ModMeta;

pub mod modrinth;
pub mod raw;

#[derive(Serialize, Deserialize, Clone)]
pub enum FileSource {
    Download { url: String, sha1: String, sha512: String, filename: String},
    Local { path: PathBuf, sha1: String, sha512: String, filename: String },
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PinnedMod {
    /// Source of the files for the mod
    pub source: Vec<FileSource>,
    /// Version of mod
    pub version: String,
    /// Pinned dependencies of a pinned mod
    pub deps: Option<HashSet<ModMeta>>
}
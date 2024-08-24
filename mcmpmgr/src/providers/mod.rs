use crate::mod_meta::ModMeta;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, fmt::Display, path::PathBuf, str::FromStr};

pub mod modrinth;
pub mod raw;

#[derive(Serialize, Deserialize, Clone)]
pub enum FileSource {
    Download {
        url: String,
        sha1: String,
        sha512: String,
        filename: String,
    },
    Local {
        path: PathBuf,
        sha1: String,
        sha512: String,
        filename: String,
    },
}

#[derive(Debug, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum DownloadSide {
    Both,
    Server,
    Client,
}

impl FromStr for DownloadSide {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "both" => Ok(DownloadSide::Both),
            "client" => Ok(DownloadSide::Client),
            "server" => Ok(DownloadSide::Server),
            _ => anyhow::bail!("Invalid side {}. Expected one of: both, server, clide", s),
        }
    }
}

impl Display for DownloadSide {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DownloadSide::Both => write!(f, "Both"),
            DownloadSide::Server => write!(f, "Server"),
            DownloadSide::Client => write!(f, "Client"),
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub struct PinnedMod {
    /// Source of the files for the mod
    pub source: Vec<FileSource>,
    /// Version of mod
    pub version: String,
    /// Pinned dependencies of a pinned mod
    pub deps: Option<HashSet<ModMeta>>,
    /// Server side
    pub server_side: bool,
    /// Required on client side
    pub client_side: bool,
}

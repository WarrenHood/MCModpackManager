use crate::mod_meta::ModMeta;
use serde::{Deserialize, Serialize};
use std::{collections::HashSet, path::PathBuf, str::FromStr};

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

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum DownloadSide {
    Both,
    Server,
    Client,
}

impl FromStr for DownloadSide {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "both" => Ok(DownloadSide::Both),
            "client" => Ok(DownloadSide::Client),
            "server" => Ok(DownloadSide::Server),
            _ => Err(format!(
                "Invalid side {}. Expected one of: both, server, clide",
                s
            )),
        }
    }
}

impl ToString for DownloadSide {
    fn to_string(&self) -> String {
        match self {
            DownloadSide::Both => "Both",
            DownloadSide::Server => "Server",
            DownloadSide::Client => "Client",
        }
        .into()
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

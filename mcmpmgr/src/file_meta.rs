use crate::providers::DownloadSide;
use serde::{Deserialize, Serialize};
use std::{fmt::Display, path::Path, str::FromStr};

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct FileMeta {
    /// Relative path of file in the instance folder
    pub target_path: String,
    /// Which side the files should be applied on
    pub side: DownloadSide,
    /// When to apply the files to the instance
    pub apply_policy: FileApplyPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub enum FileApplyPolicy {
    /// Always ensure the file or folder exactly matches that defined in the pack
    Always,
    /// Only apply the file or folder if it doesn't already exist in the pack
    Once,
    /// Merge into folders and files, retaining existing values in files when a file already exists
    MergeRetain,
    /// Merge into folders and files, overwriting existing values in files when a file already exists
    MergeOverwrite,
}

impl FromStr for FileApplyPolicy {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "always" => Ok(Self::Always),
            "once" => Ok(Self::Once),
            "mergeretain" => Ok(Self::MergeRetain),
            "mergeoverwrite" => Ok(Self::MergeOverwrite),
            _ => anyhow::bail!("Invalid apply policy {}. Expected one of: always, once", s),
        }
    }
}

impl Display for FileApplyPolicy {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Always => write!(f, "Always"),
            Self::Once => write!(f, "Once"),
            Self::MergeRetain => write!(f, "MergeRetain"),
            Self::MergeOverwrite => write!(f, "MergeOverwrite"),
        }
    }
}

impl PartialEq for FileMeta {
    fn eq(&self, other: &Self) -> bool {
        self.target_path == other.target_path
    }
}

impl PartialOrd for FileMeta {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.target_path.partial_cmp(&other.target_path)
    }
}

impl Ord for FileMeta {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.target_path.cmp(&other.target_path)
    }
}

impl Eq for FileMeta {}

/// Get a normalized relative path string in a consistent way across platforms
/// TODO: Make a nice struct for this maybe
pub fn get_normalized_relative_path(
    path_to_normalize: &Path,
    base_path: &Path,
) -> anyhow::Result<String> {
    if path_to_normalize.is_absolute() {
        anyhow::bail!(
            "Absolute paths are not supported! Will not normalise {}",
            path_to_normalize.display()
        );
    }
    let base_path = base_path.canonicalize()?;
    let full_path = base_path.join(path_to_normalize).canonicalize()?;
    let relative_path = pathdiff::diff_paths(&full_path, &base_path).ok_or(anyhow::format_err!(
        "Cannot normalize path {} relative to {}",
        &path_to_normalize.display(),
        &base_path.display()
    ))?;

    let mut normalized_path = String::new();
    for (i, component) in relative_path.components().enumerate() {
        if i > 0 {
            normalized_path.push('/');
        }
        normalized_path.push_str(&component.as_os_str().to_string_lossy());
    }

    if !normalized_path.starts_with("./") && !normalized_path.starts_with("/") {
        normalized_path.insert_str(0, "./");
    }

    Ok(normalized_path)
}

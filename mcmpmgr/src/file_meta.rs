use crate::providers::DownloadSide;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, Hash)]
pub struct FileMeta {
    pub target_path: String,
    pub side: DownloadSide,
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

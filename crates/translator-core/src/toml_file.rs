//! Shared TOML load/save for sidecar files next to the executable.

use std::path::{Path, PathBuf};

use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TomlFileError {
    #[error("IO error for {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("invalid TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error(transparent)]
    Serialize(#[from] toml::ser::Error),
    #[error(transparent)]
    Edit(#[from] toml_edit::ser::Error),
}

pub(crate) fn read_text(path: &Path) -> Result<String, TomlFileError> {
    std::fs::read_to_string(path).map_err(|source| TomlFileError::Io {
        path: path.to_path_buf(),
        source,
    })
}

pub(crate) fn load_toml_or_empty<T: DeserializeOwned + Default>(path: &Path) -> Result<T, TomlFileError> {
    if !path.exists() {
        return Ok(T::default());
    }
    Ok(toml::from_str(&read_text(path)?)?)
}

pub(crate) fn save_toml(path: &Path, value: &impl Serialize) -> Result<(), TomlFileError> {
    write_toml_str(path, &toml::to_string_pretty(value)?)
}

pub(crate) fn write_toml_str(path: &Path, text: &str) -> Result<(), TomlFileError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|source| TomlFileError::Io {
            path: parent.to_path_buf(),
            source,
        })?;
    }
    std::fs::write(path, text).map_err(|source| TomlFileError::Io {
        path: path.to_path_buf(),
        source,
    })
}

//! Shared TOML load/save for sidecar files next to the executable.

use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Serialize, de::DeserializeOwned};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum TomlFileError {
    #[error("IO error for {path}: {source}")]
    Io { path: PathBuf, source: std::io::Error },
    #[error("invalid TOML: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("invalid TOML, moved to {backup}: {source}")]
    Broken { backup: PathBuf, source: toml::de::Error },
    #[error(transparent)]
    Serialize(#[from] toml::ser::Error),
    #[error(transparent)]
    Edit(#[from] toml_edit::ser::Error),
}

/// Read and parse `path`. On a parse error the file is renamed to `<name>.broken-<unix secs>`
/// so the next save cannot overwrite what the user had.
pub(crate) fn parse_file<T>(path: &Path, parse: impl FnOnce(&str) -> Result<T, TomlFileError>) -> Result<T, TomlFileError> {
    let text = std::fs::read_to_string(path).map_err(|source| TomlFileError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    parse(&text).map_err(|e| {
        let TomlFileError::Parse(source) = e else {
            return e;
        };
        let secs = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
        let mut backup = path.as_os_str().to_owned();
        backup.push(format!(".broken-{secs}"));
        let backup = PathBuf::from(backup);
        match std::fs::rename(path, &backup) {
            Ok(()) => TomlFileError::Broken { backup, source },
            Err(_) => TomlFileError::Parse(source),
        }
    })
}

pub(crate) fn load_toml_or_empty<T: DeserializeOwned + Default>(path: &Path) -> Result<T, TomlFileError> {
    if !path.exists() {
        return Ok(T::default());
    }
    parse_file(path, |text| Ok(toml::from_str(text)?))
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
    // Write a sibling temp file, then rename over the target: a crash mid-write never leaves a truncated file.
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, text)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|source| {
            let _ = std::fs::remove_file(&tmp);
            TomlFileError::Io {
                path: path.to_path_buf(),
                source,
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_replaces_existing_file_and_leaves_no_temp() {
        let dir = std::env::temp_dir().join(format!("toml-file-test-{}", std::process::id()));
        let path = dir.join("config.toml");
        write_toml_str(&path, "a = 1\n").unwrap();
        write_toml_str(&path, "a = 2\n").unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "a = 2\n");
        assert!(!dir.join("config.toml.tmp").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn broken_file_is_moved_aside_with_its_content() {
        let dir = std::env::temp_dir().join(format!("toml-file-broken-{}", std::process::id()));
        let path = dir.join("api-profiles.toml");
        write_toml_str(&path, "[[[not valid").unwrap();
        let Err(TomlFileError::Broken { backup, .. }) = load_toml_or_empty::<toml::Table>(&path) else {
            panic!("expected Broken");
        };
        assert!(!path.exists(), "the next save must start from an empty slot");
        assert_eq!(std::fs::read_to_string(&backup).unwrap(), "[[[not valid");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}

//! Resolve paths relative to the executable directory (portable install).

use std::path::{Path, PathBuf};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PathError {
    #[error("failed to resolve current executable path: {0}")]
    CurrentExe(#[from] std::io::Error),
    #[error("executable has no parent directory")]
    NoParent,
}

/// Directory that contains the running executable.
///
/// In debug builds, if `TRANSLATOR_OVERLAY_ROOT` is set, that path is used
/// instead (convenient for `cargo run` without writing next to `target/debug`).
pub fn exe_dir() -> Result<PathBuf, PathError> {
    if cfg!(debug_assertions)
        && let Ok(root) = std::env::var("TRANSLATOR_OVERLAY_ROOT")
    {
        return Ok(PathBuf::from(root));
    }

    let exe = std::env::current_exe()?;
    exe.parent()
        .map(Path::to_path_buf)
        .ok_or(PathError::NoParent)
}

/// Path to `config.toml` next to the executable.
pub fn config_path() -> Result<PathBuf, PathError> {
    Ok(exe_dir()?.join("config.toml"))
}

/// Resolve a path that may be relative to the executable directory.
pub fn resolve_under_exe(path: impl AsRef<Path>) -> Result<PathBuf, PathError> {
    let path = path.as_ref();
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Ok(exe_dir()?.join(path))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolve_absolute_unchanged() {
        let abs = if cfg!(windows) {
            PathBuf::from(r"C:\models")
        } else {
            PathBuf::from("/models")
        };
        let resolved = resolve_under_exe(&abs).expect("resolve");
        assert_eq!(resolved, abs);
    }
}

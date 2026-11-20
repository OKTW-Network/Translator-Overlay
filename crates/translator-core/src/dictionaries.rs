//! Named user dictionaries stored as one TOML file per profile under `dictionaries/`.

use std::path::{Path, PathBuf};

use thiserror::Error;

use crate::{
    paths::{PathError, dictionaries_dir},
    toml_file::{TomlFileError, parse_file, write_toml_str},
};

const FORBIDDEN_NAME_CHARS: [char; 9] = ['/', '\\', ':', '*', '?', '"', '<', '>', '|'];
const RESERVED_STEMS: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4",
    "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

#[derive(Debug, Error)]
pub enum DictionaryError {
    #[error(transparent)]
    Path(#[from] PathError),
    #[error(transparent)]
    Toml(#[from] TomlFileError),
    #[error("{0}")]
    Invalid(String),
}

/// One source → target mapping.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DictionaryPair {
    pub source: String,
    pub target: String,
}

/// True when `name` can be a `dictionaries/{name}.toml` stem on Windows.
pub fn is_valid_profile_name(name: &str) -> bool {
    let name = name.trim();
    if name.is_empty() || name == "." || name == ".." {
        return false;
    }
    if name.ends_with('.') || name.chars().any(char::is_control) {
        return false;
    }
    if name.contains(FORBIDDEN_NAME_CHARS) {
        return false;
    }
    let stem = name.split('.').next().unwrap_or(name);
    !RESERVED_STEMS.iter().any(|reserved| stem.eq_ignore_ascii_case(reserved))
}

/// Trim, drop empty sources, keep the last target for a repeated source.
pub fn sanitize_pairs(pairs: &[DictionaryPair]) -> Vec<DictionaryPair> {
    let mut out: Vec<DictionaryPair> = Vec::new();
    for pair in pairs {
        let source = pair.source.trim();
        if source.is_empty() {
            continue;
        }
        let target = pair.target.trim().to_string();
        if let Some(existing) = out.iter_mut().find(|p| p.source == source) {
            existing.target = target;
        } else {
            out.push(DictionaryPair {
                source: source.to_string(),
                target,
            });
        }
    }
    out
}

/// Sorted profile names from `*.toml` stems in `dir`. Missing dir is an empty list.
pub fn list_dictionary_names(dir: &Path) -> Result<Vec<String>, DictionaryError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|source| TomlFileError::Io {
        path: dir.to_path_buf(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| TomlFileError::Io {
            path: dir.to_path_buf(),
            source,
        })?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().is_none_or(|ext| ext != "toml") {
            continue;
        }
        let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
            continue;
        };
        if stem == stem.trim() && is_valid_profile_name(stem) {
            names.push(stem.to_string());
        }
    }
    names.sort();
    Ok(names)
}

/// Profile names next to the executable.
pub fn list_dictionary_names_default() -> Result<Vec<String>, DictionaryError> {
    list_dictionary_names(&dictionaries_dir()?)
}

pub fn dictionary_profile_path(dir: &Path, name: &str) -> Result<PathBuf, DictionaryError> {
    let name = name.trim();
    if !is_valid_profile_name(name) {
        return Err(DictionaryError::Invalid("Invalid dictionary name.".into()));
    }
    Ok(dir.join(format!("{name}.toml")))
}

/// Read one profile. Missing file is an empty list.
pub fn load_profile_pairs(dir: &Path, name: &str) -> Result<Vec<DictionaryPair>, DictionaryError> {
    let path = dictionary_profile_path(dir, name)?;
    if !path.exists() {
        return Ok(Vec::new());
    }
    parse_pairs_file(&path)
}

pub fn save_profile_pairs(dir: &Path, name: &str, pairs: &[DictionaryPair]) -> Result<(), DictionaryError> {
    let path = dictionary_profile_path(dir, name)?;
    write_pairs_file(&path, &sanitize_pairs(pairs))
}

pub fn delete_profile(dir: &Path, name: &str) -> Result<(), DictionaryError> {
    let path = dictionary_profile_path(dir, name)?;
    if path.exists() {
        std::fs::remove_file(&path).map_err(|source| TomlFileError::Io { path, source })?;
    }
    Ok(())
}

/// Load the active profile from the exe `dictionaries/` dir. Missing name or file is empty.
pub fn load_active_dictionary(profile: Option<&str>) -> Result<Vec<DictionaryPair>, DictionaryError> {
    let Some(name) = profile.map(str::trim).filter(|s| !s.is_empty()) else {
        return Ok(Vec::new());
    };
    load_profile_pairs(&dictionaries_dir()?, name)
}

fn parse_pairs_file(path: &Path) -> Result<Vec<DictionaryPair>, DictionaryError> {
    let table: toml::Table = parse_file(path, |text| Ok(toml::from_str(text)?))?;
    let mut pairs = Vec::new();
    for (source, value) in table {
        let Some(target) = value.as_str() else {
            return Err(DictionaryError::Invalid("Glossary entries must be strings.".into()));
        };
        pairs.push(DictionaryPair {
            source,
            target: target.to_string(),
        });
    }
    Ok(sanitize_pairs(&pairs))
}

fn write_pairs_file(path: &Path, pairs: &[DictionaryPair]) -> Result<(), DictionaryError> {
    let mut doc = toml_edit::DocumentMut::new();
    for pair in pairs {
        doc.insert(&pair.source, toml_edit::value(pair.target.as_str()));
    }
    Ok(write_toml_str(path, &doc.to_string())?)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[test]
    fn name_validation() {
        assert!(is_valid_profile_name("Game A"));
        assert!(is_valid_profile_name("命運石之門"));
        assert!(!is_valid_profile_name(""));
        assert!(!is_valid_profile_name("  "));
        assert!(!is_valid_profile_name("."));
        assert!(!is_valid_profile_name(".."));
        assert!(!is_valid_profile_name("a/b"));
        assert!(!is_valid_profile_name(r"a\b"));
        assert!(!is_valid_profile_name("a:b"));
        assert!(!is_valid_profile_name("CON"));
        assert!(!is_valid_profile_name("con"));
        assert!(!is_valid_profile_name("CON.foo"));
        assert!(!is_valid_profile_name("nul.bar"));
        assert!(!is_valid_profile_name("ends."));
        assert!(!is_valid_profile_name("a\0b"));
        assert!(!is_valid_profile_name("a\nb"));
        assert!(matches!(dictionary_profile_path(Path::new("."), "a/b"), Err(DictionaryError::Invalid(_))));
    }

    #[test]
    fn sanitize_drops_empty_and_keeps_last_duplicate() {
        let cleaned = sanitize_pairs(&[
            DictionaryPair {
                source: "  ".into(),
                target: "x".into(),
            },
            DictionaryPair {
                source: "  主人公  ".into(),
                target: "主角".into(),
            },
            DictionaryPair {
                source: "主人公".into(),
                target: "  主人公  ".into(),
            },
            DictionaryPair {
                source: "セーブ".into(),
                target: "存檔".into(),
            },
        ]);
        assert_eq!(cleaned, vec![
            DictionaryPair {
                source: "主人公".into(),
                target: "主人公".into(),
            },
            DictionaryPair {
                source: "セーブ".into(),
                target: "存檔".into(),
            },
        ]);
    }

    #[test]
    fn roundtrip_one_profile_and_special_keys() {
        let dir = tempdir().unwrap();
        save_profile_pairs(dir.path(), "Game A", &[
            DictionaryPair {
                source: "主人公".into(),
                target: "主角".into(),
            },
            DictionaryPair {
                source: "Lv.1".into(),
                target: "等級1".into(),
            },
            DictionaryPair {
                source: "a = b".into(),
                target: String::new(),
            },
        ])
        .unwrap();
        let text = fs::read_to_string(dir.path().join("Game A.toml")).unwrap();
        assert!(text.contains("\"主人公\""), "{text}");
        assert!(text.contains("主角"), "{text}");
        let names = list_dictionary_names(dir.path()).unwrap();
        assert_eq!(names, vec!["Game A".to_string()]);
        let loaded = load_profile_pairs(dir.path(), "Game A").unwrap();
        assert_eq!(loaded, vec![
            DictionaryPair {
                source: "Lv.1".into(),
                target: "等級1".into(),
            },
            DictionaryPair {
                source: "a = b".into(),
                target: String::new(),
            },
            DictionaryPair {
                source: "主人公".into(),
                target: "主角".into(),
            },
        ]);
        delete_profile(dir.path(), "Game A").unwrap();
        // Deleting missing profile is a no-op; invalid names error.
        delete_profile(dir.path(), "Game A").unwrap();
        assert!(matches!(delete_profile(dir.path(), "a/b"), Err(DictionaryError::Invalid(_))));
        assert!(list_dictionary_names(dir.path()).unwrap().is_empty());
    }

    #[test]
    fn list_sorts_and_skips_invalid_or_non_toml() {
        let dir = tempdir().unwrap();
        fs::write(dir.path().join("Zeta.toml"), "a = \"b\"\n").unwrap();
        fs::write(dir.path().join("Alpha.toml"), "a = \"b\"\n").unwrap();
        fs::write(dir.path().join(" leading.toml"), "a = \"b\"\n").unwrap();
        fs::write(dir.path().join("trailing .toml"), "a = \"b\"\n").unwrap();
        fs::write(dir.path().join("notes.txt"), "a = \"b\"\n").unwrap();
        fs::write(dir.path().join("broken.toml.broken-1"), "a = \"b\"\n").unwrap();
        fs::create_dir_all(dir.path().join("sub.toml")).unwrap();

        let names = list_dictionary_names(dir.path()).unwrap();
        assert_eq!(names, vec!["Alpha".to_string(), "Zeta".to_string()]);
    }

    #[test]
    fn missing_dir_and_file_are_empty() {
        let dir = tempdir().unwrap();
        assert!(list_dictionary_names(&dir.path().join("missing")).unwrap().is_empty());
        assert!(load_profile_pairs(dir.path(), "Nope").unwrap().is_empty());
    }

    #[test]
    fn rejects_non_string_values() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("Game.toml");
        fs::write(&path, "\"主人公\" = \"主角\"\nLv.1 = \"等級1\"\n").unwrap();
        assert!(matches!(load_profile_pairs(dir.path(), "Game"), Err(DictionaryError::Invalid(_))));
        assert!(path.exists(), "valid TOML with nested table must not be renamed to .broken-*");

        fs::write(&path, "HP = 100\n").unwrap();
        assert!(matches!(load_profile_pairs(dir.path(), "Game"), Err(DictionaryError::Invalid(_))));
        assert!(path.exists(), "valid TOML with integer value must not be renamed to .broken-*");

        fs::write(&path, "\"Lv.1\" = \"等級1\"\n").unwrap();
        let loaded = load_profile_pairs(dir.path(), "Game").unwrap();
        assert_eq!(loaded, vec![DictionaryPair {
            source: "Lv.1".into(),
            target: "等級1".into(),
        }]);
    }
}

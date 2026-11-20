//! The user's own GGUF files in `models_dir`'s `custom` folder. They have no catalog entry: the
//! file's chat template builds the prompt, and llama.cpp's defaults sample.

use std::path::Path;

/// Subdirectory of `models_dir` holding the files.
pub(crate) const CUSTOM_DIR: &str = "custom";
/// Config ids are this plus the file name. Alone it picks the custom option with no file yet.
pub const CUSTOM_ID: &str = "custom:";
/// llama.cpp's own `(temperature, top_p, top_k)` defaults.
pub(crate) const SAMPLING_DEFAULTS: (f32, f32, u32) = (0.8, 0.95, 40);

/// File names of the GGUF models in `models_dir`'s `custom` folder (created if missing), sorted.
pub async fn custom_models(models_dir: &Path) -> Vec<String> {
    let dir = models_dir.join(CUSTOM_DIR);
    let _ = tokio::fs::create_dir_all(&dir).await;
    let mut files = Vec::new();
    if let Ok(mut entries) = tokio::fs::read_dir(dir).await {
        while let Ok(Some(entry)) = entries.next_entry().await {
            let (Ok(name), Ok(meta)) = (entry.file_name().into_string(), entry.metadata().await) else {
                continue;
            };
            let lower = name.to_ascii_lowercase();
            let Some(stem) = lower.strip_suffix(".gguf") else {
                continue;
            };
            // llama.cpp names split parts `<name>-00002-of-00003.gguf` and loads them from the first.
            let part = stem.rsplit_once("-of-").and_then(|(head, count)| {
                let index = head.rsplit_once('-')?.1;
                let digits = |s: &str| s.len() == 5 && s.bytes().all(|b| b.is_ascii_digit());
                (digits(index) && digits(count)).then_some(index)
            });
            // An `mmproj` file is an image encoder.
            if meta.is_file() && !lower.starts_with("mmproj") && part.is_none_or(|i| i == "00001") {
                files.push(name);
            }
        }
    }
    files.sort();
    files
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::*;

    #[tokio::test]
    async fn custom_models_list_the_folders_gguf_files() {
        let dir = tempdir().unwrap();
        assert!(custom_models(dir.path()).await.is_empty() && dir.path().join(CUSTOM_DIR).is_dir());
        for name in [
            "b.GGUF",
            "a.gguf",
            "big-00001-of-00002.gguf",
            "big-00002-of-00002.gguf",
            "mixture-of-experts.gguf",
            "mmproj-F16.gguf",
            "notes.txt",
        ] {
            fs::write(dir.path().join(CUSTOM_DIR).join(name), [0]).unwrap();
        }
        assert_eq!(custom_models(dir.path()).await, ["a.gguf", "b.GGUF", "big-00001-of-00002.gguf", "mixture-of-experts.gguf"]);
    }
}

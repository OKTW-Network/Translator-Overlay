//! Session translation cache: unique OCR source strings → last model translation.
//!
//! Eviction is LFU with LRU as the tie-break (lowest `freq`, then oldest `last_tick`).
//! In-memory only; the pipeline thread owns the instance.

use std::collections::{HashMap, HashSet};

use translator_core::{OcrBlock, TRANSLATION_CACHE_MAX_MIN, TranslatedBlock, TranslationConfig, normalize_ocr_text};

/// Per-block lookup result plus the unique misses to send to the model.
#[derive(Debug, Clone, PartialEq)]
pub struct CacheResolve {
    /// Parallel to the source page: `Some` = reuse this translation.
    pub hits: Vec<Option<String>>,
    /// Unique (by normalized text) blocks that still need the API. Original ids kept.
    pub misses: Vec<OcrBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct CacheKey {
    source: String,
    source_lang: String,
    target_lang: String,
}

#[derive(Debug, Clone)]
struct CacheEntry {
    translation: String,
    freq: u32,
    last_tick: u64,
}

/// In-memory LFU translation cache (LRU tie-break).
#[derive(Debug, Clone)]
pub struct TranslationCache {
    map: HashMap<CacheKey, CacheEntry>,
    max_entries: usize,
    tick: u64,
}

impl TranslationCache {
    pub fn new(max_entries: usize) -> Self {
        Self {
            map: HashMap::new(),
            max_entries: max_entries.max(TRANSLATION_CACHE_MAX_MIN),
            tick: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn max_entries(&self) -> usize {
        self.max_entries
    }

    pub fn set_max(&mut self, max_entries: usize) {
        self.max_entries = max_entries.max(TRANSLATION_CACHE_MAX_MIN);
        self.evict_to_max();
    }

    pub fn clear(&mut self) {
        self.map.clear();
    }

    fn next_tick(&mut self) -> u64 {
        self.tick = self.tick.saturating_add(1);
        self.tick
    }

    fn key(source: &str, source_lang: &str, target_lang: &str) -> Option<CacheKey> {
        let source = normalize_ocr_text(source);
        if source.is_empty() {
            return None;
        }
        Some(CacheKey {
            source,
            source_lang: source_lang.trim().to_string(),
            target_lang: target_lang.trim().to_string(),
        })
    }

    /// Return a cached translation and bump frequency / recency.
    pub fn lookup(&mut self, source: &str, source_lang: &str, target_lang: &str) -> Option<String> {
        let key = Self::key(source, source_lang, target_lang)?;
        let tick = self.next_tick();
        let entry = self.map.get_mut(&key)?;
        entry.freq = entry.freq.saturating_add(1);
        entry.last_tick = tick;
        Some(entry.translation.clone())
    }

    /// Store or overwrite a translation. Empty source / translation is ignored.
    pub fn insert(&mut self, source: &str, source_lang: &str, target_lang: &str, translation: &str) {
        let Some(key) = Self::key(source, source_lang, target_lang) else {
            return;
        };
        let translation = translation.trim();
        if translation.is_empty() {
            return;
        }
        let tick = self.next_tick();
        if let Some(entry) = self.map.get_mut(&key) {
            entry.translation = translation.to_string();
            entry.freq = entry.freq.saturating_add(1);
            entry.last_tick = tick;
            return;
        }
        self.evict_one_if_full();
        self.map.insert(key, CacheEntry {
            translation: translation.to_string(),
            freq: 1,
            last_tick: tick,
        });
    }

    /// Split a page into cache hits and unique misses.
    ///
    /// `force` (Retry) and a disabled cache skip lookup and send every block.
    pub fn resolve(&mut self, blocks: &[OcrBlock], cfg: &TranslationConfig, force: bool) -> CacheResolve {
        if force || !cfg.cache_enabled {
            return CacheResolve {
                hits: vec![None; blocks.len()],
                misses: if force {
                    unique_blocks_by_norm_text(blocks)
                } else {
                    blocks.to_vec()
                },
            };
        }

        let mut hits = vec![None; blocks.len()];
        let mut seen: HashMap<String, Option<String>> = HashMap::new();
        let mut misses = Vec::new();

        for (i, block) in blocks.iter().enumerate() {
            let norm = normalize_ocr_text(&block.text);
            if norm.is_empty() {
                continue;
            }
            if let Some(prior) = seen.get(&norm) {
                hits[i] = prior.clone();
                continue;
            }
            if let Some(translation) = self.lookup(&block.text, &cfg.source_lang, &cfg.target_lang) {
                seen.insert(norm, Some(translation.clone()));
                hits[i] = Some(translation);
            } else {
                seen.insert(norm, None);
                misses.push(block.clone());
            }
        }

        CacheResolve { hits, misses }
    }

    /// Persist model-returned pairs only (skip ids the model omitted).
    pub fn store_model_pairs(
        &mut self,
        source_blocks: &[OcrBlock],
        translated: &[TranslatedBlock],
        model_ids: &HashSet<u32>,
        cfg: &TranslationConfig,
    ) {
        if !cfg.cache_enabled {
            return;
        }
        for src in source_blocks {
            if !model_ids.contains(&src.id) {
                continue;
            }
            let Some(tb) = translated.iter().find(|b| b.id == src.id) else {
                continue;
            };
            self.insert(&src.text, &cfg.source_lang, &cfg.target_lang, &tb.translation);
        }
    }

    /// Rebuild a full page: cache hits first, then model rows (by id, then normalized text).
    pub fn stitch(source: &[OcrBlock], hits: &[Option<String>], model_blocks: &[TranslatedBlock]) -> Vec<TranslatedBlock> {
        source
            .iter()
            .enumerate()
            .map(|(i, src)| {
                let translation = hits
                    .get(i)
                    .and_then(|h| h.clone())
                    .or_else(|| model_blocks.iter().find(|b| b.id == src.id).map(|b| b.translation.clone()))
                    .or_else(|| {
                        let key = normalize_ocr_text(&src.text);
                        if key.is_empty() {
                            return None;
                        }
                        model_blocks
                            .iter()
                            .find(|b| normalize_ocr_text(&b.source) == key)
                            .map(|b| b.translation.clone())
                    })
                    .unwrap_or_else(|| src.text.clone());
                TranslatedBlock {
                    id: src.id,
                    source: src.text.clone(),
                    translation,
                    confidence: src.confidence,
                    bbox: src.bbox,
                    source_lines: src.source_lines.max(1),
                }
            })
            .collect()
    }

    /// Only blocks that already have a cached translation (misses omitted).
    pub fn hits_only(source: &[OcrBlock], hits: &[Option<String>]) -> Vec<TranslatedBlock> {
        source
            .iter()
            .enumerate()
            .filter_map(|(i, src)| {
                let translation = hits.get(i).and_then(|h| h.as_ref())?;
                Some(TranslatedBlock {
                    id: src.id,
                    source: src.text.clone(),
                    translation: translation.clone(),
                    confidence: src.confidence,
                    bbox: src.bbox,
                    source_lines: src.source_lines.max(1),
                })
            })
            .collect()
    }

    fn evict_one_if_full(&mut self) {
        if self.map.len() >= self.max_entries {
            self.evict_one();
        }
    }

    fn evict_to_max(&mut self) {
        while self.map.len() > self.max_entries {
            self.evict_one();
        }
    }

    fn evict_one(&mut self) {
        let victim = self.map.iter().min_by_key(|(_, e)| (e.freq, e.last_tick)).map(|(k, _)| k.clone());
        if let Some(key) = victim {
            self.map.remove(&key);
        }
    }
}

fn unique_blocks_by_norm_text(blocks: &[OcrBlock]) -> Vec<OcrBlock> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for block in blocks {
        let norm = normalize_ocr_text(&block.text);
        if norm.is_empty() || !seen.insert(norm) {
            continue;
        }
        out.push(block.clone());
    }
    out
}

#[cfg(test)]
mod tests {
    use translator_core::{Rect, TRANSLATION_CACHE_MAX_DEFAULT};

    use super::*;

    fn block(id: u32, text: &str) -> OcrBlock {
        OcrBlock {
            id,
            text: text.to_string(),
            confidence: 0.9,
            bbox: Rect::new(0.0, 0.0, 10.0, 10.0),
            source_lines: 1,
        }
    }

    fn cfg() -> TranslationConfig {
        TranslationConfig {
            source_lang: "ja".into(),
            target_lang: "zh-TW".into(),
            ..TranslationConfig::default()
        }
    }

    #[test]
    fn lookup_insert_and_lang_isolation() {
        let mut cache = TranslationCache::new(8);
        cache.insert("はい", "ja", "zh-TW", "是");
        assert_eq!(cache.lookup("はい", "ja", "zh-TW").as_deref(), Some("是"));
        assert!(cache.lookup("はい", "ja", "en").is_none());
        assert_eq!(cache.len(), 1);
    }

    #[test]
    fn whitespace_normalized_key() {
        let mut cache = TranslationCache::new(8);
        cache.insert("hello   world", "en", "zh-TW", "你好世界");
        assert_eq!(cache.lookup("hello world", "en", "zh-TW").as_deref(), Some("你好世界"));
    }

    #[test]
    fn ignores_empty_source_and_translation() {
        let mut cache = TranslationCache::new(8);
        cache.insert("   ", "ja", "zh-TW", "x");
        cache.insert("はい", "ja", "zh-TW", "  ");
        assert!(cache.is_empty());
    }

    #[test]
    fn evicts_lowest_freq_first() {
        let mut cache = TranslationCache::new(2);
        cache.insert("a", "ja", "zh-TW", "A");
        cache.insert("b", "ja", "zh-TW", "B");
        assert!(cache.lookup("a", "ja", "zh-TW").is_some());
        cache.insert("c", "ja", "zh-TW", "C");
        assert!(cache.lookup("a", "ja", "zh-TW").is_some(), "higher-freq a must remain");
        assert!(cache.lookup("b", "ja", "zh-TW").is_none(), "lowest-freq b must be evicted");
        assert!(cache.lookup("c", "ja", "zh-TW").is_some());
    }

    #[test]
    fn same_freq_evicts_oldest() {
        let mut cache = TranslationCache::new(2);
        cache.insert("old", "ja", "zh-TW", "1");
        cache.insert("new", "ja", "zh-TW", "2");
        cache.insert("newer", "ja", "zh-TW", "3");
        assert!(cache.lookup("old", "ja", "zh-TW").is_none());
        assert!(cache.lookup("new", "ja", "zh-TW").is_some());
        assert!(cache.lookup("newer", "ja", "zh-TW").is_some());
    }

    #[test]
    fn high_freq_survives_one_shot_flood() {
        let mut cache = TranslationCache::new(3);
        cache.insert("name", "ja", "zh-TW", "名");
        let _ = cache.lookup("name", "ja", "zh-TW");
        let _ = cache.lookup("name", "ja", "zh-TW");
        cache.insert("l1", "ja", "zh-TW", "1");
        cache.insert("l2", "ja", "zh-TW", "2");
        cache.insert("l3", "ja", "zh-TW", "3");
        assert!(cache.lookup("name", "ja", "zh-TW").is_some());
        assert_eq!(cache.len(), 3);
    }

    #[test]
    fn set_max_evicts_immediately() {
        let mut cache = TranslationCache::new(4);
        for i in 0..4 {
            cache.insert(&format!("k{i}"), "ja", "zh-TW", &format!("v{i}"));
        }
        cache.set_max(2);
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.max_entries(), 2);
    }

    #[test]
    fn resolve_hits_and_dedupes_misses() {
        let mut cache = TranslationCache::new(8);
        let tcfg = cfg();
        cache.insert("名前", "ja", "zh-TW", "名字");
        let blocks = vec![block(0, "名前"), block(1, "新しい台詞"), block(2, "新しい台詞")];
        let resolved = cache.resolve(&blocks, &tcfg, false);
        assert_eq!(resolved.hits[0].as_deref(), Some("名字"));
        assert!(resolved.hits[1].is_none());
        assert!(resolved.hits[2].is_none());
        assert_eq!(resolved.misses.len(), 1);
        assert_eq!(resolved.misses[0].id, 1);
    }

    #[test]
    fn resolve_force_skips_hits_and_dedupes() {
        let mut cache = TranslationCache::new(8);
        let tcfg = cfg();
        cache.insert("名前", "ja", "zh-TW", "名字");
        let blocks = vec![block(0, "名前"), block(1, "名前")];
        let resolved = cache.resolve(&blocks, &tcfg, true);
        assert!(resolved.hits.iter().all(Option::is_none));
        assert_eq!(resolved.misses.len(), 1);
    }

    #[test]
    fn resolve_disabled_sends_every_block() {
        let mut cache = TranslationCache::new(8);
        let tcfg = TranslationConfig {
            cache_enabled: false,
            ..cfg()
        };
        cache.insert("名前", "ja", "zh-TW", "名字");
        let blocks = vec![block(0, "名前"), block(1, "名前")];
        let resolved = cache.resolve(&blocks, &tcfg, false);
        assert_eq!(resolved.misses.len(), 2);
        assert!(resolved.hits.iter().all(Option::is_none));
    }

    #[test]
    fn stitch_prefers_hits_then_model_id_then_text() {
        let source = vec![block(0, "名前"), block(1, "台詞"), block(2, "台詞")];
        let hits = vec![Some("名字".into()), None, None];
        let model = vec![TranslatedBlock {
            id: 1,
            source: "台詞".into(),
            translation: "對白".into(),
            confidence: 0.9,
            bbox: Rect::new(0.0, 0.0, 10.0, 10.0),
            source_lines: 1,
        }];
        let out = TranslationCache::stitch(&source, &hits, &model);
        assert_eq!(out[0].translation, "名字");
        assert_eq!(out[1].translation, "對白");
        assert_eq!(out[2].translation, "對白");
        assert_eq!(out[0].bbox, source[0].bbox);
        assert_eq!(out[0].id, 0);
    }

    #[test]
    fn hits_only_omits_misses() {
        let source = vec![block(0, "名前"), block(1, "台詞")];
        let hits = vec![Some("名字".into()), None];
        let out = TranslationCache::hits_only(&source, &hits);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, 0);
        assert_eq!(out[0].translation, "名字");
    }

    #[test]
    fn store_skips_ids_model_did_not_return() {
        let mut cache = TranslationCache::new(8);
        let tcfg = cfg();
        let source = vec![block(1, "はい"), block(2, "いいえ")];
        let translated = vec![
            TranslatedBlock {
                id: 1,
                source: "はい".into(),
                translation: "是".into(),
                confidence: 0.9,
                bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
                source_lines: 1,
            },
            TranslatedBlock {
                id: 2,
                source: "いいえ".into(),
                translation: "いいえ".into(),
                confidence: 0.9,
                bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
                source_lines: 1,
            },
        ];
        let mut model_ids = HashSet::new();
        model_ids.insert(1);
        cache.store_model_pairs(&source, &translated, &model_ids, &tcfg);
        assert_eq!(cache.lookup("はい", "ja", "zh-TW").as_deref(), Some("是"));
        assert!(cache.lookup("いいえ", "ja", "zh-TW").is_none());
    }

    #[test]
    fn default_capacity_constant_is_sane() {
        let cache = TranslationCache::new(TRANSLATION_CACHE_MAX_DEFAULT);
        assert_eq!(cache.max_entries(), TRANSLATION_CACHE_MAX_DEFAULT);
    }
}

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
    /// `force` (Retry) and a disabled cache skip lookup and send each distinct text once.
    pub fn resolve(&mut self, blocks: &[OcrBlock], cfg: &TranslationConfig, force: bool) -> CacheResolve {
        if force || !cfg.cache_enabled {
            return CacheResolve {
                hits: vec![None; blocks.len()],
                misses: unique_blocks_by_norm_text(blocks),
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

    /// Persist model-returned pairs.
    pub fn store_model_pairs(&mut self, translated: &[TranslatedBlock], cfg: &TranslationConfig) {
        if !cfg.cache_enabled {
            return;
        }
        for tb in translated {
            self.insert(&tb.source, &cfg.source_lang, &cfg.target_lang, &tb.translation);
        }
    }

    /// Rebuild a full page: cache hits first, then model rows (by id, then normalized text).
    /// Blocks with no translation yet are omitted (not filled with source text).
    pub fn stitch(source: &[OcrBlock], hits: &[Option<String>], model_blocks: &[TranslatedBlock]) -> Vec<TranslatedBlock> {
        let mut by_id: HashMap<u32, &str> = HashMap::with_capacity(model_blocks.len());
        let mut by_norm: HashMap<String, &str> = HashMap::with_capacity(model_blocks.len());
        for b in model_blocks {
            by_id.entry(b.id).or_insert(b.translation.as_str());
            by_norm.entry(normalize_ocr_text(&b.source)).or_insert(b.translation.as_str());
        }
        source
            .iter()
            .enumerate()
            .filter_map(|(i, src)| {
                let translation = hits
                    .get(i)
                    .and_then(|h| h.clone())
                    .or_else(|| by_id.get(&src.id).map(|s| (*s).to_string()))
                    .or_else(|| {
                        let key = normalize_ocr_text(&src.text);
                        if key.is_empty() {
                            return None;
                        }
                        by_norm.get(&key).map(|s| (*s).to_string())
                    })?;
                Some(TranslatedBlock {
                    id: src.id,
                    source: src.text.clone(),
                    translation,
                    confidence: src.confidence,
                    bbox: src.bbox,
                    source_lines: src.source_lines.max(1),
                    source_height: src.source_height.max(src.bbox.height),
                    source_width: src.source_width.max(src.bbox.width),
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
    use translator_core::Rect;

    use super::*;

    fn block(id: u32, text: &str) -> OcrBlock {
        OcrBlock {
            id,
            text: text.to_string(),
            confidence: 0.9,
            bbox: Rect::new(0.0, 0.0, 10.0, 10.0),
            source_lines: 1,
            source_height: 10.0,
            source_width: 10.0,
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
        assert!(cache.lookup("k0", "ja", "zh-TW").is_none());
        assert!(cache.lookup("k1", "ja", "zh-TW").is_none());
        assert_eq!(cache.lookup("k2", "ja", "zh-TW").as_deref(), Some("v2"));
        assert_eq!(cache.lookup("k3", "ja", "zh-TW").as_deref(), Some("v3"));
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
    fn resolve_disabled_skips_hits_and_dedupes() {
        let mut cache = TranslationCache::new(8);
        let tcfg = TranslationConfig {
            cache_enabled: false,
            ..cfg()
        };
        cache.insert("名前", "ja", "zh-TW", "名字");
        let blocks = vec![block(0, "名前"), block(1, "名前")];
        let resolved = cache.resolve(&blocks, &tcfg, false);
        assert_eq!(resolved.misses.len(), 1);
        assert!(resolved.hits.iter().all(Option::is_none));
    }

    #[test]
    fn stitch_prefers_hits_then_model_id_then_text() {
        let source = vec![block(0, "同じ"), block(1, "同じ"), block(2, "別の")];
        let hits = vec![Some("hit_wins".into()), None, None];
        let tb = |id, src: &str, tr: &str| TranslatedBlock {
            id,
            source: src.into(),
            translation: tr.into(),
            confidence: 0.9,
            bbox: Rect::new(0.0, 0.0, 10.0, 10.0),
            source_lines: 1,
            source_height: 10.0,
            source_width: 10.0,
        };
        // Block 0 has hit > model id 0; block 1 has model id 1 > earlier by_norm ("by_id_0"); block 2 falls back to by_norm from id 99.
        let model = vec![
            tb(0, "同じ", "by_id_0"),
            tb(1, "同じ", "by_id_1"),
            tb(99, "別の", "by_norm_fallback"),
        ];
        let out = TranslationCache::stitch(&source, &hits, &model);
        assert_eq!(out[0].translation, "hit_wins");
        assert_eq!(out[1].translation, "by_id_1");
        assert_eq!(out[2].translation, "by_norm_fallback");
        assert_eq!(out[0].bbox, source[0].bbox);
        assert_eq!(out[0].id, 0);

        let partial = TranslationCache::stitch(&source, &[Some("名字".into()), None, None], &[]);
        assert_eq!(partial.len(), 1);
        assert_eq!(partial[0].id, 0);
    }

    #[test]
    fn store_model_pairs_inserts_returned_rows() {
        let mut cache = TranslationCache::new(8);
        let tcfg = cfg();
        let translated = vec![TranslatedBlock {
            id: 1,
            source: "はい".into(),
            translation: "是".into(),
            confidence: 0.9,
            bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
            source_lines: 1,
            source_height: 1.0,
            source_width: 1.0,
        }];
        cache.store_model_pairs(&translated, &tcfg);
        assert_eq!(cache.lookup("はい", "ja", "zh-TW").as_deref(), Some("是"));
        assert!(cache.lookup("いいえ", "ja", "zh-TW").is_none());

        let disabled = TranslationConfig {
            cache_enabled: false,
            ..tcfg
        };
        cache.store_model_pairs(
            &[TranslatedBlock {
                id: 2,
                source: "いいえ".into(),
                translation: "不是".into(),
                confidence: 0.9,
                bbox: Rect::new(0.0, 0.0, 1.0, 1.0),
                source_lines: 1,
                source_height: 1.0,
                source_width: 1.0,
            }],
            &disabled,
        );
        assert!(cache.lookup("いいえ", "ja", "zh-TW").is_none());
    }

    #[test]
    fn overwrite_at_capacity_and_lru_tiebreak_at_high_freq() {
        let mut cache = TranslationCache::new(2);
        cache.insert("a", "ja", "zh-TW", "A1");
        cache.insert("b", "ja", "zh-TW", "B1");
        // Overwriting an existing key at capacity must not evict "b".
        cache.insert("a", "ja", "zh-TW", "A2");
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.lookup("b", "ja", "zh-TW").as_deref(), Some("B1")); // b now freq=2
        // Make a reach freq=3 older than b freq=3.
        assert_eq!(cache.lookup("a", "ja", "zh-TW").as_deref(), Some("A2")); // a now freq=3
        assert!(cache.lookup("b", "ja", "zh-TW").is_some()); // b now freq=3, newer tick
        cache.insert("c", "ja", "zh-TW", "C1");
        assert!(cache.lookup("a", "ja", "zh-TW").is_none(), "oldest freq=3 entry a must be evicted");
        assert_eq!(cache.lookup("b", "ja", "zh-TW").as_deref(), Some("B1"));
        assert_eq!(cache.lookup("c", "ja", "zh-TW").as_deref(), Some("C1"));
    }

    #[test]
    fn source_lang_isolation_zero_capacity_clamps_and_clear() {
        let mut cache = TranslationCache::new(4);
        cache.insert("hi", "en", "zh-TW", "嗨");
        cache.insert("hi", "ja", "zh-TW", "是");
        assert_eq!(cache.len(), 2);
        assert_eq!(cache.lookup("hi", "en", "zh-TW").as_deref(), Some("嗨"));
        assert_eq!(cache.lookup("hi", "ja", "zh-TW").as_deref(), Some("是"));
        assert!(cache.lookup("hi", "ko", "zh-TW").is_none());

        cache.set_max(0);
        assert_eq!(cache.len(), TRANSLATION_CACHE_MAX_MIN, "set_max(0) clamps to {TRANSLATION_CACHE_MAX_MIN}");
        cache.clear();
        assert!(cache.is_empty());

        let mut clamped = TranslationCache::new(0);
        clamped.insert("hi", "en", "zh-TW", "嗨");
        clamped.insert("hi", "ja", "zh-TW", "是");
        assert_eq!(clamped.len(), TRANSLATION_CACHE_MAX_MIN, "new(0) clamps capacity to {TRANSLATION_CACHE_MAX_MIN}");
        assert!(clamped.lookup("hi", "en", "zh-TW").is_none());
        assert_eq!(clamped.lookup("hi", "ja", "zh-TW").as_deref(), Some("是"));
    }
}

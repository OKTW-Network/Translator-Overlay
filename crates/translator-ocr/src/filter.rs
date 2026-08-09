//! OCR noise filters: single-token junk and temporal block persistence.

use std::time::{Duration, Instant};

use translator_core::{OcrBlock, OcrConfig, Rect};

/// True when `text` is a single ASCII letter or digit (e.g. `"0"`, `"V"`, `"c"`).
///
/// Icons and HUD glyphs are often misread as one Latin char; real words/CJK stay.
/// Shape-only (character class), not a vocabulary allow/deny list.
pub fn is_single_latin_or_digit(text: &str) -> bool {
    let t = text.trim();
    let mut chars = t.chars();
    match (chars.next(), chars.next()) {
        (Some(c), None) => c.is_ascii_alphabetic() || c.is_ascii_digit(),
        _ => false,
    }
}

/// Drop blocks that are only one English letter or digit.
pub fn filter_single_char_blocks(blocks: Vec<OcrBlock>) -> Vec<OcrBlock> {
    reindex(
        blocks
            .into_iter()
            .filter(|b| !is_single_latin_or_digit(&b.text))
            .collect(),
    )
}

fn reindex(mut blocks: Vec<OcrBlock>) -> Vec<OcrBlock> {
    for (i, b) in blocks.iter_mut().enumerate() {
        b.id = i as u32;
    }
    blocks
}

/// Tracks per-region text over time and only emits blocks that stay put.
///
/// Fast-changing OCR (animated icons, particle effects) never reaches the
/// persistence threshold and is discarded. Static UI text is kept.
///
/// Once a track is confirmed, it keeps being emitted for `max_miss` even if
/// OCR misses it for a frame or two (common when the background animates).
#[derive(Debug)]
pub struct BlockPersistenceFilter {
    persist: Duration,
    max_miss: Duration,
    tracks: Vec<Track>,
    /// Spatial quantize step in pixels (center matching).
    quant: i32,
}

#[derive(Debug, Clone)]
struct Track {
    text: String,
    bbox: Rect,
    first_stable_since: Instant,
    last_seen: Instant,
    /// True once the text has lingered long enough at this spot.
    confirmed: bool,
    /// Last good sample (used for hysteresis when a frame misses the region).
    last_block: OcrBlock,
}

impl BlockPersistenceFilter {
    pub fn new(persist_ms: u64, max_miss_ms: u64) -> Self {
        Self {
            persist: Duration::from_millis(persist_ms),
            max_miss: Duration::from_millis(max_miss_ms.max(persist_ms.saturating_add(100))),
            tracks: Vec::new(),
            quant: 16,
        }
    }

    pub fn from_config(config: &OcrConfig) -> Self {
        Self::new(config.block_persist_ms, config.block_max_miss_ms)
    }

    pub fn reset(&mut self) {
        self.tracks.clear();
    }

    pub fn is_enabled(&self) -> bool {
        !self.persist.is_zero()
    }

    /// Update tracks from this frame and return only persistent blocks.
    ///
    /// When disabled (`block_persist_ms == 0`), returns `blocks` unchanged.
    pub fn filter(&mut self, blocks: Vec<OcrBlock>) -> Vec<OcrBlock> {
        if !self.is_enabled() {
            return blocks;
        }

        let now = Instant::now();

        for block in blocks {
            let text_key = normalize_text(&block.text);
            if text_key.is_empty() {
                continue;
            }

            if let Some(idx) = self.find_track(&block, &text_key) {
                let track = &mut self.tracks[idx];
                if normalize_text(&track.text) == text_key {
                    // Same region + same text → accumulate persistence.
                    track.last_seen = now;
                    // Once confirmed, freeze the box against detector jitter so
                    // overlay captions do not shake when the background animates.
                    // Unconfirmed tracks still track the latest sample so the
                    // first emit lands on a fresh reading.
                    if track.confirmed {
                        let bbox = track.bbox.stabilize_against(block.bbox);
                        track.bbox = bbox;
                        track.last_block.bbox = bbox;
                        track.last_block.confidence = block.confidence;
                        // Prefer the longer/newer recognition string only when it
                        // normalizes equal (already true here); keep stored text.
                    } else {
                        track.bbox = block.bbox;
                        track.last_block = block;
                    }
                    if now.saturating_duration_since(track.first_stable_since) >= self.persist {
                        track.confirmed = true;
                    }
                } else {
                    // Position match but text flipped (animation / OCR thrash): restart.
                    track.text = block.text.clone();
                    track.bbox = block.bbox;
                    track.first_stable_since = now;
                    track.last_seen = now;
                    track.confirmed = false;
                    track.last_block = block;
                }
            } else {
                self.tracks.push(Track {
                    text: block.text.clone(),
                    bbox: block.bbox,
                    first_stable_since: now,
                    last_seen: now,
                    confirmed: false,
                    last_block: block,
                });
            }
        }

        // Emit every confirmed track still within the miss grace window.
        // Hysteresis: keep showing text even if this frame's OCR missed the box.
        let mut out: Vec<OcrBlock> = self
            .tracks
            .iter()
            .filter(|t| t.confirmed && now.saturating_duration_since(t.last_seen) <= self.max_miss)
            .map(|t| t.last_block.clone())
            .collect();

        out.sort_by(|a, b| {
            (a.bbox.y as i32)
                .cmp(&(b.bbox.y as i32))
                .then_with(|| (a.bbox.x as i32).cmp(&(b.bbox.x as i32)))
        });

        // Drop tracks that vanished past the grace window.
        self.tracks
            .retain(|t| now.saturating_duration_since(t.last_seen) <= self.max_miss);

        reindex(out)
    }
}

impl BlockPersistenceFilter {
    fn find_track(&self, block: &OcrBlock, text_key: &str) -> Option<usize> {
        let cx = block.bbox.x + block.bbox.width * 0.5;
        let cy = block.bbox.y + block.bbox.height * 0.5;
        let q = self.quant.max(1);

        let mut best: Option<(usize, f32)> = None;
        for (i, track) in self.tracks.iter().enumerate() {
            let tcx = track.bbox.x + track.bbox.width * 0.5;
            let tcy = track.bbox.y + track.bbox.height * 0.5;
            let dx = (cx - tcx).abs();
            let dy = (cy - tcy).abs();
            // Match by center proximity (tolerant of OCR box jitter).
            let max_dx = (block.bbox.width.max(track.bbox.width) * 0.55).max(q as f32 * 2.0);
            let max_dy = (block.bbox.height.max(track.bbox.height) * 0.75).max(q as f32 * 2.0);
            if dx > max_dx || dy > max_dy {
                continue;
            }

            let same_text = normalize_text(&track.text) == text_key;
            // Prefer exact text match; allow position-only when IoU is high
            // so we can detect text thrashing at the same spot.
            let iou = rect_iou(block.bbox, track.bbox);
            if !same_text && iou < 0.2 {
                continue;
            }

            let score = if same_text { iou + 1.0 } else { iou };
            if best.map(|(_, s)| score > s).unwrap_or(true) {
                best = Some((i, score));
            }
        }
        best.map(|(i, _)| i)
    }
}

fn normalize_text(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn rect_iou(a: Rect, b: Rect) -> f32 {
    a.iou(b)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block(text: &str, x: f32, y: f32) -> OcrBlock {
        OcrBlock {
            id: 0,
            text: text.to_string(),
            confidence: 0.9,
            bbox: Rect::new(x, y, 80.0, 20.0),
        }
    }

    #[test]
    fn detects_single_latin_and_digit() {
        assert!(is_single_latin_or_digit("0"));
        assert!(is_single_latin_or_digit("V"));
        assert!(is_single_latin_or_digit(" c "));
        assert!(is_single_latin_or_digit("9"));
        assert!(!is_single_latin_or_digit("OK"));
        assert!(!is_single_latin_or_digit("甲"));
        assert!(!is_single_latin_or_digit("甲乙"));
        assert!(!is_single_latin_or_digit(""));
        assert!(!is_single_latin_or_digit("!"));
    }

    #[test]
    fn filter_drops_single_chars() {
        let blocks = vec![
            block("Hello", 10.0, 10.0),
            block("V", 100.0, 10.0),
            block("甲乙", 10.0, 40.0),
            block("0", 200.0, 10.0),
        ];
        let kept = filter_single_char_blocks(blocks);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0].text, "Hello");
        assert_eq!(kept[1].text, "甲乙");
        assert_eq!(kept[0].id, 0);
        assert_eq!(kept[1].id, 1);
    }

    #[test]
    fn persistence_drops_flickering_text() {
        let mut f = BlockPersistenceFilter::new(80, 200);
        // Frame 1: icon noise + real text
        let out1 = f.filter(vec![block("HP", 10.0, 10.0), block("x#?", 200.0, 50.0)]);
        assert!(out1.is_empty(), "first sight should not emit");

        // Frame 2 soon after: HP stays, noise text changes (animation)
        std::thread::sleep(Duration::from_millis(40));
        let out2 = f.filter(vec![block("HP", 11.0, 10.0), block("@@", 201.0, 51.0)]);
        assert!(out2.is_empty(), "still under persist window");

        // Frame 3 after persist: only HP remains stable
        std::thread::sleep(Duration::from_millis(50));
        let out3 = f.filter(vec![block("HP", 10.0, 11.0), block("!!", 199.0, 49.0)]);
        assert_eq!(out3.len(), 1);
        assert_eq!(out3[0].text, "HP");
    }

    #[test]
    fn persistence_keeps_confirmed_across_missed_frame() {
        let mut f = BlockPersistenceFilter::new(50, 300);
        let _ = f.filter(vec![block("Menu", 10.0, 10.0)]);
        std::thread::sleep(Duration::from_millis(60));
        let confirmed = f.filter(vec![block("Menu", 12.0, 10.0)]);
        assert_eq!(confirmed.len(), 1);

        // Frame where OCR misses the static text (busy animated background).
        let held = f.filter(vec![]);
        assert_eq!(held.len(), 1, "hysteresis should keep confirmed text");
        assert_eq!(held[0].text, "Menu");
    }

    #[test]
    fn persistence_freezes_confirmed_bbox_against_jitter() {
        let mut f = BlockPersistenceFilter::new(50, 300);
        let _ = f.filter(vec![block("Menu", 100.0, 200.0)]);
        std::thread::sleep(Duration::from_millis(60));
        let confirmed = f.filter(vec![block("Menu", 100.0, 200.0)]);
        assert_eq!(confirmed.len(), 1);
        let frozen = confirmed[0].bbox;

        // Same text, animated-background OCR box wobble.
        let mut jittered = block("Menu", 104.0, 197.0);
        jittered.bbox.width = 76.0;
        jittered.bbox.height = 22.0;
        let out = f.filter(vec![jittered]);
        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0].bbox, frozen,
            "confirmed track should keep stable bbox"
        );
    }

    #[test]
    fn persistence_adopts_confirmed_bbox_on_real_move() {
        let mut f = BlockPersistenceFilter::new(50, 300);
        let _ = f.filter(vec![block("Menu", 100.0, 200.0)]);
        std::thread::sleep(Duration::from_millis(60));
        let _ = f.filter(vec![block("Menu", 100.0, 200.0)]);

        // ~20px center shift: beyond stabilize deadzone, still within track match.
        let moved = block("Menu", 100.0, 220.0);
        let out = f.filter(vec![moved.clone()]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].bbox, moved.bbox);
    }

    #[test]
    fn persistence_disabled_passthrough() {
        let mut f = BlockPersistenceFilter::new(0, 0);
        let blocks = vec![block("now", 0.0, 0.0)];
        let out = f.filter(blocks.clone());
        assert_eq!(out, blocks);
    }
}

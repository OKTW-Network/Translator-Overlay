//! OCR noise filters: single-token junk and temporal block persistence.

use std::time::{Duration, Instant};

use translator_core::{OcrBlock, OcrConfig, Rect, normalize_ocr_text};

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
    reindex(blocks.into_iter().filter(|b| !is_single_latin_or_digit(&b.text)).collect())
}

fn reindex(mut blocks: Vec<OcrBlock>) -> Vec<OcrBlock> {
    for (i, b) in blocks.iter_mut().enumerate() {
        b.id = i as u32;
    }
    blocks
}

/// Swap a confirmed track to a new OCR reading (text + box together).
fn adopt_reading(track: &mut Track, block: OcrBlock, now: Instant) {
    track.text = block.text.clone();
    track.bbox = block.bbox;
    track.last_block = block;
    track.pending_text = None;
    track.pending_since = None;
    track.thrash_since = None;
    track.first_stable_since = now;
}

/// Tracks per-region text over time and only emits blocks that stay put.
///
/// Fast-changing OCR (animated icons, particle effects) never reaches the
/// persistence threshold and is discarded. Static UI text is kept.
///
/// Once a track is confirmed, it keeps being emitted for `max_miss` even if
/// OCR misses it for a frame or two (common when the background animates).
///
/// If the reading at a spot keeps flipping, force-confirm after `max_unstable`
/// from first sighting so thrash still reaches translation. Unconfirmed tracks
/// are retained at least that long (not only `max_miss`) so the force timer
/// can actually fire.
#[derive(Debug)]
pub struct BlockPersistenceFilter {
    persist: Duration,
    max_miss: Duration,
    /// Force-confirm thrashing tracks after this long. Zero = disabled.
    max_unstable: Duration,
    tracks: Vec<Track>,
    /// Spatial quantize step in pixels (center matching).
    quant: i32,
}

#[derive(Debug, Clone)]
struct Track {
    text: String,
    bbox: Rect,
    /// Wall clock from first sighting at this spot (not reset on text thrash).
    first_seen: Instant,
    first_stable_since: Instant,
    last_seen: Instant,
    /// True once the text has lingered long enough at this spot.
    confirmed: bool,
    /// Last good sample (used for hysteresis when a frame misses the region).
    last_block: OcrBlock,
    /// While confirmed, OCR text that differs from the frozen emit (pending adopt).
    pending_text: Option<String>,
    /// When `pending_text` first appeared (or last changed).
    pending_since: Option<Instant>,
    /// When confirmed text first diverged from OCR (continuous thrash clock).
    thrash_since: Option<Instant>,
}

impl BlockPersistenceFilter {
    pub fn new(persist_ms: u64, max_miss_ms: u64) -> Self {
        Self::with_max_unstable(persist_ms, max_miss_ms, 2_000)
    }

    pub fn with_max_unstable(persist_ms: u64, max_miss_ms: u64, max_unstable_ms: u64) -> Self {
        Self {
            persist: Duration::from_millis(persist_ms),
            max_miss: Duration::from_millis(max_miss_ms.max(persist_ms.saturating_add(100))),
            max_unstable: Duration::from_millis(max_unstable_ms),
            tracks: Vec::new(),
            quant: 16,
        }
    }

    pub fn from_config(config: &OcrConfig) -> Self {
        Self::with_max_unstable(config.block_persist_ms, config.block_max_miss_ms, config.max_unstable_ms)
    }

    pub fn reset(&mut self) {
        self.tracks.clear();
    }

    pub fn is_enabled(&self) -> bool {
        !self.persist.is_zero()
    }

    /// How long to keep a track that is not currently confirmed.
    ///
    /// Must be ≥ `max_unstable` so thrashing regions live long enough to force
    /// confirm. Confirmed tracks still expire on the shorter `max_miss` grace.
    fn track_retain(&self, confirmed: bool) -> Duration {
        if confirmed || self.max_unstable.is_zero() {
            self.max_miss
        } else {
            self.max_miss.max(self.max_unstable)
        }
    }

    /// Update tracks from this frame and return only persistent blocks.
    ///
    /// When disabled (`block_persist_ms == 0`), returns `blocks` unchanged.
    pub fn filter(&mut self, blocks: Vec<OcrBlock>) -> Vec<OcrBlock> {
        if !self.is_enabled() {
            return blocks;
        }

        let now = Instant::now();
        let persist = self.persist;
        let max_unstable = self.max_unstable;
        // One OCR box per track per frame. Stacked unmerged lines sit inside the
        // match radius of their neighbors; without this they collapse onto the
        // last couple of tracks.
        let mut claimed = vec![false; self.tracks.len()];

        for block in blocks {
            let text_key = normalize_ocr_text(&block.text);
            if text_key.is_empty() {
                continue;
            }

            if let Some(idx) = self.find_track(&block, &text_key, &claimed) {
                claimed[idx] = true;
                let track = &mut self.tracks[idx];
                if normalize_ocr_text(&track.text) == text_key {
                    // Same region + same text → accumulate persistence.
                    track.last_seen = now;
                    track.pending_text = None;
                    track.pending_since = None;
                    track.thrash_since = None;
                    // Once confirmed, freeze the box. Overlay remap already keeps
                    // the caption box; detector jitter / trailing glyphs must not
                    // walk it. Unconfirmed tracks still track the latest sample.
                    if track.confirmed {
                        track.last_block.confidence = block.confidence;
                    } else {
                        track.bbox = block.bbox;
                        track.last_block = block;
                    }
                    if now.saturating_duration_since(track.first_stable_since) >= persist {
                        track.confirmed = true;
                    }
                } else if track.confirmed {
                    // Confirmed region, different OCR string: keep emitted text
                    // *and* the last caption box so overlay does not walk to the
                    // new line (and re-fit font/width) before a real re-translate.
                    // Adopt text + bbox together after linger or max_unstable.
                    track.last_seen = now;
                    if track.thrash_since.is_none() {
                        track.thrash_since = Some(now);
                    }

                    let pending_key = track.pending_text.as_deref().map(normalize_ocr_text).unwrap_or_default();
                    if pending_key != text_key {
                        track.pending_text = Some(block.text.clone());
                        track.pending_since = Some(now);
                    }
                    let linger_ready =
                        pending_key == text_key && now.saturating_duration_since(track.pending_since.unwrap_or(now)) >= persist;
                    let thrash_force = !max_unstable.is_zero()
                        && track
                            .thrash_since
                            .map(|t| now.saturating_duration_since(t) >= max_unstable)
                            .unwrap_or(false);
                    if linger_ready || thrash_force {
                        adopt_reading(track, block, now);
                    }
                } else {
                    // Unconfirmed text flip: restart same-text timer, keep first_seen.
                    track.text = block.text.clone();
                    track.bbox = block.bbox;
                    track.first_stable_since = now;
                    track.last_seen = now;
                    track.last_block = block;
                    track.pending_text = None;
                    track.pending_since = None;
                    track.thrash_since = None;
                }
                if !track.confirmed && !max_unstable.is_zero() && now.saturating_duration_since(track.first_seen) >= max_unstable {
                    track.confirmed = true;
                }
            } else {
                self.tracks.push(Track {
                    text: block.text.clone(),
                    bbox: block.bbox,
                    first_seen: now,
                    first_stable_since: now,
                    last_seen: now,
                    confirmed: false,
                    last_block: block,
                    pending_text: None,
                    pending_since: None,
                    thrash_since: None,
                });
                claimed.push(true);
            }
        }

        // Emit every confirmed track still within the miss grace window.
        // Hysteresis: keep showing text even if this frame's OCR missed the box.
        let max_miss = self.max_miss;
        let mut out: Vec<OcrBlock> = self
            .tracks
            .iter()
            .filter(|t| t.confirmed && now.saturating_duration_since(t.last_seen) <= max_miss)
            .map(|t| t.last_block.clone())
            .collect();

        out.sort_by(|a, b| {
            (a.bbox.y as i32)
                .cmp(&(b.bbox.y as i32))
                .then_with(|| (a.bbox.x as i32).cmp(&(b.bbox.x as i32)))
        });

        // Drop tracks that vanished past their retain window. Unconfirmed thrash
        // tracks use max(max_miss, max_unstable) so force-confirm can fire.
        let retain_confirmed = self.track_retain(true);
        let retain_unconfirmed = self.track_retain(false);
        self.tracks.retain(|t| {
            let limit = if t.confirmed { retain_confirmed } else { retain_unconfirmed };
            now.saturating_duration_since(t.last_seen) <= limit
        });

        reindex(out)
    }

    fn find_track(&self, block: &OcrBlock, text_key: &str, claimed: &[bool]) -> Option<usize> {
        let cx = block.bbox.x + block.bbox.width * 0.5;
        let cy = block.bbox.y + block.bbox.height * 0.5;
        let q = self.quant.max(1);

        let mut best: Option<(usize, f32)> = None;
        for (i, track) in self.tracks.iter().enumerate() {
            if claimed[i] {
                continue;
            }
            let tcx = track.bbox.x + track.bbox.width * 0.5;
            let tcy = track.bbox.y + track.bbox.height * 0.5;
            let dx = (cx - tcx).abs();
            let dy = (cy - tcy).abs();
            // Match by center proximity (tolerant of OCR box jitter and width
            // swings when trailing glyphs appear/disappear).
            let max_dx = (block.bbox.width.max(track.bbox.width) * 0.65).max(q as f32 * 3.0).max(24.0);
            let max_dy = (block.bbox.height.max(track.bbox.height) * 0.90).max(q as f32 * 2.5).max(16.0);
            if dx > max_dx || dy > max_dy {
                continue;
            }

            let same_text = normalize_ocr_text(&track.text) == text_key;
            let iou = block.bbox.iou(track.bbox);
            // Different text at nearly the same center still matches (OCR thrash
            // often changes box size enough to tank IoU below 0.2).
            if !same_text && iou < 0.05 && (dx > max_dx * 0.45 || dy > max_dy * 0.45) {
                continue;
            }

            // Prefer same text; otherwise prefer closer centers.
            let center_score = 1.0 - (dx / max_dx).clamp(0.0, 1.0) * 0.5 - (dy / max_dy).clamp(0.0, 1.0) * 0.5;
            let score = if same_text {
                iou + 1.0 + center_score
            } else {
                iou * 0.5 + center_score
            };
            if best.map(|(_, s)| score > s).unwrap_or(true) {
                best = Some((i, score));
            }
        }
        best.map(|(i, _)| i)
    }
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
            source_lines: 1,
        }
    }

    fn block_wh(text: &str, x: f32, y: f32, w: f32, h: f32) -> OcrBlock {
        OcrBlock {
            id: 0,
            text: text.to_string(),
            confidence: 0.9,
            bbox: Rect::new(x, y, w, h),
            source_lines: 1,
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
        assert_eq!(out[0].bbox, frozen, "confirmed track should keep stable bbox");
    }

    #[test]
    fn persistence_freezes_confirmed_bbox_on_real_move() {
        let mut f = BlockPersistenceFilter::new(50, 300);
        let _ = f.filter(vec![block("Menu", 100.0, 200.0)]);
        std::thread::sleep(Duration::from_millis(60));
        let confirmed = f.filter(vec![block("Menu", 100.0, 200.0)]);
        assert_eq!(confirmed.len(), 1);
        let frozen = confirmed[0].bbox;

        // Same text, different row: overlay remap keeps the caption box anyway.
        let moved = block("Menu", 100.0, 220.0);
        let out = f.filter(vec![moved]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].bbox, frozen);
    }

    #[test]
    fn persistence_pending_text_keeps_bbox_until_adopt() {
        let mut f = BlockPersistenceFilter::new(50, 300);
        let origin = block_wh("セリフ", 100.0, 200.0, 80.0, 22.0);
        let _ = f.filter(vec![origin.clone()]);
        std::thread::sleep(Duration::from_millis(60));
        let confirmed = f.filter(vec![origin.clone()]);
        assert_eq!(confirmed.len(), 1);
        let frozen = confirmed[0].bbox;

        // New line in the same region: wider box, different text.
        let next = block_wh("次の台詞です", 100.0, 200.0, 200.0, 24.0);
        let held = f.filter(vec![next.clone()]);
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].text, "セリフ");
        assert_eq!(held[0].bbox, frozen, "pending text must not walk or resize the box");

        std::thread::sleep(Duration::from_millis(60));
        let adopted = f.filter(vec![next.clone()]);
        assert_eq!(adopted.len(), 1);
        assert_eq!(adopted[0].text, "次の台詞です");
        assert_eq!(adopted[0].bbox, next.bbox, "adopt new text and its box together");
    }

    #[test]
    fn persistence_reset_drops_confirmed_tracks() {
        let mut f = BlockPersistenceFilter::new(50, 300);
        let _ = f.filter(vec![block("Menu", 100.0, 200.0)]);
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(f.filter(vec![block("Menu", 100.0, 200.0)]).len(), 1);

        f.reset();
        // Same text at a scaled position after a capture resize: must not emit
        // the old-pixel box (or any box) until the track persists again.
        let scaled = f.filter(vec![block("Menu", 150.0, 300.0)]);
        assert!(scaled.is_empty(), "reset must drop confirmed tracks");
    }

    #[test]
    fn persistence_disabled_passthrough() {
        let mut f = BlockPersistenceFilter::with_max_unstable(0, 0, 0);
        let blocks = vec![block("now", 0.0, 0.0)];
        let out = f.filter(blocks.clone());
        assert_eq!(out, blocks);
    }

    #[test]
    fn thrashing_text_force_confirms_after_max_unstable() {
        // Persist never settles (same-text window huge); text flips every frame.
        let mut f = BlockPersistenceFilter::with_max_unstable(10_000, 500, 80);
        let _ = f.filter(vec![block("hello!", 10.0, 10.0)]);
        assert!(f.filter(vec![block("hello", 11.0, 10.0)]).is_empty());

        std::thread::sleep(Duration::from_millis(90));
        let out = f.filter(vec![block("hello?", 10.0, 11.0)]);
        assert_eq!(out.len(), 1, "thrash should force-confirm after max_unstable");
        assert_eq!(out[0].text, "hello?");
    }

    #[test]
    fn thrashing_with_bbox_width_swing_still_force_confirms() {
        // Trailing glyph flicker often changes box width / center a lot.
        let mut f = BlockPersistenceFilter::with_max_unstable(10_000, 400, 80);
        let _ = f.filter(vec![block_wh("セリフ", 100.0, 200.0, 80.0, 22.0)]);
        let _ = f.filter(vec![block_wh("セリフ。", 98.0, 199.0, 160.0, 24.0)]);
        let _ = f.filter(vec![block_wh("セリフ", 102.0, 201.0, 84.0, 20.0)]);
        std::thread::sleep(Duration::from_millis(90));
        let out = f.filter(vec![block_wh("セリフ…", 96.0, 198.0, 170.0, 26.0)]);
        assert_eq!(out.len(), 1);
        assert!(out[0].text.starts_with("セリフ"));
    }

    #[test]
    fn unconfirmed_track_survives_longer_than_max_miss_for_force() {
        // max_miss is short; max_unstable is longer — track must live to force.
        let mut f = BlockPersistenceFilter::with_max_unstable(10_000, 50, 120);
        let _ = f.filter(vec![block("a!", 10.0, 10.0)]);
        std::thread::sleep(Duration::from_millis(70)); // past max_miss, under max_unstable
        // Still matched: last_seen updates. Simulate gap then return:
        let mid = f.filter(vec![]);
        assert!(mid.is_empty());
        std::thread::sleep(Duration::from_millis(30));
        // Track retained because unconfirmed retain = max_unstable (120).
        let out = f.filter(vec![block("a", 11.0, 10.0)]);
        // first_seen ~100ms ago; may not force yet. But track should exist (no emit).
        assert!(out.is_empty() || out.len() == 1);
        std::thread::sleep(Duration::from_millis(40));
        let forced = f.filter(vec![block("a?", 10.0, 11.0)]);
        assert_eq!(forced.len(), 1, "should force after total first_seen >= 120ms");
    }

    #[test]
    fn persistence_ellipsis_length_is_same_text() {
        let mut f = BlockPersistenceFilter::new(50, 300);
        let origin = block_wh("待って…", 100.0, 200.0, 80.0, 22.0);
        let _ = f.filter(vec![origin.clone()]);
        std::thread::sleep(Duration::from_millis(60));
        let confirmed = f.filter(vec![origin.clone()]);
        assert_eq!(confirmed.len(), 1);
        let frozen = confirmed[0].bbox;

        let longer = block_wh("待って………", 96.0, 198.0, 170.0, 26.0);
        let out = f.filter(vec![longer]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "待って…");
        assert_eq!(out[0].bbox, frozen, "ellipsis-length flicker must keep frozen bbox");
    }

    #[test]
    fn persistence_fullwidth_question_is_same_text() {
        let mut f = BlockPersistenceFilter::new(50, 300);
        let origin = block_wh("何？", 100.0, 200.0, 80.0, 22.0);
        let _ = f.filter(vec![origin.clone()]);
        std::thread::sleep(Duration::from_millis(60));
        let confirmed = f.filter(vec![origin.clone()]);
        assert_eq!(confirmed.len(), 1);
        let frozen = confirmed[0].bbox;

        let half = block_wh("何?", 102.0, 201.0, 76.0, 20.0);
        let out = f.filter(vec![half]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "何？");
        assert_eq!(out[0].bbox, frozen, "？ vs ? must keep frozen bbox");
    }

    #[test]
    fn stacked_unmerged_lines_do_not_share_a_track() {
        // Four overlapping dialogue lines (merge off). Matching must be 1:1 per
        // frame — otherwise later lines steal earlier tracks and only the last
        // couple survive.
        let mut f = BlockPersistenceFilter::new(50, 300);
        let lines = vec![
            block_wh("一行目です", 100.0, 200.0, 240.0, 32.0),
            block_wh("二行目です", 100.0, 228.0, 240.0, 32.0),
            block_wh("三行目です", 100.0, 256.0, 240.0, 32.0),
            block_wh("四行目です", 100.0, 284.0, 240.0, 32.0),
        ];
        let _ = f.filter(lines.clone());
        std::thread::sleep(Duration::from_millis(60));
        let out = f.filter(lines);
        assert_eq!(out.len(), 4, "texts={:?}", out.iter().map(|b| b.text.as_str()).collect::<Vec<_>>());
        assert_eq!(out[0].text, "一行目です");
        assert_eq!(out[3].text, "四行目です");
    }
}

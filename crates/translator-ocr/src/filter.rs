//! OCR noise filters: single-token junk and temporal block persistence.

use std::time::{Duration, Instant};

use translator_core::{OcrBlock, OcrConfig, Rect, normalize_ocr_text};

use crate::reindex;

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

/// True when normalized `new` is `old` up to a small OCR misread.
///
/// Animated backgrounds make OCR swap a glyph or two between frames; treating
/// those as new text re-translated and flickered the caption. Short strings must
/// match exactly, and growth past `old` (typewriter reveal) is always a change.
fn same_reading(old: &str, new: &str) -> bool {
    if old == new {
        return true;
    }
    let a: Vec<char> = old.chars().collect();
    let b: Vec<char> = new.chars().collect();
    let longest = a.len().max(b.len());
    if longest < 6 || (b.len() > a.len() && b.starts_with(&a)) {
        return false;
    }
    // Levenshtein distance, one row at a time.
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut diag = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let next = (diag + usize::from(ca != cb)).min(row[j] + 1).min(row[j + 1] + 1);
            diag = row[j + 1];
            row[j + 1] = next;
        }
    }
    row[b.len()] <= longest / 6
}

/// Swap a track to a new OCR reading (text + box together) and restart its vote.
fn adopt_reading(track: &mut Track, block: OcrBlock, now: Instant) {
    track.votes = vec![Vote {
        key: normalize_ocr_text(&block.text),
        text: block.text.clone(),
        hits: 1,
    }];
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
/// `settle` threshold and is discarded. Static UI text is kept, and a small
/// misread of it ([`same_reading`]) counts as the same text.
///
/// Once a track is confirmed, it keeps being emitted for `hold` even if
/// OCR misses it for a frame or two (common when the background animates).
///
/// If the reading at a spot keeps flipping, force-confirm after `max_wait`
/// from first sighting so thrash still reaches translation. Unconfirmed tracks
/// are retained at least that long (not only `hold`) so the force timer
/// can actually fire.
#[derive(Debug)]
pub struct BlockPersistenceFilter {
    settle: Duration,
    hold: Duration,
    /// Force-confirm thrashing tracks after this long. Zero = disabled.
    max_wait: Duration,
    tracks: Vec<Track>,
}

/// Spatial quantize step in pixels for center matching.
const QUANT: f32 = 16.0;

/// Votes per track before all counts halve, so old readings fade out.
const VOTE_DECAY_AT: u32 = 20;
/// Votes another reading needs (and at least twice the shown one's) to replace
/// a confirmed track's text.
const VOTE_MIN_SWITCH: u32 = 3;

/// How often one normalized reading was seen on a track.
#[derive(Debug, Clone)]
struct Vote {
    key: String,
    text: String,
    hits: u32,
}

#[derive(Debug, Clone)]
struct Track {
    /// Shown reading: the vote leader while settling, then frozen once confirmed.
    text: String,
    /// Readings within [`same_reading`] of `text` since the last adopt.
    votes: Vec<Vote>,
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
    pub fn new(settle_ms: u64, hold_ms: u64, max_wait_ms: u64) -> Self {
        Self {
            settle: Duration::from_millis(settle_ms),
            hold: Duration::from_millis(hold_ms),
            max_wait: Duration::from_millis(max_wait_ms),
            tracks: Vec::new(),
        }
    }

    pub fn from_config(config: &OcrConfig) -> Self {
        Self::new(config.settle_ms, config.hold_ms, config.max_wait_ms)
    }

    pub fn reset(&mut self) {
        self.tracks.clear();
    }

    /// Update tracks from this frame and return only persistent blocks.
    ///
    /// When disabled (`settle_ms == 0`), returns `blocks` unchanged.
    pub fn filter(&mut self, blocks: Vec<OcrBlock>) -> Vec<OcrBlock> {
        if self.settle.is_zero() {
            return blocks;
        }

        let now = Instant::now();
        let settle = self.settle;
        let max_wait = self.max_wait;
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
                if same_reading(&normalize_ocr_text(&track.text), &text_key) {
                    // Same region + same text (up to a misread) → accumulate persistence.
                    track.last_seen = now;
                    track.pending_text = None;
                    track.pending_since = None;
                    track.thrash_since = None;

                    // Vote per reading so a misread caught at confirm time does not stick.
                    match track.votes.iter_mut().find(|v| v.key == text_key) {
                        Some(v) => v.hits += 1,
                        None => track.votes.push(Vote {
                            key: text_key.clone(),
                            text: block.text.clone(),
                            hits: 1,
                        }),
                    }
                    if track.votes.iter().map(|v| v.hits).sum::<u32>() > VOTE_DECAY_AT {
                        track.votes.iter_mut().for_each(|v| v.hits /= 2);
                        track.votes.retain(|v| v.hits > 0);
                    }
                    let shown_key = normalize_ocr_text(&track.text);
                    let shown_hits = track.votes.iter().find(|v| v.key == shown_key).map_or(0, |v| v.hits);
                    let leader = track
                        .votes
                        .iter()
                        .max_by_key(|v| v.hits)
                        .filter(|v| v.key != shown_key)
                        .map(|v| (v.text.clone(), v.hits));

                    // Once confirmed, freeze text and box. Overlay remap already keeps
                    // the caption box; detector jitter / misreads must not walk it or
                    // re-translate, unless another reading clearly outvotes the shown
                    // one. A whole-box shift (scroll) follows the box but keeps the
                    // emitted text; unconfirmed tracks show the vote leader.
                    if !track.confirmed {
                        if let Some((text, _)) = leader {
                            track.text = text;
                        }
                        track.bbox = block.bbox;
                        track.last_block = OcrBlock {
                            text: track.text.clone(),
                            ..block
                        };
                    } else if let Some((text, _)) = leader.filter(|&(_, hits)| hits >= VOTE_MIN_SWITCH && hits >= shown_hits * 2) {
                        adopt_reading(track, OcrBlock { text, ..block }, now);
                    } else if block.bbox.shifted_from(track.bbox) {
                        track.bbox = block.bbox;
                        track.last_block = OcrBlock {
                            text: track.text.clone(),
                            ..block
                        };
                    } else {
                        track.last_block.confidence = block.confidence;
                    }
                    if now.saturating_duration_since(track.first_stable_since) >= settle {
                        track.confirmed = true;
                    }
                } else if track.confirmed {
                    // Confirmed region, different OCR string: keep emitted text
                    // *and* the last caption box so overlay does not walk to the
                    // new line (and re-fit font/width) before a real re-translate.
                    // Adopt text + bbox together after linger or max_wait.
                    track.last_seen = now;
                    if track.thrash_since.is_none() {
                        track.thrash_since = Some(now);
                    }

                    let pending_key = track.pending_text.as_deref().map(normalize_ocr_text).unwrap_or_default();
                    let same_pending = same_reading(&pending_key, &text_key);
                    if !same_pending {
                        track.pending_since = Some(now);
                    }
                    track.pending_text = Some(block.text.clone());
                    let linger_ready = same_pending && now.saturating_duration_since(track.pending_since.unwrap_or(now)) >= settle;
                    let thrash_force =
                        !max_wait.is_zero() && track.thrash_since.is_some_and(|t| now.saturating_duration_since(t) >= max_wait);
                    if linger_ready || thrash_force {
                        adopt_reading(track, block, now);
                    }
                } else {
                    // Unconfirmed text flip: restart same-text timer, keep first_seen.
                    track.last_seen = now;
                    adopt_reading(track, block, now);
                }
                if !track.confirmed && !max_wait.is_zero() && now.saturating_duration_since(track.first_seen) >= max_wait {
                    track.confirmed = true;
                }
            } else {
                self.tracks.push(Track {
                    text: block.text.clone(),
                    votes: vec![Vote {
                        key: text_key,
                        text: block.text.clone(),
                        hits: 1,
                    }],
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

        // Emit every confirmed track still within the hold window.
        // Hysteresis: keep showing text even if this frame's OCR missed the box.
        let hold = self.hold;
        let mut out: Vec<OcrBlock> = self
            .tracks
            .iter()
            .filter(|t| t.confirmed && now.saturating_duration_since(t.last_seen) <= hold)
            .map(|t| t.last_block.clone())
            .collect();

        out.sort_by(|a, b| {
            (a.bbox.y as i32)
                .cmp(&(b.bbox.y as i32))
                .then_with(|| (a.bbox.x as i32).cmp(&(b.bbox.x as i32)))
        });

        // Drop tracks that vanished past their retain window. Unconfirmed tracks
        // outlive a missed frame while settling, and `max_wait` so force-confirm can fire.
        let retain_unconfirmed = hold.max(settle + Duration::from_millis(100)).max(max_wait);
        self.tracks.retain(|t| {
            let limit = if t.confirmed { hold } else { retain_unconfirmed };
            now.saturating_duration_since(t.last_seen) <= limit
        });

        reindex(out)
    }

    fn find_track(&self, block: &OcrBlock, text_key: &str, claimed: &[bool]) -> Option<usize> {
        let (cx, cy) = block.bbox.center();

        let mut best: Option<(usize, f32)> = None;
        for (i, track) in self.tracks.iter().enumerate() {
            if claimed[i] {
                continue;
            }
            let (tcx, tcy) = track.bbox.center();
            let dx = (cx - tcx).abs();
            let dy = (cy - tcy).abs();
            // Match by center proximity (tolerant of OCR box jitter and width
            // swings when trailing glyphs appear/disappear).
            let max_dx = (block.bbox.width.max(track.bbox.width) * 0.65).max(QUANT * 3.0).max(24.0);
            let max_dy = (block.bbox.height.max(track.bbox.height) * 0.90).max(QUANT * 2.5).max(16.0);
            if dx > max_dx || dy > max_dy {
                continue;
            }

            let same_text = same_reading(&normalize_ocr_text(&track.text), text_key);
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
            source_height: 20.0,
            source_width: 80.0,
        }
    }

    fn block_wh(text: &str, x: f32, y: f32, w: f32, h: f32) -> OcrBlock {
        OcrBlock {
            id: 0,
            text: text.to_string(),
            confidence: 0.9,
            bbox: Rect::new(x, y, w, h),
            source_lines: 1,
            source_height: h,
            source_width: w,
        }
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
        let mut f = BlockPersistenceFilter::new(80, 200, 2_000);
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
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
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
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
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
    fn persistence_follows_confirmed_bbox_on_scroll() {
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let _ = f.filter(vec![block("Menu", 100.0, 200.0)]);
        std::thread::sleep(Duration::from_millis(60));
        let confirmed = f.filter(vec![block("Menu", 100.0, 200.0)]);
        assert_eq!(confirmed.len(), 1);

        // Same text, page scrolled one row (still inside the track match radius).
        let moved = block("Menu", 100.0, 220.0);
        let out = f.filter(vec![moved.clone()]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].bbox, moved.bbox, "whole-box shift should move the track");
    }

    #[test]
    fn persistence_pending_text_keeps_bbox_until_adopt() {
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
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
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
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
        let mut f = BlockPersistenceFilter::new(0, 0, 0);
        let blocks = vec![block("now", 0.0, 0.0)];
        let out = f.filter(blocks.clone());
        assert_eq!(out, blocks);
    }

    #[test]
    fn thrashing_text_force_confirms_after_max_wait() {
        // Settle never passes (same-text window huge); text flips every frame.
        let mut f = BlockPersistenceFilter::new(10_000, 500, 80);
        let _ = f.filter(vec![block("hello!", 10.0, 10.0)]);
        assert!(f.filter(vec![block("hello", 11.0, 10.0)]).is_empty());

        std::thread::sleep(Duration::from_millis(90));
        let out = f.filter(vec![block("hello?", 10.0, 11.0)]);
        assert_eq!(out.len(), 1, "thrash should force-confirm after max_wait");
        assert_eq!(out[0].text, "hello?");
    }

    #[test]
    fn thrashing_with_bbox_width_swing_still_force_confirms() {
        // Trailing glyph flicker often changes box width / center a lot.
        let mut f = BlockPersistenceFilter::new(10_000, 400, 80);
        let _ = f.filter(vec![block_wh("セリフ", 100.0, 200.0, 80.0, 22.0)]);
        let _ = f.filter(vec![block_wh("セリフ。", 98.0, 199.0, 160.0, 24.0)]);
        let _ = f.filter(vec![block_wh("セリフ", 102.0, 201.0, 84.0, 20.0)]);
        std::thread::sleep(Duration::from_millis(90));
        let out = f.filter(vec![block_wh("セリフ…", 96.0, 198.0, 170.0, 26.0)]);
        assert_eq!(out.len(), 1);
        assert!(out[0].text.starts_with("セリフ"));
    }

    #[test]
    fn unconfirmed_track_survives_longer_than_hold_for_force() {
        // hold and settle are short; max_wait is longer — track must live to force.
        let mut f = BlockPersistenceFilter::new(20, 50, 120);
        let _ = f.filter(vec![block("a!", 10.0, 10.0)]);
        std::thread::sleep(Duration::from_millis(70)); // past hold, under max_wait
        // Still matched: last_seen updates. Simulate gap then return:
        let mid = f.filter(vec![]);
        assert!(mid.is_empty());
        std::thread::sleep(Duration::from_millis(30));
        // Track retained because unconfirmed retain = max_wait (120).
        let _ = f.filter(vec![block("a", 11.0, 10.0)]);
        std::thread::sleep(Duration::from_millis(40));
        let forced = f.filter(vec![block("a?", 10.0, 11.0)]);
        assert_eq!(forced.len(), 1, "should force after total first_seen >= 120ms");
    }

    #[test]
    fn persistence_ocr_punct_fold_is_same_text() {
        for (origin_text, flicker) in [("待って…", "待って………"), ("何？", "何?"), ("そう〜", "そう~")] {
            let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
            let origin = block_wh(origin_text, 100.0, 200.0, 80.0, 22.0);
            let _ = f.filter(vec![origin.clone()]);
            std::thread::sleep(Duration::from_millis(60));
            let confirmed = f.filter(vec![origin.clone()]);
            assert_eq!(confirmed.len(), 1, "{origin_text}");
            let frozen = confirmed[0].bbox;

            let out = f.filter(vec![block_wh(flicker, 102.0, 201.0, 76.0, 20.0)]);
            assert_eq!(out.len(), 1, "{origin_text}");
            assert_eq!(out[0].text, origin_text);
            assert_eq!(out[0].bbox, frozen, "{origin_text} fold must keep frozen bbox");
        }
    }

    #[test]
    fn same_reading_tolerates_misreads_only() {
        assert!(same_reading("今日はいい天気ですね", "今日はいい天気でずね"), "one glyph swapped");
        assert!(same_reading("今日はいい天気ですね", "今日はいい天気です"), "trailing glyph missed");
        assert!(!same_reading("今日はいい天気", "今日はいい天気ですね"), "typewriter growth is a change");
        assert!(!same_reading("はいです", "はいでず"), "short strings must match exactly");
        assert!(!same_reading("今日はいい天気ですね", "明日は悪い天気ですよ"), "real change");
    }

    #[test]
    fn confirmed_track_ignores_misread() {
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let origin = block_wh("今日はいい天気ですね", 100.0, 200.0, 200.0, 22.0);
        let _ = f.filter(vec![origin.clone()]);
        std::thread::sleep(Duration::from_millis(60));
        let confirmed = f.filter(vec![origin]);
        assert_eq!(confirmed.len(), 1);

        // Animated background: one glyph misread and a box wobble, held past settle.
        let misread = block_wh("今日はいい天気でずね", 103.0, 201.0, 196.0, 22.0);
        let _ = f.filter(vec![misread.clone()]);
        std::thread::sleep(Duration::from_millis(60));
        let out = f.filter(vec![misread]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "今日はいい天気ですね", "misread must not replace emitted text");
        assert_eq!(out[0].bbox, confirmed[0].bbox);
    }

    #[test]
    fn confirm_takes_most_frequent_reading() {
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let right = block_wh("今日はいい天気ですね", 100.0, 200.0, 200.0, 22.0);
        let misread = block_wh("今日はいい天気でずね", 100.0, 200.0, 200.0, 22.0);
        let _ = f.filter(vec![right.clone()]);
        let _ = f.filter(vec![right]);
        std::thread::sleep(Duration::from_millis(60));
        // Misread on the confirming frame must not become the shown text.
        let out = f.filter(vec![misread]);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "今日はいい天気ですね");
    }

    #[test]
    fn confirmed_misread_is_corrected_by_votes() {
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let right = block_wh("今日はいい天気ですね", 100.0, 200.0, 200.0, 22.0);
        let misread = block_wh("今日はいい天気でずね", 100.0, 200.0, 200.0, 22.0);
        let _ = f.filter(vec![misread.clone()]);
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(f.filter(vec![misread])[0].text, "今日はいい天気でずね");

        // Shown reading has 2 votes; the right one switches in at twice that.
        for _ in 0..3 {
            assert_eq!(f.filter(vec![right.clone()])[0].text, "今日はいい天気でずね");
        }
        assert_eq!(f.filter(vec![right])[0].text, "今日はいい天気ですね");
    }

    #[test]
    fn alternating_misread_does_not_flip_confirmed_text() {
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let right = block_wh("今日はいい天気ですね", 100.0, 200.0, 200.0, 22.0);
        let misread = block_wh("今日はいい天気でずね", 100.0, 200.0, 200.0, 22.0);
        let _ = f.filter(vec![right.clone()]);
        std::thread::sleep(Duration::from_millis(60));
        let _ = f.filter(vec![right.clone()]);
        for _ in 0..20 {
            assert_eq!(f.filter(vec![misread.clone()])[0].text, "今日はいい天気ですね");
            assert_eq!(f.filter(vec![right.clone()])[0].text, "今日はいい天気ですね");
        }
    }

    #[test]
    fn typewriter_growth_is_adopted_after_settle() {
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let partial = block_wh("今日はいい天気", 100.0, 200.0, 140.0, 22.0);
        let _ = f.filter(vec![partial.clone()]);
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(f.filter(vec![partial]).len(), 1);

        let full = block_wh("今日はいい天気ですね", 100.0, 200.0, 200.0, 22.0);
        assert_eq!(f.filter(vec![full.clone()])[0].text, "今日はいい天気");
        std::thread::sleep(Duration::from_millis(60));
        assert_eq!(f.filter(vec![full])[0].text, "今日はいい天気ですね");
    }

    #[test]
    fn stacked_unmerged_lines_do_not_share_a_track() {
        // Four overlapping dialogue lines (merge off). Matching must be 1:1 per
        // frame — otherwise later lines steal earlier tracks and only the last
        // couple survive.
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
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

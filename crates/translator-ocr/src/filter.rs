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
    /// Last frame had no unconfirmed, changing, or hysteresis-held text.
    settled: bool,
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
            settled: true,
        }
    }

    pub fn from_config(config: &OcrConfig) -> Self {
        Self::new(config.settle_ms, config.hold_ms, config.max_wait_ms)
    }

    pub fn reset(&mut self) {
        self.tracks.clear();
        self.settled = true;
    }

    /// The last [`Self::filter`] saw only confirmed, unchanged text, and emitted nothing
    /// held over from earlier frames: the output is a stable page.
    pub fn settled(&self) -> bool {
        self.settled
    }

    /// Update tracks from this frame and return only persistent blocks.
    ///
    /// When disabled (`settle_ms == 0`), returns `blocks` unchanged.
    pub fn filter(&mut self, blocks: Vec<OcrBlock>) -> Vec<OcrBlock> {
        self.filter_at(blocks, Instant::now())
    }

    /// Deterministic-clock variant of [`Self::filter`].
    pub fn filter_at(&mut self, blocks: Vec<OcrBlock>, now: Instant) -> Vec<OcrBlock> {
        if self.settle.is_zero() {
            return blocks;
        }

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

                    let pending_key = track.pending_text.as_deref().map_or_default(normalize_ocr_text);
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

        // Emit every confirmed track whose shown text was on screen within the hold window
        // (missed by OCR or replaced by other text counts as gone).
        let hold = self.hold;
        let within = |t: Option<Instant>| t.is_none_or(|t| now.saturating_duration_since(t) <= hold);
        let live = |t: &Track| t.confirmed && within(Some(t.last_seen)) && within(t.thrash_since);
        // Seen text all confirmed and unchanged, and nothing emitted only by hysteresis.
        self.settled = self
            .tracks
            .iter()
            .zip(&claimed)
            .all(|(t, &seen)| if seen { t.confirmed && t.pending_text.is_none() } else { !live(t) });
        let mut out: Vec<OcrBlock> = self.tracks.iter().filter(|t| live(t)).map(|t| t.last_block.clone()).collect();

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

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

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
            block(" V ", 120.0, 10.0),
            block("甲", 10.0, 40.0),
            block("あ", 40.0, 40.0),
            block("?", 70.0, 40.0),
            block("0", 200.0, 10.0),
            block(" 0 ", 220.0, 10.0),
        ];
        let kept = filter_single_char_blocks(blocks);
        let texts: Vec<_> = kept.iter().map(|b| b.text.as_str()).collect();
        assert_eq!(texts, ["Hello", "甲", "あ", "?"]);
        assert_eq!(kept.iter().map(|b| b.id).collect::<Vec<_>>(), [0, 1, 2, 3]);
        assert!(!is_single_latin_or_digit(""));
        assert!(!is_single_latin_or_digit("é"));
    }

    #[test]
    fn persistence_drops_flickering_text() {
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(80, 200, 2_000);
        // Frame 1: icon noise + real text
        let out1 = f.filter_at(vec![block("HP", 10.0, 10.0), block("x#?", 200.0, 50.0)], t0);
        assert!(out1.is_empty(), "first sight should not emit");

        // Frame 2 soon after: HP stays, noise text changes (animation)
        let out2 = f.filter_at(vec![block("HP", 11.0, 10.0), block("@@", 201.0, 51.0)], t0 + ms(40));
        assert!(out2.is_empty(), "still under persist window");

        // Frame 3 after persist: only HP remains stable
        let out3 = f.filter_at(vec![block("HP", 10.0, 11.0), block("!!", 199.0, 49.0)], t0 + ms(90));
        assert_eq!(out3.len(), 1);
        assert_eq!(out3[0].text, "HP");
    }

    #[test]
    fn persistence_keeps_confirmed_across_missed_frame_then_expires_after_hold() {
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let _ = f.filter_at(vec![block("Menu", 10.0, 10.0)], t0);
        let confirmed = f.filter_at(vec![block("Menu", 12.0, 10.0)], t0 + ms(60));
        assert_eq!(confirmed.len(), 1);
        assert!(f.settled());

        // Frame where OCR misses the static text (busy animated background): hysteresis keeps it, marked unsettled.
        let held = f.filter_at(vec![], t0 + ms(100));
        assert_eq!(held.len(), 1, "hysteresis should keep confirmed text");
        assert_eq!(held[0].text, "Menu");
        assert!(!f.settled(), "held text is not on screen this frame");

        // Past hold (last_seen at t0 + 60ms -> expired by t0 + 361ms).
        let expired = f.filter_at(vec![], t0 + ms(361));
        assert!(expired.is_empty(), "confirmed track expires after hold");
        assert!(f.settled());
    }

    #[test]
    fn persistence_freezes_confirmed_bbox_against_jitter() {
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let _ = f.filter_at(vec![block("Menu", 100.0, 200.0)], t0);
        let confirmed = f.filter_at(vec![block("Menu", 100.0, 200.0)], t0 + ms(60));
        assert_eq!(confirmed.len(), 1);
        let frozen = confirmed[0].bbox;

        // Same text, animated-background OCR box wobble.
        let mut jittered = block("Menu", 104.0, 197.0);
        jittered.bbox.width = 76.0;
        jittered.bbox.height = 22.0;
        let out = f.filter_at(vec![jittered], t0 + ms(80));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].bbox, frozen, "confirmed track should keep stable bbox");
    }

    #[test]
    fn persistence_follows_confirmed_bbox_on_scroll() {
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let _ = f.filter_at(vec![block("Menu", 100.0, 200.0)], t0);
        let confirmed = f.filter_at(vec![block("Menu", 100.0, 200.0)], t0 + ms(60));
        assert_eq!(confirmed.len(), 1);

        // Same text, page scrolled one row (still inside the track match radius).
        let moved = block("Menu", 100.0, 220.0);
        let out = f.filter_at(vec![moved.clone()], t0 + ms(80));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].bbox, moved.bbox, "whole-box shift should move the track");
    }

    #[test]
    fn persistence_pending_text_keeps_bbox_until_adopt() {
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let origin = block_wh("セリフ", 100.0, 200.0, 80.0, 22.0);
        let _ = f.filter_at(vec![origin.clone()], t0);
        let confirmed = f.filter_at(vec![origin], t0 + ms(60));
        assert_eq!(confirmed.len(), 1);
        let frozen = confirmed[0].bbox;

        // New line in the same region: wider box, different text.
        let next = block_wh("次の台詞です", 100.0, 200.0, 200.0, 24.0);
        let held = f.filter_at(vec![next.clone()], t0 + ms(80));
        assert_eq!(held.len(), 1);
        assert_eq!(held[0].text, "セリフ");
        assert_eq!(held[0].bbox, frozen, "pending text must not walk or resize the box");

        let adopted = f.filter_at(vec![next.clone()], t0 + ms(140));
        assert_eq!(adopted.len(), 1);
        assert_eq!(adopted[0].text, "次の台詞です");
        assert_eq!(adopted[0].bbox, next.bbox, "adopt new text and its box together");
    }

    #[test]
    fn persistence_reset_drops_confirmed_tracks() {
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let _ = f.filter_at(vec![block("Menu", 100.0, 200.0)], t0);
        assert_eq!(f.filter_at(vec![block("Menu", 100.0, 200.0)], t0 + ms(60)).len(), 1);

        f.reset();
        // Same text at a scaled position after a capture resize: must not emit
        // the old-pixel box (or any box) until the track persists again.
        let scaled = f.filter_at(vec![block("Menu", 150.0, 300.0)], t0 + ms(80));
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
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(10_000, 500, 80);
        let _ = f.filter_at(vec![block("hello!", 10.0, 10.0)], t0);
        assert!(f.filter_at(vec![block("hello", 11.0, 10.0)], t0 + ms(20)).is_empty());

        let out = f.filter_at(vec![block("hello?", 10.0, 11.0)], t0 + ms(90));
        assert_eq!(out.len(), 1, "thrash should force-confirm after max_wait");
        assert_eq!(out[0].text, "hello?");

        // max_wait == 0 disables force-confirm.
        let mut no_force = BlockPersistenceFilter::new(10_000, 500, 0);
        let _ = no_force.filter_at(vec![block("a!", 10.0, 10.0)], t0);
        assert!(no_force.filter_at(vec![block("a?", 10.0, 10.0)], t0 + ms(5_000)).is_empty());
    }

    #[test]
    fn confirmed_thrash_force_adopts_after_max_wait() {
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(50, 300, 80);
        let _ = f.filter_at(vec![block("old", 10.0, 10.0)], t0);
        assert_eq!(f.filter_at(vec![block("old", 10.0, 10.0)], t0 + ms(60))[0].text, "old");

        // Replacement keeps changing every frame so pending settle never completes.
        assert_eq!(f.filter_at(vec![block("new1", 10.0, 10.0)], t0 + ms(70))[0].text, "old");
        assert_eq!(f.filter_at(vec![block("new2", 10.0, 10.0)], t0 + ms(110))[0].text, "old");
        let adopted = f.filter_at(vec![block("new3", 10.0, 10.0)], t0 + ms(150));
        assert_eq!(adopted[0].text, "new3", "thrashing confirmed track force-adopts at max_wait");
    }

    #[test]
    fn thrashing_with_bbox_width_swing_still_force_confirms() {
        // Trailing glyph flicker often changes box width / center a lot;
        // distinct normalized texts still match and force-confirm.
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(10_000, 400, 80);
        let _ = f.filter_at(vec![block_wh("セリフ", 100.0, 200.0, 80.0, 22.0)], t0);
        let _ = f.filter_at(vec![block_wh("セリフＡ", 98.0, 199.0, 160.0, 24.0)], t0 + ms(30));
        let _ = f.filter_at(vec![block_wh("セリフＢ", 102.0, 201.0, 84.0, 20.0)], t0 + ms(60));
        let out = f.filter_at(vec![block_wh("セリフＣ", 96.0, 198.0, 170.0, 26.0)], t0 + ms(90));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "セリフＣ");
    }

    #[test]
    fn unconfirmed_track_survives_longer_than_hold_for_force() {
        // hold and settle are short; max_wait (200ms > settle + 100ms = 120ms) is longer — track must live to force.
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(20, 50, 200);
        let _ = f.filter_at(vec![block("a!", 10.0, 10.0)], t0);
        // Missed frame at 130ms (> hold 50ms and > settle + 100ms 120ms, <= max_wait 200ms).
        let mid = f.filter_at(vec![], t0 + ms(130));
        assert!(mid.is_empty());

        // Track retained because unconfirmed retain = max_wait (200ms).
        let _ = f.filter_at(vec![block("a", 11.0, 10.0)], t0 + ms(170));
        let forced = f.filter_at(vec![block("a?", 10.0, 11.0)], t0 + ms(210));
        assert_eq!(forced.len(), 1, "should force after total first_seen >= 200ms");
        assert_eq!(forced[0].text, "a?");
    }

    #[test]
    fn persistence_ocr_punct_fold_is_same_text() {
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let origin = block_wh("待って…", 100.0, 200.0, 80.0, 22.0);
        let _ = f.filter_at(vec![origin.clone()], t0);
        let confirmed = f.filter_at(vec![origin], t0 + ms(60));
        let frozen = confirmed[0].bbox;

        let out = f.filter_at(vec![block_wh("待って………", 102.0, 201.0, 76.0, 20.0)], t0 + ms(80));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "待って…");
        assert_eq!(out[0].bbox, frozen);
    }

    #[test]
    fn same_reading_tolerates_misreads_only() {
        assert!(same_reading("今日はいい天気ですね", "今日はいい天気でずね"), "one glyph swapped");
        assert!(same_reading("今日はいい天気ですね", "今日はいい天気です"), "trailing glyph missed");
        // 1-char growth (dist = 1 <= 10/6): caught specifically by prefix-growth guard.
        assert!(!same_reading("今日はいい天気です", "今日はいい天気ですね"), "typewriter growth is a change");
        assert!(!same_reading("はいです", "はいでず"), "short strings must match exactly");
        assert!(!same_reading("今日はいい天気ですね", "明日は悪い天気ですよ"), "real change");
    }

    #[test]
    fn confirmed_track_ignores_misread() {
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let origin = block_wh("今日はいい天気ですね", 100.0, 200.0, 200.0, 22.0);
        let _ = f.filter_at(vec![origin.clone()], t0);
        let confirmed = f.filter_at(vec![origin], t0 + ms(60));
        assert_eq!(confirmed.len(), 1);

        // Animated background: one glyph misread and a box wobble, held past settle.
        let misread = block_wh("今日はいい天気でずね", 103.0, 201.0, 196.0, 22.0);
        let _ = f.filter_at(vec![misread.clone()], t0 + ms(80));
        let out = f.filter_at(vec![misread], t0 + ms(140));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "今日はいい天気ですね", "misread must not replace emitted text");
        assert_eq!(out[0].bbox, confirmed[0].bbox);
    }

    #[test]
    fn confirm_takes_most_frequent_reading() {
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let right = block_wh("今日はいい天気ですね", 100.0, 200.0, 200.0, 22.0);
        let misread = block_wh("今日はいい天気でずね", 100.0, 200.0, 200.0, 22.0);
        let _ = f.filter_at(vec![right.clone()], t0);
        let _ = f.filter_at(vec![right], t0 + ms(20));
        // Misread on the confirming frame must not become the shown text.
        let out = f.filter_at(vec![misread], t0 + ms(60));
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].text, "今日はいい天気ですね");
    }

    #[test]
    fn confirmed_misread_is_corrected_by_votes() {
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let right = block_wh("今日はいい天気ですね", 100.0, 200.0, 200.0, 22.0);
        let misread = block_wh("今日はいい天気でずね", 100.0, 200.0, 200.0, 22.0);
        let _ = f.filter_at(vec![misread.clone()], t0);
        assert_eq!(f.filter_at(vec![misread], t0 + ms(60))[0].text, "今日はいい天気でずね");

        // Shown reading has 2 votes; the right one switches in at twice that.
        for i in 1..=3 {
            assert_eq!(f.filter_at(vec![right.clone()], t0 + ms(60 + i * 10))[0].text, "今日はいい天気でずね");
        }
        assert_eq!(f.filter_at(vec![right], t0 + ms(100))[0].text, "今日はいい天気ですね");
    }

    #[test]
    fn alternating_misread_does_not_flip_confirmed_text() {
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let right = block_wh("今日はいい天気ですね", 100.0, 200.0, 200.0, 22.0);
        let misread = block_wh("今日はいい天気でずね", 100.0, 200.0, 200.0, 22.0);
        let _ = f.filter_at(vec![right.clone()], t0);
        let _ = f.filter_at(vec![right.clone()], t0 + ms(60));
        for i in 0..20 {
            assert_eq!(f.filter_at(vec![misread.clone()], t0 + ms(70 + i * 20))[0].text, "今日はいい天気ですね");
            assert_eq!(f.filter_at(vec![right.clone()], t0 + ms(80 + i * 20))[0].text, "今日はいい天気ですね");
        }
    }

    #[test]
    fn typewriter_growth_is_adopted_after_settle() {
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let partial = block_wh("今日はいい天気です", 100.0, 200.0, 180.0, 22.0);
        let _ = f.filter_at(vec![partial.clone()], t0);
        assert_eq!(f.filter_at(vec![partial], t0 + ms(60)).len(), 1);

        let full = block_wh("今日はいい天気ですね", 100.0, 200.0, 200.0, 22.0);
        assert_eq!(f.filter_at(vec![full.clone()], t0 + ms(70))[0].text, "今日はいい天気です");
        assert_eq!(f.filter_at(vec![full], t0 + ms(130))[0].text, "今日はいい天気ですね");
    }

    #[test]
    fn replaced_text_expires_after_hold_before_adopt() {
        // Settle longer than hold: the new reading is still pending when the old text expires.
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(100, 30, 2_000);
        let old = block_wh("アオイ", 100.0, 200.0, 80.0, 22.0);
        let _ = f.filter_at(vec![old.clone()], t0);
        assert_eq!(f.filter_at(vec![old], t0 + ms(110)).len(), 1);
        assert!(f.settled());

        let new = block_wh("ハル", 100.0, 200.0, 60.0, 22.0);
        assert_eq!(f.filter_at(vec![new.clone()], t0 + ms(120))[0].text, "アオイ", "held for hold");
        assert!(!f.settled());
        assert!(f.filter_at(vec![new.clone()], t0 + ms(160)).is_empty(), "replaced text expires after hold");
        assert!(!f.settled());

        assert_eq!(f.filter_at(vec![new], t0 + ms(230))[0].text, "ハル");
        assert!(f.settled());
    }

    #[test]
    fn unconfirmed_text_and_output_sorting() {
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let input = vec![
            block("   ", 0.0, 0.0),
            block("Bottom", 10.0, 100.0),
            block("TopRight", 200.0, 10.0),
            block("TopLeft", 10.0, 10.0),
        ];
        assert!(f.filter_at(input.clone(), t0).is_empty());
        assert!(!f.settled(), "new text is still settling");

        let out = f.filter_at(input, t0 + ms(60));
        let texts: Vec<_> = out.iter().map(|b| b.text.as_str()).collect();
        assert_eq!(texts, ["TopLeft", "TopRight", "Bottom"]);
        assert_eq!(out.iter().map(|b| b.id).collect::<Vec<_>>(), [0, 1, 2]);
    }

    #[test]
    fn stacked_unmerged_lines_do_not_share_a_track() {
        // Four overlapping dialogue lines (merge off). Matching must be 1:1 per
        // frame — otherwise later lines steal earlier tracks and only the last
        // couple survive.
        let t0 = Instant::now();
        let mut f = BlockPersistenceFilter::new(50, 300, 2_000);
        let lines = vec![
            block_wh("一行目です", 100.0, 200.0, 240.0, 32.0),
            block_wh("二行目です", 100.0, 228.0, 240.0, 32.0),
            block_wh("三行目です", 100.0, 256.0, 240.0, 32.0),
            block_wh("四行目です", 100.0, 284.0, 240.0, 32.0),
        ];
        let _ = f.filter_at(lines.clone(), t0);
        let out = f.filter_at(lines, t0 + ms(60));
        assert_eq!(out.len(), 4, "texts={:?}", out.iter().map(|b| b.text.as_str()).collect::<Vec<_>>());
        assert_eq!(out[0].text, "一行目です");
        assert_eq!(out[3].text, "四行目です");
    }
}

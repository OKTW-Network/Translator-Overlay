//! Text stability gate: wait until OCR content stops changing.

use std::{
    collections::hash_map::DefaultHasher,
    hash::{Hash, Hasher},
    time::{Duration, Instant},
};

use translator_core::{OcrBlock, OcrConfig, normalize_ocr_text};

/// Fingerprint of an OCR page for stability detection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct OcrFingerprint(u64);

impl OcrFingerprint {
    /// Content fingerprint for stability: normalized text only (sorted).
    ///
    /// BBox is intentionally ignored — detector jitter and animated backgrounds
    /// shift boxes every frame even when the readable text is unchanged.
    pub fn from_blocks(blocks: &[OcrBlock]) -> Self {
        let mut lines: Vec<String> = blocks
            .iter()
            .map(|b| normalize_ocr_text(&b.text))
            .filter(|t| !t.is_empty())
            .collect();
        // Order-independent so merge/detector reordering does not reset the gate.
        lines.sort();
        let mut hasher = DefaultHasher::new();
        for line in &lines {
            line.hash(&mut hasher);
        }
        lines.len().hash(&mut hasher);
        Self(hasher.finish())
    }
}

/// Consecutive observations of a new fingerprint needed before it is translated.
/// One-off blips from detector noise (or a block confirming a frame late) do not emit.
const SWITCH_HITS: u32 = 2;

/// Waits until the page of persisted OCR blocks is stable enough to translate.
///
/// Per-block linger already happens in the persistence filter, so the gate only
/// asks for [`SWITCH_HITS`] identical observations in a row. Until then a page that
/// was already emitted stays sticky ([`StabilityOutcome::AlreadyEmitted`]).
///
/// If the content keeps changing, the gate forces Ready with the latest observation
/// once `max_wait` has passed since the first observation that did not match the
/// emitted page. That clock survives returns to the emitted page.
#[derive(Debug, Default)]
pub struct StabilityGate {
    /// Force-translate after this long without a settled emit. Zero = disabled.
    max_wait: Duration,
    pending: Option<OcrFingerprint>,
    pending_hits: u32,
    last_emitted: Option<OcrFingerprint>,
    /// Wall clock for the current unsettled wait (survives fingerprint switches).
    unstable_since: Option<Instant>,
}

impl StabilityGate {
    pub fn new(max_wait_ms: u64) -> Self {
        Self {
            max_wait: Duration::from_millis(max_wait_ms),
            ..Self::default()
        }
    }

    pub fn from_config(config: &OcrConfig) -> Self {
        Self::new(config.max_wait_ms)
    }

    /// Full reset including emitted history (e.g. new target window).
    pub fn reset_all(&mut self) {
        *self = Self::new(self.max_wait.as_millis() as u64);
    }

    /// Feed a new fingerprint from the latest OCR (after block filters).
    pub fn observe(&mut self, fp: OcrFingerprint) -> StabilityOutcome {
        let now = Instant::now();

        if self.last_emitted == Some(fp) {
            self.pending = None;
            self.pending_hits = 0;
            // Do NOT clear `unstable_since` every frame: A↔B thrash would reset the
            // force timer on every return to A. Calm it only once the force window
            // elapsed while still on A (thrash resolved without a divergent emit).
            if self.force_due(now) {
                self.unstable_since = None;
            }
            return StabilityOutcome::AlreadyEmitted { fingerprint: fp };
        }

        self.unstable_since.get_or_insert(now);
        if self.pending == Some(fp) {
            self.pending_hits = self.pending_hits.saturating_add(1);
        } else {
            self.pending = Some(fp);
            self.pending_hits = 1;
        }

        let elapsed_ms = self
            .unstable_since
            .map(|t| now.saturating_duration_since(t).as_millis() as u64)
            .unwrap_or(0);
        if self.pending_hits >= SWITCH_HITS || self.force_due(now) {
            self.pending = None;
            self.pending_hits = 0;
            self.last_emitted = Some(fp);
            self.unstable_since = None;
            return StabilityOutcome::Ready {
                fingerprint: fp,
                elapsed_ms,
            };
        }
        match self.last_emitted {
            Some(fingerprint) => StabilityOutcome::AlreadyEmitted { fingerprint },
            None => StabilityOutcome::Waiting { elapsed_ms },
        }
    }

    fn force_due(&self, now: Instant) -> bool {
        !self.max_wait.is_zero()
            && self
                .unstable_since
                .is_some_and(|t| now.saturating_duration_since(t) >= self.max_wait)
    }

    /// Force-emit current fingerprint (manual translate), bypassing wait.
    pub fn force_emit(&mut self, fp: OcrFingerprint) -> StabilityOutcome {
        self.pending = None;
        self.pending_hits = 0;
        self.unstable_since = None;
        self.last_emitted = Some(fp);
        StabilityOutcome::Ready {
            fingerprint: fp,
            elapsed_ms: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StabilityOutcome {
    Waiting { elapsed_ms: u64 },
    Ready { fingerprint: OcrFingerprint, elapsed_ms: u64 },
    AlreadyEmitted { fingerprint: OcrFingerprint },
}

#[cfg(test)]
mod tests {
    use translator_core::{OcrBlock, Rect};

    use super::*;

    fn block(text: &str) -> OcrBlock {
        OcrBlock {
            id: 0,
            text: text.to_string(),
            confidence: 0.9,
            bbox: Rect::new(0.0, 0.0, 10.0, 10.0),
            source_lines: 1,
            source_height: 10.0,
            source_width: 10.0,
        }
    }

    fn fingerprint(text: &str) -> OcrFingerprint {
        OcrFingerprint::from_blocks(&[block(text)])
    }

    #[test]
    fn cold_start_ready_after_two_hits() {
        let mut gate = StabilityGate::new(0);
        let fp = fingerprint("hello");
        assert!(matches!(gate.observe(fp), StabilityOutcome::Waiting { .. }));
        assert!(matches!(gate.observe(fp), StabilityOutcome::Ready { .. }));
        assert!(matches!(gate.observe(fp), StabilityOutcome::AlreadyEmitted { .. }));
    }

    #[test]
    fn reset_all_requires_two_hits_again() {
        // Models post-empty: clear_stale_overlay → reset_all → wait again.
        let mut gate = StabilityGate::new(0);
        let fp = fingerprint("page");
        let _ = gate.observe(fp);
        assert!(matches!(gate.observe(fp), StabilityOutcome::Ready { .. }));

        gate.reset_all();
        assert!(matches!(gate.observe(fp), StabilityOutcome::Waiting { .. }));
        assert!(matches!(gate.observe(fp), StabilityOutcome::Ready { .. }));
    }

    #[test]
    fn single_frame_blip_keeps_emitted_page() {
        let mut gate = StabilityGate::new(0);
        let a = fingerprint("menu");
        let b = fingerprint("noise");
        let _ = gate.observe(a);
        assert!(matches!(gate.observe(a), StabilityOutcome::Ready { .. }));
        // One-off different reading — keep AlreadyEmitted on sticky "menu".
        assert!(matches!(gate.observe(b), StabilityOutcome::AlreadyEmitted { fingerprint } if fingerprint == a));
        assert!(matches!(gate.observe(a), StabilityOutcome::AlreadyEmitted { .. }));
    }

    #[test]
    fn sustained_change_switches_fingerprint() {
        let mut gate = StabilityGate::new(0);
        let a = fingerprint("old");
        let b = fingerprint("new");
        let _ = gate.observe(a);
        assert!(matches!(gate.observe(a), StabilityOutcome::Ready { .. }));
        // First b is a blip — still old emitted.
        assert!(matches!(gate.observe(b), StabilityOutcome::AlreadyEmitted { .. }));
        // The second b confirms the switch.
        assert!(matches!(gate.observe(b), StabilityOutcome::Ready { fingerprint, .. } if fingerprint == b));
    }

    #[test]
    fn thrashing_force_ready_after_max_wait() {
        // Never two identical hits in a row, so only max_wait can end the a, b, a thrash.
        let mut gate = StabilityGate::new(80);
        let a = fingerprint("alpha");
        let b = fingerprint("beta");

        assert!(matches!(gate.observe(a), StabilityOutcome::Waiting { .. }));
        std::thread::sleep(Duration::from_millis(40));
        assert!(matches!(gate.observe(b), StabilityOutcome::Waiting { .. }));
        std::thread::sleep(Duration::from_millis(50));
        // Total wait ≥ 80ms across switches → force translate latest.
        assert!(matches!(
            gate.observe(a),
            StabilityOutcome::Ready {
                fingerprint: fp,
                ..
            } if fp == a
        ));
    }

    #[test]
    fn post_emit_thrash_force_ready_without_two_identical_hits() {
        // After the first translate, alternating A and B never gives B two hits in a row,
        // alone, but max_wait must still force re-translate with latest B.
        let mut gate = StabilityGate::new(90);
        let a = fingerprint("line-a");
        let b = fingerprint("line-b");

        let _ = gate.observe(a);
        assert!(matches!(gate.observe(a), StabilityOutcome::Ready { .. }));
        // Alternating blips: each is only one hit of the other fp.
        assert!(matches!(gate.observe(b), StabilityOutcome::AlreadyEmitted { .. }));
        assert!(matches!(gate.observe(a), StabilityOutcome::AlreadyEmitted { .. }));
        std::thread::sleep(Duration::from_millis(50));
        assert!(matches!(gate.observe(b), StabilityOutcome::AlreadyEmitted { .. }));
        assert!(matches!(gate.observe(a), StabilityOutcome::AlreadyEmitted { .. }));
        std::thread::sleep(Duration::from_millis(50));
        // Wait clock must survive returns to A; force Ready with divergent B.
        assert!(matches!(
            gate.observe(b),
            StabilityOutcome::Ready {
                fingerprint: fp,
                ..
            } if fp == b
        ));
    }

    #[test]
    fn fingerprint_ignores_ocr_punct_fold() {
        assert_eq!(OcrFingerprint::from_blocks(&[block("待って…")]), OcrFingerprint::from_blocks(&[block("待って………")]));
        assert_eq!(OcrFingerprint::from_blocks(&[block("待って…")]), OcrFingerprint::from_blocks(&[block("待って")]));
        assert_eq!(OcrFingerprint::from_blocks(&[block("何？")]), OcrFingerprint::from_blocks(&[block("何?")]));
        assert_eq!(OcrFingerprint::from_blocks(&[block("そう〜")]), OcrFingerprint::from_blocks(&[block("そう~")]));
        assert_eq!(OcrFingerprint::from_blocks(&[block("そう～")]), OcrFingerprint::from_blocks(&[block("そう~")]));
    }

    #[test]
    fn fingerprint_ignores_bbox_jitter() {
        let a = OcrBlock {
            id: 0,
            text: "Hello".into(),
            confidence: 0.9,
            bbox: Rect::new(10.0, 10.0, 100.0, 20.0),
            source_lines: 1,
            source_height: 20.0,
            source_width: 100.0,
        };
        let b = OcrBlock {
            id: 0,
            text: "Hello".into(),
            confidence: 0.9,
            bbox: Rect::new(14.0, 18.0, 98.0, 22.0),
            source_lines: 1,
            source_height: 22.0,
            source_width: 98.0,
        };
        assert_eq!(OcrFingerprint::from_blocks(&[a]), OcrFingerprint::from_blocks(&[b]));
    }

    #[test]
    fn fingerprint_ignores_block_order() {
        let a = block("one");
        let mut b = block("two");
        b.bbox.y = 40.0;
        assert_eq!(OcrFingerprint::from_blocks(&[a.clone(), b.clone()]), OcrFingerprint::from_blocks(&[b, a]));
    }

    #[test]
    fn force_emit_bypasses_wait() {
        let mut gate = StabilityGate::new(10_000);
        let fp = fingerprint("now");
        assert!(matches!(gate.force_emit(fp), StabilityOutcome::Ready { elapsed_ms: 0, .. }));
    }
}

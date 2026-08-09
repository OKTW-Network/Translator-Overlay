//! Text stability gate: wait until OCR content stops changing.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{Duration, Instant};

use translator_core::{OcrBlock, OcrConfig};

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
            .map(|b| normalize_fp_text(&b.text))
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

    pub fn from_text(text: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        normalize_fp_text(text).hash(&mut hasher);
        Self(hasher.finish())
    }
}

fn normalize_fp_text(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Waits until OCR content is stable enough to translate.
///
/// Design notes:
/// - **Hits OR time**: Ready when the same fingerprint is seen `min_hits` times
///   in a row, or held for `stable_duration` (whichever first). Slow OCR still
///   advances on the second identical result instead of waiting forever.
/// - **Sticky switch**: a new fingerprint must appear `switch_hits` times in a
///   row before it replaces the active one (ignores single-frame OCR thrash).
#[derive(Debug)]
pub struct StabilityGate {
    stable_duration: Duration,
    /// Consecutive identical observations required (OR with duration).
    min_hits: u32,
    /// Consecutive observations of a *new* fp required before abandoning active.
    switch_hits: u32,
    active: Option<OcrFingerprint>,
    active_since: Option<Instant>,
    active_hits: u32,
    pending: Option<OcrFingerprint>,
    pending_hits: u32,
    last_emitted: Option<OcrFingerprint>,
}

impl StabilityGate {
    pub fn new(stable_duration_ms: u64) -> Self {
        Self {
            stable_duration: Duration::from_millis(stable_duration_ms),
            // 1 = translate as soon as a fingerprint is accepted. Block persistence
            // (and sticky switch) already filter flicker; requiring 2+ OCR passes
            // left static UIs stuck on "Waiting for stable" when OCR is slow.
            min_hits: 1,
            // Ignore one-off fingerprint blips from detector noise.
            switch_hits: 2,
            active: None,
            active_since: None,
            active_hits: 0,
            pending: None,
            pending_hits: 0,
            last_emitted: None,
        }
    }

    pub fn from_config(config: &OcrConfig) -> Self {
        Self::new(config.stable_duration_ms)
    }

    pub fn reset(&mut self) {
        self.active = None;
        self.active_since = None;
        self.active_hits = 0;
        self.pending = None;
        self.pending_hits = 0;
        // Keep last_emitted so re-showing the same page after reset still dedupes
        // unless force_emit is used.
    }

    /// Full reset including emitted history (e.g. new target window).
    pub fn reset_all(&mut self) {
        self.reset();
        self.last_emitted = None;
    }

    /// Feed a new fingerprint from the latest OCR (after block filters).
    pub fn observe(&mut self, fp: OcrFingerprint) -> StabilityOutcome {
        let now = Instant::now();

        // Same as active candidate — accumulate stability.
        if self.active == Some(fp) {
            self.pending = None;
            self.pending_hits = 0;
            self.active_hits = self.active_hits.saturating_add(1);
            let since = self.active_since.get_or_insert(now);
            let elapsed = now.saturating_duration_since(*since);

            if self.last_emitted == Some(fp) {
                return StabilityOutcome::AlreadyEmitted { fingerprint: fp };
            }

            if self.is_stable(elapsed) {
                self.last_emitted = Some(fp);
                return StabilityOutcome::Ready {
                    fingerprint: fp,
                    elapsed_ms: elapsed.as_millis() as u64,
                };
            }

            return StabilityOutcome::Waiting {
                elapsed_ms: elapsed.as_millis() as u64,
            };
        }

        // Different from active — require repeated evidence before switching
        // (unless we have no active yet).
        if self.active.is_none() {
            self.commit_active(fp, now);
            return self.outcome_for_active(now);
        }

        if self.pending == Some(fp) {
            self.pending_hits = self.pending_hits.saturating_add(1);
        } else {
            self.pending = Some(fp);
            self.pending_hits = 1;
        }

        if self.pending_hits >= self.switch_hits {
            self.commit_active(fp, now);
            return self.outcome_for_active(now);
        }

        // Still holding previous active while the blip is unconfirmed.
        self.outcome_for_active(now)
    }

    fn is_stable(&self, elapsed: Duration) -> bool {
        // Prefer hit-count so slow OCR (1 frame / second) still progresses.
        // Duration is a fallback when min_hits is raised.
        self.active_hits >= self.min_hits
            || (!self.stable_duration.is_zero() && elapsed >= self.stable_duration)
    }

    fn commit_active(&mut self, fp: OcrFingerprint, now: Instant) {
        self.active = Some(fp);
        self.active_since = Some(now);
        self.active_hits = 1;
        self.pending = None;
        self.pending_hits = 0;
    }

    fn outcome_for_active(&mut self, now: Instant) -> StabilityOutcome {
        let Some(active) = self.active else {
            return StabilityOutcome::Changed;
        };
        let since = self.active_since.unwrap_or(now);
        let elapsed = now.saturating_duration_since(since);

        if self.last_emitted == Some(active) {
            return StabilityOutcome::AlreadyEmitted {
                fingerprint: active,
            };
        }
        if self.is_stable(elapsed) {
            self.last_emitted = Some(active);
            return StabilityOutcome::Ready {
                fingerprint: active,
                elapsed_ms: elapsed.as_millis() as u64,
            };
        }
        if self.active_hits <= 1 {
            StabilityOutcome::Changed
        } else {
            StabilityOutcome::Waiting {
                elapsed_ms: elapsed.as_millis() as u64,
            }
        }
    }

    /// Force-emit current fingerprint (manual translate), bypassing wait.
    pub fn force_emit(&mut self, fp: OcrFingerprint) -> StabilityOutcome {
        let now = Instant::now();
        self.commit_active(fp, now);
        self.last_emitted = Some(fp);
        StabilityOutcome::Ready {
            fingerprint: fp,
            elapsed_ms: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StabilityOutcome {
    Changed,
    Waiting {
        elapsed_ms: u64,
    },
    Ready {
        fingerprint: OcrFingerprint,
        elapsed_ms: u64,
    },
    AlreadyEmitted {
        fingerprint: OcrFingerprint,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use translator_core::{OcrBlock, Rect};

    fn block(text: &str) -> OcrBlock {
        OcrBlock {
            id: 0,
            text: text.to_string(),
            confidence: 0.9,
            bbox: Rect::new(0.0, 0.0, 10.0, 10.0),
            source_lines: 1,
        }
    }

    #[test]
    fn first_accepted_fingerprint_ready_immediately() {
        let mut gate = StabilityGate::new(10_000);
        let fp = OcrFingerprint::from_text("hello");
        // min_hits=1: no need to wait for a second OCR pass or wall clock.
        assert!(matches!(gate.observe(fp), StabilityOutcome::Ready { .. }));
        assert!(matches!(
            gate.observe(fp),
            StabilityOutcome::AlreadyEmitted { .. }
        ));
    }

    #[test]
    fn duration_alone_can_ready_when_min_hits_high() {
        let mut gate = StabilityGate::new(40);
        gate.min_hits = 100;
        let fp = OcrFingerprint::from_text("hello");
        // First hit not enough for min_hits=100.
        assert!(matches!(gate.observe(fp), StabilityOutcome::Changed));
        std::thread::sleep(Duration::from_millis(50));
        assert!(matches!(gate.observe(fp), StabilityOutcome::Ready { .. }));
    }

    #[test]
    fn single_frame_blip_does_not_reset_active() {
        let mut gate = StabilityGate::new(10_000);
        let a = OcrFingerprint::from_text("menu");
        let b = OcrFingerprint::from_text("noise");
        assert!(matches!(gate.observe(a), StabilityOutcome::Ready { .. }));
        // One-off different reading — keep AlreadyEmitted on sticky "menu".
        assert!(matches!(
            gate.observe(b),
            StabilityOutcome::AlreadyEmitted { .. }
        ));
        assert!(matches!(
            gate.observe(a),
            StabilityOutcome::AlreadyEmitted { .. }
        ));
    }

    #[test]
    fn sustained_change_switches_fingerprint() {
        let mut gate = StabilityGate::new(10_000);
        let a = OcrFingerprint::from_text("old");
        let b = OcrFingerprint::from_text("new");
        assert!(matches!(gate.observe(a), StabilityOutcome::Ready { .. }));
        // First b is a blip — still old emitted.
        assert!(matches!(
            gate.observe(b),
            StabilityOutcome::AlreadyEmitted { .. }
        ));
        // Second b confirms switch → Ready for new page.
        assert!(matches!(gate.observe(b), StabilityOutcome::Ready { .. }));
    }

    #[test]
    fn fingerprint_changes_with_text() {
        let a = OcrFingerprint::from_blocks(&[block("a")]);
        let b = OcrFingerprint::from_blocks(&[block("b")]);
        assert_ne!(a, b);
    }

    #[test]
    fn fingerprint_ignores_bbox_jitter() {
        let a = OcrBlock {
            id: 0,
            text: "Hello".into(),
            confidence: 0.9,
            bbox: Rect::new(10.0, 10.0, 100.0, 20.0),
            source_lines: 1,
        };
        let b = OcrBlock {
            id: 0,
            text: "Hello".into(),
            confidence: 0.9,
            bbox: Rect::new(14.0, 18.0, 98.0, 22.0),
            source_lines: 1,
        };
        assert_eq!(
            OcrFingerprint::from_blocks(&[a]),
            OcrFingerprint::from_blocks(&[b])
        );
    }

    #[test]
    fn fingerprint_ignores_block_order() {
        let a = block("one");
        let mut b = block("two");
        b.bbox.y = 40.0;
        assert_eq!(
            OcrFingerprint::from_blocks(&[a.clone(), b.clone()]),
            OcrFingerprint::from_blocks(&[b, a])
        );
    }

    #[test]
    fn force_emit_bypasses_wait() {
        let mut gate = StabilityGate::new(10_000);
        let fp = OcrFingerprint::from_text("now");
        assert!(matches!(
            gate.force_emit(fp),
            StabilityOutcome::Ready { elapsed_ms: 0, .. }
        ));
    }
}

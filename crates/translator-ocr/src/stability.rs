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

    pub fn from_text(text: &str) -> Self {
        let mut hasher = DefaultHasher::new();
        normalize_ocr_text(text).hash(&mut hasher);
        Self(hasher.finish())
    }
}

/// Waits until OCR content is stable enough to translate.
///
/// Design notes:
/// - **Cold start**: when nothing has been emitted yet (`last_emitted` is `None`,
///   including after [`StabilityGate::reset_all`]), Ready requires holding the
///   fingerprint for `stable_duration` (not `min_hits` alone). First OCR and
///   post-empty reappearance honor the Stable wait setting.
/// - **Hits OR time** (after an emit): Ready when the same fingerprint is seen
///   `min_hits` times in a row, or held for `stable_duration` (whichever first).
///   Slow OCR still advances on the second identical result instead of waiting
///   forever. `stable_duration == 0` disables the duration requirement entirely.
/// - **Sticky switch**: a new fingerprint must appear `switch_hits` times in a
///   row before it replaces the active one (ignores single-frame OCR thrash).
/// - **Max unstable**: if content keeps changing and never settles, force Ready
///   with the latest observation after `max_unstable` (wall clock from first
///   non-matching observation after emit). Survives sticky mid-switch blips.
#[derive(Debug)]
pub struct StabilityGate {
    stable_duration: Duration,
    /// Force-translate after this long without a settled emit. Zero = disabled.
    max_unstable: Duration,
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
    /// Wall clock for the current unsettled wait (survives fingerprint switches).
    unstable_since: Option<Instant>,
}

impl StabilityGate {
    pub fn new(stable_duration_ms: u64) -> Self {
        Self::with_max_unstable(stable_duration_ms, 2_000)
    }

    pub fn with_max_unstable(stable_duration_ms: u64, max_unstable_ms: u64) -> Self {
        Self {
            stable_duration: Duration::from_millis(stable_duration_ms),
            max_unstable: Duration::from_millis(max_unstable_ms),
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
            unstable_since: None,
        }
    }

    pub fn from_config(config: &OcrConfig) -> Self {
        Self::with_max_unstable(config.stable_duration_ms, config.max_unstable_ms)
    }

    /// Full reset including emitted history (e.g. new target window).
    pub fn reset_all(&mut self) {
        self.active = None;
        self.active_since = None;
        self.active_hits = 0;
        self.pending = None;
        self.pending_hits = 0;
        self.unstable_since = None;
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
                // Matches last emit. Do NOT clear `unstable_since` every frame —
                // A↔B thrash would reset the force timer on every return to A.
                // Only calm the clock once force window elapsed while still on A
                // (thrash resolved without a divergent emit).
                if self.force_due(now) {
                    self.unstable_since = None;
                }
                return StabilityOutcome::AlreadyEmitted { fingerprint: fp };
            }

            self.mark_unstable(now);
            if self.is_stable(elapsed) || self.force_due(now) {
                return self.emit_ready(fp, self.wait_elapsed_ms(now));
            }

            return StabilityOutcome::Waiting {
                elapsed_ms: self.wait_elapsed_ms(now),
            };
        }

        // Different from active — require repeated evidence before switching
        // (unless we have no active yet).
        if self.active.is_none() {
            self.commit_active(fp, now);
            self.mark_unstable(now);
            return self.outcome_for_active(now, fp);
        }

        // Diverged from active: keep max-unstable wait across sticky blips.
        self.mark_unstable(now);

        // Long thrash: force the latest reading (even without switch_hits).
        if self.force_due(now) && self.last_emitted != Some(fp) {
            self.commit_active(fp, now);
            return self.emit_ready(fp, self.wait_elapsed_ms(now));
        }

        if self.pending == Some(fp) {
            self.pending_hits = self.pending_hits.saturating_add(1);
        } else {
            self.pending = Some(fp);
            self.pending_hits = 1;
        }

        if self.pending_hits >= self.switch_hits {
            self.commit_active(fp, now);
            return self.outcome_for_active(now, fp);
        }

        // Still holding previous active while the blip is unconfirmed.
        self.outcome_for_active(now, fp)
    }

    fn mark_unstable(&mut self, now: Instant) {
        if self.unstable_since.is_none() {
            self.unstable_since = Some(now);
        }
    }

    fn force_due(&self, now: Instant) -> bool {
        if self.max_unstable.is_zero() {
            return false;
        }
        self.unstable_since
            .map(|t| now.saturating_duration_since(t) >= self.max_unstable)
            .unwrap_or(false)
    }

    fn wait_elapsed_ms(&self, now: Instant) -> u64 {
        self.unstable_since
            .or(self.active_since)
            .map(|t| now.saturating_duration_since(t).as_millis() as u64)
            .unwrap_or(0)
    }

    fn emit_ready(&mut self, fp: OcrFingerprint, elapsed_ms: u64) -> StabilityOutcome {
        self.last_emitted = Some(fp);
        self.unstable_since = None;
        StabilityOutcome::Ready {
            fingerprint: fp,
            elapsed_ms,
        }
    }

    fn is_stable(&self, elapsed: Duration) -> bool {
        let duration_ok = !self.stable_duration.is_zero() && elapsed >= self.stable_duration;
        // Cold start / after reset_all: honor Stable wait; do not Ready on first hit.
        if self.last_emitted.is_none() && !self.stable_duration.is_zero() {
            return duration_ok;
        }
        // After an emit: prefer hit-count so slow OCR still progresses; duration
        // remains a fallback when min_hits is raised.
        self.active_hits >= self.min_hits || duration_ok
    }

    fn commit_active(&mut self, fp: OcrFingerprint, now: Instant) {
        self.active = Some(fp);
        self.active_since = Some(now);
        self.active_hits = 1;
        self.pending = None;
        self.pending_hits = 0;
    }

    fn outcome_for_active(&mut self, now: Instant, latest: OcrFingerprint) -> StabilityOutcome {
        let Some(active) = self.active else {
            return StabilityOutcome::Changed;
        };
        let since = self.active_since.unwrap_or(now);
        let elapsed = now.saturating_duration_since(since);

        if self.last_emitted == Some(active) {
            // Sticky hold of a previously emitted page.
            if latest != active {
                self.mark_unstable(now);
                if self.force_due(now) {
                    self.commit_active(latest, now);
                    return self.emit_ready(latest, self.wait_elapsed_ms(now));
                }
            } else if self.force_due(now) {
                // Thrash window elapsed while back on emitted content — calm.
                self.unstable_since = None;
            }
            return StabilityOutcome::AlreadyEmitted { fingerprint: active };
        }

        // Prefer latest observation when force-timeout fires mid-switch.
        if self.force_due(now) {
            let emit_fp = if latest != active { latest } else { active };
            if self.last_emitted == Some(emit_fp) {
                self.unstable_since = None;
                return StabilityOutcome::AlreadyEmitted { fingerprint: emit_fp };
            }
            if emit_fp != active {
                self.commit_active(emit_fp, now);
            }
            return self.emit_ready(emit_fp, self.wait_elapsed_ms(now));
        }

        if self.is_stable(elapsed) {
            return self.emit_ready(active, self.wait_elapsed_ms(now));
        }
        if self.active_hits <= 1 {
            StabilityOutcome::Changed
        } else {
            StabilityOutcome::Waiting {
                elapsed_ms: self.wait_elapsed_ms(now),
            }
        }
    }

    /// Force-emit current fingerprint (manual translate), bypassing wait.
    pub fn force_emit(&mut self, fp: OcrFingerprint) -> StabilityOutcome {
        let now = Instant::now();
        self.commit_active(fp, now);
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
    Changed,
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
        }
    }

    #[test]
    fn cold_start_waits_stable_duration() {
        let mut gate = StabilityGate::with_max_unstable(40, 0);
        let fp = OcrFingerprint::from_text("hello");
        // Cold start ignores min_hits=1 until Stable wait elapses.
        assert!(matches!(gate.observe(fp), StabilityOutcome::Changed));
        std::thread::sleep(Duration::from_millis(50));
        assert!(matches!(gate.observe(fp), StabilityOutcome::Ready { .. }));
        assert!(matches!(gate.observe(fp), StabilityOutcome::AlreadyEmitted { .. }));
    }

    #[test]
    fn stable_duration_zero_ready_on_first_hit() {
        let mut gate = StabilityGate::with_max_unstable(0, 0);
        let fp = OcrFingerprint::from_text("hello");
        // Duration disabled: min_hits=1 may Ready on the first accept.
        assert!(matches!(gate.observe(fp), StabilityOutcome::Ready { .. }));
        assert!(matches!(gate.observe(fp), StabilityOutcome::AlreadyEmitted { .. }));
    }

    #[test]
    fn cold_start_after_reset_all_waits_duration() {
        // Models post-empty: clear_stale_overlay → reset_all → wait again.
        let mut gate = StabilityGate::with_max_unstable(40, 0);
        let fp = OcrFingerprint::from_text("page");
        assert!(matches!(gate.observe(fp), StabilityOutcome::Changed));
        std::thread::sleep(Duration::from_millis(50));
        assert!(matches!(gate.observe(fp), StabilityOutcome::Ready { .. }));

        gate.reset_all();
        assert!(matches!(gate.observe(fp), StabilityOutcome::Changed));
        std::thread::sleep(Duration::from_millis(50));
        assert!(matches!(gate.observe(fp), StabilityOutcome::Ready { .. }));
    }

    #[test]
    fn duration_alone_can_ready_when_min_hits_high() {
        let mut gate = StabilityGate::with_max_unstable(40, 0);
        gate.min_hits = 100;
        let fp = OcrFingerprint::from_text("hello");
        // First hit not enough for min_hits=100.
        assert!(matches!(gate.observe(fp), StabilityOutcome::Changed));
        std::thread::sleep(Duration::from_millis(50));
        assert!(matches!(gate.observe(fp), StabilityOutcome::Ready { .. }));
    }

    #[test]
    fn single_frame_blip_does_not_reset_active() {
        // stable_duration=0 so priming emit is immediate; test sticky hold.
        let mut gate = StabilityGate::with_max_unstable(0, 0);
        let a = OcrFingerprint::from_text("menu");
        let b = OcrFingerprint::from_text("noise");
        assert!(matches!(gate.observe(a), StabilityOutcome::Ready { .. }));
        // One-off different reading — keep AlreadyEmitted on sticky "menu".
        assert!(matches!(gate.observe(b), StabilityOutcome::AlreadyEmitted { .. }));
        assert!(matches!(gate.observe(a), StabilityOutcome::AlreadyEmitted { .. }));
    }

    #[test]
    fn sustained_change_switches_fingerprint() {
        let mut gate = StabilityGate::with_max_unstable(0, 0);
        let a = OcrFingerprint::from_text("old");
        let b = OcrFingerprint::from_text("new");
        assert!(matches!(gate.observe(a), StabilityOutcome::Ready { .. }));
        // First b is a blip — still old emitted.
        assert!(matches!(gate.observe(b), StabilityOutcome::AlreadyEmitted { .. }));
        // Second b confirms switch → Ready for new page (post-emit min_hits path).
        assert!(matches!(gate.observe(b), StabilityOutcome::Ready { .. }));
    }

    #[test]
    fn thrashing_force_ready_after_max_unstable() {
        // Never settle via min_hits/duration; thrash a↔b until max_unstable fires.
        let mut gate = StabilityGate::with_max_unstable(10_000, 80);
        gate.min_hits = 100;
        gate.switch_hits = 1;
        let a = OcrFingerprint::from_text("alpha");
        let b = OcrFingerprint::from_text("beta");

        assert!(matches!(gate.observe(a), StabilityOutcome::Changed));
        std::thread::sleep(Duration::from_millis(40));
        assert!(matches!(gate.observe(b), StabilityOutcome::Changed | StabilityOutcome::Waiting { .. }));
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
        // After first translate, alternating A/B never hits switch_hits=2 for B
        // alone, but max_unstable must still force re-translate with latest B.
        let mut gate = StabilityGate::with_max_unstable(0, 90);
        let a = OcrFingerprint::from_text("line-a");
        let b = OcrFingerprint::from_text("line-b");

        assert!(matches!(gate.observe(a), StabilityOutcome::Ready { .. }));
        // Alternating blips: each is only one hit of the other fp.
        assert!(matches!(gate.observe(b), StabilityOutcome::AlreadyEmitted { .. }));
        assert!(matches!(gate.observe(a), StabilityOutcome::AlreadyEmitted { .. }));
        std::thread::sleep(Duration::from_millis(50));
        assert!(matches!(gate.observe(b), StabilityOutcome::AlreadyEmitted { .. }));
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
    fn fingerprint_ignores_ellipsis_length() {
        let a = OcrFingerprint::from_blocks(&[block("待って…")]);
        let b = OcrFingerprint::from_blocks(&[block("待って………")]);
        let c = OcrFingerprint::from_blocks(&[block("待って")]);
        let d = OcrFingerprint::from_text("……");
        let e = OcrFingerprint::from_text("………");
        assert_eq!(a, b);
        assert_eq!(a, c);
        assert_eq!(d, e);
    }

    #[test]
    fn fingerprint_ignores_fullwidth_question() {
        let a = OcrFingerprint::from_blocks(&[block("何？")]);
        let b = OcrFingerprint::from_blocks(&[block("何?")]);
        assert_eq!(a, b);
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
        let mut gate = StabilityGate::with_max_unstable(10_000, 0);
        let fp = OcrFingerprint::from_text("now");
        assert!(matches!(gate.force_emit(fp), StabilityOutcome::Ready { elapsed_ms: 0, .. }));
    }
}

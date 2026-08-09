//! Sticky remapping of prior translations onto new OCR geometry.

use translator_core::{OcrBlock, Rect, TranslatedBlock, normalize_ocr_text};

/// Rebuild displayed translations from current OCR.
///
/// Match order:
/// 1. Exact normalized source text
/// 2. Same region (center + IoU) — OCR thrash often flips trailing glyphs so the
///    string no longer equals the last translated source, but the caption should
///    stay put until a real re-translate.
///
/// Sticky geometry: absorb OCR jitter via [`Rect::stabilize_against`], but adopt
/// real layout moves (scroll/reflow). Capture-surface resize always takes the
/// latest OCR box so scale stays correct.
pub fn remap_translations_to_ocr(translated: &[TranslatedBlock], ocr: &[OcrBlock], content_resized: bool) -> Vec<TranslatedBlock> {
    let mut used = vec![false; translated.len()];
    let mut out = Vec::new();

    for (i, ob) in ocr.iter().enumerate() {
        let key = normalize_ocr_text(&ob.text);
        if key.is_empty() {
            continue;
        }

        let text_hit = translated.iter().enumerate().find_map(|(ti, t)| {
            if used[ti] {
                return None;
            }
            if normalize_ocr_text(&t.source) == key { Some(ti) } else { None }
        });

        let spatial_hit = text_hit.or_else(|| best_spatial_translation(translated, &used, ob.bbox));

        let Some(ti) = spatial_hit else {
            continue;
        };
        used[ti] = true;
        let tb = &translated[ti];
        // Jitter-stable sticky: keep previous box under noise; follow real moves.
        let bbox = if content_resized {
            ob.bbox
        } else {
            tb.bbox.stabilize_against(ob.bbox)
        };
        out.push(TranslatedBlock {
            id: i as u32,
            // Keep the last translated source/translation; OCR string may thrash.
            source: tb.source.clone(),
            translation: tb.translation.clone(),
            confidence: ob.confidence,
            bbox,
            source_lines: tb.source_lines.max(1),
        });
    }
    out
}

/// Prefer a prior translation whose box still overlaps this OCR hit.
fn best_spatial_translation(translated: &[TranslatedBlock], used: &[bool], bbox: Rect) -> Option<usize> {
    let (cx, cy) = bbox.center();
    let mut best: Option<(usize, f32)> = None;
    for (ti, t) in translated.iter().enumerate() {
        if used[ti] {
            continue;
        }
        let (tcx, tcy) = t.bbox.center();
        let dx = (cx - tcx).abs();
        let dy = (cy - tcy).abs();
        let max_dx = (bbox.width.max(t.bbox.width) * 0.55).max(20.0);
        let max_dy = (bbox.height.max(t.bbox.height) * 0.75).max(14.0);
        if dx > max_dx || dy > max_dy {
            continue;
        }
        let iou = bbox.iou(t.bbox);
        // Require meaningful overlap so neighboring lines do not steal captions.
        if iou < 0.20 {
            continue;
        }
        let score = iou * 3.0 + (1.0 - (dx / max_dx).clamp(0.0, 1.0)) + (1.0 - (dy / max_dy).clamp(0.0, 1.0));
        if best.map(|(_, s)| score > s).unwrap_or(true) {
            best = Some((ti, score));
        }
    }
    best.map(|(i, _)| i)
}

/// True when the remapped overlay set differs in count, text, or bbox.
///
/// Match by source+translation (not zip order): OCR reorder must not force a
/// repaint that re-runs label layout and looks like the captions moved.
pub fn translated_geometry_changed(previous: &[TranslatedBlock], remapped: &[TranslatedBlock]) -> bool {
    if previous.len() != remapped.len() {
        return true;
    }
    for b in remapped {
        let key = normalize_ocr_text(&b.source);
        let Some(a) = previous
            .iter()
            .find(|p| normalize_ocr_text(&p.source) == key && p.translation == b.translation)
        else {
            return true;
        };
        if a.bbox != b.bbox {
            return true;
        }
    }
    false
}

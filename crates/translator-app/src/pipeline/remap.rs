//! Sticky remapping of prior translations onto current OCR *source text*.

use translator_core::{OcrBlock, TranslatedBlock, normalize_ocr_text};

/// Rebuild displayed translations from current OCR.
///
/// A caption is valid only while its normalized source string is still in OCR.
/// Different text is **not** reused — a new box appears only after
/// `finish_translate`. Same-text remaps keep the previous box (no follow-move)
/// so persist/detector swings cannot walk or re-fit the caption. Capture-surface
/// resize takes the latest OCR box so scale stays correct.
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

        let Some(ti) = text_hit else {
            continue;
        };
        used[ti] = true;
        let tb = &translated[ti];
        let bbox = if content_resized { ob.bbox } else { tb.bbox };
        out.push(TranslatedBlock {
            id: i as u32,
            source: tb.source.clone(),
            translation: tb.translation.clone(),
            confidence: ob.confidence,
            bbox,
            source_lines: tb.source_lines.max(1),
        });
    }
    out
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

#[cfg(test)]
mod tests {
    use translator_core::Rect;

    use super::*;

    fn ocr(text: &str, bbox: Rect) -> OcrBlock {
        OcrBlock {
            id: 0,
            text: text.to_string(),
            confidence: 0.9,
            bbox,
            source_lines: 1,
        }
    }

    fn translated(source: &str, translation: &str, bbox: Rect) -> TranslatedBlock {
        TranslatedBlock {
            id: 0,
            source: source.to_string(),
            translation: translation.to_string(),
            confidence: 0.9,
            bbox,
            source_lines: 1,
        }
    }

    #[test]
    fn remap_keeps_size_when_same_source_grows() {
        let prev_box = Rect::new(100.0, 200.0, 80.0, 24.0);
        let wider = Rect::new(100.0, 200.0, 160.0, 24.0);
        let prior = [translated("セリフ", "line", prev_box)];
        let now = [ocr("セリフ", wider)];
        let out = remap_translations_to_ocr(&prior, &now, false);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].translation, "line");
        assert_eq!(out[0].bbox, prev_box);
    }

    #[test]
    fn remap_freezes_box_until_retranslate() {
        let prev_box = Rect::new(100.0, 200.0, 180.0, 28.0);
        let moved = Rect::new(100.0, 320.0, 180.0, 28.0);
        let prior = [translated("menu", "選單", prev_box)];
        let now = [ocr("menu", moved)];
        let out = remap_translations_to_ocr(&prior, &now, false);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].bbox, prev_box, "same source must not walk the caption");
    }

    #[test]
    fn remap_drops_caption_when_source_differs() {
        let box_a = Rect::new(100.0, 200.0, 80.0, 24.0);
        let box_b = Rect::new(98.0, 198.0, 160.0, 26.0);
        let prior = [translated("セリフ", "old line", box_a)];
        let now = [ocr("次の台詞", box_b)];
        let out = remap_translations_to_ocr(&prior, &now, false);
        assert!(out.is_empty(), "different source must not reuse the caption");
    }

    #[test]
    fn remap_adopts_ocr_box_after_content_resize() {
        let prev_box = Rect::new(50.0, 80.0, 100.0, 20.0);
        let scaled = Rect::new(75.0, 120.0, 150.0, 30.0);
        let prior = [translated("hello", "你好", prev_box)];
        let now = [ocr("hello", scaled)];
        let out = remap_translations_to_ocr(&prior, &now, true);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].bbox, scaled);
    }
}

//! PP-DocLayoutV3 pre/post-processing, after PaddleX's `LayoutAnalysisProcess`.

use translator_core::Rect;

/// The model's fixed square input side (aspect ratio is not kept).
pub(crate) const SIDE: usize = 800;

// Class ids the post-processing treats specially, out of the model's 25.
const CHART: usize = 3;
const DISPLAY_FORMULA: usize = 5;
const DOC_TITLE: usize = 6;
const FOOTER_IMAGE: usize = 9;
const HEADER_IMAGE: usize = 13;
const IMAGE: usize = 14;
const INLINE_FORMULA: usize = 15;
const PARAGRAPH_TITLE: usize = 17;
const REFERENCE: usize = 18;
const SEAL: usize = 20;
const TABLE: usize = 21;
const VERTICAL_TEXT: usize = 23;
const CLASSES: usize = 25;

/// Classes that swallow any box they contain (PaddleOCR's `layout_merge_bboxes_mode: large`).
const KEEP_LARGE: [usize; 5] = [CHART, DISPLAY_FORMULA, DOC_TITLE, INLINE_FORMULA, PARAGRAPH_TITLE];
/// Pictures and tables, not prose: their lines stay single.
const NON_TEXT: [usize; 6] = [IMAGE, CHART, TABLE, SEAL, HEADER_IMAGE, FOOTER_IMAGE];

/// Minimum score of a layout box.
const THRESHOLD: f32 = 0.3;

#[derive(Debug, Clone, Copy)]
struct LayoutBox {
    class: usize,
    score: f32,
    /// `[x0, y0, x1, y1]`
    at: [f32; 4],
    /// Reading-order rank.
    order: f32,
}

impl LayoutBox {
    fn area(&self) -> f32 {
        (self.at[2] - self.at[0]) * (self.at[3] - self.at[1])
    }

    fn overlap(&self, other: &Self) -> f32 {
        let width = (self.at[2].min(other.at[2]) - self.at[0].max(other.at[0])).max(0.0);
        let height = (self.at[3].min(other.at[3]) - self.at[1].max(other.at[1])).max(0.0);
        width * height
    }
}

/// Text-bearing layout boxes of one `width × height` image, in reading order, each flagged
/// `true` for vertical text. `rows` is the model's box output, `columns` values per box:
/// class, score, x0, y0, x1, y1 and, with 7 columns, the reading-order rank.
pub(crate) fn text_boxes(rows: &[f32], columns: usize, width: f32, height: f32) -> Vec<(Rect, bool)> {
    if columns < 6 {
        return Vec::new();
    }
    let mut boxes: Vec<LayoutBox> = rows
        .chunks_exact(columns)
        .filter(|row| row[1] > THRESHOLD && row[0] >= 0.0 && (row[0] as usize) < CLASSES)
        .map(|row| LayoutBox {
            class: row[0] as usize,
            score: row[1],
            at: [row[2], row[3], row[4], row[5]].map(f32::round_ties_even),
            order: row.get(6).copied().unwrap_or(0.0),
        })
        .collect();

    // NMS, best score first: a box goes when it overlaps a kept one by IoU 0.6 (same class)
    // or 0.98 (any class). PaddleX counts box sides inclusively here.
    boxes.sort_by(|a, b| b.score.total_cmp(&a.score));
    let mut kept: Vec<LayoutBox> = Vec::with_capacity(boxes.len());
    for candidate in boxes {
        let suppressed = kept.iter().any(|keep| {
            let side = |b: &LayoutBox, axis: usize| b.at[axis + 2] - b.at[axis] + 1.0;
            let inter =
                |axis: usize| (candidate.at[axis + 2].min(keep.at[axis + 2]) - candidate.at[axis].max(keep.at[axis]) + 1.0).max(0.0);
            let inter = inter(0) * inter(1);
            let union = side(&candidate, 0) * side(&candidate, 1) + side(keep, 0) * side(keep, 1) - inter;
            inter / union >= if candidate.class == keep.class { 0.6 } else { 0.98 }
        });
        if !suppressed {
            kept.push(candidate);
        }
    }

    // PaddleX also drops page-sized `image` boxes at this point. That only ever removes
    // `image` boxes, which are not text and never displace another class below.
    let contained = |inner: &LayoutBox, outer: &LayoutBox| inner.area() > 0.0 && inner.overlap(outer) / inner.area() >= 0.9;
    let mut boxes: Vec<LayoutBox> = kept
        .iter()
        .enumerate()
        .filter(|&(i, inner)| {
            !kept
                .iter()
                .enumerate()
                .any(|(j, outer)| i != j && KEEP_LARGE.contains(&outer.class) && contained(inner, outer))
        })
        .map(|(_, &b)| b)
        .collect();
    boxes.sort_by(|a, b| a.order.total_cmp(&b.order));

    // Clip to the image, then `filter_boxes`: no reference blocks, nothing under 6 px, and of
    // two boxes that mostly overlap only the larger stays.
    for b in &mut boxes {
        b.at = [b.at[0].max(0.0), b.at[1].max(0.0), b.at[2].min(width), b.at[3].min(height)];
    }
    boxes.retain(|b| b.at[2] > b.at[0] && b.at[3] > b.at[1] && b.class != REFERENCE);
    let mut dropped = vec![false; boxes.len()];
    for i in 0..boxes.len() {
        let a = boxes[i];
        if a.at[2] - a.at[0] < 6.0 || a.at[3] - a.at[1] < 6.0 {
            dropped[i] = true;
        }
        for j in i + 1..boxes.len() {
            if dropped[i] || dropped[j] {
                continue;
            }
            let b = boxes[j];
            let ratio = a.overlap(&b) / a.area().min(b.area());
            if (a.class == INLINE_FORMULA || b.class == INLINE_FORMULA) && ratio > 0.5 {
                dropped[i] |= a.class == INLINE_FORMULA;
                dropped[j] |= b.class == INLINE_FORMULA;
                continue;
            }
            if ratio <= 0.7 {
                continue;
            }
            // A picture, seal or chart may overlap anything; a table gives way only to prose.
            let figure = |class: usize| [IMAGE, TABLE, SEAL, CHART].contains(&class);
            let no_table = a.class != TABLE && b.class != TABLE;
            if a.class != b.class && (figure(a.class) || figure(b.class)) && (no_table || (figure(a.class) && figure(b.class))) {
                continue;
            }
            if a.area() >= b.area() {
                dropped[j] = true;
            } else {
                dropped[i] = true;
            }
        }
    }

    boxes
        .iter()
        .zip(dropped)
        .filter(|&(b, dropped)| !dropped && !NON_TEXT.contains(&b.class))
        .map(|(b, _)| (Rect::new(b.at[0], b.at[1], b.at[2] - b.at[0], b.at[3] - b.at[1]), b.class == VERTICAL_TEXT))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEXT: usize = 22;

    fn run(boxes: &[(usize, f32, [f32; 4])]) -> Vec<(Rect, bool)> {
        let rows: Vec<f32> = boxes
            .iter()
            .enumerate()
            .flat_map(|(i, &(class, score, at))| [class as f32, score, at[0], at[1], at[2], at[3], i as f32])
            .collect();
        text_boxes(&rows, 7, 1000.0, 1000.0)
    }

    fn rects(boxes: &[(usize, f32, [f32; 4])]) -> Vec<[f32; 4]> {
        run(boxes).iter().map(|(r, _)| [r.x, r.y, r.x + r.width, r.y + r.height]).collect()
    }

    #[test]
    fn keeps_text_in_reading_order_and_flags_vertical() {
        let boxes = [
            (TEXT, 0.5, [100.0, 400.0, 500.0, 500.0]),
            (VERTICAL_TEXT, 0.9, [800.0, 100.0, 850.0, 600.0]),
            (IMAGE, 0.9, [100.0, 600.0, 500.0, 900.0]),
            (TEXT, 0.2, [100.0, 100.0, 500.0, 200.0]),
        ];
        let out = run(&boxes);
        assert_eq!(out.len(), 2, "the picture and the low score are gone: {out:?}");
        assert_eq!((out[0].0, out[0].1), (Rect::new(100.0, 400.0, 400.0, 100.0), false));
        assert_eq!((out[1].0, out[1].1), (Rect::new(800.0, 100.0, 50.0, 500.0), true));
    }

    #[test]
    fn six_column_output_is_accepted() {
        let rows = [TEXT as f32, 0.9, -5.0, 10.4, 2000.0, 50.6];
        assert_eq!(text_boxes(&rows, 6, 1000.0, 1000.0), [(Rect::new(0.0, 10.0, 1000.0, 41.0), false)], "rounded and clipped");
        assert!(text_boxes(&rows[..5], 5, 1000.0, 1000.0).is_empty());
    }

    #[test]
    fn nms_drops_the_weaker_duplicate() {
        let same = [(TEXT, 0.6, [100.0, 100.0, 500.0, 200.0]), (TEXT, 0.9, [105.0, 100.0, 500.0, 205.0])];
        assert_eq!(rects(&same), [[105.0, 100.0, 500.0, 205.0]]);
        // Under another class the same overlap survives.
        let other = [
            (TEXT, 0.6, [100.0, 100.0, 500.0, 200.0]),
            (IMAGE, 0.9, [105.0, 100.0, 500.0, 205.0]),
        ];
        assert_eq!(rects(&other), [[100.0, 100.0, 500.0, 200.0]]);
    }

    #[test]
    fn title_swallows_the_text_inside_it() {
        let boxes = [
            (PARAGRAPH_TITLE, 0.5, [100.0, 100.0, 600.0, 200.0]),
            (TEXT, 0.9, [120.0, 110.0, 300.0, 190.0]),
        ];
        assert_eq!(rects(&boxes), [[100.0, 100.0, 600.0, 200.0]]);
    }

    #[test]
    fn overlapping_boxes_keep_the_larger_one() {
        let boxes = [
            (TEXT, 0.9, [100.0, 100.0, 300.0, 200.0]),
            (VERTICAL_TEXT, 0.8, [250.0, 100.0, 450.0, 200.0]),
        ];
        assert_eq!(rects(&boxes).len(), 2, "25% overlap stays");
        // 75% of the small box lies inside the big one.
        let boxes = [
            (TEXT, 0.9, [100.0, 100.0, 500.0, 300.0]),
            (VERTICAL_TEXT, 0.8, [200.0, 150.0, 400.0, 350.0]),
        ];
        assert_eq!(rects(&boxes), [[100.0, 100.0, 500.0, 300.0]]);
    }

    #[test]
    fn picture_overlap_spares_text_but_a_table_does_not() {
        let caption = (TEXT, 0.9, [120.0, 120.0, 300.0, 180.0]);
        assert_eq!(rects(&[(IMAGE, 0.9, [100.0, 100.0, 600.0, 600.0]), caption]), [[120.0, 120.0, 300.0, 180.0]]);
        assert!(rects(&[(TABLE, 0.9, [100.0, 100.0, 600.0, 600.0]), caption]).is_empty(), "text inside a table is the table's");
    }

    #[test]
    fn slivers_and_reference_blocks_are_dropped() {
        let boxes = [
            (TEXT, 0.9, [100.0, 100.0, 500.0, 104.0]),
            (REFERENCE, 0.9, [100.0, 300.0, 500.0, 400.0]),
        ];
        assert!(rects(&boxes).is_empty());
    }
}

//! Merge OCR line boxes into paragraph blocks.
//!
//! Strategy (tunable via [`LineMergeConfig`]):
//! 1. **Frame-relative `|gap|`** — one threshold vs capture height, not a min/max range.
//! 2. **Paragraph** — nearest-below link when height, overlap, and gap match.
//! 3. **Whole region** — join every line in the crop (caller sets `merge_all`).
//! 4. **Reading order** — row-major or column-major join / emit order.

use translator_core::{LineMergeConfig, LineMergeOrder, OcrBlock, Rect};

/// Default test / simple-caller frame (1080p). Production always passes the capture size.
const DEFAULT_FRAME_W: u32 = 1920;
const DEFAULT_FRAME_H: u32 = 1080;

/// Scale-free slack for ratio-space compares (`px / frame` or `|d| / larger`).
const RATIO_EPS: f32 = 1e-5;

/// Merge with default config and a 1080p frame (tests / simple callers).
pub fn merge_line_blocks(blocks: Vec<OcrBlock>) -> Vec<OcrBlock> {
    merge_line_blocks_with(blocks, &LineMergeConfig::default(), DEFAULT_FRAME_W, DEFAULT_FRAME_H, false)
}

/// Merge consecutive line-level OCR boxes that belong to the same paragraph.
///
/// `frame_w` / `frame_h` are the **full capture** size even when `blocks` came
/// from a crop. `merge_all` joins every remaining line (whole selected region).
pub fn merge_line_blocks_with(blocks: Vec<OcrBlock>, cfg: &LineMergeConfig, frame_w: u32, frame_h: u32, merge_all: bool) -> Vec<OcrBlock> {
    if !cfg.enabled || blocks.len() <= 1 {
        return reindex(blocks);
    }

    let frame_w = frame_w.max(1);
    let frame_h = frame_h.max(1);

    if merge_all {
        return merge_whole_region(blocks, cfg, frame_w, frame_h);
    }

    merge_paragraph(blocks, cfg, frame_w, frame_h)
}

fn merge_whole_region(blocks: Vec<OcrBlock>, cfg: &LineMergeConfig, frame_w: u32, frame_h: u32) -> Vec<OcrBlock> {
    let members: Vec<usize> = (0..blocks.len()).collect();
    reindex(vec![assemble_group(&blocks, members, cfg, frame_w, frame_h)])
}

fn merge_paragraph(blocks: Vec<OcrBlock>, cfg: &LineMergeConfig, frame_w: u32, frame_h: u32) -> Vec<OcrBlock> {
    let n = blocks.len();
    let frame_h_f = frame_h as f32;

    let mut parent: Vec<usize> = (0..n).collect();
    let mut rank = vec![0u8; n];

    for i in 0..n {
        let mut best: Option<(usize, f32)> = None;
        for j in 0..n {
            if i == j {
                continue;
            }
            let Some(gap) = vertical_gap_if_below(&blocks[i], &blocks[j], cfg.below_mid_ratio, frame_h_f) else {
                continue;
            };
            if !approx_le(gap.abs() / frame_h_f, cfg.gap_ratio) {
                continue;
            }
            if !can_link_lines(&blocks[i], &blocks[j], frame_w, cfg) {
                continue;
            }
            if best.map(|(_, g)| approx_lt(gap.abs(), g.abs())).unwrap_or(true) {
                best = Some((j, gap));
            }
        }
        if let Some((j, _)) = best
            && !has_intervening_line(&blocks, i, j, frame_w, frame_h, cfg)
        {
            union(&mut parent, &mut rank, i, j);
        }
    }

    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        groups[find(&mut parent, i)].push(i);
    }

    let mut merged: Vec<OcrBlock> = Vec::with_capacity(n);
    for members in groups {
        if members.is_empty() {
            continue;
        }
        merged.push(assemble_group(&blocks, members, cfg, frame_w, frame_h));
    }
    sort_blocks(&mut merged, cfg, frame_w, frame_h);
    reindex(merged)
}

fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

fn union(parent: &mut [usize], rank: &mut [u8], a: usize, b: usize) {
    let mut ra = find(parent, a);
    let mut rb = find(parent, b);
    if ra == rb {
        return;
    }
    if rank[ra] < rank[rb] {
        std::mem::swap(&mut ra, &mut rb);
    }
    parent[rb] = ra;
    if rank[ra] == rank[rb] {
        rank[ra] = rank[ra].saturating_add(1);
    }
}

fn assemble_group(blocks: &[OcrBlock], mut members: Vec<usize>, cfg: &LineMergeConfig, frame_w: u32, frame_h: u32) -> OcrBlock {
    sort_indices(blocks, &mut members, cfg, frame_w, frame_h);

    let mut text = String::new();
    let mut conf = 0.0f32;
    let mut x0 = f32::MAX;
    let mut y0 = f32::MAX;
    let mut x1 = f32::MIN;
    let mut heights: Vec<f32> = Vec::with_capacity(members.len());

    for (k, &idx) in members.iter().enumerate() {
        let b = &blocks[idx];
        text = if k == 0 {
            b.text.clone()
        } else {
            join_text(&text, &b.text, cfg.join_with_space)
        };
        conf += b.confidence;
        x0 = x0.min(b.bbox.x);
        y0 = y0.min(b.bbox.y);
        x1 = x1.max(b.bbox.x + b.bbox.width);
        heights.push(b.bbox.height.max(1.0));
    }
    conf /= members.len() as f32;

    // One-line-tall anchor: full horizontal span, median line height.
    // Overlay expands height from measured translation text, not OCR union.
    let line_h = median_f32(heights);
    let bbox = Rect::new(x0, y0, (x1 - x0).max(1.0), line_h);
    let source_lines = members.iter().map(|&idx| blocks[idx].source_lines.max(1)).sum::<u32>().max(1);

    OcrBlock {
        id: 0,
        text,
        confidence: conf,
        bbox,
        source_lines,
    }
}

fn sort_blocks(blocks: &mut Vec<OcrBlock>, cfg: &LineMergeConfig, frame_w: u32, frame_h: u32) {
    if blocks.len() <= 1 {
        return;
    }
    let mut members: Vec<usize> = (0..blocks.len()).collect();
    sort_indices(blocks, &mut members, cfg, frame_w, frame_h);
    let mut slots: Vec<Option<OcrBlock>> = std::mem::take(blocks).into_iter().map(Some).collect();
    *blocks = members
        .into_iter()
        .map(|i| slots[i].take().expect("sort permutation is unique"))
        .collect();
}

fn sort_indices(blocks: &[OcrBlock], members: &mut [usize], cfg: &LineMergeConfig, frame_w: u32, frame_h: u32) {
    match cfg.order {
        LineMergeOrder::TopToBottomLeftToRight => {
            let rows = assign_bands(blocks, members, Axis::Y, cfg.order_band_ratio, frame_h as f32);
            members.sort_by(|&a, &b| {
                rows[a]
                    .cmp(&rows[b])
                    .then_with(|| (blocks[a].bbox.x as i32).cmp(&(blocks[b].bbox.x as i32)))
                    .then_with(|| (blocks[a].bbox.y as i32).cmp(&(blocks[b].bbox.y as i32)))
            });
        }
        LineMergeOrder::LeftToRightTopToBottom => {
            let cols = assign_bands(blocks, members, Axis::X, cfg.order_band_ratio, frame_w as f32);
            members.sort_by(|&a, &b| {
                cols[a]
                    .cmp(&cols[b])
                    .then_with(|| (blocks[a].bbox.y as i32).cmp(&(blocks[b].bbox.y as i32)))
                    .then_with(|| (blocks[a].bbox.x as i32).cmp(&(blocks[b].bbox.x as i32)))
            });
        }
    }
}

#[derive(Clone, Copy)]
enum Axis {
    X,
    Y,
}

/// Greedy 1-D clusters along `axis` (sorted). `band_ratio` × `axis_span` is the band width.
fn assign_bands(blocks: &[OcrBlock], members: &[usize], axis: Axis, band_ratio: f32, axis_span: f32) -> Vec<u32> {
    let mut order: Vec<usize> = members.to_vec();
    order.sort_by(|&a, &b| {
        let ca = axis_coord(&blocks[a], axis);
        let cb = axis_coord(&blocks[b], axis);
        ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut ids = vec![0u32; blocks.len()];
    let mut current = 0u32;
    let mut last: Option<f32> = None;
    let span = axis_span.max(1.0);
    for i in order {
        let c = axis_coord(&blocks[i], axis);
        if let Some(prev) = last
            && !approx_le((c - prev) / span, band_ratio)
        {
            current = current.saturating_add(1);
        }
        ids[i] = current;
        last = Some(c);
    }
    ids
}

fn axis_coord(b: &OcrBlock, axis: Axis) -> f32 {
    match axis {
        Axis::X => b.bbox.x,
        Axis::Y => b.bbox.y + b.bbox.height * 0.5,
    }
}

fn vertical_gap_if_below(upper: &OcrBlock, lower: &OcrBlock, below_mid_ratio: f32, frame_h: f32) -> Option<f32> {
    let upper_bottom = upper.bbox.y + upper.bbox.height;
    let lower_top = lower.bbox.y;
    let upper_mid = upper.bbox.y + upper.bbox.height * 0.5;
    let slack = lower.bbox.height * below_mid_ratio;
    // lower_top + slack >= upper_mid  (ratio-space ε)
    if !approx_ge((lower_top + slack - upper_mid) / frame_h.max(1.0), 0.0) {
        return None;
    }
    Some(lower_top - upper_bottom)
}

fn can_link_lines(upper: &OcrBlock, lower: &OcrBlock, frame_w: u32, cfg: &LineMergeConfig) -> bool {
    if !height_compatible(upper, lower, cfg) {
        return false;
    }
    if !horiz_compatible(upper, lower, frame_w, cfg) {
        return false;
    }
    if cfg.reject_short_long && short_into_long(upper, lower, cfg) {
        return false;
    }
    true
}

fn short_into_long(upper: &OcrBlock, lower: &OcrBlock, cfg: &LineMergeConfig) -> bool {
    let uw = upper.bbox.width.max(1.0);
    let lw = lower.bbox.width.max(1.0);
    if !approx_lt(uw, lw) {
        return false;
    }
    !approx_le((lw - uw) / lw, cfg.width_delta_ratio)
}

fn height_compatible(a: &OcrBlock, b: &OcrBlock, cfg: &LineMergeConfig) -> bool {
    let ah = a.bbox.height.max(1.0);
    let bh = b.bbox.height.max(1.0);
    let larger = ah.max(bh);
    approx_le((ah - bh).abs() / larger, cfg.height_delta_ratio)
}

fn horiz_compatible(a: &OcrBlock, b: &OcrBlock, frame_w: u32, cfg: &LineMergeConfig) -> bool {
    let a_left = a.bbox.x;
    let a_right = a.bbox.x + a.bbox.width;
    let b_left = b.bbox.x;
    let b_right = b.bbox.x + b.bbox.width;

    let overlap = (a_right.min(b_right) - a_left.max(b_left)).max(0.0);
    let shorter = a.bbox.width.min(b.bbox.width).max(1.0);
    let overlap_frac = overlap / shorter;

    if approx_ge(overlap_frac, cfg.overlap_ratio) {
        return true;
    }

    let span = (frame_w as f32).max(1.0);
    let left_frac = (a_left - b_left).abs() / span;
    let a_cx = a_left + a.bbox.width * 0.5;
    let b_cx = b_left + b.bbox.width * 0.5;
    let center_frac = (a_cx - b_cx).abs() / span;
    (approx_le(left_frac, cfg.align_ratio) || approx_le(center_frac, cfg.align_ratio)) && approx_ge(overlap_frac, cfg.align_overlap_ratio)
}

fn has_intervening_line(blocks: &[OcrBlock], upper: usize, lower: usize, frame_w: u32, frame_h: u32, cfg: &LineMergeConfig) -> bool {
    let u = &blocks[upper];
    let l = &blocks[lower];
    let y0 = u.bbox.y + u.bbox.height;
    let y1 = l.bbox.y;
    let span = (frame_h as f32).max(1.0);
    if approx_le((y1 - y0) / span, 0.0) {
        return false;
    }

    for (k, b) in blocks.iter().enumerate() {
        if k == upper || k == lower {
            continue;
        }
        let cy = b.bbox.y + b.bbox.height * 0.5;
        if approx_le((cy - y0) / span, 0.0) || approx_ge((cy - y1) / span, 0.0) {
            continue;
        }
        if horiz_compatible(u, b, frame_w, cfg) && horiz_compatible(b, l, frame_w, cfg) {
            return true;
        }
    }
    false
}

fn approx_le(a: f32, b: f32) -> bool {
    a <= b + RATIO_EPS
}

fn approx_ge(a: f32, b: f32) -> bool {
    a + RATIO_EPS >= b
}

fn approx_lt(a: f32, b: f32) -> bool {
    a + RATIO_EPS < b
}

fn median_f32(mut vals: Vec<f32>) -> f32 {
    if vals.is_empty() {
        return 16.0;
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    vals[vals.len() / 2]
}

fn join_text(a: &str, b: &str, with_space: bool) -> String {
    let a = a.trim_end();
    let b = b.trim_start();
    if a.is_empty() {
        return b.to_string();
    }
    if b.is_empty() {
        return a.to_string();
    }
    if with_space { format!("{a} {b}") } else { format!("{a}{b}") }
}

fn reindex(mut blocks: Vec<OcrBlock>) -> Vec<OcrBlock> {
    for (i, b) in blocks.iter_mut().enumerate() {
        b.id = i as u32;
    }
    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(id: u32, text: &str, x: f32, y: f32, w: f32, h: f32) -> OcrBlock {
        OcrBlock {
            id,
            text: text.to_string(),
            confidence: 0.9,
            bbox: Rect::new(x, y, w, h),
            source_lines: 1,
        }
    }

    fn merge_with(blocks: Vec<OcrBlock>, cfg: LineMergeConfig, merge_all: bool) -> Vec<OcrBlock> {
        merge_line_blocks_with(blocks, &cfg, DEFAULT_FRAME_W, DEFAULT_FRAME_H, merge_all)
    }

    #[test]
    fn merges_stacked_english_lines() {
        let blocks = vec![line(0, "Hello", 10.0, 10.0, 100.0, 18.0), line(1, "world", 12.0, 32.0, 90.0, 18.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "Hello world");
        assert!(merged[0].bbox.height <= 22.0, "h={}", merged[0].bbox.height);
        assert_eq!(merged[0].source_lines, 2, "merged block tracks line count");
    }

    #[test]
    fn merge_disabled_passthrough() {
        let cfg = LineMergeConfig {
            enabled: false,
            ..Default::default()
        };
        let blocks = vec![line(0, "Hello", 10.0, 10.0, 100.0, 18.0), line(1, "world", 12.0, 32.0, 90.0, 18.0)];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn disabled_ignores_merge_all() {
        let cfg = LineMergeConfig {
            enabled: false,
            merge_whole_region: true,
            ..Default::default()
        };
        let blocks = vec![line(0, "Hello", 10.0, 10.0, 100.0, 18.0), line(1, "world", 12.0, 32.0, 90.0, 18.0)];
        let merged = merge_with(blocks, cfg, true);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn join_without_space_concatenates() {
        let cfg = LineMergeConfig {
            join_with_space: false,
            ..Default::default()
        };
        let blocks = vec![line(0, "甲乙", 10.0, 10.0, 100.0, 20.0), line(1, "丙丁", 10.0, 34.0, 80.0, 20.0)];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "甲乙丙丁");
    }

    #[test]
    fn equal_width_nearby_lines_merge() {
        let blocks = vec![line(0, "甲乙", 10.0, 10.0, 80.0, 20.0), line(1, "丙丁", 10.0, 34.0, 80.0, 20.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "甲乙 丙丁");
    }

    #[test]
    fn short_upper_stays_separate_by_default() {
        let blocks = vec![
            line(0, "short", 10.0, 10.0, 60.0, 18.0),
            line(1, "much longer body", 10.0, 32.0, 160.0, 18.0),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn short_upper_merges_when_guard_off() {
        let cfg = LineMergeConfig {
            reject_short_long: false,
            ..Default::default()
        };
        let blocks = vec![
            line(0, "short", 10.0, 10.0, 60.0, 18.0),
            line(1, "much longer body", 10.0, 32.0, 160.0, 18.0),
        ];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "short much longer body");
    }

    #[test]
    fn merges_three_line_paragraph() {
        let h = 20.0;
        let blocks = vec![
            line(0, "This is a long", 10.0, 10.0, 220.0, h),
            line(1, "paragraph that wraps", 10.0, 10.0 + h + 5.0, 200.0, h),
            line(2, "across three lines.", 10.0, 10.0 + 2.0 * (h + 5.0), 160.0, h),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "This is a long paragraph that wraps across three lines.");
        assert!((merged[0].bbox.height - h).abs() < 1.0);
    }

    #[test]
    fn does_not_merge_side_by_side() {
        let blocks = vec![line(0, "Left", 10.0, 10.0, 40.0, 18.0), line(1, "Right", 200.0, 12.0, 40.0, 18.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn does_not_merge_far_vertical_gap() {
        let blocks = vec![line(0, "A", 10.0, 10.0, 80.0, 18.0), line(1, "B", 10.0, 120.0, 40.0, 18.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn gap_at_threshold_still_merges() {
        let cfg = LineMergeConfig::default();
        let h = 18.0;
        let gap = cfg.gap_ratio * DEFAULT_FRAME_H as f32;
        let blocks = vec![
            line(0, "Hello", 10.0, 10.0, 120.0, h),
            line(1, "world", 12.0, 10.0 + h + gap, 90.0, h),
        ];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn overlapping_gap_within_abs_threshold_merges() {
        let h = 18.0;
        // Negative gap (2px overlap) is the same |gap| as a 2px space.
        let blocks = vec![
            line(0, "Hello", 10.0, 10.0, 120.0, h),
            line(1, "world", 12.0, 10.0 + h - 2.0, 90.0, h),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
    }

    #[test]
    fn gap_just_beyond_threshold_stays_separate() {
        let cfg = LineMergeConfig::default();
        let h = 18.0;
        let gap = cfg.gap_ratio * DEFAULT_FRAME_H as f32 + 2.0;
        let blocks = vec![
            line(0, "Hello", 10.0, 10.0, 120.0, h),
            line(1, "world", 12.0, 10.0 + h + gap, 90.0, h),
        ];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn does_not_merge_paragraph_spacing() {
        let blocks = vec![
            line(0, "Line one", 10.0, 10.0, 140.0, 18.0),
            line(1, "Line two", 10.0, 30.0, 120.0, 18.0),
            line(2, "Next block", 10.0, 80.0, 120.0, 18.0),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].text, "Line one Line two");
        assert_eq!(merged[1].text, "Next block");
    }

    #[test]
    fn keeps_two_columns_separate() {
        let blocks = vec![
            line(0, "L1", 10.0, 10.0, 80.0, 16.0),
            line(1, "L2", 10.0, 30.0, 60.0, 16.0),
            line(2, "R1", 200.0, 10.0, 80.0, 16.0),
            line(3, "R2", 200.0, 30.0, 60.0, 16.0),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 2);
        let texts: Vec<_> = merged.iter().map(|b| b.text.as_str()).collect();
        assert!(texts.contains(&"L1 L2"));
        assert!(texts.contains(&"R1 R2"));
    }

    #[test]
    fn does_not_chain_merge_via_growing_bbox() {
        let blocks = vec![
            line(0, "L1", 10.0, 10.0, 100.0, 16.0),
            line(1, "L2", 10.0, 28.0, 80.0, 16.0),
            line(2, "P2", 10.0, 70.0, 80.0, 16.0),
            line(3, "P3", 10.0, 112.0, 80.0, 16.0),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].text, "L1 L2");
    }

    #[test]
    fn height_mismatch_does_not_merge() {
        let blocks = vec![line(0, "A", 10.0, 10.0, 80.0, 18.0), line(1, "B", 10.0, 32.0, 80.0, 40.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn low_overlap_unaligned_does_not_merge() {
        let blocks = vec![line(0, "A", 10.0, 10.0, 80.0, 18.0), line(1, "B", 120.0, 32.0, 80.0, 18.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn align_path_merges_weak_overlap() {
        // overlap/shorter ≈ 0.33 < 0.35, left edges within align_ratio.
        // Guard off: this pair is short-over-long, which the align path must still see.
        let cfg = LineMergeConfig {
            reject_short_long: false,
            ..Default::default()
        };
        let blocks = vec![line(0, "A", 100.0, 10.0, 15.0, 18.0), line(1, "B", 110.0, 32.0, 200.0, 18.0)];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "A B");
    }

    #[test]
    fn large_list_pitch_stays_separate() {
        let h = 42.0;
        let x = 1334.0;
        let pitch = h + 19.0;
        let blocks = vec![
            line(0, "あ", x, 224.0, 86.0, h),
            line(1, "い", x, 224.0 + pitch, 78.0, h),
            line(2, "ううう", x, 224.0 + 2.0 * pitch, 164.0, h),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 3, "{:?}", merged.iter().map(|b| &b.text).collect::<Vec<_>>());
    }

    #[test]
    fn single_line_only_passthrough() {
        let blocks = vec![line(0, "only", 322.0, 876.0, 434.0, 60.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "only");
    }

    #[test]
    fn wrap_with_positive_gap_still_merges_on_dense_page() {
        let h = 40.0;
        let mut blocks = Vec::new();
        for i in 0..5 {
            blocks.push(line(i, &format!("row{i}"), 600.0, 20.0 + i as f32 * 80.0, 400.0, h));
        }
        blocks.push(line(5, "W1A", 600.0, 500.0, 850.0, h));
        blocks.push(line(6, "W1B", 600.0, 500.0 + h + 14.0, 220.0, h));

        let merged = merge_line_blocks(blocks);
        assert!(
            merged.iter().any(|b| b.text.contains("W1A") && b.text.contains("W1B")),
            "{:?}",
            merged.iter().map(|b| &b.text).collect::<Vec<_>>()
        );
    }

    #[test]
    fn same_pixel_gap_depends_on_frame_height() {
        let blocks = vec![
            line(0, "Hello", 10.0, 10.0, 120.0, 18.0),
            line(1, "world", 12.0, 10.0 + 18.0 + 20.0, 90.0, 18.0),
        ];
        let cfg = LineMergeConfig::default();
        // 20px / 1080 ≈ 0.0185 > 0.015 → no merge; 20px / 2000 = 0.01 < 0.015 → merge.
        let tall = merge_line_blocks_with(blocks.clone(), &cfg, 1920, 2000, false);
        let short = merge_line_blocks_with(blocks, &cfg, 1920, 1080, false);
        assert_eq!(tall.len(), 1, "larger frame makes 20px a small relative gap");
        assert_eq!(short.len(), 2, "1080p treats 20px as too far");
    }

    #[test]
    fn whole_region_merges_side_by_side() {
        let cfg = LineMergeConfig::default();
        let blocks = vec![line(0, "Left", 10.0, 10.0, 40.0, 18.0), line(1, "Right", 200.0, 12.0, 40.0, 18.0)];
        let merged = merge_with(blocks, cfg, true);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "Left Right");
        assert_eq!(merged[0].source_lines, 2);
    }

    #[test]
    fn whole_region_without_merge_all_stays_paragraph() {
        let cfg = LineMergeConfig {
            merge_whole_region: true,
            ..Default::default()
        };
        let blocks = vec![line(0, "Left", 10.0, 10.0, 40.0, 18.0), line(1, "Right", 200.0, 12.0, 40.0, 18.0)];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn whole_region_order_top_to_bottom_left_to_right() {
        let cfg = LineMergeConfig {
            order: LineMergeOrder::TopToBottomLeftToRight,
            ..Default::default()
        };
        let blocks = vec![
            line(0, "A", 10.0, 10.0, 20.0, 16.0),
            line(1, "B", 80.0, 10.0, 20.0, 16.0),
            line(2, "C", 10.0, 40.0, 20.0, 16.0),
            line(3, "D", 80.0, 40.0, 20.0, 16.0),
        ];
        let merged = merge_with(blocks, cfg, true);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "A B C D");
    }

    #[test]
    fn whole_region_order_left_to_right_top_to_bottom() {
        let cfg = LineMergeConfig {
            order: LineMergeOrder::LeftToRightTopToBottom,
            ..Default::default()
        };
        let blocks = vec![
            line(0, "A", 10.0, 10.0, 20.0, 16.0),
            line(1, "B", 80.0, 10.0, 20.0, 16.0),
            line(2, "C", 10.0, 40.0, 20.0, 16.0),
            line(3, "D", 80.0, 40.0, 20.0, 16.0),
        ];
        let merged = merge_with(blocks, cfg, true);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "A C B D");
    }

    #[test]
    fn default_whole_region_order_is_left_to_right() {
        let cfg = LineMergeConfig::default();
        let blocks = vec![
            line(0, "A", 10.0, 10.0, 20.0, 16.0),
            line(1, "B", 80.0, 10.0, 20.0, 16.0),
            line(2, "C", 10.0, 40.0, 20.0, 16.0),
            line(3, "D", 80.0, 40.0, 20.0, 16.0),
        ];
        let merged = merge_with(blocks, cfg, true);
        assert_eq!(merged[0].text, "A C B D");
    }
}

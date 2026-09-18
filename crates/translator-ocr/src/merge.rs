//! Merge OCR line boxes into paragraph blocks.
//!
//! Strategy (tunable via [`LineMergeConfig`]):
//! 1. **Frame-relative `|gap|`** — vertical vs height, horizontal vs width.
//! 2. **Same-row first** — nearest-right fragments, then nearest-below (a split
//!    long line becomes one box before stacking).
//! 3. **Whole region** — join every line in the crop (caller sets `merge_all`).
//! 4. **Reading order** — row-major or column-major join / emit order.

use translator_core::{LineMergeConfig, LineMergeOrder, OcrBlock, Rect};

use crate::reindex;

/// Scale-free slack for ratio-space compares (`px / frame` or `|d| / larger`).
const RATIO_EPS: f32 = 1e-5;

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
        let members: Vec<usize> = (0..blocks.len()).collect();
        return reindex(vec![assemble_group(&blocks, members, cfg, frame_w, frame_h)]);
    }

    let rows = join_nearest(blocks, cfg, frame_w, frame_h, Neighbor::Right);
    let mut merged = join_nearest(rows, cfg, frame_w, frame_h, Neighbor::Below);
    sort_blocks(&mut merged, cfg, frame_w, frame_h);
    reindex(merged)
}

#[derive(Clone, Copy)]
enum Neighbor {
    Below,
    Right,
}

fn join_nearest(blocks: Vec<OcrBlock>, cfg: &LineMergeConfig, frame_w: u32, frame_h: u32, neighbor: Neighbor) -> Vec<OcrBlock> {
    let n = blocks.len();
    if n <= 1 {
        return blocks;
    }

    let frame_w_f = frame_w as f32;
    let frame_h_f = frame_h as f32;
    let mut parent: Vec<usize> = (0..n).collect();
    let mut rank = vec![0u8; n];
    for i in 0..n {
        let mut best: Option<(usize, f32)> = None;
        for j in 0..n {
            if i == j {
                continue;
            }
            let Some(gap) = (match neighbor {
                Neighbor::Below => vertical_gap_if_below(&blocks[i], &blocks[j], cfg.below_mid_ratio, frame_h_f),
                Neighbor::Right => horizontal_gap_if_right(&blocks[i], &blocks[j], cfg.below_mid_ratio, frame_w_f),
            }) else {
                continue;
            };
            let (span, max_gap) = match neighbor {
                Neighbor::Below => (frame_h_f, cfg.gap_ratio),
                Neighbor::Right => (frame_w_f, cfg.horizontal_gap_ratio),
            };
            if !approx_le(gap.abs() / span, max_gap) {
                continue;
            }
            let can = match neighbor {
                Neighbor::Below => can_link_stacked(&blocks[i], &blocks[j], frame_w, cfg),
                Neighbor::Right => height_compatible(&blocks[i], &blocks[j], cfg) && vert_compatible(&blocks[i], &blocks[j], frame_h, cfg),
            };
            if !can {
                continue;
            }
            if best.map(|(_, g)| approx_lt(gap.abs(), g.abs())).unwrap_or(true) {
                best = Some((j, gap));
            }
        }
        if let Some((j, _)) = best
            && !match neighbor {
                Neighbor::Below => has_intervening_below(&blocks, i, j, frame_w, frame_h, cfg),
                Neighbor::Right => has_intervening_right(&blocks, i, j, frame_w, frame_h, cfg),
            }
        {
            union(&mut parent, &mut rank, i, j);
        }
    }

    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        groups[find(&mut parent, i)].push(i);
    }
    groups
        .into_iter()
        .filter(|g| !g.is_empty())
        .map(|members| assemble_group(&blocks, members, cfg, frame_w, frame_h))
        .collect()
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
        // Across each row, then the next row down (row-major).
        LineMergeOrder::LeftToRightTopToBottom => {
            let rows = assign_bands(blocks, members, Axis::Y, cfg.order_band_ratio, frame_h as f32);
            members.sort_by(|&a, &b| {
                rows[a]
                    .cmp(&rows[b])
                    .then_with(|| (blocks[a].bbox.x as i32).cmp(&(blocks[b].bbox.x as i32)))
                    .then_with(|| (blocks[a].bbox.y as i32).cmp(&(blocks[b].bbox.y as i32)))
            });
        }
        // Down each column, then the next column (column-major).
        LineMergeOrder::TopToBottomLeftToRight => {
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

fn horizontal_gap_if_right(left: &OcrBlock, right: &OcrBlock, beside_mid_ratio: f32, frame_w: f32) -> Option<f32> {
    let left_right = left.bbox.x + left.bbox.width;
    let right_left = right.bbox.x;
    let left_mid = left.bbox.x + left.bbox.width * 0.5;
    let slack = right.bbox.width * beside_mid_ratio;
    // right_left + slack >= left_mid  (ratio-space ε)
    if !approx_ge((right_left + slack - left_mid) / frame_w.max(1.0), 0.0) {
        return None;
    }
    Some(right_left - left_right)
}

fn can_link_stacked(upper: &OcrBlock, lower: &OcrBlock, frame_w: u32, cfg: &LineMergeConfig) -> bool {
    if !height_compatible(upper, lower, cfg) {
        return false;
    }
    if !horiz_compatible(upper, lower, frame_w, cfg) {
        return false;
    }
    if cfg.reject_short_long {
        let sw = upper.bbox.width.max(1.0);
        let ww = lower.bbox.width.max(1.0);
        if approx_lt(sw, ww) && approx_ge((ww - sw) / ww, cfg.width_delta_ratio) {
            return false;
        }
    }
    true
}

fn height_compatible(a: &OcrBlock, b: &OcrBlock, cfg: &LineMergeConfig) -> bool {
    let ah = a.bbox.height.max(1.0);
    let bh = b.bbox.height.max(1.0);
    let larger = ah.max(bh);
    approx_le((ah - bh).abs() / larger, cfg.height_delta_ratio)
}

fn horiz_compatible(a: &OcrBlock, b: &OcrBlock, frame_w: u32, cfg: &LineMergeConfig) -> bool {
    let span = (frame_w as f32).max(1.0);
    let left_frac = (a.bbox.x - b.bbox.x).abs() / span;
    let a_cx = a.bbox.x + a.bbox.width * 0.5;
    let b_cx = b.bbox.x + b.bbox.width * 0.5;
    let center_frac = (a_cx - b_cx).abs() / span;
    approx_le(left_frac, cfg.align_ratio) || approx_le(center_frac, cfg.align_ratio)
}

fn vert_compatible(a: &OcrBlock, b: &OcrBlock, frame_h: u32, cfg: &LineMergeConfig) -> bool {
    let span = (frame_h as f32).max(1.0);
    let top_frac = (a.bbox.y - b.bbox.y).abs() / span;
    let a_cy = a.bbox.y + a.bbox.height * 0.5;
    let b_cy = b.bbox.y + b.bbox.height * 0.5;
    let center_frac = (a_cy - b_cy).abs() / span;
    approx_le(top_frac, cfg.align_ratio) || approx_le(center_frac, cfg.align_ratio)
}

fn has_intervening_below(blocks: &[OcrBlock], upper: usize, lower: usize, frame_w: u32, frame_h: u32, cfg: &LineMergeConfig) -> bool {
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

fn has_intervening_right(blocks: &[OcrBlock], left: usize, right: usize, frame_w: u32, frame_h: u32, cfg: &LineMergeConfig) -> bool {
    let l = &blocks[left];
    let r = &blocks[right];
    let x0 = l.bbox.x + l.bbox.width;
    let x1 = r.bbox.x;
    let span = (frame_w as f32).max(1.0);
    if approx_le((x1 - x0) / span, 0.0) {
        return false;
    }

    for (k, b) in blocks.iter().enumerate() {
        if k == left || k == right {
            continue;
        }
        let cx = b.bbox.x + b.bbox.width * 0.5;
        if approx_le((cx - x0) / span, 0.0) || approx_ge((cx - x1) / span, 0.0) {
            continue;
        }
        if vert_compatible(l, b, frame_h, cfg) && vert_compatible(b, r, frame_h, cfg) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Default test frame (1080p). Production always passes the capture size.
    const DEFAULT_FRAME_W: u32 = 1920;
    const DEFAULT_FRAME_H: u32 = 1080;

    fn merge_line_blocks(blocks: Vec<OcrBlock>) -> Vec<OcrBlock> {
        merge_line_blocks_with(blocks, &LineMergeConfig::default(), DEFAULT_FRAME_W, DEFAULT_FRAME_H, false)
    }

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
    fn merges_short_left_into_long_right() {
        let blocks = vec![
            line(0, "short", 10.0, 10.0, 60.0, 18.0),
            line(1, "much longer body", 75.0, 12.0, 160.0, 18.0),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "short much longer body");
    }

    #[test]
    fn split_first_line_short_wrap_joins_upper_not_next() {
        let h = 18.0;
        let blocks = vec![
            line(0, "Hi", 10.0, 10.0, 40.0, h),
            line(1, "there this line is long", 55.0, 10.0, 180.0, h),
            line(2, "wraps.", 10.0, 32.0, 90.0, h),
            line(3, "Next paragraph here", 10.0, 54.0, 140.0, h),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 2, "{:?}", merged.iter().map(|b| &b.text).collect::<Vec<_>>());
        assert_eq!(merged[0].text, "Hi there this line is long wraps.");
        assert_eq!(merged[1].text, "Next paragraph here");
    }

    #[test]
    fn split_first_line_all_merge_when_guard_off() {
        let cfg = LineMergeConfig {
            reject_short_long: false,
            ..Default::default()
        };
        let h = 18.0;
        let blocks = vec![
            line(0, "Hi", 10.0, 10.0, 40.0, h),
            line(1, "there this line is long", 55.0, 10.0, 180.0, h),
            line(2, "wraps.", 10.0, 32.0, 90.0, h),
            line(3, "Next paragraph here", 10.0, 54.0, 140.0, h),
        ];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "Hi there this line is long wraps. Next paragraph here");
    }

    #[test]
    fn long_short_long_wraps_upward_not_down() {
        let h = 18.0;
        let blocks = vec![
            line(0, "Long first line of paragraph", 10.0, 10.0, 200.0, h),
            line(1, "short wrap", 10.0, 32.0, 150.0, h),
            line(2, "Long next paragraph line", 10.0, 54.0, 200.0, h),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 2, "{:?}", merged.iter().map(|b| &b.text).collect::<Vec<_>>());
        assert_eq!(merged[0].text, "Long first line of paragraph short wrap");
        assert_eq!(merged[1].text, "Long next paragraph line");
    }

    #[test]
    fn three_line_wrap_last_slightly_wider_still_merges() {
        let h = 18.0;
        let blocks = vec![
            line(0, "This is a long first line", 10.0, 10.0, 220.0, h),
            line(1, "middle wraps shorter", 10.0, 32.0, 160.0, h),
            line(2, "last line a bit longer.", 10.0, 54.0, 161.0, h),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1, "{:?}", merged.iter().map(|b| &b.text).collect::<Vec<_>>());
        assert_eq!(merged[0].text, "This is a long first line middle wraps shorter last line a bit longer.");
    }

    #[test]
    fn four_long_lines_merge_despite_width_jitter() {
        let h = 18.0;
        let blocks = vec![
            line(0, "Line one of a long paragraph", 10.0, 10.0, 200.0, h),
            line(1, "Line two of a long paragraph", 10.0, 32.0, 188.0, h),
            line(2, "Line three of a long paragraph", 10.0, 54.0, 196.0, h),
            line(3, "Line four of a long paragraph", 10.0, 76.0, 190.0, h),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1, "{:?}", merged.iter().map(|b| &b.text).collect::<Vec<_>>());
        assert_eq!(
            merged[0].text,
            "Line one of a long paragraph Line two of a long paragraph Line three of a long paragraph Line four of a long paragraph"
        );
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
    fn does_not_merge_side_by_side_far_apart() {
        let blocks = vec![line(0, "Left", 10.0, 10.0, 40.0, 18.0), line(1, "Right", 200.0, 12.0, 40.0, 18.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn merges_side_by_side_when_gap_small() {
        let blocks = vec![line(0, "Left", 10.0, 10.0, 40.0, 18.0), line(1, "Right", 55.0, 12.0, 40.0, 18.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "Left Right");
    }

    #[test]
    fn horizontal_gap_at_threshold_still_merges() {
        let cfg = LineMergeConfig::default();
        let w = 40.0;
        let h = 18.0;
        let gap = cfg.horizontal_gap_ratio * DEFAULT_FRAME_W as f32;
        let blocks = vec![line(0, "Left", 10.0, 10.0, w, h), line(1, "Right", 10.0 + w + gap, 10.0, w, h)];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "Left Right");
    }

    #[test]
    fn horizontal_gap_beyond_threshold_stays_separate() {
        let cfg = LineMergeConfig::default();
        let w = 40.0;
        let h = 18.0;
        let gap = cfg.horizontal_gap_ratio * DEFAULT_FRAME_W as f32 + 2.0;
        let blocks = vec![line(0, "Left", 10.0, 10.0, w, h), line(1, "Right", 10.0 + w + gap, 10.0, w, h)];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn horizontal_gap_disabled_keeps_neighbors_separate() {
        let cfg = LineMergeConfig {
            horizontal_gap_ratio: 0.0,
            ..Default::default()
        };
        let blocks = vec![line(0, "Left", 10.0, 10.0, 40.0, 18.0), line(1, "Right", 55.0, 12.0, 40.0, 18.0)];
        let merged = merge_with(blocks, cfg, false);
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
    fn unaligned_column_does_not_merge() {
        let blocks = vec![line(0, "A", 10.0, 10.0, 80.0, 18.0), line(1, "B", 120.0, 32.0, 80.0, 18.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn aligned_short_over_long_merges_when_guard_off() {
        // Left edges within align_ratio; gap covers stacked proximity (including overlap).
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
        assert_eq!(merged[0].text, "A C B D");
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
        assert_eq!(merged[0].text, "A B C D");
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
        assert_eq!(merged[0].text, "A B C D");
    }
}

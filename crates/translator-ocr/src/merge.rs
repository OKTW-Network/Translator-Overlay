//! Merge OCR line boxes into paragraph blocks.
//!
//! [`LineMergeConfig`] tunes each step.
//!
//! 1. Gaps are measured against the frame. Vertical gaps divide by the frame height
//!    and horizontal gaps by the frame width.
//! 2. Fragments on the same row join first, to the nearest one on the right. Then
//!    lines join to the nearest one below, so a split long line becomes one box
//!    before it stacks.
//! 3. With `merge_all`, every line in the crop joins into one block.
//! 4. Lines join and blocks come out in row-major or column-major reading order.

use translator_core::{LineMergeConfig, LineMergeOrder, OcrBlock, Rect};

use crate::reindex;

/// Scale-free slack for comparing ratios such as `px / frame` or `|d| / larger`.
const RATIO_EPS: f32 = 1e-5;

/// Merge consecutive line-level OCR boxes that belong to the same paragraph.
///
/// `frame_w` and `frame_h` are the full capture size, even when `blocks` came
/// from a crop. `merge_all` joins every line, for a whole selected region.
pub fn merge_line_blocks_with(blocks: Vec<OcrBlock>, cfg: &LineMergeConfig, frame_w: u32, frame_h: u32, merge_all: bool) -> Vec<OcrBlock> {
    if !cfg.enabled || blocks.len() <= 1 {
        return reindex(blocks);
    }

    let frame = Frame {
        w: frame_w.max(1),
        h: frame_h.max(1),
    };

    if merge_all {
        let members: Vec<usize> = (0..blocks.len()).collect();
        return reindex(vec![assemble_group(&blocks, members, cfg, frame)]);
    }

    let rows = join_nearest(blocks, cfg, frame, Axis::X);
    let merged = join_nearest(rows, cfg, frame, Axis::Y);
    reindex(sort_blocks(merged, cfg, frame))
}

/// Full capture size in pixels, at least 1 on each side.
#[derive(Clone, Copy)]
struct Frame {
    w: u32,
    h: u32,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Axis {
    X,
    Y,
}

impl Axis {
    fn cross(self) -> Self {
        match self {
            Self::X => Self::Y,
            Self::Y => Self::X,
        }
    }

    /// Start and length of `r` along this axis.
    fn span(self, r: Rect) -> (f32, f32) {
        match self {
            Self::X => (r.x, r.width),
            Self::Y => (r.y, r.height),
        }
    }

    /// Frame size along this axis.
    fn extent(self, frame: Frame) -> f32 {
        match self {
            Self::X => frame.w as f32,
            Self::Y => frame.h as f32,
        }
    }
}

/// Union each block with its nearest neighbor after it along `axis` (right for X, below for Y).
fn join_nearest(blocks: Vec<OcrBlock>, cfg: &LineMergeConfig, frame: Frame, axis: Axis) -> Vec<OcrBlock> {
    let n = blocks.len();
    if n <= 1 {
        return blocks;
    }

    let span = axis.extent(frame);
    let max_gap = match axis {
        Axis::X => cfg.horizontal_gap_ratio,
        Axis::Y => cfg.gap_ratio,
    };
    let mut parent: Vec<usize> = (0..n).collect();
    let mut rank = vec![0u8; n];
    for i in 0..n {
        let (a0, alen) = axis.span(blocks[i].bbox);
        let mut best: Option<(usize, f32)> = None;
        for j in 0..n {
            if i == j {
                continue;
            }
            // `j` must start past the middle of `i`, with `below_mid_ratio` of its length as slack.
            let (b0, blen) = axis.span(blocks[j].bbox);
            if !approx_ge((b0 + blen * cfg.below_mid_ratio - (a0 + alen * 0.5)) / span, 0.0) {
                continue;
            }
            let gap = b0 - (a0 + alen);
            if !approx_le(gap.abs() / span, max_gap) || !can_link(&blocks[i], &blocks[j], axis, frame, cfg) {
                continue;
            }
            if best.is_none_or(|(_, g)| approx_lt(gap.abs(), g.abs())) {
                best = Some((j, gap));
            }
        }
        if let Some((j, _)) = best
            && !has_intervening(&blocks, i, j, axis, frame, cfg)
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
        .map(|members| assemble_group(&blocks, members, cfg, frame))
        .collect()
}

/// Union-find root with path halving.
fn find(parent: &mut [usize], mut x: usize) -> usize {
    while parent[x] != x {
        parent[x] = parent[parent[x]];
        x = parent[x];
    }
    x
}

/// Union by rank. Keep the rank. The root decides where a joined row lands in the output,
/// and that order breaks distance ties in the next pass, so another rule changes the merges.
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

fn assemble_group(blocks: &[OcrBlock], mut members: Vec<usize>, cfg: &LineMergeConfig, frame: Frame) -> OcrBlock {
    sort_indices(blocks, &mut members, cfg, frame);

    let mut text = String::new();
    let mut conf = 0.0f32;
    let mut x0 = f32::MAX;
    let mut y0 = f32::MAX;
    let mut x1 = f32::MIN;
    let mut y1 = f32::MIN;
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
        y1 = y1.max(b.bbox.y + b.source_height.max(b.bbox.height));
        heights.push(b.bbox.height.max(1.0));
    }
    conf /= members.len() as f32;

    // Keep the box one line tall so the overlay sizes its font to a single glyph row.
    // `source_height` is the ink union, and the overlay never covers less than that.
    let line_h = median_f32(heights);
    let bbox = Rect::new(x0, y0, (x1 - x0).max(1.0), line_h);
    let source_lines = members.iter().map(|&idx| blocks[idx].source_lines.max(1)).sum::<u32>().max(1);
    let source_height = (y1 - y0).max(line_h);

    OcrBlock {
        id: 0,
        text,
        confidence: conf,
        bbox,
        source_lines,
        source_height,
    }
}

fn sort_blocks(blocks: Vec<OcrBlock>, cfg: &LineMergeConfig, frame: Frame) -> Vec<OcrBlock> {
    let mut order: Vec<usize> = (0..blocks.len()).collect();
    sort_indices(&blocks, &mut order, cfg, frame);
    let mut slots: Vec<Option<OcrBlock>> = blocks.into_iter().map(Some).collect();
    order.into_iter().filter_map(|i| slots[i].take()).collect()
}

/// Sort into bands along one axis, then by start position along the other axis, then this one.
fn sort_indices(blocks: &[OcrBlock], members: &mut [usize], cfg: &LineMergeConfig, frame: Frame) {
    let band_axis = match cfg.order {
        // Across each row, then the next row down (row-major).
        LineMergeOrder::LeftToRightTopToBottom => Axis::Y,
        // Down each column, then the next column (column-major).
        LineMergeOrder::TopToBottomLeftToRight => Axis::X,
    };
    let bands = assign_bands(blocks, members, band_axis, cfg.order_band_ratio, band_axis.extent(frame));
    let start = |i: usize, axis: Axis| axis.span(blocks[i].bbox).0 as i32;
    let along = band_axis.cross();
    members.sort_by(|&a, &b| {
        bands[a]
            .cmp(&bands[b])
            .then_with(|| start(a, along).cmp(&start(b, along)))
            .then_with(|| start(a, band_axis).cmp(&start(b, band_axis)))
    });
}

/// Greedy 1-D clusters along `axis` (sorted). `band_ratio` × `axis_span` is the band width.
///
/// Columns band on the left edge; rows band on the vertical center.
fn assign_bands(blocks: &[OcrBlock], members: &[usize], axis: Axis, band_ratio: f32, axis_span: f32) -> Vec<u32> {
    let coord = |b: &OcrBlock| match axis {
        Axis::X => b.bbox.x,
        Axis::Y => b.bbox.center().1,
    };
    let mut order: Vec<usize> = members.to_vec();
    order.sort_by(|&a, &b| {
        coord(&blocks[a])
            .partial_cmp(&coord(&blocks[b]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut ids = vec![0u32; blocks.len()];
    let mut current = 0u32;
    let mut last: Option<f32> = None;
    let span = axis_span.max(1.0);
    for i in order {
        let c = coord(&blocks[i]);
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

fn can_link(a: &OcrBlock, b: &OcrBlock, axis: Axis, frame: Frame, cfg: &LineMergeConfig) -> bool {
    if !height_compatible(a, b, cfg) || !aligned(a, b, axis.cross(), frame, cfg) {
        return false;
    }
    // Keep a shorter upper line off a much wider line below it.
    if axis == Axis::Y && cfg.reject_short_long {
        let upper_w = a.bbox.width.max(1.0);
        let lower_w = b.bbox.width.max(1.0);
        if approx_lt(upper_w, lower_w) && approx_ge((lower_w - upper_w) / lower_w, cfg.width_delta_ratio) {
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

/// Starts or centers along `axis` lie within `align_ratio` of the frame.
fn aligned(a: &OcrBlock, b: &OcrBlock, axis: Axis, frame: Frame, cfg: &LineMergeConfig) -> bool {
    let span = axis.extent(frame);
    let (a0, alen) = axis.span(a.bbox);
    let (b0, blen) = axis.span(b.bbox);
    let start_frac = (a0 - b0).abs() / span;
    let center_frac = ((a0 + alen * 0.5) - (b0 + blen * 0.5)).abs() / span;
    approx_le(start_frac, cfg.align_ratio) || approx_le(center_frac, cfg.align_ratio)
}

/// True when another block sits in the gap between `first` and `second` along `axis`
/// and lines up with both of them across it.
fn has_intervening(blocks: &[OcrBlock], first: usize, second: usize, axis: Axis, frame: Frame, cfg: &LineMergeConfig) -> bool {
    let (a, b) = (&blocks[first], &blocks[second]);
    let (a0, alen) = axis.span(a.bbox);
    let start = a0 + alen;
    let end = axis.span(b.bbox).0;
    let span = axis.extent(frame);
    if approx_le((end - start) / span, 0.0) {
        return false;
    }
    let cross = axis.cross();
    blocks.iter().enumerate().any(|(k, m)| {
        let (m0, mlen) = axis.span(m.bbox);
        let mid = m0 + mlen * 0.5;
        k != first
            && k != second
            && !approx_le((mid - start) / span, 0.0)
            && !approx_ge((mid - end) / span, 0.0)
            && aligned(a, m, cross, frame, cfg)
            && aligned(m, b, cross, frame, cfg)
    })
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
    vals.sort_by(f32::total_cmp);
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
            source_height: h,
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
        assert!((merged[0].source_height - 40.0).abs() < 0.5, "cover span should be the ink union, got {}", merged[0].source_height);
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
    fn horizontal_gap_threshold() {
        let cfg = LineMergeConfig::default();
        let w = 40.0;
        let h = 18.0;
        let gap = cfg.horizontal_gap_ratio * DEFAULT_FRAME_W as f32;
        let at = vec![line(0, "Left", 10.0, 10.0, w, h), line(1, "Right", 10.0 + w + gap, 10.0, w, h)];
        assert_eq!(merge_with(at, cfg.clone(), false).len(), 1);
        let beyond = vec![
            line(0, "Left", 10.0, 10.0, w, h),
            line(1, "Right", 10.0 + w + gap + 2.0, 10.0, w, h),
        ];
        assert_eq!(merge_with(beyond, cfg, false).len(), 2);
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
    fn vertical_gap_threshold() {
        let cfg = LineMergeConfig::default();
        let h = 18.0;
        let gap = cfg.gap_ratio * DEFAULT_FRAME_H as f32;
        let at = vec![
            line(0, "Hello", 10.0, 10.0, 120.0, h),
            line(1, "world", 12.0, 10.0 + h + gap, 90.0, h),
        ];
        assert_eq!(merge_with(at, cfg.clone(), false).len(), 1);
        let beyond = vec![
            line(0, "Hello", 10.0, 10.0, 120.0, h),
            line(1, "world", 12.0, 10.0 + h + gap + 2.0, 90.0, h),
        ];
        assert_eq!(merge_with(beyond, cfg, false).len(), 2);
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
        // 20 px / 1080 ≈ 0.0185 is over 0.015, so no merge. 20 px / 2000 = 0.01 is under, so they merge.
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
        assert!((merged[0].source_height - 20.0).abs() < 0.5, "same-row union is ~one line, got {}", merged[0].source_height);
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
        let blocks = vec![
            line(0, "A", 10.0, 10.0, 20.0, 16.0),
            line(1, "B", 80.0, 10.0, 20.0, 16.0),
            line(2, "C", 10.0, 40.0, 20.0, 16.0),
            line(3, "D", 80.0, 40.0, 20.0, 16.0),
        ];
        let merged = merge_with(blocks, LineMergeConfig::default(), true);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "A B C D");
    }
}

//! Merge OCR line boxes into paragraph blocks.
//!
//! Strategy (tunable via [`LineMergeConfig`]):
//! 1. **Frame-relative distances** — every gap and tolerance is a fraction of the
//!    frame height, on both axes.
//! 2. **Same-row first** — nearest-right fragments, then nearest-below (a split
//!    long line becomes one box before stacking).
//! 3. **Whole region** — join every line in the crop (caller sets `merge_all`
//!    for [`LineMergeMode::Region`] with drawn regions).
//! 4. **Reading order** — row-major or column-major join / emit order. The row
//!    and column tolerances that gate merging also band rows and columns here.
//!
//! Vertical text (columns ≥ 1.5× taller than wide) compares column widths, reads
//! right to left, and anchors the block one column wide.
//!
//! [`LineMergeMode::Layout`] skips 1–2 and joins lines that share a layout box.

use translator_core::{LineJoin, LineMergeConfig, LineMergeMode, LineMergeOrder, OcrBlock, Rect};

use crate::reindex;

/// Scale-free slack for ratio-space compares (`px / frame` or `|d| / larger`).
const RATIO_EPS: f32 = 1e-5;

/// The next line along an axis must start past the middle of the current one,
/// with this fraction of its own length as slack.
const NEXT_SLACK: f32 = 0.25;

/// Merge consecutive line-level OCR boxes that belong to the same paragraph.
///
/// `frame_h` is the **full capture** height even when `blocks` came from a crop.
/// `merge_all` joins every remaining line (whole selected region).
/// `layout` holds the layout boxes in the same coordinates as `blocks` (Layout mode).
pub fn merge_line_blocks_with(
    blocks: Vec<OcrBlock>,
    cfg: &LineMergeConfig,
    frame_h: u32,
    merge_all: bool,
    layout: &[(Rect, bool)],
) -> Vec<OcrBlock> {
    if cfg.mode == LineMergeMode::Off || blocks.len() <= 1 {
        return reindex(blocks);
    }

    let frame = Frame(frame_h.max(1) as f32);

    if merge_all {
        let members: Vec<usize> = (0..blocks.len()).collect();
        return reindex(vec![assemble_group(&blocks, members, cfg, frame, None, false)]);
    }

    if cfg.mode == LineMergeMode::Layout {
        return reindex(sort_blocks(join_by_layout(blocks, layout, cfg, frame), cfg, frame));
    }

    let rows = join_nearest(blocks, cfg, frame, Axis::X);
    let merged = join_nearest(rows, cfg, frame, Axis::Y);
    reindex(sort_blocks(merged, cfg, frame))
}

/// Join lines whose center falls in the same layout box (the smallest one when nested).
/// Lines outside every box stay single. A `vertical_text` box (`true`) reads as columns
/// even when its lines are too short to look vertical.
///
/// A group never skips a line: one sitting between two of its lines (and beside them)
/// pulls its own group in, or the joined block would cover that line. Nested boxes from
/// a stretched, very wide crop otherwise join lines 1 + 3 around line 2.
fn join_by_layout(blocks: Vec<OcrBlock>, layout: &[(Rect, bool)], cfg: &LineMergeConfig, frame: Frame) -> Vec<OcrBlock> {
    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); layout.len()];
    let mut singles = Vec::new();
    for (i, b) in blocks.iter().enumerate() {
        let (cx, cy) = (b.bbox.x + b.bbox.width * 0.5, b.bbox.y + b.bbox.height * 0.5);
        let owner = layout
            .iter()
            .enumerate()
            .filter(|(_, (r, _))| cx >= r.x && cx <= r.x + r.width && cy >= r.y && cy <= r.y + r.height)
            .min_by(|(_, (a, _)), (_, (b, _))| (a.width * a.height).total_cmp(&(b.width * b.height)))
            .map(|(k, _)| k);
        match owner {
            Some(k) => groups[k].push(i),
            None => singles.push(i),
        }
    }
    let mut groups: Vec<(Vec<usize>, bool)> = groups
        .into_iter()
        .zip(layout.iter().map(|(_, vertical)| *vertical))
        .filter(|(g, _)| !g.is_empty())
        .chain(singles.into_iter().map(|i| (vec![i], false)))
        .collect();

    // Index of a group with a line between two lines of group `g` (reading axis) that
    // overlaps `g` across it.
    let skipped = |groups: &[(Vec<usize>, bool)], g: usize| {
        let (members, vertical) = &groups[g];
        let (axis, cross) = if *vertical || mostly_vertical(&blocks, members) {
            (Axis::X, Axis::Y)
        } else {
            (Axis::Y, Axis::X)
        };
        let mid = |i: usize| {
            let (start, len) = axis.span(blocks[i].bbox);
            start + len * 0.5
        };
        let (lo, hi) = members
            .iter()
            .fold((f32::MAX, f32::MIN), |(lo, hi), &i| (lo.min(mid(i)), hi.max(mid(i))));
        let (c0, c1) = members.iter().fold((f32::MAX, f32::MIN), |(c0, c1), &i| {
            let (start, len) = cross.span(blocks[i].bbox);
            (c0.min(start), c1.max(start + len))
        });
        (0..groups.len()).find(|&h| {
            h != g
                && groups[h].0.iter().any(|&m| {
                    let (start, len) = cross.span(blocks[m].bbox);
                    mid(m) > lo && mid(m) < hi && start < c1 && start + len > c0
                })
        })
    };
    while let Some((g, h)) = (0..groups.len()).find_map(|g| skipped(&groups, g).map(|h| (g, h))) {
        let (taken, vertical) = std::mem::take(&mut groups[h]);
        groups[g].0.extend(taken);
        groups[g].1 |= vertical;
    }

    groups
        .into_iter()
        .filter(|(g, _)| !g.is_empty())
        .map(|(members, vertical)| assemble_group(&blocks, members, cfg, frame, None, vertical))
        .collect()
}

/// Full capture height in pixels (at least 1): the base of every distance ratio.
#[derive(Clone, Copy)]
struct Frame(f32);

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

    /// Ink AABB, not the shrunk glyph box — tilt must not inflate the gap.
    fn cover_span(self, b: &OcrBlock) -> (f32, f32) {
        match self {
            Self::X => (b.bbox.x, b.bbox.width.max(b.source_width)),
            Self::Y => (b.bbox.y, b.bbox.height.max(b.source_height)),
        }
    }

    /// Offset across this axis that still counts as the same column (X) or row (Y).
    fn tolerance(self, cfg: &LineMergeConfig) -> f32 {
        match self {
            Self::X => cfg.column_tolerance,
            Self::Y => cfg.row_tolerance,
        }
    }
}

/// Union each block with its nearest neighbor after it along `axis` (right for X, below for Y).
fn join_nearest(blocks: Vec<OcrBlock>, cfg: &LineMergeConfig, frame: Frame, axis: Axis) -> Vec<OcrBlock> {
    let n = blocks.len();
    if n <= 1 {
        return blocks;
    }

    let span = frame.0;
    let max_gap = match axis {
        Axis::X => cfg.word_gap,
        Axis::Y => cfg.line_gap,
    };
    let mut parent: Vec<usize> = (0..n).collect();
    let mut rank = vec![0u8; n];
    for i in 0..n {
        let (a0, alen) = axis.cover_span(&blocks[i]);
        let mut best: Option<(usize, f32)> = None;
        for j in 0..n {
            if i == j {
                continue;
            }
            let (b0, blen) = axis.cover_span(&blocks[j]);
            if !approx_ge((b0 + blen * NEXT_SLACK - (a0 + alen * 0.5)) / span, 0.0) {
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
        .map(|members| assemble_group(&blocks, members, cfg, frame, Some(axis), false))
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

/// Union by rank. Keep the rank: the root decides where a joined row lands in the output,
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

/// `force_vertical` reads the group as columns even when its lines are not tall.
fn assemble_group(
    blocks: &[OcrBlock],
    mut members: Vec<usize>,
    cfg: &LineMergeConfig,
    frame: Frame,
    along: Option<Axis>,
    force_vertical: bool,
) -> OcrBlock {
    let vertical = force_vertical || mostly_vertical(blocks, &members);
    sort_indices(blocks, &mut members, cfg, frame, vertical);

    let mut text = String::new();
    let mut conf = 0.0f32;
    let mut x0 = f32::MAX;
    let mut y0 = f32::MAX;
    let mut x1 = f32::MIN;
    let mut y1 = f32::MIN;
    let mut cover_x1 = f32::MIN;
    let mut heights: Vec<f32> = Vec::with_capacity(members.len());
    let mut widths: Vec<f32> = Vec::with_capacity(members.len());

    for (k, &idx) in members.iter().enumerate() {
        let b = &blocks[idx];
        if k == 0 {
            text = b.text.clone();
        } else {
            // X-pass is already same-row (stagger included). Y-pass is stacked.
            // Columns swap the two: side by side is the next line, stacked is the same one.
            // merge_all mixes both: still going right *and* overlap on Y.
            let prev = &blocks[members[k - 1]];
            let same_row = match along {
                Some(Axis::X) => !vertical,
                Some(Axis::Y) => vertical,
                None => {
                    b.bbox.x >= prev.bbox.x + prev.bbox.width * 0.25
                        && (prev.bbox.y + prev.bbox.height).min(b.bbox.y + b.bbox.height) > prev.bbox.y.max(b.bbox.y)
                }
            };
            let join = if cfg.join == LineJoin::Newline && same_row {
                LineJoin::Space
            } else {
                cfg.join
            };
            text = join_text(&text, &b.text, join);
        }
        conf += b.confidence;
        x0 = x0.min(b.bbox.x);
        y0 = y0.min(b.bbox.y);
        x1 = x1.max(b.bbox.x + b.bbox.width);
        cover_x1 = cover_x1.max(b.bbox.x + b.bbox.width.max(b.source_width));
        y1 = y1.max(b.bbox.y + b.source_height.max(b.bbox.height));
        heights.push(b.bbox.height.max(1.0));
        widths.push(b.bbox.width.max(1.0));
    }
    conf /= members.len() as f32;

    // One-line anchor (one row tall, or one column wide) so overlay font tracks a single glyph.
    // `source_height` / `source_width` are the ink union; overlay uses them as min cover.
    let bbox = if vertical {
        Rect::new(x0, y0, median_f32(widths), (y1 - y0).max(1.0))
    } else {
        Rect::new(x0, y0, (x1 - x0).max(1.0), median_f32(heights))
    };
    let source_lines = members.iter().map(|&idx| blocks[idx].source_lines.max(1)).sum::<u32>().max(1);
    let source_height = (y1 - y0).max(bbox.height);
    let source_width = (cover_x1 - x0).max(bbox.width);

    OcrBlock {
        id: 0,
        text,
        confidence: conf,
        bbox,
        source_lines,
        source_height,
        source_width,
    }
}

fn sort_blocks(blocks: Vec<OcrBlock>, cfg: &LineMergeConfig, frame: Frame) -> Vec<OcrBlock> {
    let mut order: Vec<usize> = (0..blocks.len()).collect();
    let vertical = mostly_vertical(&blocks, &order);
    sort_indices(&blocks, &mut order, cfg, frame, vertical);
    let mut slots: Vec<Option<OcrBlock>> = blocks.into_iter().map(Some).collect();
    order.into_iter().filter_map(|i| slots[i].take()).collect()
}

/// Taller than wide by the same 1.5× oar-ocr uses to rotate a crop for recognition.
fn is_vertical(b: &OcrBlock) -> bool {
    b.bbox.height >= b.bbox.width * 1.5
}

fn is_horizontal(b: &OcrBlock) -> bool {
    b.bbox.width >= b.bbox.height * 1.5
}

/// More columns than clearly horizontal lines; near-square short runs count for neither.
fn mostly_vertical(blocks: &[OcrBlock], members: &[usize]) -> bool {
    let count = |f: fn(&OcrBlock) -> bool| members.iter().filter(|&&i| f(&blocks[i])).count();
    count(is_vertical) > count(is_horizontal)
}

/// Sort into bands along one axis (row / column tolerance), then by start position along
/// the other axis, then this one.
///
/// `vertical` reads columns right to left, top to bottom, whatever `cfg.order` says.
fn sort_indices(blocks: &[OcrBlock], members: &mut [usize], cfg: &LineMergeConfig, frame: Frame, vertical: bool) {
    let start = |i: usize, axis: Axis| axis.span(blocks[i].bbox).0 as i32;
    let band_axis = match cfg.order {
        _ if vertical => Axis::X,
        // Across each row, then the next row down (row-major).
        LineMergeOrder::LeftToRightTopToBottom => Axis::Y,
        // Down each column, then the next column to the left (column-major).
        LineMergeOrder::TopToBottomRightToLeft => Axis::X,
    };
    // At most half the median size along the band axis (line height for rows, column
    // width for columns): a loose tolerance must not fuse neighbouring rows of small text.
    let half = median_f32(members.iter().map(|&i| band_axis.span(blocks[i].bbox).1).collect()) * 0.5;
    let band = band_axis.tolerance(cfg).min(half / frame.0);
    let bands = assign_bands(blocks, members, band_axis, band, frame.0);
    let along = band_axis.cross();
    let band_order = |a: usize, b: usize| match band_axis {
        Axis::X => bands[b].cmp(&bands[a]),
        Axis::Y => bands[a].cmp(&bands[b]),
    };
    members.sort_by(|&a, &b| {
        band_order(a, b)
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
    // A column never joins a clearly horizontal line; near-square short runs go either way.
    if (is_vertical(a) && is_horizontal(b)) || (is_horizontal(a) && is_vertical(b)) {
        return false;
    }
    if !height_compatible(a, b, cfg) || !aligned(a, b, axis.cross(), frame, cfg) {
        return false;
    }
    // Keep a shorter upper line off a much wider line below it.
    if axis == Axis::Y && cfg.split_short_above_wide {
        let upper_w = a.bbox.width.max(1.0);
        let lower_w = b.bbox.width.max(1.0);
        if approx_lt(upper_w, lower_w) && approx_ge((lower_w - upper_w) / lower_w, cfg.width_diff) {
            return false;
        }
    }
    true
}

/// Same glyph size: line height, or column width when either side is a column
/// (column length varies with the text, the last one is often short).
fn height_compatible(a: &OcrBlock, b: &OcrBlock, cfg: &LineMergeConfig) -> bool {
    let side = |r: Rect| if is_vertical(a) || is_vertical(b) { r.width } else { r.height };
    let ah = side(a.bbox).max(1.0);
    let bh = side(b.bbox).max(1.0);
    let larger = ah.max(bh);
    approx_le((ah - bh).abs() / larger, cfg.size_diff)
}

/// Starts or centers along `axis` lie within that axis' row / column tolerance.
fn aligned(a: &OcrBlock, b: &OcrBlock, axis: Axis, frame: Frame, cfg: &LineMergeConfig) -> bool {
    let tol = axis.tolerance(cfg);
    let (a0, alen) = axis.span(a.bbox);
    let (b0, blen) = axis.span(b.bbox);
    let start_frac = (a0 - b0).abs() / frame.0;
    let center_frac = ((a0 + alen * 0.5) - (b0 + blen * 0.5)).abs() / frame.0;
    approx_le(start_frac, tol) || approx_le(center_frac, tol)
}

/// True when another block sits in the gap between `first` and `second` along `axis`
/// and lines up with both of them across it.
fn has_intervening(blocks: &[OcrBlock], first: usize, second: usize, axis: Axis, frame: Frame, cfg: &LineMergeConfig) -> bool {
    let (a, b) = (&blocks[first], &blocks[second]);
    let (a0, alen) = axis.cover_span(a);
    let start = a0 + alen;
    let end = axis.cover_span(b).0;
    let span = frame.0;
    if approx_le((end - start) / span, 0.0) {
        return false;
    }
    let cross = axis.cross();
    blocks.iter().enumerate().any(|(k, m)| {
        let (m0, mlen) = axis.cover_span(m);
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

fn join_text(a: &str, b: &str, join: LineJoin) -> String {
    let a = a.trim_end();
    let b = b.trim_start();
    if a.is_empty() {
        return b.to_string();
    }
    if b.is_empty() {
        return a.to_string();
    }
    match join {
        LineJoin::None => format!("{a}{b}"),
        LineJoin::Space => format!("{a} {b}"),
        LineJoin::Newline => format!("{a}\n{b}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Default test frame height (1080p). Production always passes the capture size.
    const DEFAULT_FRAME_H: u32 = 1080;

    fn merge_line_blocks(blocks: Vec<OcrBlock>) -> Vec<OcrBlock> {
        merge_line_blocks_with(blocks, &LineMergeConfig::default(), DEFAULT_FRAME_H, false, &[])
    }

    fn line(id: u32, text: &str, x: f32, y: f32, w: f32, h: f32) -> OcrBlock {
        OcrBlock {
            id,
            text: text.to_string(),
            confidence: 0.9,
            bbox: Rect::new(x, y, w, h),
            source_lines: 1,
            source_height: h,
            source_width: w,
        }
    }

    fn merge_with(blocks: Vec<OcrBlock>, cfg: LineMergeConfig, merge_all: bool) -> Vec<OcrBlock> {
        merge_line_blocks_with(blocks, &cfg, DEFAULT_FRAME_H, merge_all, &[])
    }

    #[test]
    fn merges_stacked_english_lines() {
        let blocks = vec![line(0, "Hello", 10.0, 10.0, 100.0, 18.0), line(1, "world", 12.0, 32.0, 90.0, 18.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "Hello\nworld");
        assert!(merged[0].bbox.height <= 22.0, "h={}", merged[0].bbox.height);
        assert_eq!(merged[0].source_lines, 2, "merged block tracks line count");
        assert!((merged[0].source_height - 40.0).abs() < 0.5, "cover span should be the ink union, got {}", merged[0].source_height);
    }

    #[test]
    fn stacked_vertical_keeps_short_width_and_aabb_cover() {
        let mut a = line(0, "上", 100.0, 50.0, 20.0, 80.0);
        a.source_width = 30.0;
        let mut b = line(1, "下", 100.0, 140.0, 20.0, 80.0);
        b.source_width = 30.0;
        let merged = merge_line_blocks(vec![a, b]);
        assert_eq!(merged.len(), 1);
        assert!((merged[0].bbox.width - 20.0).abs() < 0.5, "glyph box must stay the short side, got {}", merged[0].bbox.width);
        assert!((merged[0].source_width - 30.0).abs() < 0.5, "cover must keep AABB width, got {}", merged[0].source_width);
    }

    #[test]
    fn stacked_tilted_lines_join_on_aabb_gap_not_glyph_gap() {
        // Glyph 20px, AABB 30px: shear inflates glyph-to-glyph gap by 10px.
        // AABB gap 20px < 27 (0.025×1080); glyph gap 30px would miss.
        let mut a = line(0, "Hello", 10.0, 10.0, 200.0, 20.0);
        a.source_height = 30.0;
        let mut b = line(1, "world", 10.0, 60.0, 200.0, 20.0);
        b.source_height = 30.0;
        let merged = merge_line_blocks(vec![a, b]);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "Hello\nworld");
    }

    #[test]
    fn stacked_12px_lines_join_with_newline_on_1080p() {
        let blocks = vec![line(0, "Hello", 10.0, 10.0, 80.0, 12.0), line(1, "world", 10.0, 22.0, 70.0, 12.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "Hello\nworld");
    }

    #[test]
    fn merge_disabled_passthrough() {
        let cfg = LineMergeConfig {
            mode: LineMergeMode::Off,
            ..Default::default()
        };
        let blocks = vec![line(0, "Hello", 10.0, 10.0, 100.0, 18.0), line(1, "world", 12.0, 32.0, 90.0, 18.0)];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn join_none_concatenates() {
        let cfg = LineMergeConfig {
            join: LineJoin::None,
            ..Default::default()
        };
        let blocks = vec![line(0, "甲乙", 10.0, 10.0, 100.0, 20.0), line(1, "丙丁", 10.0, 34.0, 80.0, 20.0)];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "甲乙丙丁");
    }

    #[test]
    fn join_space_inserts_a_space() {
        let cfg = LineMergeConfig {
            join: LineJoin::Space,
            ..Default::default()
        };
        let blocks = vec![line(0, "Hello", 10.0, 10.0, 100.0, 18.0), line(1, "world", 12.0, 32.0, 90.0, 18.0)];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged[0].text, "Hello world");
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
            split_short_above_wide: false,
            ..Default::default()
        };
        let blocks = vec![
            line(0, "short", 10.0, 10.0, 60.0, 18.0),
            line(1, "much longer body", 10.0, 32.0, 160.0, 18.0),
        ];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "short\nmuch longer body");
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
        assert_eq!(merged[0].text, "Hi there this line is long\nwraps.");
        assert_eq!(merged[1].text, "Next paragraph here");
    }

    #[test]
    fn split_first_line_all_merge_when_guard_off() {
        let cfg = LineMergeConfig {
            split_short_above_wide: false,
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
        assert_eq!(merged[0].text, "Hi there this line is long\nwraps.\nNext paragraph here");
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
        assert_eq!(merged[0].text, "Long first line of paragraph\nshort wrap");
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
        assert_eq!(merged[0].text, "This is a long first line\nmiddle wraps shorter\nlast line a bit longer.");
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
            "Line one of a long paragraph\nLine two of a long paragraph\nLine three of a long paragraph\nLine four of a long paragraph"
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
        assert_eq!(merged[0].text, "This is a long\nparagraph that wraps\nacross three lines.");
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
    fn merges_side_by_side_when_vertically_staggered() {
        let blocks = vec![line(0, "Left", 10.0, 10.0, 40.0, 18.0), line(1, "Right", 55.0, 20.0, 40.0, 18.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "Left Right");
    }

    #[test]
    fn horizontal_gap_threshold() {
        let cfg = LineMergeConfig::default();
        let w = 40.0;
        let h = 18.0;
        let gap = cfg.word_gap * DEFAULT_FRAME_H as f32;
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
            word_gap: 0.0,
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
        let gap = cfg.line_gap * DEFAULT_FRAME_H as f32;
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
        assert_eq!(merged[0].text, "Line one\nLine two");
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
        assert!(texts.contains(&"L1\nL2"));
        assert!(texts.contains(&"R1\nR2"));
    }

    #[test]
    fn does_not_chain_merge_via_growing_bbox() {
        let blocks = vec![
            line(0, "L1", 10.0, 10.0, 100.0, 16.0),
            line(1, "L2", 10.0, 28.0, 80.0, 16.0),
            line(2, "P2", 10.0, 80.0, 80.0, 16.0),
            line(3, "P3", 10.0, 132.0, 80.0, 16.0),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].text, "L1\nL2");
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
        // Left edges within column_tolerance; gap covers stacked proximity (including overlap).
        let cfg = LineMergeConfig {
            split_short_above_wide: false,
            ..Default::default()
        };
        let blocks = vec![line(0, "A", 100.0, 10.0, 15.0, 18.0), line(1, "B", 110.0, 32.0, 200.0, 18.0)];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "A\nB");
    }

    #[test]
    fn large_list_pitch_stays_separate() {
        let h = 42.0;
        let x = 1334.0;
        let pitch = h + 32.0;
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
            line(1, "world", 12.0, 10.0 + 18.0 + 30.0, 90.0, 18.0),
        ];
        let cfg = LineMergeConfig::default();
        // 30px / 1080 ≈ 0.028 > 0.025 → no merge; 30px / 2000 = 0.015 < 0.025 → merge.
        let tall = merge_line_blocks_with(blocks.clone(), &cfg, 2000, false, &[]);
        let short = merge_line_blocks_with(blocks, &cfg, 1080, false, &[]);
        assert_eq!(tall.len(), 1, "larger frame makes 30px a small relative gap");
        assert_eq!(short.len(), 2, "1080p treats 30px as too far");
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
            mode: LineMergeMode::Region,
            ..Default::default()
        };
        let blocks = vec![line(0, "Left", 10.0, 10.0, 40.0, 18.0), line(1, "Right", 200.0, 12.0, 40.0, 18.0)];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn whole_region_order_top_to_bottom_right_to_left() {
        let cfg = LineMergeConfig {
            order: LineMergeOrder::TopToBottomRightToLeft,
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
        assert_eq!(merged[0].text, "B\nD\nA\nC");
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
        assert_eq!(merged[0].text, "A B\nC D");
    }

    #[test]
    fn column_tolerance_gates_stacked_lines() {
        // Left edges 10px and centers 5px apart: within the default row tolerance (16px),
        // so this splits only if stacking checks the column tolerance.
        let cfg = LineMergeConfig {
            column_tolerance: 0.0,
            ..Default::default()
        };
        let blocks = vec![line(0, "Hello", 10.0, 10.0, 120.0, 18.0), line(1, "world", 20.0, 33.0, 90.0, 18.0)];
        assert_eq!(merge_with(blocks.clone(), cfg, false).len(), 2);
        assert_eq!(merge_line_blocks(blocks).len(), 1);
    }

    #[test]
    fn loose_row_tolerance_keeps_small_text_rows_apart() {
        // 54px tolerance covers both rows, but rows band within half a 12px line.
        let cfg = LineMergeConfig {
            row_tolerance: 0.05,
            ..Default::default()
        };
        let blocks = vec![
            line(0, "A", 10.0, 10.0, 20.0, 12.0),
            line(1, "B", 80.0, 10.0, 20.0, 12.0),
            line(2, "C", 10.0, 24.0, 20.0, 12.0),
            line(3, "D", 80.0, 24.0, 20.0, 12.0),
        ];
        let merged = merge_with(blocks, cfg, true);
        assert_eq!(merged[0].text, "A B\nC D");
    }

    #[test]
    fn whole_region_stacked_12px_lines_join_with_newline() {
        let blocks = vec![line(0, "Hello", 10.0, 10.0, 80.0, 12.0), line(1, "world", 10.0, 22.0, 70.0, 12.0)];
        let merged = merge_with(blocks, LineMergeConfig::default(), true);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "Hello\nworld");
    }

    #[test]
    fn whole_region_padded_wrap_joins_with_newline() {
        // 18px OCR boxes on 16px pitch overlap on Y; same x is still a wrap.
        let blocks = vec![line(0, "Hello", 10.0, 10.0, 100.0, 18.0), line(1, "world", 10.0, 26.0, 90.0, 18.0)];
        let merged = merge_with(blocks, LineMergeConfig::default(), true);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "Hello\nworld");
    }

    #[test]
    fn layout_mode_joins_lines_per_box_and_keeps_outsiders() {
        let cfg = LineMergeConfig {
            mode: LineMergeMode::Layout,
            ..LineMergeConfig::default()
        };
        // Far apart on purpose: the rules would never join these.
        let blocks = vec![
            line(0, "B2", 600.0, 400.0, 100.0, 18.0),
            line(1, "A1", 10.0, 10.0, 100.0, 18.0),
            line(2, "lone", 10.0, 900.0, 60.0, 18.0),
            line(3, "A2", 10.0, 200.0, 100.0, 18.0),
            line(4, "B1", 600.0, 300.0, 100.0, 18.0),
        ];
        let layout = [
            (Rect::new(0.0, 0.0, 300.0, 250.0), false),
            (Rect::new(590.0, 290.0, 200.0, 150.0), false),
        ];
        let merged = merge_line_blocks_with(blocks, &cfg, DEFAULT_FRAME_H, false, &layout);
        let texts: Vec<&str> = merged.iter().map(|b| b.text.as_str()).collect();
        assert_eq!(texts, [
            "A1
A2", "B1
B2", "lone"
        ]);
        assert_eq!(merged[0].source_lines, 2);
    }

    #[test]
    fn layout_group_pulls_in_the_line_it_would_skip() {
        let cfg = LineMergeConfig {
            mode: LineMergeMode::Layout,
            ..LineMergeConfig::default()
        };
        // A 1313×268 dialog crop squashed to 800×800: one box over everything, smaller
        // nested boxes over lines 2 and 4, so the smallest-box rule leaves 1 + 3 together.
        let blocks = vec![
            line(0, "L1", 25.0, 7.0, 374.0, 68.0),
            line(1, "L2", 46.0, 74.0, 1141.0, 62.0),
            line(2, "L3", 48.0, 138.0, 486.0, 65.0),
            line(3, "L4", 47.0, 202.0, 851.0, 61.0),
        ];
        let layout = [
            (Rect::new(33.0, 9.4, 1189.0, 255.8), false),
            (Rect::new(43.6, 76.2, 1143.6, 57.8), false),
            (Rect::new(48.4, 203.2, 841.0, 62.6), false),
        ];
        let merged = merge_line_blocks_with(blocks, &cfg, DEFAULT_FRAME_H, false, &layout);
        let texts: Vec<&str> = merged.iter().map(|b| b.text.as_str()).collect();
        assert_eq!(texts, ["L1\nL2\nL3", "L4"]);
    }

    /// Three top-aligned columns, 24px glyphs, 10px column gap; the left one is short.
    fn columns() -> Vec<OcrBlock> {
        vec![
            line(0, "左", 100.0, 100.0, 24.0, 120.0),
            line(1, "右", 168.0, 100.0, 24.0, 240.0),
            line(2, "中", 134.0, 100.0, 24.0, 240.0),
        ]
    }

    #[test]
    fn columns_join_right_to_left_with_line_join() {
        let merged = merge_line_blocks(columns());
        assert_eq!(merged.len(), 1, "short last column must still join");
        assert_eq!(merged[0].text, "右\n中\n左");
        assert_eq!(merged[0].source_lines, 3);
    }

    #[test]
    fn merged_columns_anchor_one_column_wide() {
        let b = &merge_line_blocks(columns())[0];
        assert!((b.bbox.x - 100.0).abs() < 0.5, "x={}", b.bbox.x);
        assert!((b.bbox.width - 24.0).abs() < 0.5, "glyph box must be one column wide, got {}", b.bbox.width);
        assert!((b.bbox.height - 240.0).abs() < 0.5, "h={}", b.bbox.height);
        assert!((b.source_width - 92.0).abs() < 0.5, "cover must span all columns, got {}", b.source_width);
        assert!((b.source_height - 240.0).abs() < 0.5, "cover_h={}", b.source_height);
    }

    #[test]
    fn one_glyph_last_column_still_reads_as_columns() {
        let blocks = vec![line(0, "！", 100.0, 100.0, 24.0, 24.0), line(1, "なに", 134.0, 100.0, 24.0, 48.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "なに\n！");
        assert!((merged[0].bbox.width - 24.0).abs() < 0.5, "w={}", merged[0].bbox.width);
    }

    #[test]
    fn column_does_not_join_horizontal_line() {
        let blocks = vec![line(0, "縦", 100.0, 100.0, 24.0, 240.0), line(1, "row", 134.0, 100.0, 200.0, 24.0)];
        assert_eq!(merge_line_blocks(blocks).len(), 2);
    }

    #[test]
    fn whole_region_reads_columns_right_to_left() {
        let merged = merge_with(columns(), LineMergeConfig::default(), true);
        assert_eq!(merged[0].text, "右\n中\n左");
    }

    #[test]
    fn layout_vertical_text_box_reads_short_runs_as_columns() {
        let cfg = LineMergeConfig {
            mode: LineMergeMode::Layout,
            ..LineMergeConfig::default()
        };
        // One- and two-glyph runs are not tall enough to look vertical on their own.
        let blocks = vec![
            line(0, "左", 100.0, 100.0, 24.0, 24.0),
            line(1, "右", 168.0, 100.0, 24.0, 30.0),
            line(2, "中", 134.0, 100.0, 24.0, 30.0),
        ];
        let layout = [(Rect::new(90.0, 90.0, 120.0, 60.0), true)];
        let merged = merge_line_blocks_with(blocks, &cfg, DEFAULT_FRAME_H, false, &layout);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "右\n中\n左");
        assert!((merged[0].bbox.width - 24.0).abs() < 0.5, "w={}", merged[0].bbox.width);
    }
}

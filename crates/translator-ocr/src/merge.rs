//! Merge OCR line boxes into paragraph blocks.
//!
//! Strategy (tunable via [`LineMergeConfig`]):
//! 1. **Frame-relative gaps** — vertical / align tolerances use the full capture
//!    frame size, not median line height.
//! 2. **Paragraph** — nearest-below link when the upper line is longer (wrap).
//! 3. **Whole region** — join every line in the crop (caller sets `merge_all`).
//! 4. **Reading order** — row-major or column-major join / emit order.

use translator_core::{LineMergeConfig, LineMergeOrder, OcrBlock, Rect};

/// Default test / simple-caller frame (1080p). Production always passes the capture size.
const DEFAULT_FRAME_W: u32 = 1920;
const DEFAULT_FRAME_H: u32 = 1080;

/// Nameplate width/height below this looks like a speaker plate.
const NAMEPLATE_ASPECT_MAX: f32 = 4.5;
/// Body must be at least this × nameplate width.
const NAMEPLATE_BODY_WIDTH_RATIO: f32 = 1.2;
/// Nameplate only when vertical gap ≤ this × frame height.
const NAMEPLATE_MAX_GAP_RATIO: f32 = 0.02;
/// Nameplate width must stay below this × frame width.
const NAMEPLATE_MAX_WIDTH_RATIO: f32 = 0.12;
/// Row / column banding as a fraction of frame height / width.
const ORDER_BAND_RATIO: f32 = 0.012;
/// Upper must be at least this fraction longer than lower to count as a wrap.
const WRAP_WIDTH_SLACK: f32 = 0.05;

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
    let n = blocks.len();
    let mut nameplates: Vec<usize> = Vec::new();
    let mut rest: Vec<usize> = Vec::new();

    for i in 0..n {
        let is_nameplate = cfg.keep_speaker_separate
            && blocks
                .iter()
                .enumerate()
                .any(|(j, body)| i != j && is_nameplate_above_body(&blocks[i], body, frame_w, frame_h));
        if is_nameplate {
            nameplates.push(i);
        } else {
            rest.push(i);
        }
    }

    let mut merged: Vec<OcrBlock> = Vec::with_capacity(nameplates.len() + 1);
    if !rest.is_empty() {
        merged.push(assemble_group(&blocks, rest, cfg.order, frame_w, frame_h));
    }
    for idx in nameplates {
        merged.push(blocks[idx].clone());
    }
    sort_blocks(&mut merged, cfg.order, frame_w, frame_h);
    reindex(merged)
}

fn merge_paragraph(blocks: Vec<OcrBlock>, cfg: &LineMergeConfig, frame_w: u32, frame_h: u32) -> Vec<OcrBlock> {
    let n = blocks.len();
    let max_gap = cfg.max_gap_ratio * frame_h as f32;
    let min_gap = cfg.min_gap_ratio * frame_h as f32;

    let mut parent: Vec<usize> = (0..n).collect();
    let mut rank = vec![0u8; n];

    for i in 0..n {
        let mut best: Option<(usize, f32)> = None;
        for j in 0..n {
            if i == j {
                continue;
            }
            let Some(gap) = vertical_gap_if_below(&blocks[i], &blocks[j]) else {
                continue;
            };
            if gap < min_gap || gap > max_gap {
                continue;
            }
            if !can_link_lines(&blocks[i], &blocks[j], frame_w, frame_h, cfg) {
                continue;
            }
            if best.map(|(_, g)| gap < g).unwrap_or(true) {
                best = Some((j, gap));
            }
        }
        if let Some((j, _)) = best
            && !has_intervening_line(&blocks, i, j, frame_w, cfg)
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
        merged.push(assemble_group(&blocks, members, cfg.order, frame_w, frame_h));
    }
    sort_blocks(&mut merged, cfg.order, frame_w, frame_h);
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

fn assemble_group(blocks: &[OcrBlock], mut members: Vec<usize>, order: LineMergeOrder, frame_w: u32, frame_h: u32) -> OcrBlock {
    sort_indices(blocks, &mut members, order, frame_w, frame_h);

    let mut text = String::new();
    let mut conf = 0.0f32;
    let mut x0 = f32::MAX;
    let mut y0 = f32::MAX;
    let mut x1 = f32::MIN;
    let mut heights: Vec<f32> = Vec::with_capacity(members.len());

    for (k, &idx) in members.iter().enumerate() {
        let b = &blocks[idx];
        text = if k == 0 { b.text.clone() } else { join_text(&text, &b.text) };
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

fn sort_blocks(blocks: &mut Vec<OcrBlock>, order: LineMergeOrder, frame_w: u32, frame_h: u32) {
    if blocks.len() <= 1 {
        return;
    }
    let mut members: Vec<usize> = (0..blocks.len()).collect();
    sort_indices(blocks, &mut members, order, frame_w, frame_h);
    let mut slots: Vec<Option<OcrBlock>> = std::mem::take(blocks).into_iter().map(Some).collect();
    *blocks = members
        .into_iter()
        .map(|i| slots[i].take().expect("sort permutation is unique"))
        .collect();
}

fn sort_indices(blocks: &[OcrBlock], members: &mut [usize], order: LineMergeOrder, frame_w: u32, frame_h: u32) {
    match order {
        LineMergeOrder::TopToBottomLeftToRight => {
            let band = ORDER_BAND_RATIO * frame_h as f32;
            let rows = assign_bands(blocks, members, Axis::Y, band);
            members.sort_by(|&a, &b| {
                rows[a]
                    .cmp(&rows[b])
                    .then_with(|| (blocks[a].bbox.x as i32).cmp(&(blocks[b].bbox.x as i32)))
                    .then_with(|| (blocks[a].bbox.y as i32).cmp(&(blocks[b].bbox.y as i32)))
            });
        }
        LineMergeOrder::LeftToRightTopToBottom => {
            let band = ORDER_BAND_RATIO * frame_w as f32;
            let cols = assign_bands(blocks, members, Axis::X, band);
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

/// Greedy 1-D clusters along `axis` (sorted), band width in pixels.
fn assign_bands(blocks: &[OcrBlock], members: &[usize], axis: Axis, band: f32) -> Vec<u32> {
    let mut order: Vec<usize> = members.to_vec();
    order.sort_by(|&a, &b| {
        let ca = axis_coord(&blocks[a], axis);
        let cb = axis_coord(&blocks[b], axis);
        ca.partial_cmp(&cb).unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut ids = vec![0u32; blocks.len()];
    let mut current = 0u32;
    let mut last: Option<f32> = None;
    let band = band.max(1.0);
    for i in order {
        let c = axis_coord(&blocks[i], axis);
        if let Some(prev) = last
            && c - prev > band
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

fn vertical_gap_if_below(upper: &OcrBlock, lower: &OcrBlock) -> Option<f32> {
    let upper_bottom = upper.bbox.y + upper.bbox.height;
    let lower_top = lower.bbox.y;
    let upper_mid = upper.bbox.y + upper.bbox.height * 0.5;
    if lower_top + lower.bbox.height * 0.25 < upper_mid {
        return None;
    }
    Some(lower_top - upper_bottom)
}

fn can_link_lines(upper: &OcrBlock, lower: &OcrBlock, frame_w: u32, frame_h: u32, cfg: &LineMergeConfig) -> bool {
    if !height_compatible(upper, lower, cfg) {
        return false;
    }
    if !horiz_compatible(upper, lower, frame_w, cfg) {
        return false;
    }
    if upper_is_longer(upper, lower) {
        return true;
    }
    // Nameplates are shorter than the body, so they fail the wrap-width rule.
    // Off = glue speaker into the line below; on = keep them split.
    !cfg.keep_speaker_separate && is_nameplate_above_body(upper, lower, frame_w, frame_h)
}

/// Wrap remainder: the line above must be meaningfully longer than the one below.
fn upper_is_longer(upper: &OcrBlock, lower: &OcrBlock) -> bool {
    let uw = upper.bbox.width.max(1.0);
    let lw = lower.bbox.width.max(1.0);
    uw > lw * (1.0 + WRAP_WIDTH_SLACK)
}

fn height_compatible(a: &OcrBlock, b: &OcrBlock, cfg: &LineMergeConfig) -> bool {
    let ah = a.bbox.height.max(1.0);
    let bh = b.bbox.height.max(1.0);
    ah.min(bh) / ah.max(bh) >= cfg.height_ratio_min
}

/// Short, narrow plate directly above a wider line (speaker / name tag layout).
fn is_nameplate_above_body(upper: &OcrBlock, lower: &OcrBlock, frame_w: u32, frame_h: u32) -> bool {
    let uh = upper.bbox.height.max(1.0);
    let uw = upper.bbox.width.max(1.0);
    let lw = lower.bbox.width.max(1.0);
    if uw >= NAMEPLATE_ASPECT_MAX * uh {
        return false;
    }
    if lw < NAMEPLATE_BODY_WIDTH_RATIO * uw {
        return false;
    }
    if uw > NAMEPLATE_MAX_WIDTH_RATIO * frame_w as f32 {
        return false;
    }
    matches!(
        vertical_gap_if_below(upper, lower),
        Some(gap) if gap <= NAMEPLATE_MAX_GAP_RATIO * frame_h as f32
    )
}

fn horiz_compatible(a: &OcrBlock, b: &OcrBlock, frame_w: u32, cfg: &LineMergeConfig) -> bool {
    let a_left = a.bbox.x;
    let a_right = a.bbox.x + a.bbox.width;
    let b_left = b.bbox.x;
    let b_right = b.bbox.x + b.bbox.width;

    let overlap = (a_right.min(b_right) - a_left.max(b_left)).max(0.0);
    let min_w = a.bbox.width.min(b.bbox.width).max(1.0);

    if overlap >= cfg.overlap_ratio_min * min_w {
        return true;
    }

    let align = cfg.left_align_ratio * frame_w as f32;
    let left_delta = (a_left - b_left).abs();
    if left_delta <= align && overlap >= 0.10 * min_w {
        return true;
    }

    let a_cx = a_left + a.bbox.width * 0.5;
    let b_cx = b_left + b.bbox.width * 0.5;
    if (a_cx - b_cx).abs() <= align && overlap >= 0.12 * min_w {
        return true;
    }

    false
}

fn has_intervening_line(blocks: &[OcrBlock], upper: usize, lower: usize, frame_w: u32, cfg: &LineMergeConfig) -> bool {
    let u = &blocks[upper];
    let l = &blocks[lower];
    let y0 = u.bbox.y + u.bbox.height;
    let y1 = l.bbox.y;
    if y1 <= y0 {
        return false;
    }

    for (k, b) in blocks.iter().enumerate() {
        if k == upper || k == lower {
            continue;
        }
        let cy = b.bbox.y + b.bbox.height * 0.5;
        if cy <= y0 || cy >= y1 {
            continue;
        }
        if horiz_compatible(u, b, frame_w, cfg) && horiz_compatible(b, l, frame_w, cfg) {
            return true;
        }
    }
    false
}

fn median_f32(mut vals: Vec<f32>) -> f32 {
    if vals.is_empty() {
        return 16.0;
    }
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    vals[vals.len() / 2]
}

fn join_text(a: &str, b: &str) -> String {
    let a = a.trim_end();
    let b = b.trim_start();
    if a.is_empty() {
        return b.to_string();
    }
    if b.is_empty() {
        return a.to_string();
    }

    if a.ends_with('-') || a.ends_with('‐') || a.ends_with('‑') {
        let mut s = a.to_string();
        s.pop();
        s.push_str(b);
        return s;
    }

    if is_cjk_heavy(a) || is_cjk_heavy(b) {
        format!("{a}{b}")
    } else {
        format!("{a} {b}")
    }
}

fn is_cjk_heavy(s: &str) -> bool {
    let mut total = 0u32;
    let mut cjk = 0u32;
    for ch in s.chars() {
        if ch.is_whitespace() {
            continue;
        }
        total += 1;
        if is_cjk(ch) {
            cjk += 1;
        }
    }
    total > 0 && cjk * 2 >= total
}

fn is_cjk(ch: char) -> bool {
    matches!(
        ch,
        '\u{4E00}'..='\u{9FFF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{3040}'..='\u{30FF}'
            | '\u{AC00}'..='\u{D7AF}'
            | '\u{3000}'..='\u{303F}'
    )
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
    fn merges_cjk_without_space() {
        let blocks = vec![line(0, "甲乙", 10.0, 10.0, 100.0, 20.0), line(1, "丙丁", 10.0, 34.0, 80.0, 20.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "甲乙丙丁");
    }

    #[test]
    fn equal_width_lines_stay_separate() {
        let blocks = vec![line(0, "甲乙", 10.0, 10.0, 80.0, 20.0), line(1, "丙丁", 10.0, 34.0, 80.0, 20.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn shorter_upper_does_not_merge() {
        let blocks = vec![
            line(0, "short", 10.0, 10.0, 60.0, 18.0),
            line(1, "much longer body", 10.0, 32.0, 160.0, 18.0),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn slightly_longer_upper_still_merges() {
        // 6% longer than lower — just over WRAP_WIDTH_SLACK.
        let blocks = vec![
            line(0, "Hello!", 10.0, 10.0, 106.0, 18.0),
            line(1, "world", 12.0, 32.0, 100.0, 18.0),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "Hello! world");
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
    fn joins_hyphenated_break() {
        assert_eq!(join_text("exam-", "ple"), "example");
    }

    #[test]
    fn does_not_merge_vertical_settings_list() {
        // Checklist / radio stack: similar left edge, item pitch ~0.45× line height.
        let h = 42.0;
        let x = 1334.0;
        let pitch = h + 19.0;
        let blocks = vec![
            line(0, "あ", x, 224.0, 86.0, h),
            line(1, "い", x, 224.0 + pitch, 78.0, h),
            line(2, "ううう", x, 224.0 + 2.0 * pitch, 164.0, h),
            line(3, "えええ", x, 224.0 + 3.0 * pitch, 154.0, h),
            line(4, "おおおおおお", x, 224.0 + 4.0 * pitch, 238.0, h),
            line(5, "かかかかか", x, 224.0 + 5.0 * pitch, 216.0, h),
            line(6, "きききき", x, 224.0 + 6.0 * pitch, 194.0, h),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 7, "list items must stay separate: {:?}", merged.iter().map(|b| &b.text).collect::<Vec<_>>());
    }

    #[test]
    fn does_not_merge_setting_label_with_on_off() {
        let label = "あいうえおかきくけこさしすせそ";
        let blocks = vec![
            line(0, label, 60.0, 466.0, 336.0, 40.0),
            line(1, "ON", 165.0, 531.0, 58.0, 42.0),
            line(2, "OFF", 454.0, 524.0, 66.0, 44.0),
        ];
        let merged = merge_line_blocks(blocks);
        let texts: Vec<&str> = merged.iter().map(|b| b.text.as_str()).collect();
        assert!(texts.contains(&label), "{texts:?}");
        assert!(texts.contains(&"ON"), "{texts:?}");
        assert!(!texts.iter().any(|t| t.contains("そON") || t.contains("そ ON")), "{texts:?}");
    }

    #[test]
    fn dialogue_box_wrap() {
        let blocks = vec![
            line(0, "spkA", 331.0, 801.0, 104.0, 62.0),
            line(1, "「あああああああああああああああああああああああああああ", 324.0, 876.0, 1220.0, 60.0),
            line(2, "いい？」", 341.0, 937.0, 190.0, 58.0),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 2, "speaker + one wrapped dialogue block: {:?}", merged.iter().map(|b| &b.text).collect::<Vec<_>>());
        assert_eq!(merged[0].text, "spkA");
        assert_eq!(merged[1].text, "「あああああああああああああああああああああああああああいい？」");
        assert!(merged[1].bbox.height <= 62.0);
    }

    #[test]
    fn single_line_dialogue_stays_separate() {
        let blocks = vec![
            line(0, "spkB", 330.0, 798.0, 107.0, 69.0),
            line(1, "「うううううううう」", 322.0, 876.0, 434.0, 60.0),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(
            merged.len(),
            2,
            "must not glue name into the single dialogue line: {:?}",
            merged.iter().map(|b| &b.text).collect::<Vec<_>>()
        );
        assert_eq!(merged[0].text, "spkB");
        assert_eq!(merged[1].text, "「うううううううう」");
    }

    #[test]
    fn single_line_only_passthrough() {
        let blocks = vec![line(0, "「うううううううう」", 322.0, 876.0, 434.0, 60.0)];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "「うううううううう」");
    }

    #[test]
    fn backlog_wrap_and_name() {
        let blocks = vec![
            line(0, "spkB", 596.0, 86.0, 76.0, 44.0),
            line(1, "「ええええええええええええええええええええ」", 586.0, 120.0, 688.0, 40.0),
            line(2, "あああああああああああああああああああW1A", 600.0, 306.0, 854.0, 40.0),
            line(3, "W1B。", 602.0, 338.0, 220.0, 40.0),
            line(4, "いいいいいいいいいいいいいいいいいいいW2A", 602.0, 416.0, 848.0, 40.0),
            line(5, "W2B。", 604.0, 448.0, 468.0, 40.0),
            line(6, "「おおおおおおおおおおおおおおおおW3A", 553.0, 927.0, 834.0, 45.0),
            line(7, "W3B", 601.0, 959.0, 190.0, 30.0),
        ];
        let merged = merge_line_blocks(blocks);
        let texts: Vec<&str> = merged.iter().map(|b| b.text.as_str()).collect();

        assert!(texts.contains(&"spkB"), "{texts:?}");
        assert!(texts.iter().any(|t| t.contains("W1A") && t.contains("W1B")), "{texts:?}");
        assert!(texts.iter().any(|t| t.contains("W2A") && t.contains("W2B")), "{texts:?}");
        assert!(texts.iter().any(|t| t.contains("W3A") && t.contains("W3B")), "{texts:?}");
        let narr_count = texts.iter().filter(|t| t.contains("W1A") || t.contains("W2A")).count();
        assert_eq!(narr_count, 2, "{texts:?}");
    }

    #[test]
    fn backlog_short_entries_stay_separate() {
        let blocks = vec![
            line(0, "？？", 601.0, 521.0, 96.0, 46.0),
            line(1, "「かかか」", 580.0, 552.0, 192.0, 48.0),
            line(2, "spkB", 595.0, 629.0, 80.0, 48.0),
            line(3, "「き」", 580.0, 660.0, 110.0, 50.0),
        ];
        let merged = merge_line_blocks(blocks);
        assert!(merged.len() >= 3, "{:?}", merged.iter().map(|b| &b.text).collect::<Vec<_>>());
        assert!(merged.iter().any(|b| b.text == "spkB"));
    }

    #[test]
    fn backlog_wraps_and_keeps_entries() {
        let blocks = vec![
            line(0, "あああああああああああああああああああW1A", 600.0, 148.0, 852.0, 40.0),
            line(1, "W1B。", 599.0, 179.0, 222.0, 44.0),
            line(2, "いいいいいいいいいいいいいいいいいいいW2A", 602.0, 258.0, 848.0, 40.0),
            line(3, "W2B。", 602.0, 290.0, 470.0, 40.0),
            line(4, "？？？", 599.0, 363.0, 98.0, 46.0),
            line(5, "「かかか」", 580.0, 396.0, 192.0, 48.0),
            line(6, "spkB", 595.0, 473.0, 78.0, 48.0),
            line(7, "「き」", 577.0, 503.0, 114.0, 52.0),
            line(8, "くくくくくくくくくくくくくくくくくW4。", 602.0, 586.0, 806.0, 40.0),
            line(9, "けけけけけけけけW5？", 600.0, 662.0, 456.0, 40.0),
            line(10, "spkA", 594.0, 734.0, 80.0, 52.0),
            line(11, "「おおおおおおおおおおおおおおおおW3A", 580.0, 768.0, 806.0, 46.0),
            line(12, "W3B」", 600.0, 804.0, 198.0, 40.0),
            line(13, "spkB", 595.0, 877.0, 78.0, 48.0),
            line(14, "「ささささささ」", 582.0, 910.0, 328.0, 46.0),
        ];
        let merged = merge_line_blocks(blocks);
        let texts: Vec<&str> = merged.iter().map(|b| b.text.as_str()).collect();

        assert!(texts.iter().any(|t| t.contains("W1A") && t.contains("W1B")), "para1 wrap: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("W2A") && t.contains("W2B")), "para2 wrap: {texts:?}");
        assert!(texts.iter().any(|t| t.contains("W3A") && t.contains("W3B")), "dialogue wrap: {texts:?}");
        assert!(texts.contains(&"spkA"), "{texts:?}");
        assert!(texts.contains(&"spkB"), "{texts:?}");
        assert!(!texts.iter().any(|t| t.contains("spkA「") || t.starts_with("spkA「")), "{texts:?}");
        let narr = texts.iter().filter(|t| t.contains("W1A") || t.contains("W2A")).count();
        assert_eq!(narr, 2, "two narrative paragraphs: {texts:?}");
    }

    #[test]
    fn wrap_with_positive_gap_still_merges_on_dense_page() {
        let h = 40.0;
        let mut blocks = Vec::new();
        for i in 0..5 {
            blocks.push(line(i, &format!("row{i}"), 600.0, 20.0 + i as f32 * 80.0, 400.0, h));
        }
        blocks.push(line(5, "あああああああああああああああああああW1A", 600.0, 500.0, 850.0, h));
        blocks.push(line(6, "W1B。", 600.0, 500.0 + h + 14.0, 220.0, h));

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
        // Flag on config alone does not merge; engine must pass merge_all for a user region.
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
    fn paragraph_nameplate_off_joins_speaker() {
        let cfg = LineMergeConfig {
            keep_speaker_separate: false,
            ..Default::default()
        };
        let blocks = vec![
            line(0, "spkA", 331.0, 801.0, 104.0, 62.0),
            line(1, "「あああああああああああああああああああああああああああ", 324.0, 876.0, 1220.0, 60.0),
            line(2, "いい？」", 341.0, 937.0, 190.0, 58.0),
        ];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 1, "{:?}", merged.iter().map(|b| &b.text).collect::<Vec<_>>());
        assert!(merged[0].text.starts_with("spkA"));
        assert!(merged[0].text.contains("あああ") && merged[0].text.contains("いい"));
    }

    #[test]
    fn paragraph_nameplate_off_does_not_join_wide_short_label() {
        // Wide short box is not a nameplate; wrap-width rule still applies.
        let cfg = LineMergeConfig {
            keep_speaker_separate: false,
            ..Default::default()
        };
        let blocks = vec![
            line(0, "label", 10.0, 10.0, 300.0, 20.0),
            line(1, "much longer body line", 10.0, 34.0, 500.0, 20.0),
        ];
        let merged = merge_with(blocks, cfg, false);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn whole_region_nameplate_off_joins_all() {
        let cfg = LineMergeConfig {
            keep_speaker_separate: false,
            ..Default::default()
        };
        let blocks = vec![
            line(0, "spkA", 331.0, 801.0, 104.0, 62.0),
            line(1, "「あああああああああああああああああああああああああああ", 324.0, 876.0, 1220.0, 60.0),
            line(2, "いい？」", 341.0, 937.0, 190.0, 58.0),
        ];
        let merged = merge_with(blocks, cfg, true);
        assert_eq!(merged.len(), 1, "{:?}", merged.iter().map(|b| &b.text).collect::<Vec<_>>());
        assert!(merged[0].text.contains("spkA"));
        assert!(merged[0].text.contains("いい"));
    }

    #[test]
    fn whole_region_keeps_nameplate() {
        let cfg = LineMergeConfig::default();
        let blocks = vec![
            line(0, "spkA", 331.0, 801.0, 104.0, 62.0),
            line(1, "「あああああああああああああああああああああああああああ", 324.0, 876.0, 1220.0, 60.0),
            line(2, "いい？」", 341.0, 937.0, 190.0, 58.0),
        ];
        let merged = merge_with(blocks, cfg, true);
        assert_eq!(merged.len(), 2, "{:?}", merged.iter().map(|b| &b.text).collect::<Vec<_>>());
        assert!(merged.iter().any(|b| b.text == "spkA"));
        assert!(merged.iter().any(|b| b.text.contains("あああ") && b.text.contains("いい")));
    }
}

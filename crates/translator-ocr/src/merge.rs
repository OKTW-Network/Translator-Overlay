//! Merge OCR line boxes into paragraph blocks.
//!
//! Strategy (tunable via [`LineMergeConfig`]):
//! 1. **Page metrics** — median line height; sample vertical gaps between body lines.
//! 2. **Adaptive leading** — max merge gap from typical tight gaps, clamped by config.
//! 3. **Nearest-below linking** — each line links only its nearest compatible neighbour.
//! 4. **Union-Find** — components become one text block; bbox height stays **one line**
//!    tall so the overlay can size itself from the translation, not the OCR union.

use translator_core::{LineMergeConfig, OcrBlock, Rect};

/// Merge with default config (tests / simple callers).
pub fn merge_line_blocks(blocks: Vec<OcrBlock>) -> Vec<OcrBlock> {
    merge_line_blocks_with(blocks, &LineMergeConfig::default())
}

/// Merge consecutive line-level OCR boxes that belong to the same paragraph.
pub fn merge_line_blocks_with(blocks: Vec<OcrBlock>, cfg: &LineMergeConfig) -> Vec<OcrBlock> {
    if !cfg.enabled || blocks.len() <= 1 {
        return blocks;
    }

    let n = blocks.len();
    let med_h = median_f32(blocks.iter().map(|b| b.bbox.height.max(1.0)).collect());
    let max_gap = estimate_max_gap(&blocks, med_h, cfg);
    let min_gap = cfg.min_gap_ratio * med_h;

    let mut parent: Vec<usize> = (0..n).collect();
    let mut rank = vec![0u8; n];

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

    for i in 0..n {
        let mut best: Option<(usize, f32)> = None;
        for j in 0..n {
            if i == j {
                continue;
            }
            let Some(gap) = vertical_gap_if_below(&blocks[i], &blocks[j]) else {
                continue;
            };
            let pair_max = pair_max_gap(&blocks[i], &blocks[j], med_h, max_gap, cfg);
            if gap < min_gap || gap > pair_max {
                continue;
            }
            if !can_link_lines(&blocks[i], &blocks[j], med_h, cfg) {
                continue;
            }
            // Settings lists / radio stacks look like short paragraphs but sit
            // further apart than true wrapped dialogue lines.
            if is_spaced_list_pair(&blocks, i, j, med_h, cfg) {
                continue;
            }
            if best.map(|(_, g)| gap < g).unwrap_or(true) {
                best = Some((j, gap));
            }
        }
        if let Some((j, _)) = best
            && !has_intervening_line(&blocks, i, j, med_h, cfg)
        {
            union(&mut parent, &mut rank, i, j);
        }
    }

    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); n];
    for i in 0..n {
        let r = find(&mut parent, i);
        groups[r].push(i);
    }

    let mut merged: Vec<OcrBlock> = Vec::with_capacity(n);
    for mut members in groups {
        if members.is_empty() {
            continue;
        }
        members.sort_by(|&a, &b| {
            let ya = blocks[a].bbox.y as i32;
            let yb = blocks[b].bbox.y as i32;
            ya.cmp(&yb)
                .then_with(|| (blocks[a].bbox.x as i32).cmp(&(blocks[b].bbox.x as i32)))
        });

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
                join_text(&text, &b.text)
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

        merged.push(OcrBlock {
            id: 0,
            text,
            confidence: conf,
            bbox,
        });
    }

    merged.sort_by(|a, b| {
        (a.bbox.y as i32)
            .cmp(&(b.bbox.y as i32))
            .then_with(|| (a.bbox.x as i32).cmp(&(b.bbox.x as i32)))
    });
    for (i, b) in merged.iter_mut().enumerate() {
        b.id = i as u32;
    }
    merged
}

fn estimate_max_gap(blocks: &[OcrBlock], med_h: f32, cfg: &LineMergeConfig) -> f32 {
    let mut tight_gaps: Vec<f32> = Vec::new();
    // Only sample true paragraph leading (almost-touching lines). Including
    // menu item pitch inflated max_gap and glued whole lists.
    let sample_ceil = cfg.leading_sample_max_ratio * med_h;

    for (i, a) in blocks.iter().enumerate() {
        for (j, b) in blocks.iter().enumerate() {
            if i == j {
                continue;
            }
            let Some(gap) = vertical_gap_if_below(a, b) else {
                continue;
            };
            if gap < 0.0 || gap > sample_ceil {
                continue;
            }
            if !is_body_line(a, cfg) && !is_body_line(b, cfg) {
                continue;
            }
            if !can_link_lines(a, b, med_h, cfg) {
                continue;
            }
            tight_gaps.push(gap);
        }
    }

    let floor = cfg.max_gap_ratio_floor * med_h;
    let ceil = cfg.max_gap_ratio * med_h;

    if tight_gaps.is_empty() {
        // Prefer conservative leading when the page has no near-touching pairs
        // (settings UIs, sparse labels) so list pitch is not treated as wrap.
        return (cfg.empty_leading_fallback_ratio * med_h).clamp(floor.max(4.0), ceil.max(floor));
    }

    tight_gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let q_idx = (tight_gaps.len().saturating_sub(1) * 2) / 5;
    let typical = tight_gaps[q_idx];
    let adaptive = typical * cfg.gap_slack;
    adaptive.clamp(floor, ceil.max(floor))
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

fn can_link_lines(upper: &OcrBlock, lower: &OcrBlock, med_h: f32, cfg: &LineMergeConfig) -> bool {
    if !height_compatible(upper, lower, cfg) {
        return false;
    }
    if !horiz_compatible(upper, lower, med_h, cfg) {
        return false;
    }
    if is_short_label_above_long_body(upper, lower, cfg) {
        return false;
    }
    // Wide heading above a compact control (geometry only). Require non-tight
    // vertical gap so short wrap remainders (wide line → short last line) still merge.
    if is_compact_box(lower, cfg)
        && upper.bbox.width > cfg.control_under_heading_width_ratio * lower.bbox.width.max(1.0)
        && let Some(gap) = vertical_gap_if_below(upper, lower)
        && gap > cfg.control_under_heading_gap_ratio * med_h
    {
        return false;
    }
    // Short nameplate-like box directly above a wider body line (geometry only).
    if cfg.keep_speaker_separate && is_nameplate_above_body(upper, lower, med_h, cfg) {
        return false;
    }
    true
}

/// True when `i`→`j` looks like two items in a vertical UI list rather than a
/// line wrap. Decision uses **bbox geometry and column gap statistics only**
/// (no vocabulary / language-specific character lists). Thresholds come from
/// [`LineMergeConfig`].
fn is_spaced_list_pair(
    blocks: &[OcrBlock],
    i: usize,
    j: usize,
    med_h: f32,
    cfg: &LineMergeConfig,
) -> bool {
    let upper = &blocks[i];
    let lower = &blocks[j];
    let Some(gap) = vertical_gap_if_below(upper, lower) else {
        return false;
    };
    let list_gap_min = cfg.list_gap_min_ratio * med_h;
    if gap <= list_gap_min {
        return false;
    }

    // Full-width line + shorter remainder is classic wrap layout.
    let uw = upper.bbox.width.max(1.0);
    let lw = lower.bbox.width.max(1.0);
    let uh = upper.bbox.height.max(1.0);
    if uw >= cfg.wrap_width_ratio * lw && uw >= cfg.wrap_min_aspect * uh {
        return false;
    }

    let tol = (cfg.left_align_ratio * med_h).max(8.0);
    let mut col: Vec<&OcrBlock> = blocks
        .iter()
        .filter(|b| {
            (height_compatible(upper, b, cfg) || height_compatible(lower, b, cfg))
                && ((b.bbox.x - upper.bbox.x).abs() <= tol
                    || (b.bbox.x - lower.bbox.x).abs() <= tol
                    || horiz_compatible(upper, b, med_h, cfg)
                    || horiz_compatible(lower, b, med_h, cfg))
        })
        .collect();
    if col.len() < cfg.list_min_peers as usize {
        return false;
    }

    col.sort_by(|a, b| {
        (a.bbox.y as i32)
            .cmp(&(b.bbox.y as i32))
            .then_with(|| (a.bbox.x as i32).cmp(&(b.bbox.x as i32)))
    });

    // Consecutive gaps in this column → estimate regular item pitch.
    let mut gaps: Vec<f32> = Vec::new();
    for w in col.windows(2) {
        let top = w[0];
        let bot = w[1];
        let g = bot.bbox.y - (top.bbox.y + top.bbox.height);
        // Ignore heavy overlap / reverse order noise.
        if g > -0.15 * med_h {
            gaps.push(g.max(0.0));
        }
    }
    if gaps.len() < 2 {
        // Fallback: similar widths + non-tight gap + enough peers.
        let width_ratio = uw.min(lw) / uw.max(lw);
        return width_ratio >= cfg.list_width_similarity * 0.9;
    }

    gaps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    // Prefer larger gaps when estimating *item* pitch (skip pure wrap leading).
    let itemish: Vec<f32> = gaps
        .iter()
        .copied()
        .filter(|g| *g >= cfg.item_pitch_gap_min_ratio * med_h)
        .collect();
    let pitch = if itemish.is_empty() {
        gaps[gaps.len() / 2]
    } else {
        itemish[itemish.len() / 2]
    };

    // Gap sits on the column’s regular rhythm → list peers.
    if pitch >= cfg.list_pitch_min_ratio * med_h
        && gap >= cfg.list_pitch_match_lo * pitch
        && gap <= cfg.list_pitch_match_hi * pitch
    {
        return true;
    }
    // Clearly tighter than item pitch → wrap leading, not list.
    if pitch > 0.0 && gap < cfg.wrap_vs_pitch_ratio * pitch {
        return false;
    }

    // Ambiguous: similar-width stack with non-tight gap among many peers.
    let width_ratio = uw.min(lw) / uw.max(lw);
    width_ratio >= cfg.list_width_similarity && col.len() >= cfg.list_fallback_peers as usize
}

/// Compact control-sized box (pill / short button), geometry only.
fn is_compact_box(b: &OcrBlock, cfg: &LineMergeConfig) -> bool {
    let h = b.bbox.height.max(1.0);
    let w = b.bbox.width.max(1.0);
    w < cfg.compact_aspect_max * h
}

fn is_body_line(b: &OcrBlock, cfg: &LineMergeConfig) -> bool {
    let h = b.bbox.height.max(1.0);
    b.bbox.width >= cfg.body_aspect_min * h || b.bbox.width >= cfg.body_min_width
}

fn pair_max_gap(
    upper: &OcrBlock,
    lower: &OcrBlock,
    med_h: f32,
    page_max: f32,
    cfg: &LineMergeConfig,
) -> f32 {
    if is_body_line(upper, cfg) || is_body_line(lower, cfg) {
        page_max
    } else {
        (cfg.short_max_gap_ratio * med_h).min(page_max).max(4.0)
    }
}

fn height_compatible(a: &OcrBlock, b: &OcrBlock, cfg: &LineMergeConfig) -> bool {
    let ah = a.bbox.height.max(1.0);
    let bh = b.bbox.height.max(1.0);
    ah.min(bh) / ah.max(bh) >= cfg.height_ratio_min
}

fn is_short_label_above_long_body(
    upper: &OcrBlock,
    lower: &OcrBlock,
    cfg: &LineMergeConfig,
) -> bool {
    let uw = upper.bbox.width.max(1.0);
    let lw = lower.bbox.width.max(1.0);
    let uh = upper.bbox.height.max(1.0);
    uw < cfg.label_width_ratio * lw && lw >= cfg.label_body_min_aspect * uh
}

/// Short, narrow plate directly above a wider line (speaker / name tag layout).
/// Uses bbox aspect and relative widths only — no character vocabulary.
fn is_nameplate_above_body(
    upper: &OcrBlock,
    lower: &OcrBlock,
    med_h: f32,
    cfg: &LineMergeConfig,
) -> bool {
    let uh = upper.bbox.height.max(1.0);
    let uw = upper.bbox.width.max(1.0);
    let lw = lower.bbox.width.max(1.0);
    if uw >= cfg.nameplate_aspect_max * uh {
        return false;
    }
    if lw < cfg.nameplate_body_width_ratio * uw {
        return false;
    }
    let max_name_w = (cfg.speaker_max_chars as f32) * med_h * cfg.nameplate_char_width_scale;
    if uw > max_name_w.max(3.2 * uh) {
        return false;
    }
    matches!(
        vertical_gap_if_below(upper, lower),
        Some(gap) if gap <= cfg.nameplate_max_gap_ratio * med_h
    )
}

fn horiz_compatible(a: &OcrBlock, b: &OcrBlock, med_h: f32, cfg: &LineMergeConfig) -> bool {
    let a_left = a.bbox.x;
    let a_right = a.bbox.x + a.bbox.width;
    let b_left = b.bbox.x;
    let b_right = b.bbox.x + b.bbox.width;

    let overlap = (a_right.min(b_right) - a_left.max(b_left)).max(0.0);
    let min_w = a.bbox.width.min(b.bbox.width).max(1.0);

    if overlap >= cfg.overlap_ratio_min * min_w {
        return true;
    }

    let left_delta = (a_left - b_left).abs();
    if left_delta <= cfg.left_align_ratio * med_h && overlap >= 0.10 * min_w {
        return true;
    }

    let a_cx = a_left + a.bbox.width * 0.5;
    let b_cx = b_left + b.bbox.width * 0.5;
    if (a_cx - b_cx).abs() <= cfg.left_align_ratio * med_h && overlap >= 0.12 * min_w {
        return true;
    }

    false
}

fn has_intervening_line(
    blocks: &[OcrBlock],
    upper: usize,
    lower: usize,
    med_h: f32,
    cfg: &LineMergeConfig,
) -> bool {
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
        if horiz_compatible(u, b, med_h, cfg) && horiz_compatible(b, l, med_h, cfg) {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn line(id: u32, text: &str, x: f32, y: f32, w: f32, h: f32) -> OcrBlock {
        OcrBlock {
            id,
            text: text.to_string(),
            confidence: 0.9,
            bbox: Rect::new(x, y, w, h),
        }
    }

    #[test]
    fn merges_stacked_english_lines() {
        let blocks = vec![
            line(0, "Hello", 10.0, 10.0, 100.0, 18.0),
            line(1, "world", 12.0, 32.0, 90.0, 18.0),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "Hello world");
        // One-line-tall anchor for overlay.
        assert!(merged[0].bbox.height <= 22.0, "h={}", merged[0].bbox.height);
    }

    #[test]
    fn merge_disabled_passthrough() {
        let cfg = LineMergeConfig {
            enabled: false,
            ..Default::default()
        };
        let blocks = vec![
            line(0, "Hello", 10.0, 10.0, 100.0, 18.0),
            line(1, "world", 12.0, 32.0, 90.0, 18.0),
        ];
        let merged = merge_line_blocks_with(blocks, &cfg);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn merges_cjk_without_space() {
        let blocks = vec![
            line(0, "甲乙", 10.0, 10.0, 80.0, 20.0),
            line(1, "丙丁", 10.0, 34.0, 80.0, 20.0),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].text, "甲乙丙丁");
    }

    #[test]
    fn merges_three_line_paragraph() {
        let h = 20.0;
        let blocks = vec![
            line(0, "This is a long", 10.0, 10.0, 200.0, h),
            line(1, "paragraph that wraps", 10.0, 10.0 + h + 5.0, 200.0, h),
            line(
                2,
                "across three lines.",
                10.0,
                10.0 + 2.0 * (h + 5.0),
                180.0,
                h,
            ),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 1);
        assert_eq!(
            merged[0].text,
            "This is a long paragraph that wraps across three lines."
        );
        assert!((merged[0].bbox.height - h).abs() < 1.0);
    }

    #[test]
    fn does_not_merge_side_by_side() {
        let blocks = vec![
            line(0, "Left", 10.0, 10.0, 40.0, 18.0),
            line(1, "Right", 200.0, 12.0, 40.0, 18.0),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn does_not_merge_far_vertical_gap() {
        let blocks = vec![
            line(0, "A", 10.0, 10.0, 40.0, 18.0),
            line(1, "B", 10.0, 120.0, 40.0, 18.0),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn does_not_merge_paragraph_spacing() {
        let blocks = vec![
            line(0, "Line one", 10.0, 10.0, 120.0, 18.0),
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
            line(0, "L1", 10.0, 10.0, 60.0, 16.0),
            line(1, "L2", 10.0, 30.0, 60.0, 16.0),
            line(2, "R1", 200.0, 10.0, 60.0, 16.0),
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
            line(0, "L1", 10.0, 10.0, 80.0, 16.0),
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
        assert_eq!(
            merged.len(),
            7,
            "list items must stay separate: {:?}",
            merged.iter().map(|b| &b.text).collect::<Vec<_>>()
        );
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
        assert!(
            !texts
                .iter()
                .any(|t| t.contains("そON") || t.contains("そ ON")),
            "{texts:?}"
        );
    }

    #[test]
    fn dialogue_box_wrap() {
        // Speaker nameplate + wrapped dialogue (tight vertical gap).
        let blocks = vec![
            line(0, "spkA", 331.0, 801.0, 104.0, 62.0),
            line(
                1,
                "「あああああああああああああああああああああああああああ",
                324.0,
                876.0,
                1220.0,
                60.0,
            ),
            line(2, "いい？」", 341.0, 937.0, 190.0, 58.0),
        ];
        let merged = merge_line_blocks(blocks);
        assert_eq!(
            merged.len(),
            2,
            "speaker + one wrapped dialogue block: {:?}",
            merged.iter().map(|b| &b.text).collect::<Vec<_>>()
        );
        assert_eq!(merged[0].text, "spkA");
        assert_eq!(
            merged[1].text,
            "「あああああああああああああああああああああああああああいい？」"
        );
        // One-line-tall overlay anchor (not the union of both dialogue lines).
        assert!(merged[1].bbox.height <= 62.0);
    }

    #[test]
    fn single_line_dialogue_stays_separate() {
        // Nameplate + one dialogue line (must not glue name into speech).
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
        // Unique markers (W1A/W1B …) embedded in CJK filler for join/assert only.
        let blocks = vec![
            line(0, "spkB", 596.0, 86.0, 76.0, 44.0),
            line(
                1,
                "「ええええええええええええええええええええ」",
                586.0,
                120.0,
                688.0,
                40.0,
            ),
            line(
                2,
                "あああああああああああああああああああW1A",
                600.0,
                306.0,
                854.0,
                40.0,
            ),
            line(3, "W1B。", 602.0, 338.0, 220.0, 40.0),
            line(
                4,
                "いいいいいいいいいいいいいいいいいいいW2A",
                602.0,
                416.0,
                848.0,
                40.0,
            ),
            line(5, "W2B。", 604.0, 448.0, 468.0, 40.0),
            line(
                6,
                "「おおおおおおおおおおおおおおおおW3A",
                553.0,
                927.0,
                834.0,
                45.0,
            ),
            line(7, "W3B", 601.0, 959.0, 190.0, 30.0),
        ];
        let merged = merge_line_blocks(blocks);
        let texts: Vec<&str> = merged.iter().map(|b| b.text.as_str()).collect();

        assert!(texts.contains(&"spkB"), "{texts:?}");
        assert!(
            texts.iter().any(|t| t.contains("W1A") && t.contains("W1B")),
            "{texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("W2A") && t.contains("W2B")),
            "{texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("W3A") && t.contains("W3B")),
            "{texts:?}"
        );
        let narr_count = texts
            .iter()
            .filter(|t| t.contains("W1A") || t.contains("W2A"))
            .count();
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
        assert!(
            merged.len() >= 3,
            "{:?}",
            merged.iter().map(|b| &b.text).collect::<Vec<_>>()
        );
        assert!(merged.iter().any(|b| b.text == "spkB"));
    }

    #[test]
    fn backlog_wraps_and_keeps_entries() {
        // Dense backlog: wrapped paragraphs + nameplates + short lines.
        let blocks = vec![
            line(
                0,
                "あああああああああああああああああああW1A",
                600.0,
                148.0,
                852.0,
                40.0,
            ),
            line(1, "W1B。", 599.0, 179.0, 222.0, 44.0),
            line(
                2,
                "いいいいいいいいいいいいいいいいいいいW2A",
                602.0,
                258.0,
                848.0,
                40.0,
            ),
            line(3, "W2B。", 602.0, 290.0, 470.0, 40.0),
            line(4, "？？？", 599.0, 363.0, 98.0, 46.0),
            line(5, "「かかか」", 580.0, 396.0, 192.0, 48.0),
            line(6, "spkB", 595.0, 473.0, 78.0, 48.0),
            line(7, "「き」", 577.0, 503.0, 114.0, 52.0),
            line(
                8,
                "くくくくくくくくくくくくくくくくくW4。",
                602.0,
                586.0,
                806.0,
                40.0,
            ),
            line(9, "けけけけけけけけW5？", 600.0, 662.0, 456.0, 40.0),
            line(10, "spkA", 594.0, 734.0, 80.0, 52.0),
            line(
                11,
                "「おおおおおおおおおおおおおおおおW3A",
                580.0,
                768.0,
                806.0,
                46.0,
            ),
            line(12, "W3B」", 600.0, 804.0, 198.0, 40.0),
            line(13, "spkB", 595.0, 877.0, 78.0, 48.0),
            line(14, "「ささささささ」", 582.0, 910.0, 328.0, 46.0),
        ];
        let merged = merge_line_blocks(blocks);
        let texts: Vec<&str> = merged.iter().map(|b| b.text.as_str()).collect();

        assert!(
            texts.iter().any(|t| t.contains("W1A") && t.contains("W1B")),
            "para1 wrap: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("W2A") && t.contains("W2B")),
            "para2 wrap: {texts:?}"
        );
        assert!(
            texts.iter().any(|t| t.contains("W3A") && t.contains("W3B")),
            "dialogue wrap: {texts:?}"
        );
        assert!(texts.contains(&"spkA"), "{texts:?}");
        assert!(texts.contains(&"spkB"), "{texts:?}");
        // Speaker / short lines stay separate from their dialogue.
        assert!(
            !texts
                .iter()
                .any(|t| t.contains("spkA「") || t.starts_with("spkA「")),
            "{texts:?}"
        );
        let narr = texts
            .iter()
            .filter(|t| t.contains("W1A") || t.contains("W2A"))
            .count();
        assert_eq!(narr, 2, "two narrative paragraphs: {texts:?}");
    }

    #[test]
    fn wrap_with_positive_gap_still_merges_on_dense_page() {
        // Positive wrap gap on a page with many column peers.
        let h = 40.0;
        let mut blocks = Vec::new();
        // Many list-like peers above (would trip naive peer-count rejection).
        for i in 0..5 {
            blocks.push(line(
                i,
                &format!("row{i}"),
                600.0,
                20.0 + i as f32 * 80.0,
                400.0,
                h,
            ));
        }
        // Long line + short continuation, gap = 14 (> 0.28*h).
        blocks.push(line(
            5,
            "あああああああああああああああああああW1A",
            600.0,
            500.0,
            850.0,
            h,
        ));
        blocks.push(line(6, "W1B。", 600.0, 500.0 + h + 14.0, 220.0, h));

        let merged = merge_line_blocks(blocks);
        assert!(
            merged
                .iter()
                .any(|b| b.text.contains("W1A") && b.text.contains("W1B")),
            "{:?}",
            merged.iter().map(|b| &b.text).collect::<Vec<_>>()
        );
    }
}

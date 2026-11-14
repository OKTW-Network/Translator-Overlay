//! Translation labels painted onto the overlay DIB.

use crate::{
    error::OverlayError,
    gfx::{
        draw::{self, SurfaceRect, label_pad, place_label},
        text::{self, LabelStyle},
    },
    host::OverlayHost,
};

/// Original ink a caption covers: surface px span and detector line count.
#[derive(Clone, Copy)]
pub(crate) struct SourceSpan {
    pub(crate) w: i32,
    pub(crate) h: i32,
    pub(crate) lines: i32,
}

impl OverlayHost {
    pub(crate) fn repaint(&mut self) -> Result<(), OverlayError> {
        let w = self.surface_w.max(1);
        let h = self.surface_h.max(1);
        let surface = draw::SurfaceSize::new(w, h);
        let bg = draw::Rgba::from_argb(self.config.colors.background);
        let fg = draw::Rgba::from_argb(self.config.colors.text);

        let content_w = self.content_w;
        let content_h = self.content_h;
        let mut pending: Vec<(SurfaceRect, String, SourceSpan)> = self
            .blocks
            .iter()
            .filter_map(|block| {
                let text = block.translation.trim();
                if text.is_empty() {
                    return None;
                }
                let base = draw::map_rect_to_surface(block.bbox, content_w, content_h, w, h)?;
                let cover = |ink: f32, glyph: f32, side: i32| {
                    let glyph = glyph.max(1.0);
                    ((ink.max(glyph) / glyph) * side as f32).round().max(side as f32) as i32
                };
                let span = SourceSpan {
                    w: cover(block.source_width, block.bbox.width, base.w),
                    h: cover(block.source_height, block.bbox.height, base.h),
                    lines: block.source_lines.max(1) as i32,
                };
                Some((base, text.to_string(), span))
            })
            .collect();
        // Top to bottom, so each caption grows only into room no earlier caption took.
        pending.sort_by_key(|(base, ..)| (base.y, base.x));
        let sources: Vec<SurfaceRect> = pending.iter().map(|(base, ..)| *base).collect();

        let mut labels: Vec<(SurfaceRect, String, i32, bool, i32, i32)> = Vec::with_capacity(pending.len());
        for (i, (base, text, span)) in pending.into_iter().enumerate() {
            let obstacles: Vec<SurfaceRect> = sources
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != i)
                .map(|(_, r)| *r)
                .chain(labels.iter().map(|(rect, ..)| *rect))
                .collect();
            let (expanded, font_px, vertical) = self.layout_label(base, &text, span, &obstacles, surface)?;
            labels.push((expanded, text, font_px, vertical, span.lines, base.h));
        }

        self.surface.ensure(w, h)?;
        let missing = || OverlayError::Other("paint bitmap missing".into());
        self.surface.pixels().ok_or_else(missing)?.fill(0);
        let locale = self.locale.clone();
        let style = |font_px: i32, vertical: bool, line_pitch: Option<i32>| LabelStyle {
            font_px,
            color: fg,
            locale: &locale,
            vertical,
            line_pitch,
        };

        let buf = self.surface.pixels().ok_or_else(missing)?;
        for (rect, ..) in &labels {
            draw::fill_rect_replace(buf, surface, *rect, bg);
        }
        for (rect, text, font_px, vertical, lines, first_h) in &labels {
            let (px, lines) = (*font_px, *lines);
            let mut at = *rect;
            if *vertical {
                self.paint_label(at, text, style(px, true, None))?;
                continue;
            }
            // Centre each text line on a source line: the first on the first, the rest at the
            // source's line pitch. More text lines than source lines centre as one block.
            let pad = label_pad(px);
            let n = (text::measure_lines(text, px, &locale, (rect.w - pad * 2).max(8))?.1 - px) / text::line_pitch(px) + 1;
            let first_h = if lines > 1 { (*first_h).min(rect.h) } else { rect.h };
            let source_pitch = if lines > 1 { (rect.h - first_h) / (lines - 1) } else { 0 };
            // A font override too big for a source line would spill off the label from there.
            let on_source_lines = n <= lines && px + pad * 2 <= first_h && (n == 1 || source_pitch >= px);
            let (band, pitch) = if on_source_lines {
                (first_h, source_pitch)
            } else {
                (rect.h, text::line_pitch(px))
            };
            let text_h = (n - 1) * pitch + px;
            at.h = text_h + pad * 2;
            at.y = (rect.y + (band - (if on_source_lines { px } else { text_h })) / 2 - pad).max(0);
            self.paint_label(at, text, style(px, false, Some(pitch)))?;
        }
        Ok(())
    }

    /// Layout one overlay label.
    ///
    /// - Line source: start at the source-box font (or the user `font_px`
    ///   override) and fit the source line count by first shrinking the font to
    ///   70%, then widening up to 1.75×; only then wrap onto extra lines,
    ///   shrinking further (down to 8px) while that would reach the next
    ///   obstacle below. Widening stops at the next obstacle on the right.
    ///   Obstacles are the other source boxes and the captions placed so far.
    /// - Height is at least `span.h` so a short translation still covers
    ///   every original line. Width is at least `span.w` so a tilted
    ///   vertical run still covers its AABB.
    /// - Column source (`h ≥ 2w`): a Han / kana translation is laid out in
    ///   columns (returned `true`), adding columns to the left; anything else
    ///   widens until it fits the column height, centered on the source.
    pub(crate) fn layout_label(
        &mut self,
        base: SurfaceRect,
        text: &str,
        span: SourceSpan,
        obstacles: &[SurfaceRect],
        surface: draw::SurfaceSize,
    ) -> Result<(SurfaceRect, i32, bool), OverlayError> {
        let max_w = (surface.width - base.x).max(1);
        let source_w = span.w.max(base.w).clamp(1, max_w);
        let override_px = self.config.font_px_clamped();
        let font_px = override_px.unwrap_or_else(|| draw::font_height_for(base));
        let min_h = span.h.max(base.h);

        if draw::glyph_box(base).1 >= 1.0 {
            let pad = label_pad(font_px);
            // Mostly Han / kana (punctuation not counted): reads naturally in columns.
            let cjk = |c: &char| matches!(c, '\u{3040}'..='\u{30FF}' | '\u{3400}'..='\u{4DBF}' | '\u{4E00}'..='\u{9FFF}' | '\u{F900}'..='\u{FAFF}');
            let letters = text.chars().filter(|c| c.is_alphanumeric());
            if letters.clone().filter(cjk).count() * 2 > letters.count() {
                let cols_w = text::measure_columns(text, font_px, &self.locale, min_h - pad * 2, surface.width)?;
                let box_w = (cols_w + pad * 2).max(source_w);
                let at = SurfaceRect {
                    x: base.x + source_w - box_w,
                    ..base
                };
                return Ok((place_label(at, box_w, min_h, surface), font_px, true));
            }
            let mut box_w = source_w;
            let text_h = loop {
                let text_h = text::measure_lines(text, font_px, &self.locale, (box_w - pad * 2).max(8))?.1 + pad * 2;
                if text_h <= min_h || box_w >= surface.width {
                    break text_h;
                }
                box_w = (box_w + font_px * 2).min(surface.width);
            };
            let at = SurfaceRect {
                x: base.x + (source_w - box_w) / 2,
                ..base
            };
            return Ok((place_label(at, box_w, text_h.max(min_h), surface), font_px, false));
        }

        let right = obstacles
            .iter()
            .filter(|o| o.y < base.y + min_h && o.y + o.h > base.y && o.x > base.x)
            .fold(surface.width, |r, o| r.min(o.x));
        let bottom = obstacles
            .iter()
            .filter(|o| o.x < right && o.x + o.w > base.x && o.y > base.y)
            .fold(surface.height, |b, o| b.min(o.y));
        let room_h = (bottom - base.y).max(min_h);
        let cap = (((source_w as f32) * 1.75).round() as i32).min(right - base.x).max(source_w);
        // A font the user set keeps its size: it widens and wraps instead of shrinking to fit.
        let floor = if override_px.is_some() {
            font_px
        } else {
            (font_px * 7 / 10).max(8)
        };

        let mut px = font_px;
        loop {
            let pad = label_pad(px);
            // The source's line count at this font's own pitch: text that short is painted on
            // the source lines, so it fits `min_h` even where tight line spacing leaves that a
            // few px short. Not when the source lines are closer than the font is tall (a
            // small line merged in): then the text is one block and must fit as measured.
            let lines_h = if span.lines == 1 || (min_h - base.h) / (span.lines - 1) >= px {
                (span.lines - 1) * text::line_pitch(px) + px
            } else {
                0
            };
            // Tilt inflates `min_h` past one line.
            let source_text_h = (min_h - pad * 2).max(lines_h);
            // Widen past the source only once shrinking the font has not made the text fit.
            let widest = if px > floor { source_w } else { cap };
            let mut box_w = source_w;
            let (text_w, text_h) = loop {
                let (text_w, text_h) = text::measure_lines(text, px, &self.locale, (box_w - pad * 2).max(8))?;
                if text_h <= source_text_h || box_w >= widest {
                    break (text_w, text_h);
                }
                box_w = (box_w + px * 2).min(widest);
            };
            // Extra lines only once the font is at its floor, and only down to the next obstacle.
            let limit = if px > floor {
                source_text_h
            } else {
                (room_h - pad * 2).max(source_text_h)
            };
            if text_h <= limit || px <= 8 {
                let box_w = (text_w + pad * 2).max(source_w).min(cap);
                // Text on the source's own lines keeps the source height; extra lines, or a
                // font override larger than the source text, grow it.
                let box_h = if text_h <= lines_h && px <= draw::font_height_for(base) {
                    min_h
                } else {
                    (text_h + pad * 2).max(min_h)
                };
                return Ok((place_label(base, box_w, box_h, surface), px, false));
            }
            px = (px * 9 / 10).min(px - 1).max(8);
        }
    }

    pub(crate) fn paint_label(&mut self, rect: SurfaceRect, text: &str, style: LabelStyle<'_>) -> Result<(), OverlayError> {
        let hdc = self.surface.hdc();
        let (w, h) = self.surface.size();
        let surface = draw::SurfaceSize::new(w, h);
        let buf = self
            .surface
            .pixels()
            .ok_or_else(|| OverlayError::Other("paint bitmap missing".into()))?;
        text::draw_text_label(hdc, buf, surface, rect, text, style)
    }
}

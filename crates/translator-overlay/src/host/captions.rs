//! Translation labels painted onto the overlay DIB.

use windows::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{
        DT_CALCRECT, DT_EDITCONTROL, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_TOP, DT_WORDBREAK, DeleteObject, DrawTextW, GetTextMetricsW,
        HGDIOBJ, SelectObject, TEXTMETRICW,
    },
};

use crate::{
    error::OverlayError,
    gfx::{
        draw::{self, SurfaceRect, label_pad, place_label},
        text::{self, LabelStyle},
    },
    host::OverlayHost,
};

impl OverlayHost {
    pub(crate) fn repaint(&mut self) -> Result<(), OverlayError> {
        let w = self.surface_w.max(1);
        let h = self.surface_h.max(1);
        let surface = draw::SurfaceSize::new(w, h);
        let bg = draw::Rgba::from_argb(self.config.colors.background);
        let fg = draw::Rgba::from_argb(self.config.colors.text);

        // Layout first (needs GDI measure / font) before borrowing DIB pixels.
        let content_w = self.content_w;
        let content_h = self.content_h;
        let mut pending: Vec<(SurfaceRect, String, i32, i32)> = self
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
                let source_span_h = cover(block.source_height, block.bbox.height, base.h);
                let source_span_w = cover(block.source_width, block.bbox.width, base.w);
                Some((base, text.to_string(), source_span_h, source_span_w))
            })
            .collect();
        // Top to bottom, so each caption grows only into room no earlier caption took.
        pending.sort_by_key(|(base, ..)| (base.y, base.x));
        let sources: Vec<SurfaceRect> = pending.iter().map(|(base, ..)| *base).collect();

        let mut labels: Vec<(SurfaceRect, String, i32, bool)> = Vec::with_capacity(pending.len());
        for (i, (base, text, source_span_h, source_span_w)) in pending.into_iter().enumerate() {
            let obstacles: Vec<SurfaceRect> = sources
                .iter()
                .enumerate()
                .filter(|&(j, _)| j != i)
                .map(|(_, r)| *r)
                .chain(labels.iter().map(|(rect, ..)| *rect))
                .collect();
            let (expanded, font_px, vertical) = self.layout_label(base, &text, source_span_h, source_span_w, &obstacles, surface)?;
            labels.push((expanded, text, font_px, vertical));
        }

        self.surface.ensure(w, h)?;
        {
            let buf = self
                .surface
                .pixels()
                .ok_or_else(|| OverlayError::Other("paint bitmap missing".into()))?;
            buf.fill(0);
            for (rect, ..) in &labels {
                draw::fill_rect_replace(buf, surface, *rect, bg);
            }
        }
        let locale = self.locale.clone();
        for (rect, text, font_px, vertical) in &labels {
            self.paint_label(*rect, text, LabelStyle {
                font_px: *font_px,
                color: fg,
                vcenter: false,
                vertical: vertical.then_some(locale.as_str()),
            })?;
        }
        Ok(())
    }

    /// Pick CreateFont height so the GDI cell fits inside the OCR glyph box.
    ///
    /// Fixed ratios still overshoot (Segoe UI cell > requested height; vertical
    /// OCR width is padded). Shrink until ascent+descent ≤ ~90% of the glyph side.
    fn fit_font_to_source_box(&mut self, base: SurfaceRect) -> Result<i32, OverlayError> {
        let target = draw::glyph_box(base).0.max(1);
        // Leave a little air so ClearType stems don't look larger than source ink.
        let max_cell = ((target as f32) * 0.90).round().max(8.0) as i32;
        let mut px = draw::font_height_for(base);

        for _ in 0..6 {
            self.ensure_font(px)?;
            let _ = unsafe { SelectObject(self.surface.hdc(), HGDIOBJ(self.hfont.0)) };
            let mut tm = TEXTMETRICW::default();
            let cell = if unsafe { GetTextMetricsW(self.surface.hdc(), &mut tm) }.as_bool() {
                (tm.tmAscent + tm.tmDescent).max(1)
            } else {
                px
            };
            if cell <= max_cell {
                break;
            }
            let next = ((px as f32) * (max_cell as f32) / (cell as f32)).floor().max(8.0) as i32;
            if next >= px {
                px = (px - 1).max(8);
            } else {
                px = next;
            }
        }
        Ok(px)
    }

    /// Layout one overlay label.
    ///
    /// - Line source: start at the source-box font (or the user `font_px`
    ///   override) and fit the source line count by first widening up to 1.75×,
    ///   then shrinking the font to 70%; only then wrap onto extra lines,
    ///   shrinking further (down to 8px) while that would reach the next
    ///   obstacle below. Widening stops at the next obstacle on the right.
    ///   Obstacles are the other source boxes and the captions placed so far.
    /// - Height is at least `source_span_h` so a short translation still covers
    ///   every original line. Width is at least `source_span_w` so a tilted
    ///   vertical run still covers its AABB.
    /// - Column source (`h ≥ 2w`): a Han / kana translation is laid out in
    ///   columns (returned `true`), adding columns to the left; anything else
    ///   widens until it fits the column height, centered on the source.
    pub(crate) fn layout_label(
        &mut self,
        base: SurfaceRect,
        text: &str,
        source_span_h: i32,
        source_span_w: i32,
        obstacles: &[SurfaceRect],
        surface: draw::SurfaceSize,
    ) -> Result<(SurfaceRect, i32, bool), OverlayError> {
        let max_w = (surface.width - base.x).max(1);
        let source_w = source_span_w.max(base.w).clamp(1, max_w);
        let override_px = self.config.font_px_clamped();
        let font_px = match override_px {
            Some(px) => px,
            None => self.fit_font_to_source_box(base)?,
        };
        let min_h = source_span_h.max(base.h);

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
                let text_h = self.measure_wrapped(text, font_px, (box_w - pad * 2).max(8))?.1 + pad * 2;
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
        let floor = (font_px * 7 / 10).max(8);

        let mut px = font_px;
        loop {
            let pad = label_pad(px);
            let line_h = self.measure_single_line(text, px)?.1;
            let source_text_h = (min_h - pad * 2).max(line_h);
            let mut box_w = source_w;
            let (text_w, text_h) = loop {
                let (text_w, text_h) = self.measure_wrapped(text, px, (box_w - pad * 2).max(8))?;
                if text_h <= source_text_h || box_w >= cap {
                    break (text_w, text_h);
                }
                box_w = (box_w + px * 2).min(cap);
            };
            // Extra lines only once the font is at its floor, and only down to the next obstacle.
            let limit = if px > floor {
                source_text_h
            } else {
                (room_h - pad * 2).max(line_h)
            };
            if text_h <= limit || px <= 8 {
                let box_w = (text_w + pad * 2).max(source_w).min(cap);
                let box_h = (text_h + pad * 2).max(px + pad * 2).max(min_h);
                return Ok((place_label(base, box_w, box_h, surface), px, false));
            }
            px = (px * 9 / 10).min(px - 1).max(8);
        }
    }

    /// Unwrapped single-line extent (width, height) in pixels.
    fn measure_single_line(&mut self, text: &str, font_px: i32) -> Result<(i32, i32), OverlayError> {
        self.ensure_font(font_px)?;
        let hdc = self.surface.hdc();
        let _ = unsafe { SelectObject(hdc, HGDIOBJ(self.hfont.0)) };
        let mut calc = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        let mut wide: Vec<u16> = text.encode_utf16().collect();
        let flags = DT_LEFT | DT_TOP | DT_SINGLELINE | DT_NOPREFIX | DT_CALCRECT;
        let measured_h = unsafe { DrawTextW(hdc, &mut wide, &mut calc, flags) };
        let w = (calc.right - calc.left).max(1);
        let h = if measured_h > 0 { measured_h } else { font_px + 2 };
        Ok((w, h))
    }

    /// Word-wrapped extent for a fixed text area width.
    fn measure_wrapped(&mut self, text: &str, font_px: i32, text_area_w: i32) -> Result<(i32, i32), OverlayError> {
        self.ensure_font(font_px)?;
        let hdc = self.surface.hdc();
        let _ = unsafe { SelectObject(hdc, HGDIOBJ(self.hfont.0)) };
        let mut calc = RECT {
            left: 0,
            top: 0,
            right: text_area_w.max(8),
            bottom: 0,
        };
        let mut wide: Vec<u16> = text.encode_utf16().collect();
        let flags = DT_LEFT | DT_TOP | DT_WORDBREAK | DT_EDITCONTROL | DT_NOPREFIX | DT_CALCRECT;
        let measured_h = unsafe { DrawTextW(hdc, &mut wide, &mut calc, flags) };
        let w = (calc.right - calc.left).max(1);
        let h = if measured_h > 0 { measured_h } else { font_px + 2 };
        Ok((w, h))
    }

    pub(crate) fn ensure_font(&mut self, font_px: i32) -> Result<(), OverlayError> {
        if font_px == self.font_px && !self.hfont.is_invalid() {
            return Ok(());
        }
        if !self.hfont.is_invalid() {
            let _ = unsafe { DeleteObject(self.hfont.into()) };
        }
        self.hfont = text::create_segoe_font(font_px)?;
        self.font_px = font_px;
        Ok(())
    }

    pub(crate) fn paint_label(&mut self, rect: SurfaceRect, text: &str, style: LabelStyle<'_>) -> Result<(), OverlayError> {
        self.ensure_font(style.font_px)?;
        let hdc = self.surface.hdc();
        let hfont = self.hfont;
        let (w, h) = self.surface.size();
        let surface = draw::SurfaceSize::new(w, h);
        let buf = self
            .surface
            .pixels()
            .ok_or_else(|| OverlayError::Other("paint bitmap missing".into()))?;
        text::draw_text_label(hdc, hfont, buf, surface, rect, text, style)
    }
}

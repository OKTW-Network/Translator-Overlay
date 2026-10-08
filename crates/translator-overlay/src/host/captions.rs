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
        let bg = draw::Rgba::from_argb(self.config.background_color_argb);
        let fg = draw::Rgba::from_argb(self.config.text_color_argb);

        // Lay out first, since that needs GDI measuring and fonts, before borrowing the DIB pixels.
        let content_w = self.content_w;
        let content_h = self.content_h;
        let pending: Vec<(SurfaceRect, String, u32, i32)> = self
            .blocks
            .iter()
            .filter_map(|block| {
                let text = block.translation.trim();
                if text.is_empty() {
                    return None;
                }
                let base = draw::map_rect_to_surface(block.bbox, content_w, content_h, w, h)?;
                let bbox_h = block.bbox.height.max(1.0);
                let span = block.source_height.max(bbox_h);
                let source_span_h = ((span / bbox_h) * base.h as f32).round().max(base.h as f32) as i32;
                Some((base, text.to_string(), block.source_lines.max(1), source_span_h))
            })
            .collect();

        let mut labels: Vec<(SurfaceRect, String, i32)> = Vec::with_capacity(pending.len());
        for (base, text, source_lines, source_span_h) in pending {
            let (expanded, font_px) = self.layout_label(base, &text, source_lines, source_span_h, surface)?;
            labels.push((expanded, text, font_px));
        }

        self.surface.ensure(w, h)?;
        {
            let buf = self
                .surface
                .pixels()
                .ok_or_else(|| OverlayError::Other("paint bitmap missing".into()))?;
            buf.fill(0);
            for (rect, _, _) in &labels {
                draw::fill_rect(buf, surface, *rect, bg);
            }
        }
        for (rect, text, font_px) in &labels {
            self.paint_label(*rect, text, LabelStyle {
                font_px: *font_px,
                color: fg,
                vcenter: false,
            })?;
        }
        Ok(())
    }

    /// Pick a CreateFont height so the GDI cell fits inside the OCR glyph box.
    ///
    /// Fixed ratios still overshoot, because the Segoe UI cell is taller than the
    /// requested height and vertical OCR boxes are padded. Shrink until ascent plus
    /// descent is at most about 90% of the box's short side.
    fn fit_font_to_source_box(&mut self, base: SurfaceRect) -> Result<i32, OverlayError> {
        let target = base.w.min(base.h).max(1);
        // Leave a little air so ClearType stems do not look larger than the source ink.
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
    /// A merged paragraph (`source_lines > 1`) keeps the OCR column width and wraps.
    /// A single line shrinks its font, down to about 70%, to fit the source width.
    /// If the translation is still longer, the box grows up to 1.75 times as wide.
    /// The height is at least `source_span_h`, so a short translation still covers
    /// every original line.
    fn layout_label(
        &mut self,
        base: SurfaceRect,
        text: &str,
        source_lines: u32,
        source_span_h: i32,
        surface: draw::SurfaceSize,
    ) -> Result<(SurfaceRect, i32), OverlayError> {
        let max_w = (surface.width - base.x).max(1);
        let source_w = base.w.clamp(1, max_w);
        let mut font_px = self.fit_font_to_source_box(base)?;
        let min_h = source_span_h.max(base.h);

        if source_lines <= 1 {
            // First shrink the font so the translation fits on one line inside source_w.
            let min_font = ((font_px as f32) * 0.70).round().max(8.0) as i32;
            loop {
                let pad = label_pad(font_px);
                let natural = self.measure_single_line(text, font_px)?;
                let need_w = natural.0 + pad * 2;
                if need_w <= source_w || font_px <= min_font {
                    break;
                }
                font_px = (font_px - 1).max(min_font);
            }

            // If it is still wider than the source at the smallest font, grow the box up to 1.75 times.
            let pad = label_pad(font_px);
            let natural = self.measure_single_line(text, font_px)?;
            let need_w = (natural.0 + pad * 2).max(1);
            let expand_cap = ((source_w as f32) * 1.75).round() as i32;
            let box_w = if need_w <= source_w {
                source_w
            } else {
                need_w.min(expand_cap).min(max_w).max(source_w)
            };

            // Stay on one line so the caption cannot cover the OCR line below.
            // Wrapping and then clipping the rect used to cut glyphs off.
            let box_h = (natural.1 + pad * 2).max(font_px + pad * 2).max(min_h);
            return Ok((place_label(base, box_w, box_h, surface), font_px));
        }

        // A multi-line source keeps the OCR column width, and only the height follows the wrap.
        let pad = label_pad(font_px);
        let box_w = source_w;
        let text_h = self.measure_wrapped(text, font_px, (box_w - pad * 2).max(8))?.1;
        let box_h = (text_h + pad * 2).max(font_px + pad * 2).max(min_h);
        Ok((place_label(base, box_w, box_h, surface), font_px))
    }

    /// Width and height in pixels of the text on one unwrapped line.
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

    pub(crate) fn paint_label(&mut self, rect: SurfaceRect, text: &str, style: LabelStyle) -> Result<(), OverlayError> {
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

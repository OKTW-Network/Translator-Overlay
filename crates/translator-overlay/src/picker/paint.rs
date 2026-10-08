//! Picker veil, handles, and rubber band painted on the overlay DIB.

use crate::{
    error::OverlayError,
    gfx::{
        draw::{self, Rgba, SurfaceRect},
        text::LabelStyle,
    },
    host::OverlayHost,
    picker::HANDLE_SIZE,
};

impl OverlayHost {
    pub(crate) fn repaint_picker(&mut self) -> Result<(), OverlayError> {
        let w = self.surface_w.max(1);
        let h = self.surface_h.max(1);
        self.surface.ensure(w, h)?;

        let surface = draw::SurfaceSize::new(w, h);
        let bounds = Rgba::new(0, 200, 255, 220);
        let region_stroke = Rgba::new(80, 220, 255, 230);
        let selected_stroke = Rgba::new(255, 210, 60, 255);
        let fill = Rgba::new(80, 220, 255, 24);
        let handle = Rgba::new(255, 255, 255, 240);
        let text = Rgba::new(255, 255, 255, 255);

        let (rects, band) = {
            let Some(picker) = self.picker.as_ref() else {
                return Ok(());
            };
            (picker.live_pixel_rects(), picker.rubber_band())
        };

        {
            let buf = self
                .surface
                .pixels()
                .ok_or_else(|| OverlayError::Other("paint bitmap missing".into()))?;
            buf.fill(0);
            // UpdateLayeredWindow hit-tests per-pixel alpha before WM_NCHITTEST.
            // Pixels with alpha 0 are click-through, so the whole client needs a
            // non-zero veil or empty areas cannot start a drag.
            draw::fill_rect(buf, surface, SurfaceRect::new(0, 0, w, h), Rgba::new(6, 14, 24, 20));

            // The whole client is selectable. Inset the outline so its stroke is not clipped.
            draw::stroke_rect(buf, surface, SurfaceRect::new(2, 2, (w - 4).max(1), (h - 4).max(1)), bounds, 2);

            for (pr, selected) in rects.iter() {
                let stroke = if *selected { selected_stroke } else { region_stroke };
                let thick = if *selected { 3 } else { 2 };
                draw::fill_rect(buf, surface, *pr, fill);
                draw::stroke_rect(buf, surface, *pr, stroke, thick);
                for (hx, hy) in [(pr.x, pr.y), (pr.x + pr.w, pr.y), (pr.x, pr.y + pr.h), (pr.x + pr.w, pr.y + pr.h)] {
                    let knob = SurfaceRect::new(hx - HANDLE_SIZE / 2, hy - HANDLE_SIZE / 2, HANDLE_SIZE, HANDLE_SIZE);
                    draw::fill_rect(buf, surface, knob, handle);
                }
                draw::fill_rect(buf, surface, number_rect(*pr), Rgba::new(0, 0, 0, 160));
            }

            if let Some(band) = band {
                draw::stroke_rect(buf, surface, band, selected_stroke, 2);
            }
        }

        for (i, (pr, _selected)) in rects.into_iter().enumerate() {
            self.paint_label(number_rect(pr), &(i + 1).to_string(), LabelStyle { font_px: 13, color: text, vcenter: false })?;
        }
        Ok(())
    }
}

/// Badge in the top-left corner of a region that shows its number.
fn number_rect(region: SurfaceRect) -> SurfaceRect {
    SurfaceRect::new(region.x + 4, region.y + 4, 22, 18)
}

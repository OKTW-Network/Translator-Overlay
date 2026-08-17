//! Picker veil, handles, and rubber-band on the overlay DIB.

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
            draw::clear(buf);
            // UpdateLayeredWindow hit-tests per-pixel alpha *before* WM_NCHITTEST.
            // Alpha 0 pixels are click-through, so the whole client must have a
            // non-zero veil or empty areas cannot start a drag.
            draw::fill_rect(buf, surface, SurfaceRect { x: 0, y: 0, w, h }, Rgba::new(6, 14, 24, 20));

            // Selectable area = full client; inset so the stroke is not clipped.
            draw::stroke_rect(
                buf,
                surface,
                SurfaceRect {
                    x: 2,
                    y: 2,
                    w: (w - 4).max(1),
                    h: (h - 4).max(1),
                },
                bounds,
                2,
            );

            for (pr, selected) in rects.iter() {
                let stroke = if *selected { selected_stroke } else { region_stroke };
                let thick = if *selected { 3 } else { 2 };
                draw::fill_rect(buf, surface, pr.to_surface(), fill);
                draw::stroke_rect(buf, surface, pr.to_surface(), stroke, thick);
                for (hx, hy) in [(pr.x, pr.y), (pr.x + pr.w, pr.y), (pr.x, pr.y + pr.h), (pr.x + pr.w, pr.y + pr.h)] {
                    draw::fill_rect(
                        buf,
                        surface,
                        SurfaceRect {
                            x: hx - HANDLE_SIZE / 2,
                            y: hy - HANDLE_SIZE / 2,
                            w: HANDLE_SIZE,
                            h: HANDLE_SIZE,
                        },
                        handle,
                    );
                }
                let label_rect = SurfaceRect {
                    x: pr.x + 4,
                    y: pr.y + 4,
                    w: 22,
                    h: 18,
                };
                draw::fill_rect(buf, surface, label_rect, Rgba::new(0, 0, 0, 160));
            }

            if let Some(band) = band {
                draw::stroke_rect(buf, surface, band.to_surface(), selected_stroke, 2);
            }
        }

        for (i, (pr, _selected)) in rects.into_iter().enumerate() {
            let label = format!("{}", i + 1);
            let label_rect = SurfaceRect {
                x: pr.x + 4,
                y: pr.y + 4,
                w: 22,
                h: 18,
            };
            self.paint_label(label_rect, &label, LabelStyle { font_px: 13, color: text })?;
        }
        Ok(())
    }
}

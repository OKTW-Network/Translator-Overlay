//! Overlay appearance: display toggles, colours, caption size, translation-window font.

use rust_i18n::t;
use translator_core::{OVERLAY_FONT_PX_MAX, OVERLAY_FONT_PX_MIN, OverlayConfig, READER_FONT_PX_MAX, READER_FONT_PX_MIN};
use translator_winui::{Border, Result, StackPanel};

use crate::{
    children,
    ui::{
        state::{UiCx, UiShared, mark_dirty, send_overlay_display},
        widgets::{
            Binds,
            cards::section_header,
            column,
            controls::{ColorPopupParams, SliderNumberParams, card_color_popup, card_slider_number, card_toggle, optional_slider_row},
        },
    },
};

pub fn overlay_page(b: &mut Binds<UiShared>, cx: &UiCx) -> Result<StackPanel> {
    // Display switches apply at once, without Save.
    let display_toggle = |b: &mut Binds<UiShared>,
                          (key, header, desc): (&str, &str, &str),
                          get: fn(&OverlayConfig) -> bool,
                          set: fn(&mut OverlayConfig, bool)|
     -> Result<Border> {
        let cx = cx.clone();
        card_toggle(
            b,
            (key, header, Some(desc)),
            move |ui| get(&ui.draft.overlay),
            move |on| {
                cx.with_mut(|ui| {
                    if get(&ui.draft.overlay) != on {
                        set(&mut ui.draft.overlay, on);
                        send_overlay_display(ui);
                    }
                });
            },
        )
    };
    let display = column(4.0)?;
    children!(display;
        section_header(&t!("overlay.display"))?,
        display_toggle(
            b,
            ("ov-enabled", &t!("overlay.inplace"), &t!("overlay.inplace_desc")),
            |o| o.enabled,
            |o, on| o.enabled = on,
        )?,
        display_toggle(
            b,
            ("ov-reader", &t!("overlay.reader"), &t!("overlay.reader_desc")),
            |o| o.reader_enabled,
            |o, on| o.reader_enabled = on,
        )?,
        display_toggle(b, ("ov-hud", &t!("overlay.hud"), &t!("overlay.hud_desc")), |o| o.hud_enabled, |o, on| o.hud_enabled = on)?,
    );

    let colors = column(4.0)?;
    children!(colors;
        section_header(&t!("overlay.appearance"))?,
        card_color_popup(
            b,
            ColorPopupParams {
                key: "ov-text-color",
                header: t!("overlay.text_color"),
                placeholder: "#FFFFFFFF".into(),
            },
            |ui| (ui.text_argb_str.clone(), ui.text_color_picker_open),
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        if ui.text_argb_str != v {
                            ui.text_argb_str = v;
                            mark_dirty(ui);
                        }
                    });
                }
            },
            {
                let cx = cx.clone();
                move || {
                    cx.with_mut(|ui| {
                        ui.text_color_picker_open = !ui.text_color_picker_open;
                        if ui.text_color_picker_open {
                            ui.bg_color_picker_open = false;
                        }
                    });
                }
            },
        )?,
        card_color_popup(
            b,
            ColorPopupParams {
                key: "ov-bg-color",
                header: t!("overlay.bg_color"),
                placeholder: "#C8000000".into(),
            },
            |ui| (ui.bg_argb_str.clone(), ui.bg_color_picker_open),
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        if ui.bg_argb_str != v {
                            ui.bg_argb_str = v;
                            mark_dirty(ui);
                        }
                    });
                }
            },
            {
                let cx = cx.clone();
                move || {
                    cx.with_mut(|ui| {
                        ui.bg_color_picker_open = !ui.bg_color_picker_open;
                        if ui.bg_color_picker_open {
                            ui.text_color_picker_open = false;
                        }
                    });
                }
            },
        )?,
        optional_slider_row(
            b,
            SliderNumberParams {
                key: "ov-caption-font",
                header: t!("overlay.caption_font"),
                description: Some(t!("overlay.caption_font_desc")),
                min: f64::from(OVERLAY_FONT_PX_MIN),
                max: f64::from(OVERLAY_FONT_PX_MAX),
                step: 1.0,
            },
            |ui| (ui.overlay_font_val, ui.overlay_font_enabled, true),
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        let px = v.round().clamp(f64::from(OVERLAY_FONT_PX_MIN), f64::from(OVERLAY_FONT_PX_MAX));
                        if (ui.overlay_font_val - px).abs() > f64::EPSILON {
                            ui.overlay_font_val = px;
                            mark_dirty(ui);
                        }
                    });
                }
            },
            {
                let cx = cx.clone();
                move |on| {
                    cx.with_mut(|ui| {
                        if ui.overlay_font_enabled != on {
                            ui.overlay_font_enabled = on;
                            mark_dirty(ui);
                        }
                    });
                }
            },
        )?,
        card_slider_number(
            b,
            SliderNumberParams {
                key: "ov-reader-font",
                header: t!("overlay.font"),
                description: Some(t!("overlay.font_desc")),
                min: f64::from(READER_FONT_PX_MIN),
                max: f64::from(READER_FONT_PX_MAX),
                step: 1.0,
            },
            |ui| (f64::from(ui.draft.overlay.reader_font_px), true),
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        let px = v.round().clamp(f64::from(READER_FONT_PX_MIN), f64::from(READER_FONT_PX_MAX)) as u32;
                        if ui.draft.overlay.reader_font_px != px {
                            ui.draft.overlay.reader_font_px = px;
                            mark_dirty(ui);
                        }
                    });
                }
            },
        )?
        .root,
    );

    let body = column(8.0)?;
    children!(body; display, colors);
    Ok(body)
}

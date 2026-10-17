//! Overlay appearance: display toggles, colours, caption size, translation-window font.

use std::sync::Arc;

use parking_lot::Mutex;
use rust_i18n::t;
use translator_core::{OVERLAY_FONT_PX_MAX, OVERLAY_FONT_PX_MIN, READER_FONT_PX_MAX, READER_FONT_PX_MIN};
use windows_reactor::{ChildrenControl, LocalSender, StackPanel, View};

use crate::ui::{
    cards::section_header,
    chrome::settings_page_shell,
    controls::{
        ColorPopupParams, OptionalSliderParams, SliderNumberParams, card_color_popup, card_slider_number, card_toggle, optional_slider_row,
    },
    shared::{AppMsg, ChromeSnap, UiCx, UiShared, mark_dirty, send_overlay_display},
};

pub fn overlay_page(shared: &Arc<Mutex<UiShared>>, chrome: &ChromeSnap, bump: &LocalSender<AppMsg>) -> View {
    let cx = UiCx::new(shared, bump);
    let (overlay, text_argb_str, bg_argb_str, text_color_picker_open, bg_color_picker_open, overlay_font_val, overlay_font_enabled) = {
        let ui = shared.lock();
        (
            ui.draft.overlay.clone(),
            ui.text_argb_str.clone(),
            ui.bg_argb_str.clone(),
            ui.text_color_picker_open,
            ui.bg_color_picker_open,
            ui.overlay_font_val,
            ui.overlay_font_enabled,
        )
    };

    let display = StackPanel::new().spacing(4.0).children((
        section_header(t!("overlay.display")),
        card_toggle("ov-enabled", t!("overlay.inplace"), Some(&t!("overlay.inplace_desc")), overlay.enabled, {
            let cx = cx.clone();
            move |on| {
                cx.with_mut(|ui| {
                    if ui.draft.overlay.enabled != on {
                        let reader = ui.draft.overlay.reader_enabled;
                        let hud = ui.draft.overlay.hud_enabled;
                        send_overlay_display(ui, on, reader, hud);
                    }
                });
            }
        }),
        card_toggle("ov-reader", t!("overlay.reader"), Some(&t!("overlay.reader_desc")), overlay.reader_enabled, {
            let cx = cx.clone();
            move |on| {
                cx.with_mut(|ui| {
                    if ui.draft.overlay.reader_enabled != on {
                        let enabled = ui.draft.overlay.enabled;
                        let hud = ui.draft.overlay.hud_enabled;
                        send_overlay_display(ui, enabled, on, hud);
                    }
                });
            }
        }),
        card_toggle("ov-hud", t!("overlay.hud"), Some(&t!("overlay.hud_desc")), overlay.hud_enabled, {
            let cx = cx.clone();
            move |on| {
                cx.with_mut(|ui| {
                    if ui.draft.overlay.hud_enabled != on {
                        let enabled = ui.draft.overlay.enabled;
                        let reader = ui.draft.overlay.reader_enabled;
                        send_overlay_display(ui, enabled, reader, on);
                    }
                });
            }
        }),
    ));

    let colors = StackPanel::new().spacing(4.0).children((
        section_header(t!("overlay.appearance")),
        card_color_popup(
            ColorPopupParams {
                key: "ov-text-color",
                header: t!("overlay.text_color"),
                hex: text_argb_str.clone(),
                open: text_color_picker_open,
                placeholder: "#FFFFFFFF".into(),
            },
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
        ),
        card_color_popup(
            ColorPopupParams {
                key: "ov-bg-color",
                header: t!("overlay.bg_color"),
                hex: bg_argb_str.clone(),
                open: bg_color_picker_open,
                placeholder: "#C8000000".into(),
            },
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
        ),
        optional_slider_row(
            OptionalSliderParams {
                key: "ov-caption-font",
                header: t!("overlay.caption_font"),
                description: t!("overlay.caption_font_desc"),
                value: overlay_font_val,
                enabled: overlay_font_enabled,
                applicable: true,
                min: f64::from(OVERLAY_FONT_PX_MIN),
                max: f64::from(OVERLAY_FONT_PX_MAX),
                step: 1.0,
            },
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
        ),
        card_slider_number(
            SliderNumberParams {
                key: "ov-reader-font",
                header: t!("overlay.font"),
                description: Some(t!("overlay.font_desc")),
                value: f64::from(overlay.reader_font_px),
                min: f64::from(READER_FONT_PX_MIN),
                max: f64::from(READER_FONT_PX_MAX),
                step: 1.0,
            },
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
        ),
    ));

    settings_page_shell(shared, chrome, bump, StackPanel::new().spacing(8.0).children((display, colors)))
}

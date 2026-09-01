//! Overlay appearance and display settings.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_core::{READER_FONT_PX_MAX, READER_FONT_PX_MIN};
use windows_reactor::{StackPanel, TextStyleExt, ThemeRef, Updater, text_block, vstack};

use crate::ui::{
    chrome::{section_header, settings_page_shell},
    controls::{ColorPopupParams, SliderNumberParams, card_color_popup, card_slider_number, card_toggle},
    shared::{ChromeSnap, UiCx, UiShared, mark_dirty, parts_to_argb_u32, send_overlay_display},
};

pub fn overlay_page(shared: &Arc<Mutex<UiShared>>, chrome: &ChromeSnap, bump: &Updater<u32>) -> StackPanel {
    let cx = UiCx::new(shared, bump);
    let (overlay, text_argb_str, bg_argb_str, text_color_picker_open, bg_color_picker_open) = {
        let ui = shared.lock();
        (ui.draft.overlay.clone(), ui.text_argb_str.clone(), ui.bg_argb_str.clone(), ui.text_color_picker_open, ui.bg_color_picker_open)
    };

    let display = vstack((
        section_header("Display"),
        card_toggle("ov-enabled", "In-place overlay", Some("Draw translations on the target window (click-through)."), overlay.enabled, {
            let cx = cx.clone();
            move |on| {
                cx.with_mut(|ui| {
                    if ui.draft.overlay.enabled != on {
                        let reader = ui.draft.overlay.reader_enabled;
                        send_overlay_display(ui, on, reader);
                    }
                });
            }
        }),
        card_toggle(
            "ov-reader",
            "Translation window",
            Some("Borderless always-on-top window. Drag to move, resize from the edges. Hide with this switch."),
            overlay.reader_enabled,
            {
                let cx = cx.clone();
                move |on| {
                    cx.with_mut(|ui| {
                        if ui.draft.overlay.reader_enabled != on {
                            let enabled = ui.draft.overlay.enabled;
                            send_overlay_display(ui, enabled, on);
                        }
                    });
                }
            },
        ),
    ))
    .spacing(4.0);

    let colors = vstack((
        section_header("Appearance"),
        card_color_popup(
            ColorPopupParams {
                key: "ov-text-color",
                header: "Text color".into(),
                description: Some("Click the swatch to pick colour and opacity, or type ARGB hex.".into()),
                hex: text_argb_str.clone(),
                open: text_color_picker_open,
                alpha_enabled: true,
                placeholder: "FFFFFFFF".into(),
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
                move |(a, r, g, b)| {
                    cx.with_mut(|ui| {
                        let v = parts_to_argb_u32(a, r, g, b);
                        let hex = format!("{v:08X}");
                        if ui.text_argb_str != hex {
                            ui.text_argb_str = hex;
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
                header: "Background color".into(),
                description: Some("Click the swatch to pick colour and opacity, or type ARGB hex.".into()),
                hex: bg_argb_str.clone(),
                open: bg_color_picker_open,
                alpha_enabled: true,
                placeholder: "C8000000".into(),
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
                move |(a, r, g, b)| {
                    cx.with_mut(|ui| {
                        let v = parts_to_argb_u32(a, r, g, b);
                        let hex = format!("{v:08X}");
                        if ui.bg_argb_str != hex {
                            ui.bg_argb_str = hex;
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
        card_slider_number(
            SliderNumberParams {
                key: "ov-reader-font",
                header: "Font size".into(),
                description: Some("Segoe UI size for the translation window. Overlay captions still fit the source text.".into()),
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
    ))
    .spacing(4.0);

    let notes = vstack((
        section_header("Notes"),
        text_block("The in-place overlay is click-through and follows the target window only while it is in the foreground. The translation window is borderless and semi-transparent, uses the same colours and typeface, and stays visible independently.")
            .font_size(12.0)
            .foreground(ThemeRef::SecondaryText)
            .wrap(),
    ))
    .spacing(4.0);

    settings_page_shell(shared, chrome, bump, vstack((display, colors, notes)).spacing(8.0))
}

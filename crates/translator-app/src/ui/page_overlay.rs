//! Overlay appearance settings.

use std::sync::Arc;

use parking_lot::Mutex;
use windows_reactor::*;

use crate::ui::{
    chrome::{section_header, settings_page_shell},
    controls::{ColorPopupParams, card_color_popup},
    shared::{Snapshot, UiCx, UiShared, mark_dirty, parts_to_argb_u32},
};

pub fn overlay_page(shared: &Arc<Mutex<UiShared>>, snap: &Snapshot, bump: &Updater<u32>) -> Element {
    let cx = UiCx::new(shared, bump);

    let colors = vstack((
        section_header("Appearance"),
        card_color_popup(
            ColorPopupParams {
                key: "ov-text-color",
                header: "Text color".into(),
                description: Some("Click the swatch to pick colour and opacity, or type ARGB hex.".into()),
                hex: snap.text_argb_str.clone(),
                open: snap.text_color_picker_open,
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
                hex: snap.bg_argb_str.clone(),
                open: snap.bg_color_picker_open,
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
    ))
    .spacing(4.0);

    let notes = vstack((
        section_header("Notes"),
        text_block("Click-through overlay. Follows the target window and shows only while it is in the foreground.")
            .font_size(12.0)
            .foreground(ThemeRef::SecondaryText)
            .wrap(),
    ))
    .spacing(4.0);

    settings_page_shell(shared, snap, bump, vstack((colors, notes)).spacing(8.0).into())
}

//! Overlay appearance settings.

use std::sync::{Arc, Mutex};

use windows_reactor::*;

use super::chrome::{section_header, settings_page_shell};
use super::controls::{ColorPopupParams, card_color_popup};
use super::shared::{Snapshot, UiShared, mark_dirty, parts_to_argb_u32};

pub fn overlay_page(
    shared: &Arc<Mutex<UiShared>>,
    snap: &Snapshot,
    bump: &Updater<u32>,
) -> Element {
    let s_t = Arc::clone(shared);
    let s_t2 = Arc::clone(shared);
    let s_t3 = Arc::clone(shared);
    let s_b = Arc::clone(shared);
    let s_b2 = Arc::clone(shared);
    let s_b3 = Arc::clone(shared);
    let bump_t = bump.clone();
    let bump_tp = bump.clone();
    let bump_to = bump.clone();
    let bump_b = bump.clone();
    let bump_bp = bump.clone();
    let bump_bo = bump.clone();

    let colors = vstack((
        section_header("Appearance"),
        card_color_popup(
            ColorPopupParams {
                key: "ov-text-color",
                header: "Text color".into(),
                description: Some(
                    "Click the swatch to pick colour and opacity, or type ARGB hex.".into(),
                ),
                hex: snap.text_argb_str.clone(),
                open: snap.text_color_picker_open,
                alpha_enabled: true,
                placeholder: "FFFFFFFF".into(),
            },
            move |v| {
                if let Ok(mut ui) = s_t2.lock()
                    && ui.text_argb_str != v
                {
                    ui.text_argb_str = v;
                    mark_dirty(&mut ui);
                }
                bump_t.call(|n| n.wrapping_add(1));
            },
            move |(a, r, g, b)| {
                if let Ok(mut ui) = s_t.lock() {
                    let v = parts_to_argb_u32(a, r, g, b);
                    let hex = format!("{v:08X}");
                    if ui.text_argb_str != hex {
                        ui.text_argb_str = hex;
                        mark_dirty(&mut ui);
                    }
                }
                bump_tp.call(|n| n.wrapping_add(1));
            },
            move || {
                if let Ok(mut ui) = s_t3.lock() {
                    ui.text_color_picker_open = !ui.text_color_picker_open;
                    if ui.text_color_picker_open {
                        ui.bg_color_picker_open = false;
                    }
                }
                bump_to.call(|n| n.wrapping_add(1));
            },
        ),
        card_color_popup(
            ColorPopupParams {
                key: "ov-bg-color",
                header: "Background color".into(),
                description: Some(
                    "Click the swatch to pick colour and opacity, or type ARGB hex.".into(),
                ),
                hex: snap.bg_argb_str.clone(),
                open: snap.bg_color_picker_open,
                alpha_enabled: true,
                placeholder: "C8000000".into(),
            },
            move |v| {
                if let Ok(mut ui) = s_b2.lock()
                    && ui.bg_argb_str != v
                {
                    ui.bg_argb_str = v;
                    mark_dirty(&mut ui);
                }
                bump_b.call(|n| n.wrapping_add(1));
            },
            move |(a, r, g, b)| {
                if let Ok(mut ui) = s_b.lock() {
                    let v = parts_to_argb_u32(a, r, g, b);
                    let hex = format!("{v:08X}");
                    if ui.bg_argb_str != hex {
                        ui.bg_argb_str = hex;
                        mark_dirty(&mut ui);
                    }
                }
                bump_bp.call(|n| n.wrapping_add(1));
            },
            move || {
                if let Ok(mut ui) = s_b3.lock() {
                    ui.bg_color_picker_open = !ui.bg_color_picker_open;
                    if ui.bg_color_picker_open {
                        ui.text_color_picker_open = false;
                    }
                }
                bump_bo.call(|n| n.wrapping_add(1));
            },
        ),
    ))
    .spacing(4.0);

    let notes = vstack((
        section_header("Notes"),
        text_block(
            "Click-through overlay. Follows the target window and shows only while it is in the foreground.",
        )
        .font_size(12.0)
        .foreground(ThemeRef::SecondaryText)
        .wrap(),
    ))
    .spacing(4.0);

    settings_page_shell(
        shared,
        snap,
        bump,
        vstack((colors, notes)).spacing(8.0).into(),
    )
}

//! Overlay, translation window, and control bar: one section each with its switch, colours, and text size.

use rust_i18n::t;
use translator_core::{OVERLAY_FONT_PX_MAX, OVERLAY_FONT_PX_MIN, OverlayConfig, READER_FONT_PX_MAX, READER_FONT_PX_MIN, format_argb_hex};
use translator_winui::{Border, Result, StackPanel};

use crate::{
    children,
    ui::{
        state::{COLOR_SLOTS, UiCx, UiShared, mark_dirty, send_overlay_display},
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
                          (key, desc): (&str, &str),
                          get: fn(&OverlayConfig) -> bool,
                          set: fn(&mut OverlayConfig, bool)|
     -> Result<Border> {
        let cx = cx.clone();
        card_toggle(
            b,
            (key, &t!("overlay.display"), Some(desc)),
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
    // `i` indexes `COLOR_SLOTS`.
    let color_card = |b: &mut Binds<UiShared>, i: usize| -> Result<Border> {
        let slot = &COLOR_SLOTS[i];
        let key = slot.key;
        card_color_popup(
            b,
            ColorPopupParams {
                key,
                header: if slot.background {
                    t!("overlay.bg_color")
                } else {
                    t!("overlay.text_color")
                },
                placeholder: format_argb_hex(*(slot.field)(&mut OverlayConfig::default())).into(),
            },
            move |ui| (ui.color_hex[i].clone(), ui.open_color_picker == Some(key)),
            {
                let cx = cx.clone();
                move |v| {
                    cx.with_mut(|ui| {
                        if ui.color_hex[i] != v {
                            ui.color_hex[i] = v;
                            mark_dirty(ui);
                        }
                    });
                }
            },
            {
                let cx = cx.clone();
                move || cx.with_mut(|ui| ui.open_color_picker = if ui.open_color_picker == Some(key) { None } else { Some(key) })
            },
        )
    };

    let overlay = column(4.0)?;
    children!(overlay;
        section_header(&t!("overlay.inplace"))?,
        display_toggle(b, ("ov-enabled", &t!("overlay.inplace_desc")), |o| o.enabled, |o, on| o.enabled = on)?,
        color_card(b, 0)?,
        color_card(b, 1)?,
        optional_slider_row(
            b,
            SliderNumberParams {
                key: "ov-caption-font",
                header: t!("overlay.font"),
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
    );

    let reader = column(4.0)?;
    children!(reader;
        section_header(&t!("overlay.reader"))?,
        display_toggle(b, ("ov-reader", &t!("overlay.reader_desc")), |o| o.reader_enabled, |o, on| o.reader_enabled = on)?,
        color_card(b, 2)?,
        color_card(b, 3)?,
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

    let hud = column(4.0)?;
    children!(hud;
        section_header(&t!("overlay.hud"))?,
        display_toggle(b, ("ov-hud", &t!("overlay.hud_desc")), |o| o.hud_enabled, |o, on| o.hud_enabled = on)?,
        color_card(b, 4)?,
        color_card(b, 5)?,
    );

    let body = column(8.0)?;
    children!(body; overlay, reader, hud);
    Ok(body)
}

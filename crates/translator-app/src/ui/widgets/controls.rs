//! Reusable setting-row controls (text, toggle, slider, optional toggle+slider).

use std::borrow::Cow;

use rust_i18n::t;
use translator_core::{format_argb_hex, parse_argb_hex};
use translator_winui::{
    Border, Button, Color, HorizontalAlignment, Layout, ProgressRing, Result, TextBlock, TextWrapping, VerticalAlignment, corners,
    solid_brush, uniform,
};

use crate::{
    children,
    ui::{
        state::UiShared,
        widgets::{
            Binds, ButtonKind, Prop, Theme, button_style,
            cards::{Card, settings_card, settings_card_with_below},
            click,
            fields::{ColorField, NumberField, PasswordField, Radio, SliderNumber, SuggestField, TextField, Toggle},
            font_icon, hs, icon_button, row, text, themed_border, ui,
        },
    },
};

/// Radio button in a group; `width` pins it, otherwise it hugs the left.
pub fn radio(group: &str, label: &str, width: Option<f64>, on: impl Fn() + Send + 'static) -> Result<Radio> {
    let r = Radio::new(group, label, on)?;
    r.rb.valign(VerticalAlignment::Center)?;
    match width {
        Some(w) => {
            r.rb.min_width(w)?.width(w)?;
        }
        None => {
            r.rb.halign(HorizontalAlignment::Left)?;
        }
    }
    Ok(r)
}

/// Wrapping tooltip content, 280px wide at most.
pub fn wrap_tooltip(s: &str) -> Result<TextBlock> {
    let tb = text(s)?;
    tb.SetTextWrapping(TextWrapping::WrapWholeWords)?;
    tb.max_width(280.0)?;
    Ok(tb)
}

pub fn compact_toggle(on_toggled: impl Fn(bool) + Send + 'static) -> Result<Toggle> {
    let t = Toggle::new(on_toggled)?;
    t.ts.width(40.0)?
        .min_width(40.0)?
        .max_width(44.0)?
        .halign(HorizontalAlignment::Right)?
        .valign(VerticalAlignment::Center)?;
    Ok(t)
}

/// Label + single-line text box as a settings card.
pub fn card_text(
    b: &mut Binds<UiShared>,
    (key, header, description): (&str, &str, Option<&str>),
    placeholder: &str,
    get: impl Fn(&UiShared) -> String + 'static,
    on_changed: impl Fn(String) + Send + 'static,
) -> Result<Border> {
    let field = TextField::new(placeholder, on_changed)?;
    field.tb.min_width(200.0)?.width(280.0)?.valign(VerticalAlignment::Center)?;
    let card = settings_card(key, header, description, &field.tb)?;
    b.add(move |ui, _| field.set(&get(ui)));
    Ok(card.root)
}

pub struct ModelSuggestParams {
    pub key: &'static str,
    pub header: Cow<'static, str>,
    pub placeholder: Cow<'static, str>,
}

/// Label + AutoSuggestBox + refresh as a settings card. `get` returns (description, value, suggestions, loading).
pub fn card_model_suggest(
    b: &mut Binds<UiShared>,
    p: ModelSuggestParams,
    get: impl Fn(&UiShared) -> (Cow<'static, str>, String, Vec<String>, bool) + 'static,
    on_changed: impl Fn(String) + Clone + Send + 'static,
    on_refresh: impl Fn() + Send + 'static,
) -> Result<Border> {
    let mut suggest = SuggestField::new(&p.placeholder, on_changed)?;
    suggest.asb.min_width(200.0)?.width(280.0)?.valign(VerticalAlignment::Center)?;

    let refresh = icon_button("\u{E72C}", on_refresh)?;
    let ring = ProgressRing::new()?;
    ring.SetIsIndeterminate(true)?;
    ring.SetIsActive(true)?;
    ring.width(20.0)?.height(20.0)?;
    let icon = font_icon("\u{E72C}")?;

    let controls = row(8.0)?;
    children!(controls; suggest.asb, refresh);
    let card = settings_card(p.key, &p.header, Some(" "), &controls)?;
    let desc = card.desc;
    let mut loading_prop = Prop::default();
    let mut desc_prop = Prop::default();
    b.add(move |ui, _| {
        let (description, value, suggestions, loading) = get(ui);
        suggest.set(&value, suggestions)?;
        if let Some(desc) = &desc {
            desc_prop.set(description.into_owned(), |d| desc.SetText(&hs(d)))?;
        }
        loading_prop.set(loading, |&l| {
            refresh.SetIsEnabled(!l)?;
            if l {
                refresh.SetContent(&ring)?;
                refresh.tooltip(&t!("api.loading_models"))?;
            } else {
                refresh.SetContent(&icon)?;
                refresh.tooltip(&t!("api.reload_models"))?;
            }
            Ok(())
        })
    });
    Ok(card.root)
}

/// Label + password box + show/hide toggle as a settings card.
pub fn card_password(
    b: &mut Binds<UiShared>,
    (key, header, description): (&str, &str, Option<&str>),
    get: impl Fn(&UiShared) -> (String, bool) + 'static,
    on_changed: impl Fn(String) + Send + 'static,
    on_reveal_toggled: impl Fn() + Send + 'static,
) -> Result<Border> {
    let mut field = PasswordField::new(on_changed)?;
    field.pb.min_width(180.0)?.width(240.0)?.valign(VerticalAlignment::Center)?;
    let reveal = icon_button("\u{E890}", on_reveal_toggled)?;
    let controls = row(8.0)?;
    children!(controls; field.pb, reveal);
    let card = settings_card(key, header, description, &controls)?;
    let mut revealed_prop = Prop::default();
    b.add(move |ui, _| {
        let (value, revealed) = get(ui);
        field.set(&value, revealed)?;
        revealed_prop.set(revealed, |&r| {
            reveal.SetContent(&font_icon(if r { "\u{E7B3}" } else { "\u{E890}" })?)?;
            reveal.tooltip(&if r { t!("api.hide_key") } else { t!("api.show_key") })?;
            Ok(())
        })
    });
    Ok(card.root)
}

/// Label + toggle as a settings card (switch flush-right).
pub fn card_toggle(
    b: &mut Binds<UiShared>,
    (key, header, description): (&str, &str, Option<&str>),
    get: impl Fn(&UiShared) -> bool + 'static,
    on_toggled: impl Fn(bool) + Send + 'static,
) -> Result<Border> {
    let mut toggle = compact_toggle(on_toggled)?;
    let card = settings_card(key, header, description, &toggle.ts)?;
    b.add(move |ui, _| toggle.set(get(ui), true));
    Ok(card.root)
}

/// Params for the numeric rows: [`card_slider_number`], [`optional_slider_row`], [`optional_number_row`].
pub struct SliderNumberParams {
    pub key: &'static str,
    pub header: Cow<'static, str>,
    pub description: Option<Cow<'static, str>>,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

/// Snap `v` to the nearest step within [min, max].
pub fn quantize_to_step(v: f64, min: f64, max: f64, step: f64) -> f64 {
    const SCALE: f64 = 1_000_000.0;
    let v = v.clamp(min, max);
    if !(step.is_finite() && step > 0.0) {
        return v;
    }
    let min_i = (min * SCALE).round() as i64;
    let step_i = (step * SCALE).round() as i64;
    if step_i == 0 {
        return v;
    }
    let n = ((v - min) / step).round() as i64;
    let out = (min_i.saturating_add(n.saturating_mul(step_i))) as f64 / SCALE;
    out.clamp(min, max)
}

/// Slider and NumberBox bound to one value snapped to `step`. `widths` is (slider, slider minimum, box).
fn slider_and_box(
    (min, max, step): (f64, f64, f64),
    widths: (f64, f64, f64),
    on_changed: impl Fn(f64) + Clone + Send + 'static,
) -> Result<SliderNumber> {
    let pair = SliderNumber::new((min, max, step), move |v| on_changed(quantize_to_step(v, min, max, step)))?;
    pair.slider
        .width(widths.0)?
        .min_width(widths.1)?
        .valign(VerticalAlignment::Center)?;
    pair.number.width(widths.2)?.valign(VerticalAlignment::Center)?;
    Ok(pair)
}

/// Slider + NumberBox on one row (labels left, controls flush-right). `get` returns (value, enabled).
pub fn card_slider_number(
    b: &mut Binds<UiShared>,
    p: SliderNumberParams,
    get: impl Fn(&UiShared) -> (f64, bool) + 'static,
    on_changed: impl Fn(f64) + Clone + Send + 'static,
) -> Result<Card> {
    let (min, max, step) = (p.min, p.max, p.step);
    let mut pair = slider_and_box((min, max, step), (180.0, 140.0, 100.0), on_changed)?;
    let controls = row(12.0)?;
    children!(controls; pair.slider, pair.number);
    let card = settings_card(p.key, &p.header, p.description.as_deref(), &controls)?;
    b.add(move |ui, _| {
        let (value, enabled) = get(ui);
        pair.set(quantize_to_step(value, min, max, step), enabled)
    });
    Ok(card)
}

/// Optional number: Slider + NumberBox; Off omits from config (value kept for restore).
/// `get` returns (value, enabled, applicable).
pub fn optional_slider_row(
    b: &mut Binds<UiShared>,
    p: SliderNumberParams,
    get: impl Fn(&UiShared) -> (f64, bool, bool) + 'static,
    on_value: impl Fn(f64) + Clone + Send + 'static,
    on_enabled: impl Fn(bool) + Send + 'static,
) -> Result<Border> {
    let (min, max, step) = (p.min, p.max, p.step);
    let mut pair = slider_and_box((min, max, step), (160.0, 120.0, 88.0), on_value)?;
    let mut toggle = compact_toggle(on_enabled)?;
    let controls = row(12.0)?;
    controls.valign(VerticalAlignment::Center)?.halign(HorizontalAlignment::Right)?;
    children!(controls; pair.slider, pair.number, toggle.ts);
    let card = settings_card(p.key, &p.header, p.description.as_deref(), &controls)?;
    b.add(move |ui, _| {
        let (value, enabled, applicable) = get(ui);
        pair.set(quantize_to_step(value, min, max, step), applicable && enabled)?;
        toggle.set(enabled, applicable)
    });
    Ok(card.root)
}

/// Optional integer/float with master toggle and no slider (e.g. max_tokens).
/// `get` returns (value, enabled, applicable).
pub fn optional_number_row(
    b: &mut Binds<UiShared>,
    p: SliderNumberParams,
    get: impl Fn(&UiShared) -> (f64, bool, bool) + 'static,
    on_value: impl Fn(f64) + Send + 'static,
    on_enabled: impl Fn(bool) + Send + 'static,
) -> Result<Border> {
    let (min, max, step) = (p.min, p.max, p.step);
    let mut number = NumberField::new((min, max), move |v| on_value(quantize_to_step(v, min, max, step)))?;
    number.number.width(120.0)?.valign(VerticalAlignment::Center)?;
    let mut toggle = compact_toggle(on_enabled)?;
    let controls = row(12.0)?;
    controls.valign(VerticalAlignment::Center)?.halign(HorizontalAlignment::Right)?;
    children!(controls; number.number, toggle.ts);
    let card = settings_card(p.key, &p.header, p.description.as_deref(), &controls)?;
    b.add(move |ui, _| {
        let (value, enabled, applicable) = get(ui);
        number.set(quantize_to_step(value, min, max, step), applicable && enabled)?;
        toggle.set(enabled, applicable)
    });
    Ok(card.root)
}

/// Params for [`optional_text_row`].
pub struct OptionalTextParams {
    pub key: &'static str,
    pub header: Cow<'static, str>,
    pub description: Option<Cow<'static, str>>,
    pub placeholder: Cow<'static, str>,
}

/// Optional string field with master toggle (e.g. reasoning_effort). `get` returns (text, enabled).
pub fn optional_text_row(
    b: &mut Binds<UiShared>,
    p: OptionalTextParams,
    get: impl Fn(&UiShared) -> (String, bool) + 'static,
    on_text: impl Fn(String) + Send + 'static,
    on_enabled: impl Fn(bool) + Send + 'static,
) -> Result<Border> {
    let field = TextField::new(&p.placeholder, on_text)?;
    field.tb.min_width(140.0)?.width(180.0)?.valign(VerticalAlignment::Center)?;
    let mut toggle = compact_toggle(on_enabled)?;
    let controls = row(12.0)?;
    controls.valign(VerticalAlignment::Center)?.halign(HorizontalAlignment::Right)?;
    children!(controls; field.tb, toggle.ts);
    let card = settings_card(p.key, &p.header, p.description.as_deref(), &controls)?;
    let mut enabled_prop = Prop::default();
    b.add(move |ui, _| {
        let (value, enabled) = get(ui);
        field.set(&value)?;
        let tb = &field.tb;
        enabled_prop.set(enabled, |&e| tb.SetIsEnabled(e))?;
        toggle.set(enabled, true)
    });
    Ok(card.root)
}

/// Params for [`card_color_popup`].
pub struct ColorPopupParams {
    pub key: &'static str,
    pub header: Cow<'static, str>,
    pub placeholder: Cow<'static, str>,
}

/// Compact color row: swatch + hex; ColorPicker expands under the same header row.
///
/// Not a Flyout (ThemeShadow over Mica paints as a solid block). Subtle Button
/// keeps native PointerOver; the 32×32 chip is content so colour stays full size.
/// `get` returns (hex, open).
pub fn card_color_popup(
    b: &mut Binds<UiShared>,
    p: ColorPopupParams,
    get: impl Fn(&UiShared) -> (String, bool) + 'static,
    on_hex_changed: impl Fn(String) + Clone + Send + 'static,
    on_toggle_open: impl Fn() + Send + 'static,
) -> Result<Border> {
    let chip = themed_border(None, Some(Theme::CardStroke))?;
    chip.width(32.0)?.height(32.0)?;
    chip.SetCornerRadius(corners(4.0))?;
    chip.SetBorderThickness(uniform(1.0))?;
    let swatch = Button::new()?;
    button_style(&swatch, ButtonKind::Subtle)?;
    swatch.min_width(0.0)?.min_height(0.0)?.valign(VerticalAlignment::Center)?;
    swatch.SetContent(&chip)?;
    click(&swatch, on_toggle_open)?;

    let hex = TextField::new(&p.placeholder, on_hex_changed.clone())?;
    hex.tb.width(120.0)?.valign(VerticalAlignment::Center)?;
    let controls = row(8.0)?;
    controls.valign(VerticalAlignment::Center)?;
    children!(controls; swatch, hex.tb);

    let picker = ColorField::new(move |[a, r, g, b]| on_hex_changed(format_argb_hex(u32::from_be_bytes([a, r, g, b]))))?;
    picker.picker.visible(false)?;
    let card = settings_card_with_below(p.key, &p.header, Some(&t!("overlay.color_swatch_desc")), &controls, Some(&ui(&picker.picker)?))?;
    let mut chip_prop = Prop::default();
    let mut open_prop = Prop::default();
    b.add(move |ui, _| {
        let (value, open) = get(ui);
        hex.set(&value)?;
        let argb = parse_argb_hex(&value).unwrap_or(0xFF00_0000).to_be_bytes();
        // Opaque RGB so low-alpha colours stay visible on the card.
        chip_prop.set(argb, |&[_, r, g, b]| chip.SetBackground(&solid_brush(Color { A: 255, R: r, G: g, B: b })?))?;
        let picker_el = &picker.picker;
        let swatch = &swatch;
        open_prop.set(open, |&o| {
            picker_el.visible(o)?;
            swatch.tooltip(&if o { t!("overlay.close_picker") } else { t!("overlay.open_picker") })?;
            Ok(())
        })?;
        if open {
            picker.set(argb)?;
        }
        Ok(())
    });
    Ok(card.root)
}

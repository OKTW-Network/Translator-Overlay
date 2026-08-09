//! Reusable setting-row controls (text, toggle, slider, optional toggle+slider).

use windows_reactor::*;

use crate::ui::{
    chrome::{settings_card, settings_card_stack, settings_row},
    shared::{argb_u32_to_parts, parse_hex_u32},
};

/// Compact right-edge ToggleSwitch (Windows Settings style).
///
/// Default ToggleSwitch is wide: On/Off content presenters + theme MinWidth
/// leave empty space to the right of the track, so the knob looks off-edge.
/// Force a track-sized width and never set On/Off labels.
fn compact_toggle(is_on: bool, on_toggled: impl Fn(bool) + 'static) -> ToggleSwitch {
    let mut sw = ToggleSwitch::new(is_on).on_toggled(on_toggled);
    // Track-only size (~40); max keeps theme from expanding empty content columns.
    sw.modifiers.width = Some(40.0);
    sw.modifiers.min_width = Some(40.0);
    sw.modifiers.max_width = Some(44.0);
    sw.modifiers.horizontal_alignment = Some(HorizontalAlignment::Right);
    sw.modifiers.vertical_alignment = Some(VerticalAlignment::Center);
    sw
}

/// Label + single-line text box as a settings card.
pub fn card_text(
    key: &str,
    header: impl Into<String>,
    description: Option<&str>,
    value: String,
    placeholder: impl Into<String>,
    on_changed: impl Fn(String) + 'static,
) -> Element {
    let mut tb = text_box(value).placeholder_text(placeholder).on_text_changed(on_changed);
    tb.modifiers.min_width = Some(200.0);
    tb.modifiers.width = Some(280.0);
    tb.modifiers.vertical_alignment = Some(VerticalAlignment::Center);
    settings_card(key, header, description, tb)
}

/// Label + password box + show/hide toggle as a settings card.
///
/// WinUI Peek reveal button only appears under narrow focus/width conditions and
/// often never shows. We use Hidden/Visible + a custom eye button instead.
pub fn card_password(
    key: &str,
    header: impl Into<String>,
    description: Option<&str>,
    value: String,
    revealed: bool,
    on_changed: impl Fn(String) + 'static,
    on_reveal_toggled: impl Fn() + 'static,
) -> Element {
    let mut pb = PasswordBox::new()
        .value(value)
        .reveal_button_enabled(false)
        .password_reveal_mode(if revealed {
            PasswordRevealMode::Visible
        } else {
            PasswordRevealMode::Hidden
        })
        .on_password_changed(on_changed);
    pb.modifiers.min_width = Some(180.0);
    pb.modifiers.width = Some(240.0);
    pb.modifiers.vertical_alignment = Some(VerticalAlignment::Center);

    // Segoe MDL2: View (E890) / Hide (E7B3) — show-password affordance.
    let glyph = if revealed { "\u{E7B3}" } else { "\u{E890}" };
    let tip = if revealed { "Hide key" } else { "Show key" };
    let reveal = button(glyph)
        .font_family("Segoe MDL2 Assets")
        .font_size(14.0)
        .subtle()
        .padding(Thickness::uniform(8.0))
        .min_width(36.0)
        .min_height(36.0)
        .vertical_alignment(VerticalAlignment::Center)
        .tooltip(tip)
        .on_click(on_reveal_toggled);

    settings_card(key, header, description, hstack((pb, reveal)).spacing(8.0))
}

/// Label + toggle as a settings card (switch flush-right).
pub fn card_toggle(
    key: &str,
    header: impl Into<String>,
    description: Option<&str>,
    is_on: bool,
    on_toggled: impl Fn(bool) + 'static,
) -> Element {
    settings_card(key, header, description, compact_toggle(is_on, on_toggled))
}

/// Label + toggle as a flat expander item (no nested card chrome).
pub fn row_toggle(
    key: &str,
    header: impl Into<String>,
    description: Option<&str>,
    is_on: bool,
    on_toggled: impl Fn(bool) + 'static,
) -> Element {
    settings_row(key, header, description, compact_toggle(is_on, on_toggled))
}

/// Params for [`card_slider_number`] / [`row_slider_number`] (keeps call sites clippy-clean).
pub struct SliderNumberParams {
    pub key: &'static str,
    pub header: String,
    pub description: Option<String>,
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

/// Snap `v` to the nearest step within [min, max].
///
/// Slider/f32 round-trips (e.g. `0.55f32 as f64`) produce noise like
/// `0.5500000119`. Rounding to the control's step keeps NumberBox clean.
pub fn quantize_to_step(v: f64, min: f64, max: f64, step: f64) -> f64 {
    let v = v.clamp(min, max);
    if !(step.is_finite() && step > 0.0) {
        return v;
    }
    let steps = ((v - min) / step).round();
    (min + steps * step).clamp(min, max)
}

fn slider_number_controls(p: &SliderNumberParams, on_changed: impl Fn(f64) + Clone + 'static) -> Element {
    let min = p.min;
    let max = p.max;
    let step = p.step;
    let value = quantize_to_step(p.value, min, max, step);

    let on_slider = {
        let on_changed = on_changed.clone();
        move |v: f64| on_changed(quantize_to_step(v, min, max, step))
    };
    let on_box = move |v: f64| on_changed(quantize_to_step(v, min, max, step));

    let mut slider = Slider::new(value).range(min, max).step(step).on_value_changed(on_slider);
    // Fixed width so the control column stays Auto-sized and right-aligned.
    slider.modifiers.width = Some(180.0);
    slider.modifiers.min_width = Some(140.0);
    slider.modifiers.vertical_alignment = Some(VerticalAlignment::Center);

    let mut nb = NumberBox::new(value).range(min, max).on_value_changed(on_box);
    nb.modifiers.width = Some(100.0);
    nb.modifiers.vertical_alignment = Some(VerticalAlignment::Center);

    hstack((slider, nb)).spacing(12.0).into()
}

/// Slider + NumberBox on one row (labels left, controls flush-right) — standalone card.
pub fn card_slider_number(p: SliderNumberParams, on_changed: impl Fn(f64) + Clone + 'static) -> Element {
    let controls = slider_number_controls(&p, on_changed);
    settings_card(p.key, p.header, p.description.as_deref(), controls)
}

/// Slider + NumberBox as a flat expander item (no nested card chrome).
pub fn row_slider_number(p: SliderNumberParams, on_changed: impl Fn(f64) + Clone + 'static) -> Element {
    let controls = slider_number_controls(&p, on_changed);
    settings_row(p.key, p.header, p.description.as_deref(), controls)
}

/// Params for optional numeric rows (toggle + slider + NumberBox).
pub struct OptionalSliderParams {
    pub key: &'static str,
    pub header: String,
    pub description: String,
    pub value: f64,
    pub enabled: bool,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

/// Optional number: Slider + NumberBox; Off omits from config (value kept for restore).
pub fn optional_slider_row(
    p: OptionalSliderParams,
    on_value: impl Fn(f64) + Clone + 'static,
    on_enabled: impl Fn(bool) + Clone + 'static,
) -> Element {
    let key = p.key;
    let min = p.min;
    let max = p.max;
    let step = p.step;
    let enabled = p.enabled;
    let value = quantize_to_step(p.value, min, max, step);

    let on_slider = {
        let on_value = on_value.clone();
        move |v: f64| on_value(quantize_to_step(v, min, max, step))
    };
    let on_box = {
        let on_value = on_value;
        move |v: f64| on_value(quantize_to_step(v, min, max, step))
    };

    let toggle = compact_toggle(enabled, on_enabled);

    let labels = vstack((
        text_block(p.header).semibold().font_size(14.0),
        text_block(p.description).font_size(12.0).foreground(ThemeRef::SecondaryText).wrap(),
    ))
    .spacing(2.0)
    .horizontal_alignment(HorizontalAlignment::Stretch)
    .vertical_alignment(VerticalAlignment::Center);

    let mut slider = Slider::new(value)
        .range(min, max)
        .step(step)
        .enabled(enabled)
        .on_value_changed(on_slider);
    slider.modifiers.width = Some(160.0);
    slider.modifiers.min_width = Some(120.0);
    slider.modifiers.vertical_alignment = Some(VerticalAlignment::Center);

    let mut nb = NumberBox::new(value).range(min, max).enabled(enabled).on_value_changed(on_box);
    nb.modifiers.width = Some(88.0);
    nb.modifiers.vertical_alignment = Some(VerticalAlignment::Center);

    let right = hstack((slider, nb, toggle))
        .spacing(12.0)
        .vertical_alignment(VerticalAlignment::Center)
        .horizontal_alignment(HorizontalAlignment::Right);

    let body = grid((
        labels
            .grid_row(0)
            .grid_column(0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Center)
            .margin(Thickness {
                left: 0.0,
                top: 0.0,
                right: 16.0,
                bottom: 0.0,
            }),
        right
            .grid_row(0)
            .grid_column(1)
            .horizontal_alignment(HorizontalAlignment::Right)
            .vertical_alignment(VerticalAlignment::Center),
    ))
    .rows([GridLength::Auto])
    .columns([GridLength::Star(1.0), GridLength::Auto])
    .horizontal_alignment(HorizontalAlignment::Stretch);

    border(body)
        .background(ThemeRef::CardBackground)
        .corner_radius(8.0)
        .padding(Thickness::uniform(16.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .with_key(key)
        .into()
}

/// Params for [`optional_number_row`] (NumberBox only, no slider).
pub struct OptionalNumberParams {
    pub key: &'static str,
    pub header: String,
    pub description: Option<String>,
    pub value: f64,
    pub enabled: bool,
    pub min: f64,
    pub max: f64,
    pub step: f64,
}

/// Optional integer/float with master toggle (e.g. max_tokens).
pub fn optional_number_row(p: OptionalNumberParams, on_value: impl Fn(f64) + 'static, on_enabled: impl Fn(bool) + 'static) -> Element {
    let min = p.min;
    let max = p.max;
    let step = p.step;
    let value = quantize_to_step(p.value, min, max, step);
    let toggle = compact_toggle(p.enabled, on_enabled);

    let header_el = text_block(p.header).semibold().font_size(14.0);
    let labels: Element = match p.description.as_deref() {
        Some(d) if !d.is_empty() => vstack((header_el, text_block(d).font_size(12.0).foreground(ThemeRef::SecondaryText).wrap()))
            .spacing(2.0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Center)
            .into(),
        _ => header_el.vertical_alignment(VerticalAlignment::Center).into(),
    };

    let mut nb = NumberBox::new(value)
        .range(min, max)
        .enabled(p.enabled)
        .on_value_changed(move |v: f64| on_value(quantize_to_step(v, min, max, step)));
    nb.modifiers.width = Some(120.0);
    nb.modifiers.vertical_alignment = Some(VerticalAlignment::Center);

    let right = hstack((nb, toggle))
        .spacing(12.0)
        .vertical_alignment(VerticalAlignment::Center)
        .horizontal_alignment(HorizontalAlignment::Right);

    let body = grid((
        labels
            .grid_row(0)
            .grid_column(0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Center)
            .margin(Thickness {
                left: 0.0,
                top: 0.0,
                right: 16.0,
                bottom: 0.0,
            }),
        right
            .grid_row(0)
            .grid_column(1)
            .horizontal_alignment(HorizontalAlignment::Right)
            .vertical_alignment(VerticalAlignment::Center),
    ))
    .rows([GridLength::Auto])
    .columns([GridLength::Star(1.0), GridLength::Auto])
    .horizontal_alignment(HorizontalAlignment::Stretch);

    border(body)
        .background(ThemeRef::CardBackground)
        .corner_radius(8.0)
        .padding(Thickness::uniform(16.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .with_key(p.key)
        .into()
}

/// Params for [`optional_text_row`].
pub struct OptionalTextParams {
    pub key: &'static str,
    pub header: String,
    pub description: Option<String>,
    pub text: String,
    pub enabled: bool,
    pub placeholder: String,
}

/// Optional string field with master toggle (e.g. reasoning_effort).
pub fn optional_text_row(p: OptionalTextParams, on_text: impl Fn(String) + 'static, on_enabled: impl Fn(bool) + 'static) -> Element {
    let toggle = compact_toggle(p.enabled, on_enabled);

    let header_el = text_block(p.header).semibold().font_size(14.0);
    let labels: Element = match p.description.as_deref() {
        Some(d) if !d.is_empty() => vstack((header_el, text_block(d).font_size(12.0).foreground(ThemeRef::SecondaryText).wrap()))
            .spacing(2.0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Center)
            .into(),
        _ => header_el.vertical_alignment(VerticalAlignment::Center).into(),
    };

    let mut tb = text_box(p.text)
        .placeholder_text(p.placeholder)
        .enabled(p.enabled)
        .on_text_changed(on_text);
    tb.modifiers.min_width = Some(140.0);
    tb.modifiers.width = Some(180.0);
    tb.modifiers.vertical_alignment = Some(VerticalAlignment::Center);

    let right = hstack((tb, toggle))
        .spacing(12.0)
        .vertical_alignment(VerticalAlignment::Center)
        .horizontal_alignment(HorizontalAlignment::Right);

    let body = grid((
        labels
            .grid_row(0)
            .grid_column(0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Center)
            .margin(Thickness {
                left: 0.0,
                top: 0.0,
                right: 16.0,
                bottom: 0.0,
            }),
        right
            .grid_row(0)
            .grid_column(1)
            .horizontal_alignment(HorizontalAlignment::Right)
            .vertical_alignment(VerticalAlignment::Center),
    ))
    .rows([GridLength::Auto])
    .columns([GridLength::Star(1.0), GridLength::Auto])
    .horizontal_alignment(HorizontalAlignment::Stretch);

    border(body)
        .background(ThemeRef::CardBackground)
        .corner_radius(8.0)
        .padding(Thickness::uniform(16.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .with_key(p.key)
        .into()
}

/// Params for [`card_color_popup`].
pub struct ColorPopupParams {
    pub key: &'static str,
    pub header: String,
    pub description: Option<String>,
    pub hex: String,
    /// Whether the ColorPicker panel is open.
    pub open: bool,
    pub alpha_enabled: bool,
    pub placeholder: String,
}

/// Compact color row: clickable swatch + hex; ColorPicker only when the swatch is open.
///
/// windows-reactor `Button::flyout` is text-only, so the swatch button toggles an
/// expandable panel (ColorPickerButton-style).
pub fn card_color_popup(
    p: ColorPopupParams,
    on_hex_changed: impl Fn(String) + 'static,
    on_color_changed: impl Fn((u8, u8, u8, u8)) + 'static,
    on_toggle_open: impl Fn() + 'static,
) -> Element {
    let key = p.key;
    let (a, r, g, b) = argb_u32_to_parts(parse_hex_u32(&p.hex).unwrap_or(0xFF00_0000));
    // Swatch fill uses opaque RGB so low-alpha colors stay visible on the card.
    let swatch_fill = Color::rgb(r, g, b);

    // Click the preview to open/close the picker (no separate Pick button).
    let swatch = button("")
        .background(swatch_fill)
        .width(32.0)
        .height(32.0)
        .min_width(32.0)
        .min_height(32.0)
        .padding(Thickness::uniform(0.0))
        .vertical_alignment(VerticalAlignment::Center)
        .tooltip(if p.open { "Close color picker" } else { "Open color picker" })
        .on_click(on_toggle_open);

    let mut hex_tb = text_box(p.hex).placeholder_text(p.placeholder).on_text_changed(on_hex_changed);
    hex_tb.modifiers.width = Some(120.0);
    hex_tb.modifiers.vertical_alignment = Some(VerticalAlignment::Center);

    let row = hstack((swatch, hex_tb)).spacing(8.0).vertical_alignment(VerticalAlignment::Center);

    if p.open {
        // Spectrum + alpha only — hex is on the row; channel boxes clutter the popup.
        // Open state needs full-width picker below the one-line header/controls.
        let picker = color_picker(ColorArgb::with_alpha(a, r, g, b))
            .alpha_enabled(p.alpha_enabled)
            .hex_input_visible(false)
            .color_channel_text_input_visible(false)
            .color_slider_visible(true)
            .on_color_changed(on_color_changed);

        let panel = border(picker)
            .border_brush(ThemeRef::CardStroke)
            .border_thickness(Thickness::uniform(1.0))
            .corner_radius(8.0)
            .padding(Thickness::uniform(12.0))
            .background(ThemeRef::SubtleFill)
            .with_key(format!("{key}-popup"));

        settings_card_stack(
            key,
            p.header,
            p.description.as_deref(),
            vstack((row.horizontal_alignment(HorizontalAlignment::Right), panel)).spacing(10.0),
        )
    } else {
        // Closed: one-line card, swatch + hex flush-right.
        settings_card(key, p.header, p.description.as_deref(), row)
    }
}

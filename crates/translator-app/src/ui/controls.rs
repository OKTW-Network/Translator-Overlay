//! Reusable setting-row controls (text, toggle, slider, optional toggle+slider).

use translator_core::{format_argb_hex, parse_argb_hex};
use translator_overlay::argb_channels;
use windows_reactor::{
    AutoSuggestBox, Border, Button, ButtonStyle, ChildrenControl, Color, ColorPicker, ContentControl, FontIcon, HorizontalAlignment,
    LayoutControl, NumberBox, Orientation, PasswordBox, PasswordRevealMode, ProgressRing, Slider, StackPanel, TextBox, ThemeBrush,
    Thickness, ToggleSwitch, TooltipExt, VerticalAlignment, View,
};

use crate::ui::chrome::{settings_card, settings_card_with_below};

fn compact_toggle(is_on: bool, on_toggled: impl Fn(bool) + 'static) -> ToggleSwitch {
    ToggleSwitch::new()
        .is_on(is_on)
        .on_toggled(on_toggled)
        .width(40.0)
        .min_width(40.0)
        .max_width(44.0)
        .horizontal_alignment(HorizontalAlignment::Right)
        .vertical_alignment(VerticalAlignment::Center)
}

/// Label + single-line text box as a settings card.
pub fn card_text(
    key: &str,
    header: impl Into<String>,
    description: Option<&str>,
    value: String,
    placeholder: impl Into<String>,
    on_changed: impl Fn(String) + 'static,
) -> View {
    let tb = TextBox::new()
        .text(value)
        .placeholder_text(placeholder)
        .on_text_changed(on_changed)
        .min_width(200.0)
        .width(280.0)
        .vertical_alignment(VerticalAlignment::Center);
    settings_card(key, header, description, tb)
}

pub struct ModelSuggestParams {
    pub key: &'static str,
    pub header: String,
    pub description: String,
    pub value: String,
    pub placeholder: String,
    pub suggestions: Vec<String>,
    pub loading: bool,
}

/// Label + AutoSuggestBox + refresh as a settings card.
pub fn card_model_suggest(p: ModelSuggestParams, on_changed: impl Fn(String) + Clone + 'static, on_refresh: impl Fn() + 'static) -> View {
    let suggest = AutoSuggestBox::new()
        .text(p.value)
        .placeholder_text(p.placeholder)
        .items_source(p.suggestions)
        .on_text_changed(on_changed.clone())
        .on_suggestion_chosen(on_changed)
        .min_width(200.0)
        .width(280.0)
        .vertical_alignment(VerticalAlignment::Center);

    let refresh_content: View = if p.loading {
        ProgressRing::new()
            .is_indeterminate(true)
            .is_active(true)
            .width(20.0)
            .height(20.0)
            .into()
    } else {
        FontIcon::new().glyph("\u{E72C}").into()
    };
    let refresh = Button::new()
        .style(ButtonStyle::Subtle)
        .min_width(36.0)
        .min_height(36.0)
        .horizontal_content_alignment(HorizontalAlignment::Center)
        .vertical_content_alignment(VerticalAlignment::Center)
        .vertical_alignment(VerticalAlignment::Center)
        .is_enabled(!p.loading)
        .on_click(on_refresh)
        .content(refresh_content)
        .tooltip(if p.loading { "Loading model list" } else { "Reload model list" });

    settings_card(
        p.key,
        p.header,
        Some(p.description.as_str()),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((suggest, refresh)),
    )
}

/// Label + password box + show/hide toggle as a settings card.
pub fn card_password(
    key: &str,
    header: impl Into<String>,
    description: Option<&str>,
    value: String,
    revealed: bool,
    on_changed: impl Fn(String) + 'static,
    on_reveal_toggled: impl Fn() + 'static,
) -> View {
    let pb = PasswordBox::new()
        .password(value)
        .password_reveal_mode(if revealed {
            PasswordRevealMode::Visible
        } else {
            PasswordRevealMode::Hidden
        })
        .on_password_changed(on_changed)
        .min_width(180.0)
        .width(240.0)
        .vertical_alignment(VerticalAlignment::Center);

    let glyph = if revealed { "\u{E7B3}" } else { "\u{E890}" };
    let tip = if revealed { "Hide key" } else { "Show key" };
    let reveal = Button::new()
        .style(ButtonStyle::Subtle)
        .min_width(36.0)
        .min_height(36.0)
        .vertical_alignment(VerticalAlignment::Center)
        .on_click(on_reveal_toggled)
        .content(FontIcon::new().glyph(glyph))
        .tooltip(tip);

    settings_card(
        key,
        header,
        description,
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(8.0)
            .children((pb, reveal)),
    )
}

/// Label + toggle as a settings card (switch flush-right).
pub fn card_toggle(
    key: &str,
    header: impl Into<String>,
    description: Option<&str>,
    is_on: bool,
    on_toggled: impl Fn(bool) + 'static,
) -> View {
    settings_card(key, header, description, compact_toggle(is_on, on_toggled))
}

/// Params for [`card_slider_number`] (keeps call sites clippy-clean).
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

fn slider_number_controls(p: &SliderNumberParams, on_changed: impl Fn(f64) + Clone + 'static) -> View {
    let min = p.min;
    let max = p.max;
    let step = p.step;
    let value = quantize_to_step(p.value, min, max, step);

    let on_slider = {
        let on_changed = on_changed.clone();
        move |v: f64| on_changed(quantize_to_step(v, min, max, step))
    };
    let on_box = move |v: Option<f64>| {
        if let Some(v) = v {
            on_changed(quantize_to_step(v, min, max, step));
        }
    };

    let slider = Slider::new()
        .value(value)
        .minimum(min)
        .maximum(max)
        .step_frequency(step)
        .on_value_changed(on_slider)
        .width(180.0)
        .min_width(140.0)
        .vertical_alignment(VerticalAlignment::Center);

    let nb = NumberBox::new()
        .value(value)
        .minimum(min)
        .maximum(max)
        .on_value_changed(on_box)
        .width(100.0)
        .vertical_alignment(VerticalAlignment::Center);

    StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(12.0)
        .children((slider, nb))
}

/// Slider + NumberBox on one row (labels left, controls flush-right) — standalone card.
pub fn card_slider_number(p: SliderNumberParams, on_changed: impl Fn(f64) + Clone + 'static) -> View {
    let controls = slider_number_controls(&p, on_changed);
    settings_card(p.key, p.header, p.description.as_deref(), controls)
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
) -> View {
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
        move |v: Option<f64>| {
            if let Some(v) = v {
                on_value(quantize_to_step(v, min, max, step));
            }
        }
    };

    settings_card(
        p.key,
        p.header,
        Some(p.description.as_str()),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .vertical_alignment(VerticalAlignment::Center)
            .horizontal_alignment(HorizontalAlignment::Right)
            .children((
                Slider::new()
                    .value(value)
                    .minimum(min)
                    .maximum(max)
                    .step_frequency(step)
                    .is_enabled(enabled)
                    .on_value_changed(on_slider)
                    .width(160.0)
                    .min_width(120.0)
                    .vertical_alignment(VerticalAlignment::Center),
                NumberBox::new()
                    .value(value)
                    .minimum(min)
                    .maximum(max)
                    .is_enabled(enabled)
                    .on_value_changed(on_box)
                    .width(88.0)
                    .vertical_alignment(VerticalAlignment::Center),
                compact_toggle(enabled, on_enabled),
            )),
    )
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
pub fn optional_number_row(p: OptionalNumberParams, on_value: impl Fn(f64) + 'static, on_enabled: impl Fn(bool) + 'static) -> View {
    let min = p.min;
    let max = p.max;
    let step = p.step;
    let value = quantize_to_step(p.value, min, max, step);
    settings_card(
        p.key,
        p.header,
        p.description.as_deref(),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .vertical_alignment(VerticalAlignment::Center)
            .horizontal_alignment(HorizontalAlignment::Right)
            .children((
                NumberBox::new()
                    .value(value)
                    .minimum(min)
                    .maximum(max)
                    .is_enabled(p.enabled)
                    .on_value_changed(move |v: Option<f64>| {
                        if let Some(v) = v {
                            on_value(quantize_to_step(v, min, max, step));
                        }
                    })
                    .width(120.0)
                    .vertical_alignment(VerticalAlignment::Center),
                compact_toggle(p.enabled, on_enabled),
            )),
    )
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
pub fn optional_text_row(p: OptionalTextParams, on_text: impl Fn(String) + 'static, on_enabled: impl Fn(bool) + 'static) -> View {
    settings_card(
        p.key,
        p.header,
        p.description.as_deref(),
        StackPanel::new()
            .orientation(Orientation::Horizontal)
            .spacing(12.0)
            .vertical_alignment(VerticalAlignment::Center)
            .horizontal_alignment(HorizontalAlignment::Right)
            .children((
                TextBox::new()
                    .text(p.text)
                    .placeholder_text(p.placeholder)
                    .is_enabled(p.enabled)
                    .on_text_changed(on_text)
                    .min_width(140.0)
                    .width(180.0)
                    .vertical_alignment(VerticalAlignment::Center),
                compact_toggle(p.enabled, on_enabled),
            )),
    )
}

/// Params for [`card_color_popup`].
pub struct ColorPopupParams {
    pub key: &'static str,
    pub header: String,
    pub hex: String,
    pub open: bool,
    pub placeholder: String,
}

/// Compact color row: swatch + hex; ColorPicker expands under the same header row.
///
/// Not a Flyout (ThemeShadow over Mica paints as a solid block). Subtle Button
/// keeps native PointerOver; the 32×32 chip is content so colour stays full size.
pub fn card_color_popup(
    p: ColorPopupParams,
    on_hex_changed: impl Fn(String) + Clone + 'static,
    on_toggle_open: impl Fn() + 'static,
) -> View {
    let (a, r, g, b) = argb_channels(parse_argb_hex(&p.hex).unwrap_or(0xFF00_0000));
    // Opaque RGB so low-alpha colours stay visible on the card.
    let swatch_fill = Color::rgb(r, g, b);

    let swatch = Button::new()
        .style(ButtonStyle::Subtle)
        .min_width(0.0)
        .min_height(0.0)
        .vertical_alignment(VerticalAlignment::Center)
        .on_click(on_toggle_open)
        .content(
            Border::new()
                .background(swatch_fill)
                .width(32.0)
                .height(32.0)
                .corner_radius(4.0)
                .border_brush(ThemeBrush::CardStroke)
                .border_thickness(Thickness::uniform(1.0)),
        )
        .tooltip(if p.open { "Close color picker" } else { "Open color picker" });

    let hex_tb = TextBox::new()
        .text(p.hex)
        .placeholder_text(p.placeholder)
        .on_text_changed(on_hex_changed.clone())
        .width(120.0)
        .vertical_alignment(VerticalAlignment::Center);

    let row = StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .vertical_alignment(VerticalAlignment::Center)
        .children((swatch, hex_tb));

    let below = p.open.then(|| {
        ColorPicker::new()
            .color(Color::argb(a, r, g, b))
            .is_alpha_enabled(true)
            .is_hex_input_visible(false)
            .is_color_channel_text_input_visible(false)
            .on_color_changed(move |c: Color| {
                on_hex_changed(format_argb_hex((u32::from(c.a) << 24) | (u32::from(c.r) << 16) | (u32::from(c.g) << 8) | u32::from(c.b)))
            })
            .into()
    });

    settings_card_with_below(p.key, p.header, Some("Click the swatch to pick colour and opacity, or type #AARRGGBB hex."), row, below)
}

//! Reusable controls over `translator_winui`, shared by every page.
//!
//! Pages build their controls once and register [`Binds`] that copy state into them on every
//! sync. This module holds the basic builders; the submodules hold controlled fields, card
//! layout, setting rows, dialogs, and the saved-items toolbar.

pub mod cards;
pub mod controls;
pub mod dialog;
pub mod fields;
pub mod named;

use translator_winui::{
    Application, Border, Button, FontIcon, HSTRING, HorizontalAlignment, Interface, Layout, Orientation, Result, StackPanel, Stretch,
    Style, TextBlock, UIElement, VerticalAlignment, Viewbox, boxed, keep, theme_style,
};

use crate::ui::{app::quiet, state::ChromeSnap};

/// Sync callbacks a page registers while building its controls.
type Bind<S> = Box<dyn FnMut(&S, &ChromeSnap) -> Result<()>>;

pub struct Binds<S>(Vec<Bind<S>>);

impl<S> Default for Binds<S> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<S> Binds<S> {
    pub fn add(&mut self, f: impl FnMut(&S, &ChromeSnap) -> Result<()> + 'static) {
        self.0.push(Box::new(f));
    }

    pub fn run(&mut self, state: &S, chrome: &ChromeSnap) -> Result<()> {
        for bind in &mut self.0 {
            bind(state, chrome)?;
        }
        Ok(())
    }
}

/// Theme brushes the control window uses. Applied through styles so they follow light/dark.
#[derive(Clone, Copy)]
pub enum Theme {
    Accent,
    PrimaryText,
    CardBackground,
    CardStroke,
    SystemCritical,
}

impl Theme {
    fn key(self) -> &'static str {
        match self {
            Self::Accent => "AccentFillColorDefaultBrush",
            Self::PrimaryText => "TextFillColorPrimaryBrush",
            Self::CardBackground => "CardBackgroundFillColorDefaultBrush",
            Self::CardStroke => "CardStrokeColorDefaultBrush",
            Self::SystemCritical => "SystemFillColorCriticalBrush",
        }
    }
}

pub fn hs(s: &str) -> HSTRING {
    HSTRING::from(s)
}

pub fn text(s: &str) -> Result<TextBlock> {
    let tb = TextBlock::new()?;
    tb.SetText(&hs(s))?;
    Ok(tb)
}

pub fn foreground(tb: &TextBlock, brush: Theme) -> Result<()> {
    tb.style(Some(&theme_style("TextBlock", &[("Foreground", brush.key())])?))?;
    Ok(())
}

/// 12px secondary text: primary text brush at 72% opacity.
pub fn muted(s: &str) -> Result<TextBlock> {
    let tb = text(s)?;
    tb.SetFontSize(12.0)?;
    foreground(&tb, Theme::PrimaryText)?;
    tb.opacity(0.72)?;
    Ok(tb)
}

/// Border with theme `Background` and optional theme `BorderBrush`.
pub fn themed_border(background: Option<Theme>, stroke: Option<Theme>) -> Result<Border> {
    let border = Border::new()?;
    let mut setters = Vec::new();
    if let Some(b) = background {
        setters.push(("Background", b.key()));
    }
    if let Some(b) = stroke {
        setters.push(("BorderBrush", b.key()));
    }
    if !setters.is_empty() {
        border.style(Some(&theme_style("Border", &setters)?))?;
    }
    Ok(border)
}

/// 1px divider line in the card stroke colour.
pub fn rule() -> Result<Border> {
    themed_border(Some(Theme::CardStroke), None)
}

pub fn stack(orientation: Orientation, spacing: f64) -> Result<StackPanel> {
    let panel = StackPanel::new()?;
    panel.SetOrientation(orientation)?;
    if spacing != 0.0 {
        panel.SetSpacing(spacing)?;
    }
    Ok(panel)
}

pub fn row(spacing: f64) -> Result<StackPanel> {
    stack(Orientation::Horizontal, spacing)
}

pub fn column(spacing: f64) -> Result<StackPanel> {
    stack(Orientation::Vertical, spacing)
}

/// Appends each element to a panel's `Children`.
#[macro_export]
macro_rules! children {
    ($panel:expr; $($child:expr),+ $(,)?) => {{
        let children = $panel.Children()?;
        $(children.Append(&$child)?;)+
    }};
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Default,
    Accent,
    Subtle,
}

pub fn button_style(button: &Button, kind: ButtonKind) -> Result<()> {
    let key = match kind {
        ButtonKind::Default => return button.style(None).map(drop),
        ButtonKind::Accent => "AccentButtonStyle",
        ButtonKind::Subtle => "SubtleButtonStyle",
    };
    let style: Style = Application::Current()?.Resources()?.Lookup(&boxed(key))?.cast()?;
    button.style(Some(&style))?;
    Ok(())
}

/// Button whose content is a text block with `label`.
pub fn button(label: &str, on_click: impl Fn() + Send + 'static) -> Result<Button> {
    let b = Button::new()?;
    b.SetContent(&text(label)?)?;
    click(&b, on_click)?;
    Ok(b)
}

pub fn click(button: &Button, f: impl Fn() + Send + 'static) -> Result<()> {
    keep(button.Click(move |_, _| {
        if !quiet() {
            f();
        }
    }))
}

pub fn font_icon(glyph: &str) -> Result<FontIcon> {
    let icon = FontIcon::new()?;
    icon.SetGlyph(&hs(glyph))?;
    Ok(icon)
}

/// Subtle 36px button showing one glyph (button styles centre it in its row).
pub fn icon_button(glyph: &str, on_click: impl Fn() + Send + 'static) -> Result<Button> {
    let b = Button::new()?;
    button_style(&b, ButtonKind::Subtle)?;
    b.min_width(36.0)?.min_height(36.0)?;
    b.SetContent(&font_icon(glyph)?)?;
    click(&b, on_click)?;
    Ok(b)
}

/// 16px glyph boxed in a uniform `Viewbox`, centred.
pub fn glyph_box(glyph: &str) -> Result<Viewbox> {
    let vb = Viewbox::new()?;
    vb.width(16.0)?
        .height(16.0)?
        .halign(HorizontalAlignment::Center)?
        .valign(VerticalAlignment::Center)?;
    vb.SetStretch(Stretch::Uniform)?;
    vb.SetChild(&font_icon(glyph)?)?;
    Ok(vb)
}

pub fn ui(el: &impl Interface) -> Result<UIElement> {
    el.cast()
}

/// Cached plain property: applies `set` only when the value changes.
pub struct Prop<T>(Option<T>);

impl<T> Default for Prop<T> {
    fn default() -> Self {
        Self(None)
    }
}

impl<T: PartialEq> Prop<T> {
    pub fn set(&mut self, v: T, apply: impl FnOnce(&T) -> Result<()>) -> Result<()> {
        if self.0.as_ref() != Some(&v) {
            apply(&v)?;
            self.0 = Some(v);
        }
        Ok(())
    }
}

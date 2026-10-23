//! Controlled-value wrappers: each remembers the last value a sync applied, so the echo of a
//! programmatic set (WinUI raises change events for those) is not taken as a user edit.

use std::sync::Arc;

use parking_lot::Mutex;
use translator_winui::{
    AutoSuggestBox, Color, ColorPicker, ComboBox, NumberBox, PasswordBox, PasswordRevealMode, RadioButton, Result, Slider, TextBox,
    ToggleSwitch, keep, string_items, unboxed,
};

use crate::ui::{
    app::quiet,
    widgets::{Prop, hs, text},
};

/// Last value a sync wrote, shared with the change handler.
pub struct Last<T>(Arc<Mutex<Option<T>>>);

impl<T> Clone for Last<T> {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl<T> Default for Last<T> {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(None)))
    }
}

impl<T: PartialEq + Clone> Last<T> {
    /// Records `v` and returns true when it differs from the previous sync.
    pub fn changed(&self, v: &T) -> bool {
        let mut last = self.0.lock();
        if last.as_ref() == Some(v) {
            return false;
        }
        *last = Some(v.clone());
        true
    }

    /// True when `v` is what the last sync wrote (an echo, not an edit).
    pub fn is(&self, v: &T) -> bool {
        self.0.lock().as_ref() == Some(v)
    }

    pub fn forget(&self) {
        *self.0.lock() = None;
    }
}

#[derive(Clone)]
pub struct TextField {
    pub tb: TextBox,
    last: Last<String>,
}

impl TextField {
    pub fn new(placeholder: &str, on_changed: impl Fn(String) + Send + 'static) -> Result<Self> {
        let tb = TextBox::new()?;
        if !placeholder.is_empty() {
            tb.SetPlaceholderText(&hs(placeholder))?;
        }
        let last = Last::default();
        let (source, seen) = (tb.clone(), last.clone());
        keep(tb.TextChanged(move |_, _| {
            let Ok(t) = source.Text() else {
                return;
            };
            let t = t.to_string_lossy();
            if !quiet() && !seen.is(&t) {
                on_changed(t);
            }
        }))?;
        Ok(Self { tb, last })
    }

    pub fn set(&self, v: &str) -> Result<()> {
        if self.last.changed(&v.to_string()) && self.tb.Text()?.to_string_lossy() != v {
            self.tb.SetText(&hs(v))?;
        }
        Ok(())
    }
}

pub struct PasswordField {
    pub pb: PasswordBox,
    last: Last<String>,
    reveal: Prop<bool>,
}

impl PasswordField {
    pub fn new(on_changed: impl Fn(String) + Send + 'static) -> Result<Self> {
        let pb = PasswordBox::new()?;
        let last = Last::default();
        let (source, seen) = (pb.clone(), last.clone());
        keep(pb.PasswordChanged(move |_, _| {
            let Ok(t) = source.Password() else {
                return;
            };
            let t = t.to_string_lossy();
            if !quiet() && !seen.is(&t) {
                on_changed(t);
            }
        }))?;
        Ok(Self {
            pb,
            last,
            reveal: Prop::default(),
        })
    }

    pub fn set(&mut self, v: &str, revealed: bool) -> Result<()> {
        if self.last.changed(&v.to_string()) && self.pb.Password()?.to_string_lossy() != v {
            self.pb.SetPassword(&hs(v))?;
        }
        let pb = &self.pb;
        self.reveal.set(revealed, |&r| {
            pb.SetPasswordRevealMode(if r {
                PasswordRevealMode::Visible
            } else {
                PasswordRevealMode::Hidden
            })
        })
    }
}

pub struct SuggestField {
    pub asb: AutoSuggestBox,
    last: Last<String>,
    items: Prop<Vec<String>>,
}

impl SuggestField {
    pub fn new(placeholder: &str, on_changed: impl Fn(String) + Clone + Send + 'static) -> Result<Self> {
        let asb = AutoSuggestBox::new()?;
        if !placeholder.is_empty() {
            asb.SetPlaceholderText(&hs(placeholder))?;
        }
        let last = Last::default();
        let (seen, typed) = (last.clone(), on_changed.clone());
        keep(asb.TextChanged(move |source, _| {
            let Some(t) = source.as_ref().and_then(|s| s.Text().ok()) else {
                return;
            };
            let t = t.to_string_lossy();
            if !quiet() && !seen.is(&t) {
                typed(t);
            }
        }))?;
        keep(asb.SuggestionChosen(move |_, args| {
            if let Some(item) = args.as_ref().and_then(|a| a.SelectedItem().ok()).and_then(|i| unboxed(&i))
                && !quiet()
            {
                on_changed(item);
            }
        }))?;
        Ok(Self {
            asb,
            last,
            items: Prop::default(),
        })
    }

    pub fn set(&mut self, v: &str, items: Vec<String>) -> Result<()> {
        if self.last.changed(&v.to_string()) && self.asb.Text()?.to_string_lossy() != v {
            self.asb.SetText(&hs(v))?;
        }
        let asb = &self.asb;
        self.items.set(items, |items| asb.SetItemsSource(&string_items(items)?))
    }
}

pub struct Toggle {
    pub ts: ToggleSwitch,
    last: Last<bool>,
    enabled: Prop<bool>,
}

impl Toggle {
    pub fn new(on_toggled: impl Fn(bool) + Send + 'static) -> Result<Self> {
        let ts = ToggleSwitch::new()?;
        let last = Last::default();
        let (source, seen) = (ts.clone(), last.clone());
        keep(ts.Toggled(move |_, _| {
            let Ok(on) = source.IsOn() else {
                return;
            };
            if !quiet() && !seen.is(&on) {
                on_toggled(on);
            }
        }))?;
        Ok(Self {
            ts,
            last,
            enabled: Prop::default(),
        })
    }

    pub fn set(&mut self, on: bool, enabled: bool) -> Result<()> {
        if self.last.changed(&on) && self.ts.IsOn()? != on {
            self.ts.SetIsOn(on)?;
        }
        let ts = &self.ts;
        self.enabled.set(enabled, |&e| ts.SetIsEnabled(e))
    }
}

pub struct Radio {
    pub rb: RadioButton,
    last: Last<bool>,
    enabled: Prop<bool>,
}

impl Radio {
    pub fn new(group: &str, label: &str, on_checked: impl Fn() + Send + 'static) -> Result<Self> {
        let rb = RadioButton::new()?;
        rb.SetGroupName(&hs(group))?;
        rb.SetContent(&text(label)?)?;
        let last = Last::default();
        let seen = last.clone();
        keep(rb.Checked(move |_, _| {
            if !quiet() && !seen.is(&true) {
                on_checked();
            }
        }))?;
        Ok(Self {
            rb,
            last,
            enabled: Prop::default(),
        })
    }

    pub fn set(&mut self, checked: bool, enabled: bool) -> Result<()> {
        if self.last.changed(&checked) && self.rb.IsChecked()? != checked {
            self.rb.SetIsChecked(Some(checked))?;
        }
        let rb = &self.rb;
        self.enabled.set(enabled, |&e| rb.SetIsEnabled(e))
    }
}

pub struct Combo {
    pub cb: ComboBox,
    last: Last<Option<usize>>,
    items: Prop<Vec<String>>,
    enabled: Prop<bool>,
}

impl Combo {
    pub fn new(placeholder: &str, on_selected: impl Fn(Option<usize>) + Send + 'static) -> Result<Self> {
        let cb = ComboBox::new()?;
        if !placeholder.is_empty() {
            cb.SetPlaceholderText(&hs(placeholder))?;
        }
        let last = Last::default();
        let (source, seen) = (cb.clone(), last.clone());
        keep(cb.SelectionChanged(move |_, _| {
            let Ok(i) = source.SelectedIndex() else {
                return;
            };
            let idx = usize::try_from(i).ok();
            if !quiet() && !seen.is(&idx) {
                on_selected(idx);
            }
        }))?;
        Ok(Self {
            cb,
            last,
            items: Prop::default(),
            enabled: Prop::default(),
        })
    }

    pub fn set(&mut self, items: Vec<String>, selected: Option<usize>, enabled: bool) -> Result<()> {
        let cb = &self.cb;
        let last = &self.last;
        // A new items source drops the selection, so reapply it.
        self.items.set(items, |items| {
            last.forget();
            cb.SetItemsSource(&string_items(items)?)
        })?;
        if self.last.changed(&selected) {
            let i = selected.and_then(|i| i32::try_from(i).ok()).unwrap_or(-1);
            if self.cb.SelectedIndex()? != i {
                self.cb.SetSelectedIndex(i)?;
            }
        }
        self.enabled.set(enabled, |&e| cb.SetIsEnabled(e))
    }
}

/// `Slider` and `NumberBox` bound to one value.
pub struct SliderNumber {
    pub slider: Slider,
    pub number: NumberBox,
    last: Last<u64>,
    enabled: Prop<bool>,
}

impl SliderNumber {
    pub fn new((min, max, step): (f64, f64, f64), on_changed: impl Fn(f64) + Clone + Send + 'static) -> Result<Self> {
        let slider = Slider::new()?;
        slider.SetMinimum(min)?;
        slider.SetMaximum(max)?;
        slider.SetStepFrequency(step)?;
        // Arrow keys move by SmallChange (default 1), not StepFrequency.
        slider.SetSmallChange(step)?;
        let number = NumberBox::new()?;
        number.SetMinimum(min)?;
        number.SetMaximum(max)?;
        number.SetSmallChange(step)?;
        let last = Last::default();
        let (seen, on_slider) = (last.clone(), on_changed.clone());
        keep(slider.ValueChanged(move |_, args| {
            if let Some(v) = args.as_ref().and_then(|a| a.NewValue().ok())
                && !quiet()
                && !seen.is(&v.to_bits())
            {
                on_slider(v);
            }
        }))?;
        let seen = last.clone();
        keep(number.ValueChanged(move |_, args| {
            if let Some(v) = args.as_ref().and_then(|a| a.NewValue().ok()).filter(|v| !v.is_nan())
                && !quiet()
                && !seen.is(&v.to_bits())
            {
                on_changed(v);
            }
        }))?;
        Ok(Self {
            slider,
            number,
            last,
            enabled: Prop::default(),
        })
    }

    pub fn set(&mut self, v: f64, enabled: bool) -> Result<()> {
        if self.last.changed(&v.to_bits()) {
            if self.slider.Value()? != v {
                self.slider.SetValue(v)?;
            }
            if self.number.Value()? != v {
                self.number.SetValue(v)?;
            }
        }
        let (slider, number) = (&self.slider, &self.number);
        self.enabled.set(enabled, |&e| {
            slider.SetIsEnabled(e)?;
            number.SetIsEnabled(e)
        })
    }
}

/// `NumberBox` alone.
pub struct NumberField {
    pub number: NumberBox,
    last: Last<u64>,
    enabled: Prop<bool>,
}

impl NumberField {
    pub fn new((min, max): (f64, f64), on_changed: impl Fn(f64) + Send + 'static) -> Result<Self> {
        let number = NumberBox::new()?;
        number.SetMinimum(min)?;
        number.SetMaximum(max)?;
        let last = Last::default();
        let seen = last.clone();
        keep(number.ValueChanged(move |_, args| {
            if let Some(v) = args.as_ref().and_then(|a| a.NewValue().ok()).filter(|v| !v.is_nan())
                && !quiet()
                && !seen.is(&v.to_bits())
            {
                on_changed(v);
            }
        }))?;
        Ok(Self {
            number,
            last,
            enabled: Prop::default(),
        })
    }

    pub fn set(&mut self, v: f64, enabled: bool) -> Result<()> {
        if self.last.changed(&v.to_bits()) && self.number.Value()? != v {
            self.number.SetValue(v)?;
        }
        let number = &self.number;
        self.enabled.set(enabled, |&e| number.SetIsEnabled(e))
    }
}

pub struct ColorField {
    pub picker: ColorPicker,
    last: Last<[u8; 4]>,
}

impl ColorField {
    pub fn new(on_changed: impl Fn([u8; 4]) + Send + 'static) -> Result<Self> {
        let picker = ColorPicker::new()?;
        picker.SetIsAlphaEnabled(true)?;
        picker.SetIsHexInputVisible(false)?;
        picker.SetIsColorChannelTextInputVisible(false)?;
        let last = Last::default();
        let seen = last.clone();
        keep(picker.ColorChanged(move |_, args| {
            if let Some(c) = args.as_ref().and_then(|a| a.NewColor().ok()) {
                let argb = [c.A, c.R, c.G, c.B];
                if !quiet() && !seen.is(&argb) {
                    on_changed(argb);
                }
            }
        }))?;
        Ok(Self { picker, last })
    }

    pub fn set(&self, [a, r, g, b]: [u8; 4]) -> Result<()> {
        if self.last.changed(&[a, r, g, b]) {
            self.picker.SetColor(Color { A: a, R: r, G: g, B: b })?;
        }
        Ok(())
    }
}

//! Small helpers over the generated projection: layout setters, values, styles, UI-thread posting.

use std::{
    cell::RefCell,
    collections::HashMap,
    ffi::c_void,
    sync::{Mutex, OnceLock},
};

use windows_core::{Error, EventRevoker, HSTRING, IInspectable, IUnknown, IUnknown_Vtbl, Interface, Result, imp::CanInto};
use windows_reference::IReference;

use crate::generated::bindings::{
    AutomationProperties, Color, ColumnDefinition, CornerRadius, DispatcherQueue, DispatcherQueueHandler, DispatcherQueuePriority,
    FrameworkElement, Grid, GridLength, GridUnitType, HorizontalAlignment, PlacementMode, RowDefinition, SolidColorBrush, Style, TextBlock,
    Thickness, ToolTipService, UIElement, VerticalAlignment, Visibility, Window, WriteableBitmap, XamlReader,
};

const E_INVALIDARG: windows_core::HRESULT = windows_core::HRESULT(0x8007_0057_u32 as i32);

/// Builder-style setters shared by every `FrameworkElement`. Each returns `self` for chaining.
pub trait Layout: Interface + CanInto<FrameworkElement> + CanInto<UIElement> {
    fn fe(&self) -> Result<FrameworkElement> {
        self.cast()
    }
    fn width(&self, v: f64) -> Result<&Self> {
        self.fe()?.SetWidth(v)?;
        Ok(self)
    }
    fn height(&self, v: f64) -> Result<&Self> {
        self.fe()?.SetHeight(v)?;
        Ok(self)
    }
    fn min_width(&self, v: f64) -> Result<&Self> {
        self.fe()?.SetMinWidth(v)?;
        Ok(self)
    }
    fn max_width(&self, v: f64) -> Result<&Self> {
        self.fe()?.SetMaxWidth(v)?;
        Ok(self)
    }
    fn min_height(&self, v: f64) -> Result<&Self> {
        self.fe()?.SetMinHeight(v)?;
        Ok(self)
    }
    fn max_height(&self, v: f64) -> Result<&Self> {
        self.fe()?.SetMaxHeight(v)?;
        Ok(self)
    }
    fn margin(&self, left: f64, top: f64, right: f64, bottom: f64) -> Result<&Self> {
        self.fe()?.SetMargin(thickness(left, top, right, bottom))?;
        Ok(self)
    }
    fn halign(&self, v: HorizontalAlignment) -> Result<&Self> {
        self.fe()?.SetHorizontalAlignment(v)?;
        Ok(self)
    }
    fn valign(&self, v: VerticalAlignment) -> Result<&Self> {
        self.fe()?.SetVerticalAlignment(v)?;
        Ok(self)
    }
    /// Stretch on both axes.
    fn fill(&self) -> Result<&Self> {
        self.halign(HorizontalAlignment::Stretch)?.valign(VerticalAlignment::Stretch)
    }
    fn opacity(&self, v: f64) -> Result<&Self> {
        self.fe()?.SetOpacity(v)?;
        Ok(self)
    }
    fn visible(&self, v: bool) -> Result<&Self> {
        self.fe()?
            .SetVisibility(if v { Visibility::Visible } else { Visibility::Collapsed })?;
        Ok(self)
    }
    /// `None` clears the local style; a null local value would hide the implicit default style.
    fn style(&self, v: Option<&Style>) -> Result<&Self> {
        let fe = self.fe()?;
        match v {
            Some(style) => fe.SetStyle(style)?,
            None => fe.ClearValue(&FrameworkElement::StyleProperty()?)?,
        }
        Ok(self)
    }
    fn row(&self, v: i32) -> Result<&Self> {
        Grid::SetRow(&self.fe()?, v)?;
        Ok(self)
    }
    fn column(&self, v: i32) -> Result<&Self> {
        Grid::SetColumn(&self.fe()?, v)?;
        Ok(self)
    }
    fn automation_id(&self, v: &str) -> Result<&Self> {
        AutomationProperties::SetAutomationId(&self.fe()?, &HSTRING::from(v))?;
        Ok(self)
    }
    fn automation_name(&self, v: &str) -> Result<&Self> {
        AutomationProperties::SetName(&self.fe()?, &HSTRING::from(v))?;
        Ok(self)
    }
    /// Plain-text tooltip shown above the element.
    fn tooltip(&self, v: &str) -> Result<&Self> {
        let text = TextBlock::new()?;
        text.SetText(&HSTRING::from(v))?;
        self.tooltip_with(&text)
    }
    fn tooltip_with(&self, content: &impl Interface) -> Result<&Self> {
        let fe = self.fe()?;
        ToolTipService::SetToolTip(&fe, &content.cast::<IInspectable>()?)?;
        ToolTipService::SetPlacement(&fe, PlacementMode::Top)?;
        Ok(self)
    }
}

impl<T: Interface + CanInto<FrameworkElement> + CanInto<UIElement>> Layout for T {}

pub fn thickness(left: f64, top: f64, right: f64, bottom: f64) -> Thickness {
    Thickness {
        Left: left,
        Top: top,
        Right: right,
        Bottom: bottom,
    }
}

pub fn uniform(v: f64) -> Thickness {
    thickness(v, v, v, v)
}

pub fn corners(v: f64) -> CornerRadius {
    CornerRadius {
        TopLeft: v,
        TopRight: v,
        BottomRight: v,
        BottomLeft: v,
    }
}

pub fn auto() -> GridLength {
    GridLength {
        Value: 1.0,
        GridUnitType: GridUnitType::Auto,
    }
}

pub fn star(v: f64) -> GridLength {
    GridLength {
        Value: v,
        GridUnitType: GridUnitType::Star,
    }
}

pub fn pixels(v: f64) -> GridLength {
    GridLength {
        Value: v,
        GridUnitType: GridUnitType::Pixel,
    }
}

/// Grid with the given row and column definitions.
pub fn grid(rows: &[GridLength], columns: &[GridLength]) -> Result<Grid> {
    let grid = Grid::new()?;
    let defs = grid.RowDefinitions()?;
    for &height in rows {
        let row = RowDefinition::new()?;
        row.SetHeight(height)?;
        defs.Append(&row)?;
    }
    let defs = grid.ColumnDefinitions()?;
    for &width in columns {
        let column = ColumnDefinition::new()?;
        column.SetWidth(width)?;
        defs.Append(&column)?;
    }
    Ok(grid)
}

/// A boxed string, for `Content`, `Title`, and `Tag` slots.
pub fn boxed(v: &str) -> IInspectable {
    IReference::<HSTRING>::from(HSTRING::from(v)).into()
}

/// A boxed `f64`, for numeric resources.
pub fn boxed_f64(v: f64) -> IInspectable {
    IReference::<f64>::from(v).into()
}

/// Unboxes a string `Tag` / item.
pub fn unboxed(v: &IInspectable) -> Option<String> {
    v.cast::<IReference<HSTRING>>().ok()?.Value().ok().map(|s| s.to_string_lossy())
}

/// `ItemsSource` value holding one boxed string per item.
pub fn string_items(items: &[String]) -> Result<IInspectable> {
    let values: Vec<Option<IInspectable>> = items.iter().map(|item| Some(boxed(item))).collect();
    windows_collections::IVector::<IInspectable>::from(values).cast()
}

pub fn solid_brush(color: Color) -> Result<SolidColorBrush> {
    let brush = SolidColorBrush::new()?;
    brush.SetColor(color)?;
    Ok(brush)
}

thread_local! {
    static STYLES: RefCell<HashMap<String, Style>> = RefCell::new(HashMap::new());
}

/// Style whose setters point at theme resources, so the values follow light/dark switches.
///
/// `setters` are `(property, resource key)` pairs. Styles are cached per markup.
///
/// Adapted from windows-reactor 0.100.0 (theme styles in `src/native/winui/mod.rs`),
/// <https://github.com/microsoft/windows-rs>. Copyright (c) Microsoft Corporation. Licensed under MIT OR Apache-2.0.
pub fn theme_style(target: &str, setters: &[(&str, &str)]) -> Result<Style> {
    let mut xaml = format!("<Style xmlns='http://schemas.microsoft.com/winfx/2006/xaml/presentation' TargetType='{target}'>");
    for (property, key) in setters {
        xaml.push_str(&format!("<Setter Property='{property}' Value='{{ThemeResource {key}}}'/>"));
    }
    xaml.push_str("</Style>");
    if let Some(style) = STYLES.with(|cache| cache.borrow().get(&xaml).cloned()) {
        return Ok(style);
    }
    let style: Style = XamlReader::Load(&HSTRING::from(xaml.as_str()))?.cast()?;
    STYLES.with(|cache| cache.borrow_mut().insert(xaml, style.clone()));
    Ok(style)
}

/// Keeps an event subscription for the lifetime of its source.
pub fn keep(revoker: Result<EventRevoker>) -> Result<()> {
    revoker.map(EventRevoker::forget)
}

windows_core::imp::define_interface!(IWindowNative, IWindowNative_Vtbl, 0xeecdbf0e_bae9_4cb6_a68e_9598e1cb57bb);
windows_core::imp::interface_hierarchy!(IWindowNative, IUnknown);
#[repr(C)]
#[allow(non_snake_case)]
pub struct IWindowNative_Vtbl {
    base__: IUnknown_Vtbl,
    WindowHandle: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> windows_core::HRESULT,
}

/// Win32 handle of a WinUI window.
pub fn window_hwnd(window: &Window) -> Result<isize> {
    let native: IWindowNative = window.cast()?;
    let mut hwnd = std::ptr::null_mut();
    // SAFETY: `WindowHandle` writes the HWND of this live window.
    unsafe { (native.vtable().WindowHandle)(native.as_raw(), &mut hwnd).ok()? };
    Ok(hwnd as isize)
}

windows_core::imp::define_interface!(IBufferByteAccess, IBufferByteAccess_Vtbl, 0x905a0fef_bc53_11df_8c49_001e4fc686da);
windows_core::imp::interface_hierarchy!(IBufferByteAccess, IUnknown);
#[repr(C)]
#[allow(non_snake_case)]
pub struct IBufferByteAccess_Vtbl {
    base__: IUnknown_Vtbl,
    Buffer: unsafe extern "system" fn(*mut c_void, *mut *mut u8) -> windows_core::HRESULT,
}

/// Bitmap of `width`×`height` premultiplied BGRA8 `pixels`, for `Image.Source`.
pub fn bitmap(width: u32, height: u32, pixels: &[u8]) -> Result<WriteableBitmap> {
    if pixels.len() != width as usize * height as usize * 4 {
        return Err(Error::new(E_INVALIDARG, "pixel buffer does not match the bitmap size"));
    }
    let bitmap = WriteableBitmap::CreateInstanceWithDimensions(width as i32, height as i32)?;
    let access: IBufferByteAccess = bitmap.PixelBuffer()?.cast()?;
    let mut data = std::ptr::null_mut();
    // SAFETY: `Buffer` points at the bitmap's own width×height×4 bytes, checked against `pixels` above.
    unsafe {
        (access.vtable().Buffer)(access.as_raw(), &mut data).ok()?;
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), data, pixels.len());
    }
    bitmap.Invalidate()?;
    Ok(bitmap)
}

static DISPATCHER: OnceLock<DispatcherQueue> = OnceLock::new();

/// Remembers the calling thread's dispatcher as the UI thread for [`post`]. First call wins.
pub fn set_ui_thread() -> Result<()> {
    let _ = DISPATCHER.set(DispatcherQueue::GetForCurrentThread()?);
    Ok(())
}

/// Runs `f` on the UI thread from the queue. Returns false before [`set_ui_thread`] or after shutdown.
pub fn post(f: impl FnOnce() + Send + 'static) -> bool {
    let Some(queue) = DISPATCHER.get() else {
        return false;
    };
    let once = Mutex::new(Some(f));
    let handler = DispatcherQueueHandler::new(move || {
        if let Some(f) = once.lock().unwrap_or_else(|e| e.into_inner()).take() {
            f();
        }
        Ok(())
    });
    queue
        .TryEnqueueWithPriority(DispatcherQueuePriority::Normal, &handler)
        .unwrap_or(false)
}

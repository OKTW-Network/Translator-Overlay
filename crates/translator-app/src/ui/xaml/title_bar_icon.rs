//! Set WinUI `TitleBar.IconSource` (reactor 0.100 has no `icon` slot).
//!
//! [`crate::ui::xaml::apply`] finds the `NavigationView` on `GotFocus` and re-runs
//! [`sync`] on `Rendering` after the title bar is in the tree.

#![allow(non_snake_case)]

use std::{cell::Cell, ffi::c_void, ptr::null_mut};

use tracing::{debug, warn};
use windows::{
    Win32::{
        System::WinRT::{BSOS_DEFAULT, CreateRandomAccessStreamOverStream},
        UI::Shell::SHCreateMemStream,
    },
    core::Interface as WinInterface,
};
use windows_core::{Error, HRESULT, IInspectable, IInspectable_Vtbl, Interface, Result, RuntimeName, imp::IGenericFactory};

use crate::ui::xaml::{inspectable, visual_tree};

const PNG: &[u8] = include_bytes!("../../../assets/icon.png");

windows_core::imp::define_interface!(ITitleBar, ITitleBar_Vtbl, 0xc552714d_5d30_5a2b_9c7a_d68bea3dde8d);
#[repr(C)]
pub struct ITitleBar_Vtbl {
    base__: IInspectable_Vtbl,
    _title: usize,
    _set_title: usize,
    _subtitle: usize,
    _set_subtitle: usize,
    IconSource: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
    SetIconSource: unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT,
}

impl ITitleBar {
    fn IconSource(&self) -> Result<Option<IInspectable>> {
        let mut result = null_mut();
        unsafe {
            // SAFETY: `result` receives an `IconSource` or null; `self` is a live `TitleBar`.
            (Interface::vtable(self).IconSource)(Interface::as_raw(self), &mut result).ok()?;
        }
        if result.is_null() {
            Ok(None)
        } else {
            Ok(Some(unsafe { IInspectable::from_raw(result) }))
        }
    }

    fn SetIconSource(&self, value: &IInspectable) -> Result<()> {
        unsafe {
            // SAFETY: `value` is an `ImageIconSource`; `self` is a live `TitleBar`.
            (Interface::vtable(self).SetIconSource)(Interface::as_raw(self), Interface::as_raw(value)).ok()
        }
    }
}

windows_core::imp::define_interface!(IImageIconSource, IImageIconSource_Vtbl, 0x67f75be0_c84d_57ff_9f68_039c81ea7896);
#[repr(C)]
pub struct IImageIconSource_Vtbl {
    base__: IInspectable_Vtbl,
    _image_source: usize,
    SetImageSource: unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT,
}

impl IImageIconSource {
    fn SetImageSource(&self, value: &IInspectable) -> Result<()> {
        unsafe {
            // SAFETY: `value` is a `BitmapImage` (`ImageSource`); `self` is a live `ImageIconSource`.
            (Interface::vtable(self).SetImageSource)(Interface::as_raw(self), Interface::as_raw(value)).ok()
        }
    }
}

windows_core::imp::define_interface!(IImageIconSourceFactory, IImageIconSourceFactory_Vtbl, 0x24f76321_71bd_530a_8cc8_3f615cd1437a);
#[repr(C)]
pub struct IImageIconSourceFactory_Vtbl {
    base__: IInspectable_Vtbl,
    CreateInstance: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut *mut c_void, *mut *mut c_void) -> HRESULT,
}

windows_core::imp::define_interface!(IBitmapSource, IBitmapSource_Vtbl, 0x8424269d_9b82_534f_8fea_af5b5ef96bf2);
#[repr(C)]
pub struct IBitmapSource_Vtbl {
    base__: IInspectable_Vtbl,
    _pixel_width: usize,
    _pixel_height: usize,
    SetSource: unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT,
}

impl IBitmapSource {
    fn SetSource(&self, stream: &IInspectable) -> Result<()> {
        unsafe {
            // SAFETY: `stream` is an `IRandomAccessStream`; `self` is a live `BitmapSource`.
            (Interface::vtable(self).SetSource)(Interface::as_raw(self), Interface::as_raw(stream)).ok()
        }
    }
}

struct ImageIconSourceName;
impl RuntimeName for ImageIconSourceName {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.ImageIconSource";
}

struct BitmapImageName;
impl RuntimeName for BitmapImageName {
    const NAME: &'static str = "Microsoft.UI.Xaml.Media.Imaging.BitmapImage";
}

thread_local! {
    static WARNED: Cell<bool> = const { Cell::new(false) };
}

fn title_bar(start: &IInspectable) -> Option<IInspectable> {
    let statics = visual_tree().ok()?;
    let mut current = start.clone();
    for _ in 0..32 {
        if current.cast::<ITitleBar>().is_ok() {
            return Some(current);
        }
        if let Ok(n) = statics.GetChildrenCount(&current) {
            for i in 0..n.min(16) {
                if let Ok(child) = statics.GetChild(&current, i)
                    && child.cast::<ITitleBar>().is_ok()
                {
                    return Some(child);
                }
            }
        }
        current = statics.GetParent(&current).ok()?;
    }
    None
}

fn bitmap_image() -> Result<IInspectable> {
    let mem = unsafe {
        // SAFETY: `PNG` is a static encoded bitmap; `SHCreateMemStream` copies it.
        SHCreateMemStream(Some(PNG))
    }
    .ok_or_else(Error::empty)?;
    let ras: windows::core::IInspectable =
        unsafe { CreateRandomAccessStreamOverStream(&mem, BSOS_DEFAULT) }.map_err(|e| HRESULT(e.code().0))?;
    // SAFETY: `ras` is a newly returned WinRT `IRandomAccessStream`; `into_raw` transfers it.
    let stream = unsafe { IInspectable::from_raw(WinInterface::into_raw(ras)) };
    let factory: IGenericFactory = windows_core::factory::<BitmapImageName, IGenericFactory>()?;
    let bitmap: IInspectable = factory.ActivateInstance()?;
    bitmap.cast::<IBitmapSource>()?.SetSource(&stream)?;
    Ok(bitmap)
}

fn image_icon_source() -> Result<IInspectable> {
    let factory: IImageIconSourceFactory = windows_core::factory::<ImageIconSourceName, IImageIconSourceFactory>()?;
    unsafe {
        let mut result = null_mut();
        // SAFETY: `CreateInstance` writes the new source into `result`; base/inner are unused (null).
        (Interface::vtable(&factory).CreateInstance)(Interface::as_raw(&factory), null_mut(), null_mut(), &mut result)
            .and_then(|| inspectable(result))
    }
}

fn apply_icon(bar: &ITitleBar) -> Result<()> {
    if bar.IconSource()?.is_some() {
        return Ok(());
    }
    let bitmap = bitmap_image()?;
    let source = image_icon_source()?;
    source.cast::<IImageIconSource>()?.SetImageSource(&bitmap)?;
    bar.SetIconSource(&source)?;
    debug!("title-bar-icon: set IconSource");
    Ok(())
}

pub(crate) fn sync(nav: &IInspectable) {
    let Some(bar) = title_bar(nav) else {
        return;
    };
    let Ok(bar) = bar.cast::<ITitleBar>() else {
        return;
    };
    if let Err(e) = apply_icon(&bar) {
        WARNED.with(|warned| {
            if !warned.get() {
                warned.set(true);
                warn!(error = %e, "title-bar-icon: set failed");
            }
        });
    }
}

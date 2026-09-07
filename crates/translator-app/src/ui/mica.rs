//! NavigationView has no `resource_overrides` in reactor 0.100, so patch
//! `Application.Current.Resources` (content fill + border) to transparent.

#![allow(non_snake_case)]

use windows_collections::IMap;
use windows_core::{IInspectable, Interface, Result, RuntimeName};
use windows_reference::IReference;

windows_core::imp::define_interface!(IApplicationStatics, IApplicationStatics_Vtbl, 0x4e0d09f5_4358_512c_a987_503b52848e95);
#[repr(C)]
pub struct IApplicationStatics_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
    Current: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    Start: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
}

windows_core::imp::define_interface!(IApplication, IApplication_Vtbl, 0x06a8f4e7_1146_55af_820d_ebd55643b021);
#[repr(C)]
pub struct IApplication_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
    Resources: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}

windows_core::imp::define_interface!(ISolidColorBrush, ISolidColorBrush_Vtbl, 0xb3865c31_37c8_55c1_8a72_d41c67642e2a);
#[repr(C)]
pub struct ISolidColorBrush_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
    Color: usize,
    SetColor: unsafe extern "system" fn(*mut core::ffi::c_void, WinuiColor) -> windows_core::HRESULT,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct WinuiColor {
    a: u8,
    r: u8,
    g: u8,
    b: u8,
}

struct ApplicationName;
impl RuntimeName for ApplicationName {
    const NAME: &'static str = "Microsoft.UI.Xaml.Application";
}

struct SolidColorBrushName;
impl RuntimeName for SolidColorBrushName {
    const NAME: &'static str = "Microsoft.UI.Xaml.Media.SolidColorBrush";
}

/// Override NavigationView content fill/border so Mica shows between cards.
pub fn apply() {
    let _ = apply_inner();
}

fn apply_inner() -> Result<()> {
    let statics: IApplicationStatics = windows_core::factory::<ApplicationName, IApplicationStatics>()?;
    let app = unsafe {
        let mut result = core::mem::zeroed();
        (Interface::vtable(&statics).Current)(Interface::as_raw(&statics), &mut result).and_then(|| inspectable(result))?
    };
    let app: IApplication = app.cast()?;
    let resources = unsafe {
        let mut result = core::mem::zeroed();
        (Interface::vtable(&app).Resources)(Interface::as_raw(&app), &mut result).and_then(|| inspectable(result))?
    };
    let map: IMap<IInspectable, IInspectable> = resources.cast()?;
    let brush = transparent_brush()?;
    for key in ["NavigationViewContentBackground", "NavigationViewContentGridBorderBrush"] {
        let boxed: IInspectable = IReference::<windows_core::HSTRING>::from(windows_core::HSTRING::from(key)).into();
        map.Insert(&boxed, &brush)?;
    }
    Ok(())
}

fn transparent_brush() -> Result<IInspectable> {
    let factory: windows_core::imp::IGenericFactory = windows_core::factory::<SolidColorBrushName, windows_core::imp::IGenericFactory>()?;
    let brush: ISolidColorBrush = factory.ActivateInstance()?;
    unsafe {
        (Interface::vtable(&brush).SetColor)(Interface::as_raw(&brush), WinuiColor { a: 0, r: 0, g: 0, b: 0 }).ok()?;
    }
    brush.cast()
}

fn inspectable(ptr: *mut core::ffi::c_void) -> Result<IInspectable> {
    if ptr.is_null() {
        Err(windows_core::Error::empty())
    } else {
        Ok(unsafe { IInspectable::from_raw(ptr) })
    }
}

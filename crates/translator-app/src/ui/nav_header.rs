//! `NavViewItem::header` maps to `NavigationViewItemHeader` (plain text, not a
//! selectable item). WinUI binds `Text="{TemplateBinding Content}"`, but
//! windows-reactor sets a TextBlock as Content, so the label is blank.
//! After the native tree is attached, replace header Content with a boxed string.

use std::sync::atomic::{AtomicBool, Ordering};

use windows_core::{IInspectable, IUnknown, Interface, Result};
static DONE: AtomicBool = AtomicBool::new(false);

windows_core::imp::define_interface!(IWindow, IWindow_Vtbl, 0x61f0ec79_5d52_56b5_86fb_40fa4af288b0);
#[repr(C)]
#[allow(non_snake_case)]
pub struct IWindow_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
    _bounds: usize,
    _visible: usize,
    Content: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}

impl IWindow {
    fn content(&self) -> Result<IInspectable> {
        unsafe {
            let mut result = core::mem::zeroed();
            (Interface::vtable(self).Content)(Interface::as_raw(self), &mut result).and_then(|| inspectable(result))
        }
    }
}

windows_core::imp::define_interface!(IPanel, IPanel_Vtbl, 0x27a1b418_56f3_525e_b883_cefed905eed3);
#[repr(C)]
#[allow(non_snake_case)]
pub struct IPanel_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
    Children: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}

impl IPanel {
    fn children(&self) -> Result<ObjectVector> {
        unsafe {
            let mut result = core::mem::zeroed();
            (Interface::vtable(self).Children)(Interface::as_raw(self), &mut result).and_then(|| ObjectVector::from_abi(result))
        }
    }
}

windows_core::imp::define_interface!(IContentControl, IContentControl_Vtbl, 0x07e81761_11b2_52ae_8f8b_4d53d2b5900a);
#[repr(C)]
#[allow(non_snake_case)]
pub struct IContentControl_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
    _content: usize,
    SetContent: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
}

impl IContentControl {
    fn set_content(&self, value: &IInspectable) -> Result<()> {
        unsafe { (Interface::vtable(self).SetContent)(Interface::as_raw(self), Interface::as_raw(value)).ok() }
    }
}

windows_core::imp::define_interface!(INavigationView, INavigationView_Vtbl, 0xe77a4b36_3dd1_53d9_9f97_65dccaa74a5c);
#[repr(C)]
#[allow(non_snake_case)]
pub struct INavigationView_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
    _before_menu_items: [usize; 30],
    MenuItems: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}

impl INavigationView {
    fn menu_items(&self) -> Result<ObjectVector> {
        unsafe {
            let mut result = core::mem::zeroed();
            (Interface::vtable(self).MenuItems)(Interface::as_raw(self), &mut result).and_then(|| ObjectVector::from_abi(result))
        }
    }
}

windows_core::imp::define_interface!(INavigationViewItemHeader, INavigationViewItemHeader_Vtbl, 0x432bc062_45bc_57ef_a2d3_11851a56a882);
#[repr(C)]
#[allow(non_snake_case)]
pub struct INavigationViewItemHeader_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
}

/// `IVector<T>` GetAt/Size prefix. T is only in the IID, not this layout.
#[repr(transparent)]
#[derive(Clone)]
struct ObjectVector(IUnknown);

#[repr(C)]
#[allow(non_snake_case)]
pub struct ObjectVector_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
    GetAt: unsafe extern "system" fn(*mut core::ffi::c_void, u32, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    Size: unsafe extern "system" fn(*mut core::ffi::c_void, *mut u32) -> windows_core::HRESULT,
}

unsafe impl Interface for ObjectVector {
    type Vtable = ObjectVector_Vtbl;

    const IID: windows_core::GUID = windows_core::GUID::from_u128(0);
}

impl ObjectVector {
    fn from_abi(ptr: *mut core::ffi::c_void) -> Result<Self> {
        if ptr.is_null() {
            Err(windows_core::Error::empty())
        } else {
            Ok(unsafe { Self::from_raw(ptr) })
        }
    }

    fn size(&self) -> Result<u32> {
        unsafe {
            let mut result = 0;
            (Interface::vtable(self).Size)(Interface::as_raw(self), &mut result).map(|| result)
        }
    }

    fn get_at(&self, index: u32) -> Result<IInspectable> {
        unsafe {
            let mut result = core::mem::zeroed();
            (Interface::vtable(self).GetAt)(Interface::as_raw(self), index, &mut result).and_then(|| inspectable(result))
        }
    }
}

fn inspectable(ptr: *mut core::ffi::c_void) -> Result<IInspectable> {
    if ptr.is_null() {
        Err(windows_core::Error::empty())
    } else {
        Ok(unsafe { IInspectable::from_raw(ptr) })
    }
}

/// Replace TextBlock Content on every `NavigationViewItemHeader` with a boxed string.
/// Returns `true` once that has succeeded.
pub fn retarget() -> bool {
    if DONE.load(Ordering::Relaxed) {
        return true;
    }
    let Some(content) =
        windows_reactor::with_active_host(|host| host.window().cast::<IWindow>().ok().and_then(|w| w.content().ok())).flatten()
    else {
        return false;
    };
    if let Some(nav) = find_nav(&content)
        && retarget_headers(&nav)
    {
        DONE.store(true, Ordering::Relaxed);
        return true;
    }
    false
}

fn find_nav(root: &IInspectable) -> Option<INavigationView> {
    if let Ok(nav) = root.cast() {
        return Some(nav);
    }
    let children = root.cast::<IPanel>().ok()?.children().ok()?;
    let n = children.size().ok()?;
    (0..n).find_map(|i| children.get_at(i).ok()?.cast().ok())
}

fn retarget_headers(nav: &INavigationView) -> bool {
    let Ok(items) = nav.menu_items() else {
        return false;
    };
    let Ok(n) = items.size() else {
        return false;
    };
    let mut any = false;
    for i in 0..n {
        let Ok(item) = items.get_at(i) else {
            continue;
        };
        if item.cast::<INavigationViewItemHeader>().is_err() {
            continue;
        }
        let Ok(cc) = item.cast::<IContentControl>() else {
            continue;
        };
        let boxed: IInspectable = windows_reference::IReference::<windows_core::HSTRING>::from("Settings").into();
        if cc.set_content(&boxed).is_ok() {
            any = true;
        }
    }
    any
}

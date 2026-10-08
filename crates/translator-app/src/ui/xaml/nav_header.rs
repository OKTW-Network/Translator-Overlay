//! Insert a native `NavigationViewItemHeader` ("Settings") into the pane.
//!
//! windows-reactor 0.100 has no header widget, so this talks to WinUI over COM.
//! [`crate::ui::xaml::apply`] finds the `NavigationView` on `GotFocus` and runs
//! [`sync`] again on `Rendering` after windows-reactor reconciles `MenuItems`.

#![allow(non_snake_case)]

use std::{ffi::c_void, mem::zeroed, ptr::null_mut};

use rust_i18n::t;
use tracing::{debug, warn};
use windows_collections::IVector;
use windows_core::{HRESULT, HSTRING, IInspectable, IInspectable_Vtbl, Interface, Result, RuntimeName};
use windows_reference::IReference;

use crate::ui::xaml::{inspectable, visual_tree};

windows_core::imp::define_interface!(INavigationView, INavigationView_Vtbl, 0xe77a4b36_3dd1_53d9_9f97_65dccaa74a5c);
#[repr(C)]
pub struct INavigationView_Vtbl {
    base__: IInspectable_Vtbl,
    // Vtable slots before `MenuItems` (do not reorder).
    _before_menu_items: [usize; 30],
    MenuItems: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
}

windows_core::imp::define_interface!(INavigationViewItemHeader, INavigationViewItemHeader_Vtbl, 0x432bc062_45bc_57ef_a2d3_11851a56a882);
#[repr(C)]
pub struct INavigationViewItemHeader_Vtbl {
    base__: IInspectable_Vtbl,
}

windows_core::imp::define_interface!(
    INavigationViewItemHeaderFactory,
    INavigationViewItemHeaderFactory_Vtbl,
    0x6a5447cd_2918_5fe3_899b_93d6961285e6
);
#[repr(C)]
pub struct INavigationViewItemHeaderFactory_Vtbl {
    base__: IInspectable_Vtbl,
    CreateInstance: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut *mut c_void, *mut *mut c_void) -> HRESULT,
}

windows_core::imp::define_interface!(IContentControl, IContentControl_Vtbl, 0x07e81761_11b2_52ae_8f8b_4d53d2b5900a);
#[repr(C)]
pub struct IContentControl_Vtbl {
    base__: IInspectable_Vtbl,
    // `get_Content` (unused).
    _content: usize,
    SetContent: unsafe extern "system" fn(*mut c_void, *mut c_void) -> HRESULT,
}

struct NavigationViewItemHeaderName;
impl RuntimeName for NavigationViewItemHeaderName {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.NavigationViewItemHeader";
}

/// Walk the ancestors, and a few children, until a `NavigationView` turns up.
pub(crate) fn navigation_view(start: &IInspectable) -> Option<IInspectable> {
    let statics = visual_tree().ok()?;
    let mut current = start.clone();
    for _ in 0..32 {
        if current.cast::<INavigationView>().is_ok() {
            return Some(current);
        }
        if let Ok(n) = statics.GetChildrenCount(&current) {
            for i in 0..n.min(16) {
                if let Ok(child) = statics.GetChild(&current, i)
                    && child.cast::<INavigationView>().is_ok()
                {
                    return Some(child);
                }
            }
        }
        current = statics.GetParent(&current).ok()?;
    }
    None
}

/// Insert "Settings" at `MenuItems[1]` unless a header is already there.
fn ensure_settings_header(nav: &IInspectable) -> Result<()> {
    let nav: INavigationView = nav.cast()?;
    let items = unsafe {
        let mut result = zeroed();
        // SAFETY: `MenuItems` writes a new `IVector` pointer into `result`.
        (Interface::vtable(&nav).MenuItems)(Interface::as_raw(&nav), &mut result).and_then(|| inspectable(result))?
    };
    let items: IVector<IInspectable> = items.cast()?;
    // Dashboard is item 0, and at least one settings item must follow the header slot.
    if items.Size()? < 2 {
        return Ok(());
    }
    if items.GetAt(1)?.cast::<INavigationViewItemHeader>().is_ok() {
        set_settings_label(&items.GetAt(1)?)?;
        return Ok(());
    }
    let header_factory: INavigationViewItemHeaderFactory =
        windows_core::factory::<NavigationViewItemHeaderName, INavigationViewItemHeaderFactory>()?;
    let header = unsafe {
        let mut result = zeroed();
        // SAFETY: `CreateInstance` writes the new header into `result`. The base and inner pointers are null and unused.
        (Interface::vtable(&header_factory).CreateInstance)(Interface::as_raw(&header_factory), null_mut(), null_mut(), &mut result)
            .and_then(|| inspectable(result))?
    };
    set_settings_label(&header)?;
    items.InsertAt(1, &header)?;
    debug!("nav-header: inserted Settings header");
    Ok(())
}

fn set_settings_label(header: &IInspectable) -> Result<()> {
    let content: IContentControl = header.cast()?;
    let label = t!("nav.settings");
    let boxed: IInspectable = IReference::<HSTRING>::from(HSTRING::from(&*label)).into();
    unsafe {
        // SAFETY: `boxed` is a boxed `HSTRING` matching `ContentControl.Content`.
        (Interface::vtable(&content).SetContent)(Interface::as_raw(&content), Interface::as_raw(&boxed)).ok()?;
    }
    Ok(())
}

pub(crate) fn sync(nav: &IInspectable) {
    if let Err(e) = ensure_settings_header(nav) {
        warn!(error = %e, "nav-header: insert failed");
    }
}

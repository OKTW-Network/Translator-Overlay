//! Insert a native `NavigationViewItemHeader` ("Settings") into the pane.
//!
//! windows-reactor 0.100 has no header widget, so this talks to WinUI over COM:
//! `FocusManager.GotFocus` walks to the `NavigationView`, then
//! `CompositionTarget.Rendering` re-inserts the header after reactor reconciles
//! `MenuItems`. Cached COM pointers are forgotten on TLS drop — WinRT `Release`
//! during thread teardown aborts WinUI (`0xC0000409`).

#![allow(non_snake_case)]

use std::{
    cell::RefCell,
    ffi::c_void,
    marker::PhantomData,
    mem::{forget, transmute, transmute_copy, zeroed},
    ptr::null_mut,
};

use tracing::{debug, warn};
use windows_collections::IVector;
use windows_core::{
    Error, EventRevoker, GUID, HRESULT, HSTRING, IInspectable, IInspectable_Vtbl, IUnknown, IUnknown_Vtbl, Interface, Ref, Result,
    RuntimeName, RuntimeType,
    imp::{AbiType, ConstBuffer, DelegateBox, box_new},
};
use windows_reference::IReference;

windows_core::imp::define_interface!(IFocusManagerStatics, IFocusManagerStatics_Vtbl, 0xe73dce04_e23a_5fb3_96ab_7df04c51dff2);
#[repr(C)]
pub struct IFocusManagerStatics_Vtbl {
    base__: IInspectable_Vtbl,
    GotFocus: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut i64) -> HRESULT,
    RemoveGotFocus: unsafe extern "system" fn(*mut c_void, i64) -> HRESULT,
}

windows_core::imp::define_interface!(
    IFocusManagerGotFocusEventArgs,
    IFocusManagerGotFocusEventArgs_Vtbl,
    0x50aca341_4519_59cf_83b1_c9c45cfdb816
);
#[repr(C)]
pub struct IFocusManagerGotFocusEventArgs_Vtbl {
    base__: IInspectable_Vtbl,
    NewFocusedElement: unsafe extern "system" fn(*mut c_void, *mut *mut c_void) -> HRESULT,
}

impl RuntimeType for IFocusManagerGotFocusEventArgs {
    const SIGNATURE: ConstBuffer = ConstBuffer::for_interface::<Self>();
}

#[repr(transparent)]
#[derive(Clone, PartialEq, Eq, Debug)]
struct FocusManagerGotFocusEventArgs(IUnknown);

windows_core::imp::interface_hierarchy!(FocusManagerGotFocusEventArgs, IUnknown, IInspectable);

unsafe impl Interface for FocusManagerGotFocusEventArgs {
    type Vtable = IFocusManagerGotFocusEventArgs_Vtbl;

    const IID: GUID = <IFocusManagerGotFocusEventArgs as Interface>::IID;
}

impl RuntimeName for FocusManagerGotFocusEventArgs {
    const NAME: &'static str = "Microsoft.UI.Xaml.Input.FocusManagerGotFocusEventArgs";
}

impl RuntimeType for FocusManagerGotFocusEventArgs {
    const SIGNATURE: ConstBuffer = ConstBuffer::for_class::<Self, IFocusManagerGotFocusEventArgs>();
}

windows_core::imp::define_interface!(ICompositionTargetStatics, ICompositionTargetStatics_Vtbl, 0x12a4be6f_6db1_5165_b622_d57ab782745b);
#[repr(C)]
pub struct ICompositionTargetStatics_Vtbl {
    base__: IInspectable_Vtbl,
    Rendering: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut i64) -> HRESULT,
    RemoveRendering: unsafe extern "system" fn(*mut c_void, i64) -> HRESULT,
}

windows_core::imp::define_interface!(IVisualTreeHelperStatics, IVisualTreeHelperStatics_Vtbl, 0x5aece43c_7651_5bb5_855c_2198496e455e);
#[repr(C)]
pub struct IVisualTreeHelperStatics_Vtbl {
    base__: IInspectable_Vtbl,
    // `FindElementsInHostCoordinates` overloads (unused).
    _find: [usize; 4],
    GetChild: unsafe extern "system" fn(*mut c_void, *mut c_void, i32, *mut *mut c_void) -> HRESULT,
    GetChildrenCount: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut i32) -> HRESULT,
    GetParent: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut *mut c_void) -> HRESULT,
}

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

/// WinRT `Windows.Foundation.EventHandler<T>` (`{9de1c535-6ae1-11e0-84e1-18a905bcc53f}`).
#[repr(transparent)]
#[derive(Clone, PartialEq, Eq, Debug)]
struct EventHandler<T>(IUnknown, PhantomData<T>);

unsafe impl<T: RuntimeType + 'static> Interface for EventHandler<T> {
    type Vtable = EventHandlerVtbl<T>;

    const IID: GUID = GUID::from_signature(<Self as RuntimeType>::SIGNATURE);
}

impl<T: RuntimeType + 'static> RuntimeType for EventHandler<T> {
    const SIGNATURE: ConstBuffer = ConstBuffer::new()
        .push_slice(b"pinterface({9de1c535-6ae1-11e0-84e1-18a905bcc53f}")
        .push_slice(b";")
        .push_other(T::SIGNATURE)
        .push_slice(b")");
}

#[repr(C)]
struct EventHandlerVtbl<T: RuntimeType + 'static> {
    base__: IUnknown_Vtbl,
    Invoke: unsafe extern "system" fn(*mut c_void, *mut c_void, AbiType<T>) -> HRESULT,
    _t: PhantomData<T>,
}

impl<T: RuntimeType + 'static> EventHandler<T> {
    fn new<F>(invoke: F) -> Self
    where
        F: Fn(Ref<IInspectable>, Ref<T>) + 'static,
    {
        let com = DelegateBox::<Self, F>::new(&EventHandlerBox::<T, F>::VTABLE, invoke);
        // SAFETY: `box_new` produces a COM object whose payload is this `EventHandler`.
        unsafe { transmute(box_new(com)) }
    }
}

struct EventHandlerBox<T, F>(PhantomData<(T, fn() -> F)>);

impl<T, F> EventHandlerBox<T, F>
where
    T: RuntimeType + 'static,
    F: Fn(Ref<IInspectable>, Ref<T>) + 'static,
{
    const VTABLE: EventHandlerVtbl<T> = EventHandlerVtbl::<T> {
        base__: IUnknown_Vtbl {
            QueryInterface: DelegateBox::<EventHandler<T>, F>::QueryInterface,
            AddRef: DelegateBox::<EventHandler<T>, F>::AddRef,
            Release: DelegateBox::<EventHandler<T>, F>::Release,
        },
        Invoke: Self::Invoke,
        _t: PhantomData,
    };

    unsafe extern "system" fn Invoke(this: *mut c_void, sender: *mut c_void, args: AbiType<T>) -> HRESULT {
        // SAFETY: `this` is a `DelegateBox<EventHandler<T>, F>` from `EventHandler::new`.
        // `sender` / `args` are WinRT in-pointers for this invoke; `transmute_copy` borrows them.
        unsafe {
            let this = &mut *(this as *mut *mut c_void as *mut DelegateBox<EventHandler<T>, F>);
            (this.invoke)(transmute_copy(&sender), transmute_copy(&args));
            HRESULT(0)
        }
    }
}

struct FocusManagerName;
impl RuntimeName for FocusManagerName {
    const NAME: &'static str = "Microsoft.UI.Xaml.Input.FocusManager";
}

struct CompositionTargetName;
impl RuntimeName for CompositionTargetName {
    const NAME: &'static str = "Microsoft.UI.Xaml.Media.CompositionTarget";
}

struct VisualTreeHelperName;
impl RuntimeName for VisualTreeHelperName {
    const NAME: &'static str = "Microsoft.UI.Xaml.Media.VisualTreeHelper";
}

struct NavigationViewItemHeaderName;
impl RuntimeName for NavigationViewItemHeaderName {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.NavigationViewItemHeader";
}

/// Cached `NavigationView`. Drop must not `Release` (WinUI aborts in TLS teardown).
struct CachedNavView(Option<IInspectable>);

impl Drop for CachedNavView {
    fn drop(&mut self) {
        if let Some(nav) = self.0.take() {
            forget(nav);
        }
    }
}

thread_local! {
    static NAV_VIEW: RefCell<CachedNavView> = const { RefCell::new(CachedNavView(None)) };
}

fn inspectable(ptr: *mut c_void) -> Result<IInspectable> {
    if ptr.is_null() {
        Err(Error::empty())
    } else {
        // SAFETY: `ptr` is a newly returned WinRT object; `from_raw` takes ownership.
        Ok(unsafe { IInspectable::from_raw(ptr) })
    }
}

/// Walk ancestors (and a few children) until a `NavigationView` is found.
fn find_navigation_view(start: &IInspectable) -> Option<IInspectable> {
    let statics: IVisualTreeHelperStatics = windows_core::factory::<VisualTreeHelperName, IVisualTreeHelperStatics>().ok()?;
    let mut current = start.clone();
    for _ in 0..32 {
        if current.cast::<INavigationView>().is_ok() {
            return Some(current);
        }
        let mut n = 0i32;
        // SAFETY: `current` is a live XAML `DependencyObject`; count/child/parent out-pointers are written by WinRT.
        if unsafe { (Interface::vtable(&statics).GetChildrenCount)(Interface::as_raw(&statics), Interface::as_raw(&current), &mut n) }
            .is_ok()
        {
            for i in 0..n.min(16) {
                let mut child = null_mut();
                if unsafe {
                    (Interface::vtable(&statics).GetChild)(Interface::as_raw(&statics), Interface::as_raw(&current), i, &mut child)
                }
                .is_ok()
                    && let Ok(child) = inspectable(child)
                    && child.cast::<INavigationView>().is_ok()
                {
                    return Some(child);
                }
            }
        }
        let mut parent = null_mut();
        unsafe { (Interface::vtable(&statics).GetParent)(Interface::as_raw(&statics), Interface::as_raw(&current), &mut parent) }
            .ok()
            .ok()?;
        current = inspectable(parent).ok()?;
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
    // Dashboard is [0]; need at least one settings item after the header slot.
    if items.Size()? < 2 {
        return Ok(());
    }
    if items.GetAt(1)?.cast::<INavigationViewItemHeader>().is_ok() {
        return Ok(());
    }
    let header_factory: INavigationViewItemHeaderFactory =
        windows_core::factory::<NavigationViewItemHeaderName, INavigationViewItemHeaderFactory>()?;
    let header = unsafe {
        let mut result = zeroed();
        // SAFETY: `CreateInstance` writes the new header into `result`; base/inner are unused (null).
        (Interface::vtable(&header_factory).CreateInstance)(Interface::as_raw(&header_factory), null_mut(), null_mut(), &mut result)
            .and_then(|| inspectable(result))?
    };
    let content: IContentControl = header.cast()?;
    let boxed: IInspectable = IReference::<HSTRING>::from(HSTRING::from("Settings")).into();
    unsafe {
        // SAFETY: `boxed` is a boxed `HSTRING` matching `ContentControl.Content`.
        (Interface::vtable(&content).SetContent)(Interface::as_raw(&content), Interface::as_raw(&boxed)).ok()?;
    }
    items.InsertAt(1, &header)?;
    debug!("nav-header: inserted Settings header");
    Ok(())
}

fn subscribe_got_focus() -> Result<()> {
    let statics: IFocusManagerStatics = windows_core::factory::<FocusManagerName, IFocusManagerStatics>()?;
    let handler = EventHandler::<FocusManagerGotFocusEventArgs>::new(|_sender, args| {
        let Some(args) = args.as_ref() else {
            return;
        };
        let mut result = null_mut();
        let Ok(element) = (unsafe {
            // SAFETY: `NewFocusedElement` writes a XAML element pointer into `result`.
            (Interface::vtable(args).NewFocusedElement)(Interface::as_raw(args), &mut result).and_then(|| inspectable(result))
        }) else {
            return;
        };
        let Some(nav) = find_navigation_view(&element) else {
            return;
        };
        NAV_VIEW.with(|slot| slot.borrow_mut().0 = Some(nav.clone()));
        if let Err(e) = ensure_settings_header(&nav) {
            warn!(error = %e, "nav-header: insert failed");
        }
    });
    unsafe {
        let mut token = 0i64;
        // SAFETY: `GotFocus` writes the registration token; `EventRevoker` takes it and is forgotten for process lifetime.
        (Interface::vtable(&statics).GotFocus)(Interface::as_raw(&statics), Interface::as_raw(&handler), &mut token).ok()?;
        let remove = Interface::vtable(&statics).RemoveGotFocus;
        EventRevoker::new(statics, token, remove).forget();
    }
    Ok(())
}

fn subscribe_rendering() -> Result<()> {
    let statics: ICompositionTargetStatics = windows_core::factory::<CompositionTargetName, ICompositionTargetStatics>()?;
    let handler = EventHandler::<IInspectable>::new(|_, _| {
        let Some(nav) = NAV_VIEW.with(|slot| slot.borrow().0.clone()) else {
            return;
        };
        if let Err(e) = ensure_settings_header(&nav) {
            warn!(error = %e, "nav-header: insert failed");
        }
    });
    unsafe {
        let mut token = 0i64;
        // SAFETY: `Rendering` writes the registration token; `EventRevoker` takes it and is forgotten for process lifetime.
        (Interface::vtable(&statics).Rendering)(Interface::as_raw(&statics), Interface::as_raw(&handler), &mut token).ok()?;
        let remove = Interface::vtable(&statics).RemoveRendering;
        EventRevoker::new(statics, token, remove).forget();
    }
    Ok(())
}

/// Subscribe to `GotFocus` + `Rendering` so the Settings header survives reconcile.
pub fn apply() {
    if let Err(e) = subscribe_got_focus() {
        warn!(error = %e, "nav-header: GotFocus subscribe failed");
    }
    if let Err(e) = subscribe_rendering() {
        warn!(error = %e, "nav-header: Rendering subscribe failed");
    }
}

//! COM-insert NavigationViewItemHeader; GotFocus finds the view, Rendering reinstalls after reconcile.

#![allow(non_snake_case)]

use std::cell::RefCell;

use tracing::{info, warn};
use windows_collections::IVector;
use windows_core::{IInspectable, Interface, Result, RuntimeName, RuntimeType};
use windows_reference::IReference;

windows_core::imp::define_interface!(IFocusManagerStatics, IFocusManagerStatics_Vtbl, 0xe73dce04_e23a_5fb3_96ab_7df04c51dff2);
#[repr(C)]
pub struct IFocusManagerStatics_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
    GotFocus: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    RemoveGotFocus: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}

windows_core::imp::define_interface!(IGotFocusArgs, IGotFocusArgs_Vtbl, 0x50aca341_4519_59cf_83b1_c9c45cfdb816);
#[repr(C)]
pub struct IGotFocusArgs_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
    NewFocusedElement: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}

impl RuntimeType for IGotFocusArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}

#[repr(transparent)]
#[derive(Clone, PartialEq, Eq, Debug)]
struct GotFocusArgs(windows_core::IUnknown);

windows_core::imp::interface_hierarchy!(GotFocusArgs, windows_core::IUnknown, windows_core::IInspectable);

unsafe impl Interface for GotFocusArgs {
    type Vtable = IGotFocusArgs_Vtbl;

    const IID: windows_core::GUID = <IGotFocusArgs as Interface>::IID;
}

impl RuntimeName for GotFocusArgs {
    const NAME: &'static str = "Microsoft.UI.Xaml.Input.FocusManagerGotFocusEventArgs";
}

impl RuntimeType for GotFocusArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IGotFocusArgs>();
}

windows_core::imp::define_interface!(ICompositionTargetStatics, ICompositionTargetStatics_Vtbl, 0x12a4be6f_6db1_5165_b622_d57ab782745b);
#[repr(C)]
pub struct ICompositionTargetStatics_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
    Rendering: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    RemoveRendering: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}

windows_core::imp::define_interface!(IVisualTreeHelperStatics, IVisualTreeHelperStatics_Vtbl, 0x5aece43c_7651_5bb5_855c_2198496e455e);
#[repr(C)]
pub struct IVisualTreeHelperStatics_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
    _find: [usize; 4],
    GetChild: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        i32,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
    GetChildrenCount: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i32) -> windows_core::HRESULT,
    GetParent:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}

windows_core::imp::define_interface!(INavigationView, INavigationView_Vtbl, 0xe77a4b36_3dd1_53d9_9f97_65dccaa74a5c);
#[repr(C)]
pub struct INavigationView_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
    _before_menu_items: [usize; 30],
    MenuItems: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}

windows_core::imp::define_interface!(INavigationViewItemHeader, INavigationViewItemHeader_Vtbl, 0x432bc062_45bc_57ef_a2d3_11851a56a882);
#[repr(C)]
pub struct INavigationViewItemHeader_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
}

windows_core::imp::define_interface!(IHeaderFactory, IHeaderFactory_Vtbl, 0x6a5447cd_2918_5fe3_899b_93d6961285e6);
#[repr(C)]
pub struct IHeaderFactory_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
    CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}

windows_core::imp::define_interface!(IContentControl, IContentControl_Vtbl, 0x07e81761_11b2_52ae_8f8b_4d53d2b5900a);
#[repr(C)]
pub struct IContentControl_Vtbl {
    base__: windows_core::IInspectable_Vtbl,
    _content: usize,
    SetContent: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
}

#[repr(transparent)]
#[derive(Clone, PartialEq, Eq, Debug)]
struct EventHandler<T>(windows_core::IUnknown, core::marker::PhantomData<T>);

unsafe impl<T: RuntimeType + 'static> Interface for EventHandler<T> {
    type Vtable = EventHandlerVtbl<T>;

    const IID: windows_core::GUID = windows_core::GUID::from_signature(<Self as RuntimeType>::SIGNATURE);
}

impl<T: RuntimeType + 'static> RuntimeType for EventHandler<T> {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::new()
        .push_slice(b"pinterface({9de1c535-6ae1-11e0-84e1-18a905bcc53f}")
        .push_slice(b";")
        .push_other(T::SIGNATURE)
        .push_slice(b")");
}

#[repr(C)]
struct EventHandlerVtbl<T: RuntimeType + 'static> {
    base__: windows_core::IUnknown_Vtbl,
    Invoke:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, windows_core::imp::AbiType<T>) -> windows_core::HRESULT,
    _t: core::marker::PhantomData<T>,
}

impl<T: RuntimeType + 'static> EventHandler<T> {
    fn new<F>(invoke: F) -> Self
    where
        F: Fn(windows_core::Ref<IInspectable>, windows_core::Ref<T>) + 'static,
    {
        let com = windows_core::imp::DelegateBox::<Self, F>::new(&EventHandlerBox::<T, F>::VTABLE, invoke);
        unsafe { core::mem::transmute(windows_core::imp::box_new(com)) }
    }
}

struct EventHandlerBox<T, F>(core::marker::PhantomData<(T, fn() -> F)>);

impl<T, F> EventHandlerBox<T, F>
where
    T: RuntimeType + 'static,
    F: Fn(windows_core::Ref<IInspectable>, windows_core::Ref<T>) + 'static,
{
    const VTABLE: EventHandlerVtbl<T> = EventHandlerVtbl::<T> {
        base__: windows_core::IUnknown_Vtbl {
            QueryInterface: windows_core::imp::DelegateBox::<EventHandler<T>, F>::QueryInterface,
            AddRef: windows_core::imp::DelegateBox::<EventHandler<T>, F>::AddRef,
            Release: windows_core::imp::DelegateBox::<EventHandler<T>, F>::Release,
        },
        Invoke: Self::Invoke,
        _t: core::marker::PhantomData,
    };

    unsafe extern "system" fn Invoke(
        this: *mut core::ffi::c_void,
        sender: *mut core::ffi::c_void,
        args: windows_core::imp::AbiType<T>,
    ) -> windows_core::HRESULT {
        unsafe {
            let this = &mut *(this as *mut *mut core::ffi::c_void as *mut windows_core::imp::DelegateBox<EventHandler<T>, F>);
            (this.invoke)(core::mem::transmute_copy(&sender), core::mem::transmute_copy(&args));
            windows_core::HRESULT(0)
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

struct HeaderName;
impl RuntimeName for HeaderName {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.NavigationViewItemHeader";
}

/// WinRT `Release` from a TLS destructor aborts WinUI (`0xC0000409`).
struct TlsNav(Option<IInspectable>);

impl Drop for TlsNav {
    fn drop(&mut self) {
        if let Some(nav) = self.0.take() {
            core::mem::forget(nav);
        }
    }
}

thread_local! {
    static NAV: RefCell<TlsNav> = const { RefCell::new(TlsNav(None)) };
}

fn inspectable(ptr: *mut core::ffi::c_void) -> Result<IInspectable> {
    if ptr.is_null() {
        Err(windows_core::Error::empty())
    } else {
        Ok(unsafe { IInspectable::from_raw(ptr) })
    }
}

fn walk_nav(start: &IInspectable) -> Option<IInspectable> {
    let statics: IVisualTreeHelperStatics = windows_core::factory::<VisualTreeHelperName, IVisualTreeHelperStatics>().ok()?;
    let mut cur = start.clone();
    for _ in 0..32 {
        if cur.cast::<INavigationView>().is_ok() {
            return Some(cur);
        }
        let mut n = 0i32;
        if unsafe { (Interface::vtable(&statics).GetChildrenCount)(Interface::as_raw(&statics), Interface::as_raw(&cur), &mut n) }.is_ok() {
            for i in 0..n.min(16) {
                let mut child = core::ptr::null_mut();
                if unsafe { (Interface::vtable(&statics).GetChild)(Interface::as_raw(&statics), Interface::as_raw(&cur), i, &mut child) }
                    .is_ok()
                    && let Ok(child) = inspectable(child)
                    && child.cast::<INavigationView>().is_ok()
                {
                    return Some(child);
                }
            }
        }
        let mut parent = core::ptr::null_mut();
        unsafe { (Interface::vtable(&statics).GetParent)(Interface::as_raw(&statics), Interface::as_raw(&cur), &mut parent) }
            .ok()
            .ok()?;
        cur = inspectable(parent).ok()?;
    }
    None
}

fn install(nav: &IInspectable) -> Result<()> {
    let nav: INavigationView = nav.cast()?;
    let items = unsafe {
        let mut result = core::mem::zeroed();
        (Interface::vtable(&nav).MenuItems)(Interface::as_raw(&nav), &mut result).and_then(|| inspectable(result))?
    };
    let items: IVector<IInspectable> = items.cast()?;
    if items.Size()? < 2 {
        return Ok(());
    }
    if items.GetAt(1)?.cast::<INavigationViewItemHeader>().is_ok() {
        return Ok(());
    }
    let factory: IHeaderFactory = windows_core::factory::<HeaderName, IHeaderFactory>()?;
    let header = unsafe {
        let mut result = core::mem::zeroed();
        (Interface::vtable(&factory).CreateInstance)(Interface::as_raw(&factory), core::ptr::null_mut(), core::ptr::null_mut(), &mut result)
            .and_then(|| inspectable(result))?
    };
    let content: IContentControl = header.cast()?;
    let boxed: IInspectable = IReference::<windows_core::HSTRING>::from(windows_core::HSTRING::from("Settings")).into();
    unsafe {
        (Interface::vtable(&content).SetContent)(Interface::as_raw(&content), Interface::as_raw(&boxed)).ok()?;
    }
    items.InsertAt(1, &header)?;
    info!("nav-header: inserted HeaderText");
    Ok(())
}

fn subscribe_got_focus() -> Result<()> {
    let statics: IFocusManagerStatics = windows_core::factory::<FocusManagerName, IFocusManagerStatics>()?;
    let handler = EventHandler::<GotFocusArgs>::new(|_sender, args| {
        let Some(args) = args.as_ref() else {
            return;
        };
        let mut result = core::ptr::null_mut();
        let Ok(el) =
            (unsafe { (Interface::vtable(args).NewFocusedElement)(Interface::as_raw(args), &mut result).and_then(|| inspectable(result)) })
        else {
            return;
        };
        let Some(nav) = walk_nav(&el) else {
            return;
        };
        NAV.with(|slot| slot.borrow_mut().0 = Some(nav.clone()));
        if let Err(e) = install(&nav) {
            warn!(error = %e, "nav-header: insert failed");
        }
    });
    unsafe {
        let mut token = 0i64;
        (Interface::vtable(&statics).GotFocus)(Interface::as_raw(&statics), Interface::as_raw(&handler), &mut token).ok()?;
        let remove = Interface::vtable(&statics).RemoveGotFocus;
        windows_core::EventRevoker::new(statics, token, remove).forget();
    }
    Ok(())
}

fn subscribe_rendering() -> Result<()> {
    let statics: ICompositionTargetStatics = windows_core::factory::<CompositionTargetName, ICompositionTargetStatics>()?;
    let handler = EventHandler::<IInspectable>::new(|_, _| {
        let Some(nav) = NAV.with(|nav| nav.borrow().0.clone()) else {
            return;
        };
        if let Err(e) = install(&nav) {
            warn!(error = %e, "nav-header: insert failed");
        }
    });
    unsafe {
        let mut token = 0i64;
        (Interface::vtable(&statics).Rendering)(Interface::as_raw(&statics), Interface::as_raw(&handler), &mut token).ok()?;
        let remove = Interface::vtable(&statics).RemoveRendering;
        windows_core::EventRevoker::new(statics, token, remove).forget();
    }
    Ok(())
}

pub fn apply() {
    if let Err(e) = subscribe_got_focus() {
        warn!(error = %e, "nav-header: GotFocus subscribe failed");
    }
    if let Err(e) = subscribe_rendering() {
        warn!(error = %e, "nav-header: Rendering subscribe failed");
    }
}

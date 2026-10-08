//! Shared WinUI `GotFocus` + `Rendering` hook, plus COM patches (Mica, nav header, slider step).
//!
//! Cached COM pointers are forgotten on TLS drop — WinRT `Release` during thread
//! teardown aborts WinUI (`0xC0000409`).

#![allow(non_snake_case)]

mod mica;
mod nav_header;
mod range_step;

use std::{
    cell::{Cell, RefCell},
    ffi::c_void,
    marker::PhantomData,
    mem::{forget, transmute, transmute_copy},
    ptr::null_mut,
};

use tracing::warn;
use windows_core::{
    Error, EventRevoker, GUID, HRESULT, IInspectable, IInspectable_Vtbl, IUnknown, IUnknown_Vtbl, Interface, Ref, Result, RuntimeName,
    RuntimeType,
    imp::{AbiType, ConstBuffer, DelegateBox, box_new},
};

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

impl FocusManagerGotFocusEventArgs {
    fn NewFocusedElement(&self) -> Result<IInspectable> {
        let mut result = null_mut();
        unsafe {
            // SAFETY: `NewFocusedElement` writes a XAML element pointer into `result`.
            (Interface::vtable(self).NewFocusedElement)(Interface::as_raw(self), &mut result).and_then(|| inspectable(result))
        }
    }
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
    _find: [usize; 4],
    GetChild: unsafe extern "system" fn(*mut c_void, *mut c_void, i32, *mut *mut c_void) -> HRESULT,
    GetChildrenCount: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut i32) -> HRESULT,
    GetParent: unsafe extern "system" fn(*mut c_void, *mut c_void, *mut *mut c_void) -> HRESULT,
}

impl IVisualTreeHelperStatics {
    pub(crate) fn GetChild(&self, reference: &IInspectable, child_index: i32) -> Result<IInspectable> {
        let mut child = null_mut();
        unsafe {
            // SAFETY: `reference` is a live XAML `DependencyObject`; `child` is written by WinRT.
            (Interface::vtable(self).GetChild)(Interface::as_raw(self), Interface::as_raw(reference), child_index, &mut child)
                .and_then(|| inspectable(child))
        }
    }

    pub(crate) fn GetChildrenCount(&self, reference: &IInspectable) -> Result<i32> {
        let mut n = 0i32;
        unsafe {
            // SAFETY: `reference` is a live XAML `DependencyObject`; `n` is written by WinRT.
            (Interface::vtable(self).GetChildrenCount)(Interface::as_raw(self), Interface::as_raw(reference), &mut n).map(|| n)
        }
    }

    pub(crate) fn GetParent(&self, reference: &IInspectable) -> Result<IInspectable> {
        let mut parent = null_mut();
        unsafe {
            // SAFETY: `reference` is a live XAML `DependencyObject`; `parent` is written by WinRT.
            (Interface::vtable(self).GetParent)(Interface::as_raw(self), Interface::as_raw(reference), &mut parent)
                .and_then(|| inspectable(parent))
        }
    }
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
    /// Rendering ticks left in which to re-run [`on_tree`] after the last view rebuild.
    static PENDING_FRAMES: Cell<u8> = const { Cell::new(0) };
}

/// Frames to re-sync after a view rebuild. Reactor reconciles `MenuItems` after `view`,
/// and new pages get their visual children during the next layout pass.
const SYNC_FRAMES: u8 = 3;

pub(crate) fn inspectable(ptr: *mut c_void) -> Result<IInspectable> {
    if ptr.is_null() {
        Err(Error::empty())
    } else {
        // SAFETY: `ptr` is a newly returned WinRT object; `from_raw` takes ownership.
        Ok(unsafe { IInspectable::from_raw(ptr) })
    }
}

pub(crate) fn visual_tree() -> Result<IVisualTreeHelperStatics> {
    windows_core::factory::<VisualTreeHelperName, IVisualTreeHelperStatics>()
}

pub(crate) fn refresh_nav_header() {
    PENDING_FRAMES.set(SYNC_FRAMES);
    NAV_VIEW.with(|slot| {
        if let Some(nav) = slot.borrow().0.clone() {
            nav_header::sync(&nav);
        }
    });
}

fn on_tree(nav: &IInspectable) {
    nav_header::sync(nav);
    range_step::sync(nav);
}

fn subscribe_got_focus() -> Result<()> {
    let statics: IFocusManagerStatics = windows_core::factory::<FocusManagerName, IFocusManagerStatics>()?;
    let handler = EventHandler::<FocusManagerGotFocusEventArgs>::new(|_sender, args| {
        let Some(args) = args.as_ref() else {
            return;
        };
        let Ok(element) = args.NewFocusedElement() else {
            return;
        };
        let Some(nav) = nav_header::navigation_view(&element) else {
            return;
        };
        NAV_VIEW.with(|slot| slot.borrow_mut().0 = Some(nav.clone()));
        on_tree(&nav);
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
        // Walking the tree every frame burns CPU while idle; only run after a rebuild.
        let left = PENDING_FRAMES.get();
        if left == 0 {
            return;
        }
        PENDING_FRAMES.set(left - 1);
        let Some(nav) = NAV_VIEW.with(|slot| slot.borrow().0.clone()) else {
            return;
        };
        on_tree(&nav);
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

/// Subscribe once; Settings header and slider steps both run from this hook.
pub fn apply() {
    if let Err(e) = mica::apply() {
        warn!(error = %e, "xaml: Mica resource override failed");
    }
    if let Err(e) = subscribe_got_focus() {
        warn!(error = %e, "xaml: GotFocus subscribe failed");
    }
    if let Err(e) = subscribe_rendering() {
        warn!(error = %e, "xaml: Rendering subscribe failed");
    }
}

//! `Application` subclass and the WinUI message loop.
//!
//! Adapted from windows-reactor 0.100.0 (`src/native/winui/app_shim.rs`, `run_with` in `src/app.rs`,
//! `initialize_ui_thread` in `src/native/winui/mod.rs`), <https://github.com/microsoft/windows-rs>.
//! Copyright (c) Microsoft Corporation. Licensed under MIT OR Apache-2.0.

use std::{cell::RefCell, rc::Rc};

use windows::Win32::{
    Foundation::RPC_E_CHANGED_MODE,
    System::Com::{COINIT_APARTMENTTHREADED, CoInitializeEx},
};
use windows_core::{Array, HSTRING, Interface, Ref, Result, implement_decl};

use crate::{
    bootstrap::bootstrap,
    generated::{
        app_bindings::{
            Application as ComposedApplication, ApplicationInitializationCallback, IApplicationOverrides, IApplicationOverrides_Impl,
            IXamlMetadataProvider, IXamlMetadataProvider_Impl, IXamlType, LaunchActivatedEventArgs, TypeName,
            XamlControlsXamlMetaDataProvider, XmlnsDefinition,
        },
        bindings::{Application, ResourceDictionary, XamlControlsResources},
    },
};

type Launch = Box<dyn FnOnce() -> Result<()>>;

struct Overrides {
    provider: RefCell<Option<XamlControlsXamlMetaDataProvider>>,
    launch: RefCell<Option<Launch>>,
    result: Rc<RefCell<Result<()>>>,
}

implement_decl! {
    impl Overrides as Overrides_Impl: [IApplicationOverrides, IXamlMetadataProvider]
}

impl Overrides {
    // Built-in controls (NavigationView, NumberBox, ...) resolve their XAML types through this provider.
    fn provider(&self) -> Result<XamlControlsXamlMetaDataProvider> {
        let mut slot = self.provider.borrow_mut();
        if let Some(provider) = slot.as_ref() {
            return Ok(provider.clone());
        }
        let provider = XamlControlsXamlMetaDataProvider::new()?;
        *slot = Some(provider.clone());
        Ok(provider)
    }
}

impl IApplicationOverrides_Impl for Overrides_Impl {
    fn OnLaunched(&self, _args: Ref<LaunchActivatedEventArgs>) -> Result<()> {
        let Some(launch) = self.launch.borrow_mut().take() else {
            return Ok(());
        };
        let launched = (|| {
            let controls: ResourceDictionary = XamlControlsResources::new()?.cast()?;
            Application::Current()?.Resources()?.MergedDictionaries()?.Append(&controls)?;
            launch()
        })();
        if let Err(e) = &launched {
            *self.result.borrow_mut() = Err(e.clone());
            Application::Current()?.Exit()?;
        }
        launched
    }
}

impl IXamlMetadataProvider_Impl for Overrides_Impl {
    fn GetXamlType(&self, r#type: &TypeName) -> Result<IXamlType> {
        self.provider()?.GetXamlType(r#type)
    }

    fn GetXamlTypeByFullName(&self, full_name: &HSTRING) -> Result<IXamlType> {
        self.provider()?.GetXamlTypeByFullName(&full_name.to_string_lossy())
    }

    fn GetXmlnsDefinitions(&self) -> Result<Array<XmlnsDefinition>> {
        self.provider()?.GetXmlnsDefinitions()
    }
}

/// Bootstraps the Windows App Runtime and runs the WinUI message loop on this thread.
///
/// `launch` runs once from `OnLaunched`, after the default control styles are loaded.
/// Returns after the last window closes (or `launch` fails).
pub fn run(launch: impl FnOnce() -> Result<()> + 'static) -> Result<()> {
    bootstrap()?;
    // SAFETY: initializes COM for this thread; WinUI requires an STA.
    let hr = unsafe { CoInitializeEx(None, COINIT_APARTMENTTHREADED) };
    if hr == RPC_E_CHANGED_MODE {
        return Err(windows_core::Error::new(windows_core::HRESULT(hr.0), "WinUI needs an STA thread"));
    }
    let result = Rc::new(RefCell::new(Ok(())));
    let app_slot: Rc<RefCell<Option<ComposedApplication>>> = Rc::default();
    let launch: RefCell<Option<Launch>> = RefCell::new(Some(Box::new(launch)));
    let callback = {
        let result = Rc::clone(&result);
        let app_slot = Rc::clone(&app_slot);
        ApplicationInitializationCallback::new(move |_| {
            let overrides = Overrides {
                provider: RefCell::new(None),
                launch: RefCell::new(launch.borrow_mut().take()),
                result: Rc::clone(&result),
            };
            match ComposedApplication::compose(overrides) {
                Ok(app) => *app_slot.borrow_mut() = Some(app),
                Err(e) => *result.borrow_mut() = Err(e),
            }
        })
    };
    let started = ComposedApplication::Start(&callback);
    // Releasing XAML objects after the loop can fault inside WinUI; the process is exiting anyway.
    std::mem::forget(app_slot.borrow_mut().take());
    started?;
    result.replace(Ok(()))
}

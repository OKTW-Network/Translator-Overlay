#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppWindow(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(AppWindow, windows_core::IUnknown, windows_core::IInspectable);
impl AppWindow {
    pub fn TitleBar(&self) -> windows_core::Result<AppWindowTitleBar> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).TitleBar)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetIcon(&self, iconpath: &windows_core::HSTRING) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetIcon)(windows_core::Interface::as_raw(self), core::mem::transmute_copy(iconpath)).ok()
        }
    }

    pub fn ResizeClient(&self, size: SizeInt32) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IAppWindow2>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ResizeClient)(windows_core::Interface::as_raw(this), size).ok() }
    }

    fn IAppWindowStatics<R, F: FnOnce(&IAppWindowStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<AppWindow, IAppWindowStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IAppWindowStatics2<R, F: FnOnce(&IAppWindowStatics2) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<AppWindow, IAppWindowStatics2> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IAppWindowStatics3<R, F: FnOnce(&IAppWindowStatics3) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<AppWindow, IAppWindowStatics3> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for AppWindow {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IAppWindow>();
}
unsafe impl windows_core::Interface for AppWindow {
    type Vtable = <IAppWindow as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IAppWindow as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for AppWindow {
    const NAME: &'static str = "Microsoft.UI.Windowing.AppWindow";
}
unsafe impl Send for AppWindow {}
unsafe impl Sync for AppWindow {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AppWindowTitleBar(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(AppWindowTitleBar, windows_core::IUnknown, windows_core::IInspectable);
impl AppWindowTitleBar {
    pub fn SetPreferredHeightOption(&self, value: TitleBarHeightOption) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IAppWindowTitleBar2>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetPreferredHeightOption)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetPreferredTheme(&self, value: TitleBarTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IAppWindowTitleBar3>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetPreferredTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    fn IAppWindowTitleBarStatics<R, F: FnOnce(&IAppWindowTitleBarStatics) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<AppWindowTitleBar, IAppWindowTitleBarStatics> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for AppWindowTitleBar {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IAppWindowTitleBar>();
}
unsafe impl windows_core::Interface for AppWindowTitleBar {
    type Vtable = <IAppWindowTitleBar as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IAppWindowTitleBar as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for AppWindowTitleBar {
    const NAME: &'static str = "Microsoft.UI.Windowing.AppWindowTitleBar";
}
unsafe impl Send for AppWindowTitleBar {}
unsafe impl Sync for AppWindowTitleBar {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Application(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(Application, windows_core::IUnknown, windows_core::IInspectable);
impl Application {
    pub fn Resources(&self) -> windows_core::Result<ResourceDictionary> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Resources)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn Exit(&self) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).Exit)(windows_core::Interface::as_raw(self)).ok() }
    }

    pub fn Current() -> windows_core::Result<Self> {
        Self::IApplicationStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Current)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    fn IApplicationFactory<R, F: FnOnce(&IApplicationFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Application, IApplicationFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IApplicationStatics<R, F: FnOnce(&IApplicationStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Application, IApplicationStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for Application {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IApplication>();
}
unsafe impl windows_core::Interface for Application {
    type Vtable = <IApplication as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IApplication as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for Application {
    const NAME: &'static str = "Microsoft.UI.Xaml.Application";
}
unsafe impl Send for Application {}
unsafe impl Sync for Application {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoSuggestBox(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(AutoSuggestBox, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(AutoSuggestBox, ItemsControl, Control, FrameworkElement, UIElement, DependencyObject);
impl AutoSuggestBox {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }

    fn IActivationFactory<R, F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<AutoSuggestBox, windows_core::imp::IGenericFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    pub fn Text(&self) -> windows_core::Result<windows_core::HSTRING> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Text)(windows_core::Interface::as_raw(self), &mut result__)
                .map(|| core::mem::transmute(result__))
        }
    }

    pub fn SetText(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetText)(windows_core::Interface::as_raw(self), core::mem::transmute_copy(value)).ok()
        }
    }

    pub fn SetPlaceholderText(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetPlaceholderText)(
                windows_core::Interface::as_raw(self),
                core::mem::transmute_copy(value),
            )
            .ok()
        }
    }

    pub fn SuggestionChosen<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<Self>, windows_core::Ref<AutoSuggestBoxSuggestionChosenEventArgs>) + Send + 'static,
    {
        let handler = <TypedEventHandler<Self, AutoSuggestBoxSuggestionChosenEventArgs>>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).SuggestionChosen)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemoveSuggestionChosen))
        }
    }

    pub fn TextChanged<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<Self>, windows_core::Ref<AutoSuggestBoxTextChangedEventArgs>) + Send + 'static,
    {
        let handler = <TypedEventHandler<Self, AutoSuggestBoxTextChangedEventArgs>>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).TextChanged)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemoveTextChanged))
        }
    }

    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetItemsSource<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IItemsControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetItemsSource)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IAutoSuggestBoxStatics<R, F: FnOnce(&IAutoSuggestBoxStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<AutoSuggestBox, IAutoSuggestBoxStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for AutoSuggestBox {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IAutoSuggestBox>();
}
unsafe impl windows_core::Interface for AutoSuggestBox {
    type Vtable = <IAutoSuggestBox as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IAutoSuggestBox as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for AutoSuggestBox {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.AutoSuggestBox";
}
unsafe impl Send for AutoSuggestBox {}
unsafe impl Sync for AutoSuggestBox {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoSuggestBoxSuggestionChosenEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(AutoSuggestBoxSuggestionChosenEventArgs, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(AutoSuggestBoxSuggestionChosenEventArgs, DependencyObject);
impl AutoSuggestBoxSuggestionChosenEventArgs {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }

    fn IActivationFactory<R, F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<AutoSuggestBoxSuggestionChosenEventArgs, windows_core::imp::IGenericFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    pub fn SelectedItem(&self) -> windows_core::Result<windows_core::IInspectable> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).SelectedItem)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }
}
impl windows_core::RuntimeType for AutoSuggestBoxSuggestionChosenEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IAutoSuggestBoxSuggestionChosenEventArgs>();
}
unsafe impl windows_core::Interface for AutoSuggestBoxSuggestionChosenEventArgs {
    type Vtable = <IAutoSuggestBoxSuggestionChosenEventArgs as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IAutoSuggestBoxSuggestionChosenEventArgs as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for AutoSuggestBoxSuggestionChosenEventArgs {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.AutoSuggestBoxSuggestionChosenEventArgs";
}
unsafe impl Send for AutoSuggestBoxSuggestionChosenEventArgs {}
unsafe impl Sync for AutoSuggestBoxSuggestionChosenEventArgs {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutoSuggestBoxTextChangedEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(AutoSuggestBoxTextChangedEventArgs, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(AutoSuggestBoxTextChangedEventArgs, DependencyObject);
impl AutoSuggestBoxTextChangedEventArgs {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }

    fn IActivationFactory<R, F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<AutoSuggestBoxTextChangedEventArgs, windows_core::imp::IGenericFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    pub fn Reason(&self) -> windows_core::Result<AutoSuggestionBoxTextChangeReason> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Reason)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    fn IAutoSuggestBoxTextChangedEventArgsStatics<R, F: FnOnce(&IAutoSuggestBoxTextChangedEventArgsStatics) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<AutoSuggestBoxTextChangedEventArgs, IAutoSuggestBoxTextChangedEventArgsStatics> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for AutoSuggestBoxTextChangedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, IAutoSuggestBoxTextChangedEventArgs>();
}
unsafe impl windows_core::Interface for AutoSuggestBoxTextChangedEventArgs {
    type Vtable = <IAutoSuggestBoxTextChangedEventArgs as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IAutoSuggestBoxTextChangedEventArgs as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for AutoSuggestBoxTextChangedEventArgs {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.AutoSuggestBoxTextChangedEventArgs";
}
unsafe impl Send for AutoSuggestBoxTextChangedEventArgs {}
unsafe impl Sync for AutoSuggestBoxTextChangedEventArgs {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct AutoSuggestionBoxTextChangeReason(pub i32);
impl AutoSuggestionBoxTextChangeReason {
    pub const ProgrammaticChange: Self = Self(1);
    pub const SuggestionChosen: Self = Self(2);
    pub const UserInput: Self = Self(0);
}
impl windows_core::imp::TypeKind for AutoSuggestionBoxTextChangeReason {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for AutoSuggestionBoxTextChangeReason {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.AutoSuggestionBoxTextChangeReason");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.Controls.AutoSuggestionBoxTextChangeReason;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutomationProperties(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(AutomationProperties, windows_core::IUnknown, windows_core::IInspectable);
impl AutomationProperties {
    pub fn SetAutomationId<P0>(element: P0, value: &windows_core::HSTRING) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyObject>,
    {
        Self::IAutomationPropertiesStatics(|this| unsafe {
            (windows_core::Interface::vtable(this).SetAutomationId)(
                windows_core::Interface::as_raw(this),
                element.param().abi(),
                core::mem::transmute_copy(value),
            )
            .ok()
        })
    }

    pub fn SetName<P0>(element: P0, value: &windows_core::HSTRING) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyObject>,
    {
        Self::IAutomationPropertiesStatics(|this| unsafe {
            (windows_core::Interface::vtable(this).SetName)(
                windows_core::Interface::as_raw(this),
                element.param().abi(),
                core::mem::transmute_copy(value),
            )
            .ok()
        })
    }

    fn IAutomationPropertiesStatics<R, F: FnOnce(&IAutomationPropertiesStatics) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<AutomationProperties, IAutomationPropertiesStatics> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IAutomationPropertiesStatics2<R, F: FnOnce(&IAutomationPropertiesStatics2) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<AutomationProperties, IAutomationPropertiesStatics2> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for AutomationProperties {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IAutomationProperties>();
}
unsafe impl windows_core::Interface for AutomationProperties {
    type Vtable = <IAutomationProperties as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IAutomationProperties as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for AutomationProperties {
    const NAME: &'static str = "Microsoft.UI.Xaml.Automation.AutomationProperties";
}
unsafe impl Send for AutomationProperties {}
unsafe impl Sync for AutomationProperties {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BitmapSource(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(BitmapSource, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(BitmapSource, ImageSource, DependencyObject);
impl BitmapSource {
    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    fn IBitmapSourceFactory<R, F: FnOnce(&IBitmapSourceFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<BitmapSource, IBitmapSourceFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IBitmapSourceStatics<R, F: FnOnce(&IBitmapSourceStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<BitmapSource, IBitmapSourceStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for BitmapSource {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IBitmapSource>();
}
unsafe impl windows_core::Interface for BitmapSource {
    type Vtable = <IBitmapSource as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IBitmapSource as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for BitmapSource {
    const NAME: &'static str = "Microsoft.UI.Xaml.Media.Imaging.BitmapSource";
}
unsafe impl Send for BitmapSource {}
unsafe impl Sync for BitmapSource {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Border(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(Border, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(Border, FrameworkElement, UIElement, DependencyObject);
impl Border {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }

    fn IActivationFactory<R, F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Border, windows_core::imp::IGenericFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    pub fn SetBorderBrush<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Brush>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetBorderBrush)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn SetBorderThickness(&self, value: Thickness) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetBorderThickness)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetBackground<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Brush>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetBackground)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn SetCornerRadius(&self, value: CornerRadius) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetCornerRadius)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetPadding(&self, value: Thickness) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetPadding)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetChild<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<UIElement>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetChild)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IBorderStatics<R, F: FnOnce(&IBorderStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Border, IBorderStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for Border {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IBorder>();
}
unsafe impl windows_core::Interface for Border {
    type Vtable = <IBorder as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IBorder as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for Border {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.Border";
}
unsafe impl Send for Border {}
unsafe impl Sync for Border {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Brush(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(Brush, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(Brush, DependencyObject);
impl Brush {
    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    fn IBrushFactory<R, F: FnOnce(&IBrushFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Brush, IBrushFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IBrushStatics<R, F: FnOnce(&IBrushStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Brush, IBrushStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for Brush {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IBrush>();
}
unsafe impl windows_core::Interface for Brush {
    type Vtable = <IBrush as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IBrush as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for Brush {
    const NAME: &'static str = "Microsoft.UI.Xaml.Media.Brush";
}
unsafe impl Send for Brush {}
unsafe impl Sync for Brush {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Button(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(Button, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(Button, ButtonBase, ContentControl, Control, FrameworkElement, UIElement, DependencyObject);
impl Button {
    pub fn Click<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IButtonBase>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Click)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveClick))
        }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::IButtonFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::IButtonFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn SetContent<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IContentControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetContent)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IButtonFactory<R, F: FnOnce(&IButtonFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Button, IButtonFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IButtonStatics<R, F: FnOnce(&IButtonStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Button, IButtonStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for Button {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IButton>();
}
unsafe impl windows_core::Interface for Button {
    type Vtable = <IButton as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IButton as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for Button {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.Button";
}
unsafe impl Send for Button {}
unsafe impl Sync for Button {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ButtonBase(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(ButtonBase, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(ButtonBase, ContentControl, Control, FrameworkElement, UIElement, DependencyObject);
impl ButtonBase {
    pub fn Click<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).Click)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemoveClick))
        }
    }

    pub fn SetContent<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IContentControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetContent)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IButtonBaseFactory<R, F: FnOnce(&IButtonBaseFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ButtonBase, IButtonBaseFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IButtonBaseStatics<R, F: FnOnce(&IButtonBaseStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ButtonBase, IButtonBaseStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for ButtonBase {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IButtonBase>();
}
unsafe impl windows_core::Interface for ButtonBase {
    type Vtable = <IButtonBase as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IButtonBase as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ButtonBase {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.Primitives.ButtonBase";
}
unsafe impl Send for ButtonBase {}
unsafe impl Sync for ButtonBase {}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Color {
    pub A: u8,
    pub R: u8,
    pub G: u8,
    pub B: u8,
}
impl windows_core::imp::TypeKind for Color {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for Color {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Windows.UI.Color");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"struct(Windows.UI.Color;u1;u1;u1;u1)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColorChangedEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(ColorChangedEventArgs, windows_core::IUnknown, windows_core::IInspectable);
impl ColorChangedEventArgs {
    pub fn NewColor(&self) -> windows_core::Result<Color> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).NewColor)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }
}
impl windows_core::RuntimeType for ColorChangedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IColorChangedEventArgs>();
}
unsafe impl windows_core::Interface for ColorChangedEventArgs {
    type Vtable = <IColorChangedEventArgs as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IColorChangedEventArgs as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ColorChangedEventArgs {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.ColorChangedEventArgs";
}
unsafe impl Send for ColorChangedEventArgs {}
unsafe impl Sync for ColorChangedEventArgs {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColorPicker(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(ColorPicker, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(ColorPicker, Control, FrameworkElement, UIElement, DependencyObject);
impl ColorPicker {
    pub fn Color(&self) -> windows_core::Result<Color> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Color)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }

    pub fn SetColor(&self, value: Color) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetColor)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetIsAlphaEnabled(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetIsAlphaEnabled)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetIsColorChannelTextInputVisible(&self, value: bool) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetIsColorChannelTextInputVisible)(windows_core::Interface::as_raw(self), value).ok()
        }
    }

    pub fn SetIsHexInputVisible(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetIsHexInputVisible)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn ColorChanged<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<Self>, windows_core::Ref<ColorChangedEventArgs>) + Send + 'static,
    {
        let handler = <TypedEventHandler<Self, ColorChangedEventArgs>>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).ColorChanged)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemoveColorChanged))
        }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::IColorPickerFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::IColorPickerFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IColorPickerFactory<R, F: FnOnce(&IColorPickerFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ColorPicker, IColorPickerFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IColorPickerStatics<R, F: FnOnce(&IColorPickerStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ColorPicker, IColorPickerStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IColorPickerStatics2<R, F: FnOnce(&IColorPickerStatics2) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ColorPicker, IColorPickerStatics2> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for ColorPicker {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IColorPicker>();
}
unsafe impl windows_core::Interface for ColorPicker {
    type Vtable = <IColorPicker as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IColorPicker as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ColorPicker {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.ColorPicker";
}
unsafe impl Send for ColorPicker {}
unsafe impl Sync for ColorPicker {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColumnDefinition(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(ColumnDefinition, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(ColumnDefinition, DependencyObject);
impl ColumnDefinition {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }

    fn IActivationFactory<R, F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ColumnDefinition, windows_core::imp::IGenericFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    pub fn SetWidth(&self, value: GridLength) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetWidth)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    fn IColumnDefinitionStatics<R, F: FnOnce(&IColumnDefinitionStatics) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ColumnDefinition, IColumnDefinitionStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for ColumnDefinition {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IColumnDefinition>();
}
unsafe impl windows_core::Interface for ColumnDefinition {
    type Vtable = <IColumnDefinition as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IColumnDefinition as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ColumnDefinition {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.ColumnDefinition";
}
unsafe impl Send for ColumnDefinition {}
unsafe impl Sync for ColumnDefinition {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ColumnDefinitionCollection(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    ColumnDefinitionCollection,
    windows_core::IUnknown,
    windows_core::IInspectable,
    IVector<ColumnDefinition>
);
windows_core::imp::required_hierarchy!(ColumnDefinitionCollection, IIterable<ColumnDefinition>);
impl ColumnDefinitionCollection {
    pub fn First(&self) -> windows_core::Result<windows_collections::IIterator<ColumnDefinition>> {
        let this = &windows_core::Interface::cast::<IIterable<ColumnDefinition>>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).First)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn GetAt(&self, index: u32) -> windows_core::Result<ColumnDefinition> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).GetAt)(windows_core::Interface::as_raw(self), index, &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn Size(&self) -> windows_core::Result<u32> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Size)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }

    pub fn IndexOf<P0>(&self, value: P0, index: &mut u32) -> windows_core::Result<bool>
    where
        P0: windows_core::Param<ColumnDefinition>,
    {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).IndexOf)(
                windows_core::Interface::as_raw(self),
                value.param().abi(),
                index,
                &mut result__,
            )
            .map(|| result__)
        }
    }

    pub fn InsertAt<P1>(&self, index: u32, value: P1) -> windows_core::Result<()>
    where
        P1: windows_core::Param<ColumnDefinition>,
    {
        unsafe { (windows_core::Interface::vtable(self).InsertAt)(windows_core::Interface::as_raw(self), index, value.param().abi()).ok() }
    }

    pub fn RemoveAt(&self, index: u32) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).RemoveAt)(windows_core::Interface::as_raw(self), index).ok() }
    }

    pub fn Append<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<ColumnDefinition>,
    {
        unsafe { (windows_core::Interface::vtable(self).Append)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn Clear(&self) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).Clear)(windows_core::Interface::as_raw(self)).ok() }
    }
}
impl windows_core::RuntimeType for ColumnDefinitionCollection {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IVector<ColumnDefinition>>();
}
unsafe impl windows_core::Interface for ColumnDefinitionCollection {
    type Vtable = <IVector<ColumnDefinition> as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IVector<ColumnDefinition> as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ColumnDefinitionCollection {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.ColumnDefinitionCollection";
}
unsafe impl Send for ColumnDefinitionCollection {}
unsafe impl Sync for ColumnDefinitionCollection {}
impl IntoIterator for ColumnDefinitionCollection {
    type IntoIter = windows_collections::BufferedIterator<Self::Item>;
    type Item = ColumnDefinition;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(&self)
    }
}
impl IntoIterator for &ColumnDefinitionCollection {
    type IntoIter = windows_collections::BufferedIterator<Self::Item>;
    type Item = ColumnDefinition;

    fn into_iter(self) -> Self::IntoIter {
        windows_collections::BufferedIterator::new(self.First().unwrap())
    }
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComboBox(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(ComboBox, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(ComboBox, Selector, ItemsControl, Control, FrameworkElement, UIElement, DependencyObject);
impl ComboBox {
    pub fn SetPlaceholderText(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetPlaceholderText)(
                windows_core::Interface::as_raw(self),
                core::mem::transmute_copy(value),
            )
            .ok()
        }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::IComboBoxFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::IComboBoxFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetItemsSource<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IItemsControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetItemsSource)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SelectedIndex(&self) -> windows_core::Result<i32> {
        let this = &windows_core::Interface::cast::<ISelector>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).SelectedIndex)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetSelectedIndex(&self, value: i32) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<ISelector>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetSelectedIndex)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SelectionChanged<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<SelectionChangedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<ISelector>(self)?;
        let handler = <SelectionChangedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).SelectionChanged)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveSelectionChanged))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IComboBoxFactory<R, F: FnOnce(&IComboBoxFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ComboBox, IComboBoxFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IComboBoxStatics<R, F: FnOnce(&IComboBoxStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ComboBox, IComboBoxStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for ComboBox {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IComboBox>();
}
unsafe impl windows_core::Interface for ComboBox {
    type Vtable = <IComboBox as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IComboBox as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ComboBox {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.ComboBox";
}
unsafe impl Send for ComboBox {}
unsafe impl Sync for ComboBox {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentControl(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(ContentControl, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(ContentControl, Control, FrameworkElement, UIElement, DependencyObject);
impl ContentControl {
    pub fn SetContent<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetContent)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IContentControlFactory<R, F: FnOnce(&IContentControlFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ContentControl, IContentControlFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IContentControlStatics<R, F: FnOnce(&IContentControlStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ContentControl, IContentControlStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for ContentControl {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IContentControl>();
}
unsafe impl windows_core::Interface for ContentControl {
    type Vtable = <IContentControl as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IContentControl as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ContentControl {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.ContentControl";
}
unsafe impl Send for ContentControl {}
unsafe impl Sync for ContentControl {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentDialog(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(ContentDialog, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(ContentDialog, ContentControl, Control, FrameworkElement, UIElement, DependencyObject);
impl ContentDialog {
    pub fn SetContent<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IContentControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetContent)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetTitle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetTitle)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn SetPrimaryButtonText(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetPrimaryButtonText)(
                windows_core::Interface::as_raw(self),
                core::mem::transmute_copy(value),
            )
            .ok()
        }
    }

    pub fn SetCloseButtonText(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetCloseButtonText)(
                windows_core::Interface::as_raw(self),
                core::mem::transmute_copy(value),
            )
            .ok()
        }
    }

    pub fn SetIsPrimaryButtonEnabled(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetIsPrimaryButtonEnabled)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn Closed<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<Self>, windows_core::Ref<ContentDialogClosedEventArgs>) + Send + 'static,
    {
        let handler = <TypedEventHandler<Self, ContentDialogClosedEventArgs>>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).Closed)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemoveClosed))
        }
    }

    pub fn Hide(&self) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).Hide)(windows_core::Interface::as_raw(self)).ok() }
    }

    pub fn ShowAsync(&self) -> windows_core::Result<windows_future::IAsyncOperation<ContentDialogResult>> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).ShowAsync)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::IContentDialogFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::IContentDialogFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IContentDialogFactory<R, F: FnOnce(&IContentDialogFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ContentDialog, IContentDialogFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IContentDialogStatics<R, F: FnOnce(&IContentDialogStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ContentDialog, IContentDialogStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for ContentDialog {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IContentDialog>();
}
unsafe impl windows_core::Interface for ContentDialog {
    type Vtable = <IContentDialog as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IContentDialog as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ContentDialog {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.ContentDialog";
}
unsafe impl Send for ContentDialog {}
unsafe impl Sync for ContentDialog {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ContentDialogClosedEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(ContentDialogClosedEventArgs, windows_core::IUnknown, windows_core::IInspectable);
impl ContentDialogClosedEventArgs {
    pub fn Result(&self) -> windows_core::Result<ContentDialogResult> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Result)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }
}
impl windows_core::RuntimeType for ContentDialogClosedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IContentDialogClosedEventArgs>();
}
unsafe impl windows_core::Interface for ContentDialogClosedEventArgs {
    type Vtable = <IContentDialogClosedEventArgs as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IContentDialogClosedEventArgs as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ContentDialogClosedEventArgs {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.ContentDialogClosedEventArgs";
}
unsafe impl Send for ContentDialogClosedEventArgs {}
unsafe impl Sync for ContentDialogClosedEventArgs {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ContentDialogResult(pub i32);
impl ContentDialogResult {
    pub const None: Self = Self(0);
    pub const Primary: Self = Self(1);
    pub const Secondary: Self = Self(2);
}
impl windows_core::imp::TypeKind for ContentDialogResult {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for ContentDialogResult {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ContentDialogResult");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.Controls.ContentDialogResult;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Control(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(Control, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(Control, FrameworkElement, UIElement, DependencyObject);
impl Control {
    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetIsEnabled)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetVerticalContentAlignment)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IControlFactory<R, F: FnOnce(&IControlFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Control, IControlFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IControlStatics<R, F: FnOnce(&IControlStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Control, IControlStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for Control {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IControl>();
}
unsafe impl windows_core::Interface for Control {
    type Vtable = <IControl as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IControl as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for Control {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.Control";
}
unsafe impl Send for Control {}
unsafe impl Sync for Control {}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct CornerRadius {
    pub TopLeft: f64,
    pub TopRight: f64,
    pub BottomRight: f64,
    pub BottomLeft: f64,
}
impl windows_core::imp::TypeKind for CornerRadius {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for CornerRadius {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.CornerRadius");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"struct(Microsoft.UI.Xaml.CornerRadius;f8;f8;f8;f8)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyObject(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(DependencyObject, windows_core::IUnknown, windows_core::IInspectable);
impl DependencyObject {
    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        unsafe { (windows_core::Interface::vtable(self).ClearValue)(windows_core::Interface::as_raw(self), dp.param().abi()).ok() }
    }

    fn IDependencyObjectFactory<R, F: FnOnce(&IDependencyObjectFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<DependencyObject, IDependencyObjectFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for DependencyObject {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IDependencyObject>();
}
unsafe impl windows_core::Interface for DependencyObject {
    type Vtable = <IDependencyObject as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IDependencyObject as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for DependencyObject {
    const NAME: &'static str = "Microsoft.UI.Xaml.DependencyObject";
}
unsafe impl Send for DependencyObject {}
unsafe impl Sync for DependencyObject {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencyProperty(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(DependencyProperty, windows_core::IUnknown, windows_core::IInspectable);
impl DependencyProperty {
    fn IDependencyPropertyStatics<R, F: FnOnce(&IDependencyPropertyStatics) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<DependencyProperty, IDependencyPropertyStatics> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for DependencyProperty {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IDependencyProperty>();
}
unsafe impl windows_core::Interface for DependencyProperty {
    type Vtable = <IDependencyProperty as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IDependencyProperty as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for DependencyProperty {
    const NAME: &'static str = "Microsoft.UI.Xaml.DependencyProperty";
}
unsafe impl Send for DependencyProperty {}
unsafe impl Sync for DependencyProperty {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DispatcherQueue(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(DispatcherQueue, windows_core::IUnknown, windows_core::IInspectable);
impl DispatcherQueue {
    pub fn TryEnqueueWithPriority<P1>(&self, priority: DispatcherQueuePriority, callback: P1) -> windows_core::Result<bool>
    where
        P1: windows_core::Param<DispatcherQueueHandler>,
    {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).TryEnqueueWithPriority)(
                windows_core::Interface::as_raw(self),
                priority,
                callback.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }

    pub fn GetForCurrentThread() -> windows_core::Result<Self> {
        Self::IDispatcherQueueStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).GetForCurrentThread)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    fn IDispatcherQueueStatics<R, F: FnOnce(&IDispatcherQueueStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<DispatcherQueue, IDispatcherQueueStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for DispatcherQueue {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IDispatcherQueue>();
}
unsafe impl windows_core::Interface for DispatcherQueue {
    type Vtable = <IDispatcherQueue as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IDispatcherQueue as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for DispatcherQueue {
    const NAME: &'static str = "Microsoft.UI.Dispatching.DispatcherQueue";
}
unsafe impl Send for DispatcherQueue {}
unsafe impl Sync for DispatcherQueue {}
windows_core::imp::define_interface!(DispatcherQueueHandler, DispatcherQueueHandler_Vtbl, 0x2e0872a9_4e29_5f14_b688_fb96d5f9d5f8);
impl windows_core::RuntimeType for DispatcherQueueHandler {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
impl DispatcherQueueHandler {
    pub fn new<F: Fn() -> windows_core::Result<()> + Send + 'static>(invoke: F) -> Self {
        let com = windows_core::imp::DelegateBox::<Self, F>::new(&DispatcherQueueHandlerBox::<F>::VTABLE, invoke);
        unsafe { core::mem::transmute(windows_core::imp::box_new(com)) }
    }

    pub fn Invoke(&self) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).Invoke)(windows_core::Interface::as_raw(self)).ok() }
    }
}
#[repr(C)]
pub struct DispatcherQueueHandler_Vtbl {
    base__: windows_core::IUnknown_Vtbl,
    Invoke: unsafe extern "system" fn(this: *mut core::ffi::c_void) -> windows_core::HRESULT,
}
struct DispatcherQueueHandlerBox<F: Fn() -> windows_core::Result<()> + Send + 'static>(core::marker::PhantomData<(fn() -> F,)>);
impl<F: Fn() -> windows_core::Result<()> + Send + 'static> DispatcherQueueHandlerBox<F> {
    const VTABLE: DispatcherQueueHandler_Vtbl = DispatcherQueueHandler_Vtbl {
        base__: windows_core::IUnknown_Vtbl {
            QueryInterface: windows_core::imp::DelegateBox::<DispatcherQueueHandler, F>::QueryInterface,
            AddRef: windows_core::imp::DelegateBox::<DispatcherQueueHandler, F>::AddRef,
            Release: windows_core::imp::DelegateBox::<DispatcherQueueHandler, F>::Release,
        },
        Invoke: Self::Invoke,
    };

    unsafe extern "system" fn Invoke(this: *mut core::ffi::c_void) -> windows_core::HRESULT {
        unsafe {
            let this = &mut *(this as *mut *mut core::ffi::c_void as *mut windows_core::imp::DelegateBox<DispatcherQueueHandler, F>);
            (this.invoke)().into()
        }
    }
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct DispatcherQueuePriority(pub i32);
impl DispatcherQueuePriority {
    pub const High: Self = Self(10);
    pub const Low: Self = Self(-10);
    pub const Normal: Self = Self(0);
}
impl windows_core::imp::TypeKind for DispatcherQueuePriority {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for DispatcherQueuePriority {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Dispatching.DispatcherQueuePriority");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Dispatching.DispatcherQueuePriority;i4)");
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ElementTheme(pub i32);
impl ElementTheme {
    pub const Dark: Self = Self(2);
    pub const Default: Self = Self(0);
    pub const Light: Self = Self(1);
}
impl windows_core::imp::TypeKind for ElementTheme {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for ElementTheme {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.ElementTheme");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.ElementTheme;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FontIcon(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(FontIcon, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(FontIcon, IconElement, FrameworkElement, UIElement, DependencyObject);
impl FontIcon {
    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn SetGlyph(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetGlyph)(windows_core::Interface::as_raw(self), core::mem::transmute_copy(value)).ok()
        }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::IFontIconFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::IFontIconFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IFontIconFactory<R, F: FnOnce(&IFontIconFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<FontIcon, IFontIconFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IFontIconStatics<R, F: FnOnce(&IFontIconStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<FontIcon, IFontIconStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for FontIcon {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IFontIcon>();
}
unsafe impl windows_core::Interface for FontIcon {
    type Vtable = <IFontIcon as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IFontIcon as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for FontIcon {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.FontIcon";
}
unsafe impl Send for FontIcon {}
unsafe impl Sync for FontIcon {}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FontWeight {
    pub Weight: u16,
}
impl windows_core::imp::TypeKind for FontWeight {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for FontWeight {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Windows.UI.Text.FontWeight");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"struct(Windows.UI.Text.FontWeight;u2)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FrameworkElement(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(FrameworkElement, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(FrameworkElement, UIElement, DependencyObject);
impl FrameworkElement {
    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Tag)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetTag)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).ActualWidth)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).ActualHeight)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetWidth)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetHeight)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetMinWidth)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetMaxWidth)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetMinHeight)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetMaxHeight)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetHorizontalAlignment)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetVerticalAlignment)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetMargin)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetStyle)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetRequestedTheme)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).Loaded)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemoveLoaded))
        }
    }

    pub fn StyleProperty() -> windows_core::Result<DependencyProperty> {
        Self::IFrameworkElementStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).StyleProperty)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IFrameworkElementFactory<R, F: FnOnce(&IFrameworkElementFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<FrameworkElement, IFrameworkElementFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IFrameworkElementStatics<R, F: FnOnce(&IFrameworkElementStatics) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<FrameworkElement, IFrameworkElementStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for FrameworkElement {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IFrameworkElement>();
}
unsafe impl windows_core::Interface for FrameworkElement {
    type Vtable = <IFrameworkElement as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IFrameworkElement as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for FrameworkElement {
    const NAME: &'static str = "Microsoft.UI.Xaml.FrameworkElement";
}
unsafe impl Send for FrameworkElement {}
unsafe impl Sync for FrameworkElement {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Grid(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(Grid, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(Grid, Panel, FrameworkElement, UIElement, DependencyObject);
impl Grid {
    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn RowDefinitions(&self) -> windows_core::Result<RowDefinitionCollection> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).RowDefinitions)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn ColumnDefinitions(&self) -> windows_core::Result<ColumnDefinitionCollection> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).ColumnDefinitions)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::IGridFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::IGridFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn SetRow<P0>(element: P0, value: i32) -> windows_core::Result<()>
    where
        P0: windows_core::Param<FrameworkElement>,
    {
        Self::IGridStatics(|this| unsafe {
            (windows_core::Interface::vtable(this).SetRow)(windows_core::Interface::as_raw(this), element.param().abi(), value).ok()
        })
    }

    pub fn SetColumn<P0>(element: P0, value: i32) -> windows_core::Result<()>
    where
        P0: windows_core::Param<FrameworkElement>,
    {
        Self::IGridStatics(|this| unsafe {
            (windows_core::Interface::vtable(this).SetColumn)(windows_core::Interface::as_raw(this), element.param().abi(), value).ok()
        })
    }

    pub fn Children(&self) -> windows_core::Result<UIElementCollection> {
        let this = &windows_core::Interface::cast::<IPanel>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Children)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetBackground<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Brush>,
    {
        let this = &windows_core::Interface::cast::<IPanel>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetBackground)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IGridFactory<R, F: FnOnce(&IGridFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Grid, IGridFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IGridStatics<R, F: FnOnce(&IGridStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Grid, IGridStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for Grid {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IGrid>();
}
unsafe impl windows_core::Interface for Grid {
    type Vtable = <IGrid as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IGrid as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for Grid {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.Grid";
}
unsafe impl Send for Grid {}
unsafe impl Sync for Grid {}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct GridLength {
    pub Value: f64,
    pub GridUnitType: GridUnitType,
}
impl windows_core::imp::TypeKind for GridLength {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for GridLength {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.GridLength");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"struct(Microsoft.UI.Xaml.GridLength;f8;enum(Microsoft.UI.Xaml.GridUnitType;i4))");
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct GridUnitType(pub i32);
impl GridUnitType {
    pub const Auto: Self = Self(0);
    pub const Pixel: Self = Self(1);
    pub const Star: Self = Self(2);
}
impl windows_core::imp::TypeKind for GridUnitType {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for GridUnitType {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.GridUnitType");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.GridUnitType;i4)");
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HorizontalAlignment(pub i32);
impl HorizontalAlignment {
    pub const Center: Self = Self(1);
    pub const Left: Self = Self(0);
    pub const Right: Self = Self(2);
    pub const Stretch: Self = Self(3);
}
impl windows_core::imp::TypeKind for HorizontalAlignment {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for HorizontalAlignment {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.HorizontalAlignment");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.HorizontalAlignment;i4)");
}
windows_core::imp::define_interface!(IAppWindow, IAppWindow_Vtbl, 0xcfa788b3_643b_5c5e_ad4e_321d48a82acd);
impl windows_core::RuntimeType for IAppWindow {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Windowing.IAppWindow");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAppWindow_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Id: usize,
    IsShownInSwitchers: usize,
    SetIsShownInSwitchers: usize,
    IsVisible: usize,
    OwnerWindowId: usize,
    Position: usize,
    Presenter: usize,
    Size: usize,
    Title: usize,
    SetTitle: usize,
    pub TitleBar: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    Destroy: usize,
    Hide: usize,
    Move: usize,
    MoveAndResize: usize,
    MoveAndResizeRelativeToDisplayArea: usize,
    Resize: usize,
    pub SetIcon: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IAppWindow2, IAppWindow2_Vtbl, 0x6cd41292_794c_5cac_8961_210d012c6ebc);
impl windows_core::RuntimeType for IAppWindow2 {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Windowing.IAppWindow2");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAppWindow2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    ClientSize: usize,
    MoveInZOrderAtBottom: usize,
    MoveInZOrderAtTop: usize,
    MoveInZOrderBelow: usize,
    pub ResizeClient: unsafe extern "system" fn(*mut core::ffi::c_void, SizeInt32) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IAppWindowStatics, IAppWindowStatics_Vtbl, 0x3c315c24_d540_5d72_b518_b226b83627cb);
impl windows_core::RuntimeType for IAppWindowStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Windowing.IAppWindowStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAppWindowStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IAppWindowStatics2, IAppWindowStatics2_Vtbl, 0xcabc23db_4606_5d6e_89a5_06de1d8bd3e2);
impl windows_core::RuntimeType for IAppWindowStatics2 {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Windowing.IAppWindowStatics2");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAppWindowStatics2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IAppWindowStatics3, IAppWindowStatics3_Vtbl, 0x0bcc835a_1286_5113_9f59_f1093d2e087a);
impl windows_core::RuntimeType for IAppWindowStatics3 {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Windowing.IAppWindowStatics3");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAppWindowStatics3_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IAppWindowTitleBar, IAppWindowTitleBar_Vtbl, 0x5574efa2_c91c_5700_a363_539c71a7aaf4);
impl windows_core::RuntimeType for IAppWindowTitleBar {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Windowing.IAppWindowTitleBar");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAppWindowTitleBar_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IAppWindowTitleBar2, IAppWindowTitleBar2_Vtbl, 0x86faed38_748a_5b4b_9ccf_3ba0496c9041);
impl windows_core::RuntimeType for IAppWindowTitleBar2 {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Windowing.IAppWindowTitleBar2");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAppWindowTitleBar2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    PreferredHeightOption: usize,
    pub SetPreferredHeightOption: unsafe extern "system" fn(*mut core::ffi::c_void, TitleBarHeightOption) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IAppWindowTitleBar3, IAppWindowTitleBar3_Vtbl, 0x07146e74_0410_5597_aba7_1af276d2ae07);
impl windows_core::RuntimeType for IAppWindowTitleBar3 {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Windowing.IAppWindowTitleBar3");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAppWindowTitleBar3_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    PreferredTheme: usize,
    pub SetPreferredTheme: unsafe extern "system" fn(*mut core::ffi::c_void, TitleBarTheme) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IAppWindowTitleBarStatics, IAppWindowTitleBarStatics_Vtbl, 0x9e1da52e_8b15_54d6_a886_f7b9f9d930b2);
impl windows_core::RuntimeType for IAppWindowTitleBarStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Windowing.IAppWindowTitleBarStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAppWindowTitleBarStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IApplication, IApplication_Vtbl, 0x06a8f4e7_1146_55af_820d_ebd55643b021);
impl windows_core::RuntimeType for IApplication {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IApplication");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IApplication_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Resources: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    SetResources: usize,
    DebugSettings: usize,
    RequestedTheme: usize,
    SetRequestedTheme: usize,
    FocusVisualKind: usize,
    SetFocusVisualKind: usize,
    HighContrastAdjustment: usize,
    SetHighContrastAdjustment: usize,
    UnhandledException: usize,
    RemoveUnhandledException: usize,
    pub Exit: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IApplicationFactory, IApplicationFactory_Vtbl, 0x9fd96657_5294_5a65_a1db_4fea143597da);
impl windows_core::RuntimeType for IApplicationFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IApplicationFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IApplicationFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IApplicationStatics, IApplicationStatics_Vtbl, 0x4e0d09f5_4358_512c_a987_503b52848e95);
impl windows_core::RuntimeType for IApplicationStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IApplicationStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IApplicationStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Current: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IAutoSuggestBox, IAutoSuggestBox_Vtbl, 0x3eea809e_b2db_521d_97db_e0648fb5d798);
impl windows_core::RuntimeType for IAutoSuggestBox {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IAutoSuggestBox");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAutoSuggestBox_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    MaxSuggestionListHeight: usize,
    SetMaxSuggestionListHeight: usize,
    IsSuggestionListOpen: usize,
    SetIsSuggestionListOpen: usize,
    TextMemberPath: usize,
    SetTextMemberPath: usize,
    pub Text: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    pub SetText: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    UpdateTextOnSelect: usize,
    SetUpdateTextOnSelect: usize,
    PlaceholderText: usize,
    pub SetPlaceholderText: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    Header: usize,
    SetHeader: usize,
    AutoMaximizeSuggestionArea: usize,
    SetAutoMaximizeSuggestionArea: usize,
    TextBoxStyle: usize,
    SetTextBoxStyle: usize,
    QueryIcon: usize,
    SetQueryIcon: usize,
    LightDismissOverlayMode: usize,
    SetLightDismissOverlayMode: usize,
    Description: usize,
    SetDescription: usize,
    pub SuggestionChosen: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemoveSuggestionChosen: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    pub TextChanged: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemoveTextChanged: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IAutoSuggestBoxStatics, IAutoSuggestBoxStatics_Vtbl, 0x9dfa2ff9_1094_54a8_beee_b345fa3ee089);
impl windows_core::RuntimeType for IAutoSuggestBoxStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IAutoSuggestBoxStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAutoSuggestBoxStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(
    IAutoSuggestBoxSuggestionChosenEventArgs,
    IAutoSuggestBoxSuggestionChosenEventArgs_Vtbl,
    0x7547c7e9_7429_5045_ad98_338a96b270b1
);
impl windows_core::RuntimeType for IAutoSuggestBoxSuggestionChosenEventArgs {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IAutoSuggestBoxSuggestionChosenEventArgs");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAutoSuggestBoxSuggestionChosenEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub SelectedItem: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IAutoSuggestBoxTextChangedEventArgs,
    IAutoSuggestBoxTextChangedEventArgs_Vtbl,
    0xd7191d84_e886_547f_a3e2_12f0e05b20fa
);
impl windows_core::RuntimeType for IAutoSuggestBoxTextChangedEventArgs {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IAutoSuggestBoxTextChangedEventArgs");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAutoSuggestBoxTextChangedEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Reason: unsafe extern "system" fn(*mut core::ffi::c_void, *mut AutoSuggestionBoxTextChangeReason) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IAutoSuggestBoxTextChangedEventArgsStatics,
    IAutoSuggestBoxTextChangedEventArgsStatics_Vtbl,
    0x40155fb9_253b_5624_a17a_294be7a07f87
);
impl windows_core::RuntimeType for IAutoSuggestBoxTextChangedEventArgsStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IAutoSuggestBoxTextChangedEventArgsStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAutoSuggestBoxTextChangedEventArgsStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IAutomationProperties, IAutomationProperties_Vtbl, 0x525c6a71_dd8a_52a0_977b_db1b02f8e896);
impl windows_core::RuntimeType for IAutomationProperties {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Automation.IAutomationProperties");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAutomationProperties_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(
    IAutomationPropertiesStatics,
    IAutomationPropertiesStatics_Vtbl,
    0xb1e3e0f3_112f_5966_87dc_7862d4ad50e5
);
impl windows_core::RuntimeType for IAutomationPropertiesStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Automation.IAutomationPropertiesStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAutomationPropertiesStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    AcceleratorKeyProperty: usize,
    GetAcceleratorKey: usize,
    SetAcceleratorKey: usize,
    AccessKeyProperty: usize,
    GetAccessKey: usize,
    SetAccessKey: usize,
    AutomationIdProperty: usize,
    GetAutomationId: usize,
    pub SetAutomationId:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    HelpTextProperty: usize,
    GetHelpText: usize,
    SetHelpText: usize,
    IsRequiredForFormProperty: usize,
    GetIsRequiredForForm: usize,
    SetIsRequiredForForm: usize,
    ItemStatusProperty: usize,
    GetItemStatus: usize,
    SetItemStatus: usize,
    ItemTypeProperty: usize,
    GetItemType: usize,
    SetItemType: usize,
    LabeledByProperty: usize,
    GetLabeledBy: usize,
    SetLabeledBy: usize,
    NameProperty: usize,
    GetName: usize,
    pub SetName: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IAutomationPropertiesStatics2,
    IAutomationPropertiesStatics2_Vtbl,
    0xd933a3ed_e90a_5df0_853d_cad17a0b9ec8
);
impl windows_core::RuntimeType for IAutomationPropertiesStatics2 {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Automation.IAutomationPropertiesStatics2");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IAutomationPropertiesStatics2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IBitmapSource, IBitmapSource_Vtbl, 0x8424269d_9b82_534f_8fea_af5b5ef96bf2);
impl windows_core::RuntimeType for IBitmapSource {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.Imaging.IBitmapSource");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IBitmapSource_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IBitmapSourceFactory, IBitmapSourceFactory_Vtbl, 0x0392f025_1868_5876_ad67_12e94a8da5bf);
impl windows_core::RuntimeType for IBitmapSourceFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.Imaging.IBitmapSourceFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IBitmapSourceFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IBitmapSourceStatics, IBitmapSourceStatics_Vtbl, 0xefa3745e_4400_5f0b_bdc7_3f2911a3d719);
impl windows_core::RuntimeType for IBitmapSourceStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.Imaging.IBitmapSourceStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IBitmapSourceStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IBorder, IBorder_Vtbl, 0x1ca13b47_ff5c_5abc_a411_a177df9482a9);
impl windows_core::RuntimeType for IBorder {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IBorder");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IBorder_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    BorderBrush: usize,
    pub SetBorderBrush: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    BorderThickness: usize,
    pub SetBorderThickness: unsafe extern "system" fn(*mut core::ffi::c_void, Thickness) -> windows_core::HRESULT,
    Background: usize,
    pub SetBackground: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    BackgroundSizing: usize,
    SetBackgroundSizing: usize,
    CornerRadius: usize,
    pub SetCornerRadius: unsafe extern "system" fn(*mut core::ffi::c_void, CornerRadius) -> windows_core::HRESULT,
    Padding: usize,
    pub SetPadding: unsafe extern "system" fn(*mut core::ffi::c_void, Thickness) -> windows_core::HRESULT,
    Child: usize,
    pub SetChild: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IBorderStatics, IBorderStatics_Vtbl, 0x9385cd66_c178_5176_b4a5_160e00e93935);
impl windows_core::RuntimeType for IBorderStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IBorderStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IBorderStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IBrush, IBrush_Vtbl, 0x2de3cb83_1329_5679_88f8_c822bc5442cb);
impl windows_core::RuntimeType for IBrush {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.IBrush");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IBrush_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IBrushFactory, IBrushFactory_Vtbl, 0xb5258717_6c49_5ba5_87fd_35df382647a5);
impl windows_core::RuntimeType for IBrushFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.IBrushFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IBrushFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IBrushStatics, IBrushStatics_Vtbl, 0x5b854f50_f818_5f01_91b0_28132d3f5957);
impl windows_core::RuntimeType for IBrushStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.IBrushStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IBrushStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IBuffer, IBuffer_Vtbl, 0x905a0fe0_bc53_11df_8c49_001e4fc686da);
impl windows_core::RuntimeType for IBuffer {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Windows.Storage.Streams.IBuffer");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
windows_core::imp::interface_hierarchy!(IBuffer, windows_core::IUnknown, windows_core::IInspectable);
impl windows_core::RuntimeName for IBuffer {
    const NAME: &'static str = "Windows.Storage.Streams.IBuffer";
}
#[repr(C)]
pub struct IBuffer_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IButton, IButton_Vtbl, 0x216c183d_d07a_5aa5_b8a4_0300a2683e87);
impl windows_core::RuntimeType for IButton {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IButton");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IButton_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IButtonBase, IButtonBase_Vtbl, 0x65714269_2473_5327_a652_0ea6bce7f403);
impl windows_core::RuntimeType for IButtonBase {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.Primitives.IButtonBase");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IButtonBase_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    ClickMode: usize,
    SetClickMode: usize,
    IsPointerOver: usize,
    IsPressed: usize,
    Command: usize,
    SetCommand: usize,
    CommandParameter: usize,
    SetCommandParameter: usize,
    pub Click: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemoveClick: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IButtonBaseFactory, IButtonBaseFactory_Vtbl, 0x21251aa9_6fd1_5e51_ab3b_e6fcaf3395ed);
impl windows_core::RuntimeType for IButtonBaseFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.Primitives.IButtonBaseFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IButtonBaseFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IButtonBaseStatics, IButtonBaseStatics_Vtbl, 0xdbe812f6_adf8_51d3_8137_a8fbf6445b3c);
impl windows_core::RuntimeType for IButtonBaseStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.Primitives.IButtonBaseStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IButtonBaseStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IButtonFactory, IButtonFactory_Vtbl, 0xfe393422_d91c_57b1_9a9c_2c7e3f41f77c);
impl windows_core::RuntimeType for IButtonFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IButtonFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IButtonFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IButtonStatics, IButtonStatics_Vtbl, 0x57823d25_b26a_5e0f_94f6_bbae70683dc5);
impl windows_core::RuntimeType for IButtonStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IButtonStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IButtonStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IColorChangedEventArgs, IColorChangedEventArgs_Vtbl, 0x148d57a2_b1cb_5f5d_b6b5_512805d71761);
impl windows_core::RuntimeType for IColorChangedEventArgs {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IColorChangedEventArgs");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IColorChangedEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    OldColor: usize,
    pub NewColor: unsafe extern "system" fn(*mut core::ffi::c_void, *mut Color) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IColorPicker, IColorPicker_Vtbl, 0xae72b24b_f93f_5a19_8ce4_a18b73c3356d);
impl windows_core::RuntimeType for IColorPicker {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IColorPicker");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IColorPicker_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Color: unsafe extern "system" fn(*mut core::ffi::c_void, *mut Color) -> windows_core::HRESULT,
    pub SetColor: unsafe extern "system" fn(*mut core::ffi::c_void, Color) -> windows_core::HRESULT,
    PreviousColor: usize,
    SetPreviousColor: usize,
    IsAlphaEnabled: usize,
    pub SetIsAlphaEnabled: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    IsColorSpectrumVisible: usize,
    SetIsColorSpectrumVisible: usize,
    IsColorPreviewVisible: usize,
    SetIsColorPreviewVisible: usize,
    IsColorSliderVisible: usize,
    SetIsColorSliderVisible: usize,
    IsAlphaSliderVisible: usize,
    SetIsAlphaSliderVisible: usize,
    IsMoreButtonVisible: usize,
    SetIsMoreButtonVisible: usize,
    IsColorChannelTextInputVisible: usize,
    pub SetIsColorChannelTextInputVisible: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    IsAlphaTextInputVisible: usize,
    SetIsAlphaTextInputVisible: usize,
    IsHexInputVisible: usize,
    pub SetIsHexInputVisible: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    MinHue: usize,
    SetMinHue: usize,
    MaxHue: usize,
    SetMaxHue: usize,
    MinSaturation: usize,
    SetMinSaturation: usize,
    MaxSaturation: usize,
    SetMaxSaturation: usize,
    MinValue: usize,
    SetMinValue: usize,
    MaxValue: usize,
    SetMaxValue: usize,
    ColorSpectrumShape: usize,
    SetColorSpectrumShape: usize,
    ColorSpectrumComponents: usize,
    SetColorSpectrumComponents: usize,
    pub ColorChanged: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemoveColorChanged: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IColorPickerFactory, IColorPickerFactory_Vtbl, 0x72c350e2_0a20_5b9b_ac54_633b97d7ffde);
impl windows_core::RuntimeType for IColorPickerFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IColorPickerFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IColorPickerFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IColorPickerStatics, IColorPickerStatics_Vtbl, 0x920d2d62_172e_5771_b677_4069b5134574);
impl windows_core::RuntimeType for IColorPickerStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IColorPickerStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IColorPickerStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IColorPickerStatics2, IColorPickerStatics2_Vtbl, 0x2f4af95b_81cf_5304_9950_49ec934c1b35);
impl windows_core::RuntimeType for IColorPickerStatics2 {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IColorPickerStatics2");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IColorPickerStatics2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IColumnDefinition, IColumnDefinition_Vtbl, 0x454cea14_87ec_5890_bb62_f1d82a94758e);
impl windows_core::RuntimeType for IColumnDefinition {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IColumnDefinition");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IColumnDefinition_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Width: usize,
    pub SetWidth: unsafe extern "system" fn(*mut core::ffi::c_void, GridLength) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IColumnDefinitionStatics, IColumnDefinitionStatics_Vtbl, 0xaba4b755_ca63_5ed7_ae31_45f9f91920b0);
impl windows_core::RuntimeType for IColumnDefinitionStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IColumnDefinitionStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IColumnDefinitionStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IComboBox, IComboBox_Vtbl, 0xc77da58b_4fd7_51e0_a431_f84658a83e9e);
impl windows_core::RuntimeType for IComboBox {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IComboBox");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IComboBox_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    IsDropDownOpen: usize,
    SetIsDropDownOpen: usize,
    IsEditable: usize,
    SetIsEditable: usize,
    IsSelectionBoxHighlighted: usize,
    MaxDropDownHeight: usize,
    SetMaxDropDownHeight: usize,
    SelectionBoxItem: usize,
    SelectionBoxItemTemplate: usize,
    TemplateSettings: usize,
    Header: usize,
    SetHeader: usize,
    HeaderTemplate: usize,
    SetHeaderTemplate: usize,
    PlaceholderText: usize,
    pub SetPlaceholderText: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IComboBoxFactory, IComboBoxFactory_Vtbl, 0x71c1014b_acdf_5c03_b5ed_02871caaeb6b);
impl windows_core::RuntimeType for IComboBoxFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IComboBoxFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IComboBoxFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IComboBoxStatics, IComboBoxStatics_Vtbl, 0x7da7cd09_e256_5238_9fed_02b2f3afba25);
impl windows_core::RuntimeType for IComboBoxStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IComboBoxStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IComboBoxStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IContentControl, IContentControl_Vtbl, 0x07e81761_11b2_52ae_8f8b_4d53d2b5900a);
impl windows_core::RuntimeType for IContentControl {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IContentControl");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IContentControl_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Content: usize,
    pub SetContent: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IContentControlFactory, IContentControlFactory_Vtbl, 0x3dea958e_5acd_5f80_8938_38634f51493a);
impl windows_core::RuntimeType for IContentControlFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IContentControlFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IContentControlFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IContentControlStatics, IContentControlStatics_Vtbl, 0xf25484f4_2fed_5a0a_8864_7d6d4ac43ef8);
impl windows_core::RuntimeType for IContentControlStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IContentControlStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IContentControlStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IContentDialog, IContentDialog_Vtbl, 0xac2145a3_4a32_5305_a81d_47509515bfce);
impl windows_core::RuntimeType for IContentDialog {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IContentDialog");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IContentDialog_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Title: usize,
    pub SetTitle: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    TitleTemplate: usize,
    SetTitleTemplate: usize,
    FullSizeDesired: usize,
    SetFullSizeDesired: usize,
    PrimaryButtonText: usize,
    pub SetPrimaryButtonText: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    SecondaryButtonText: usize,
    SetSecondaryButtonText: usize,
    CloseButtonText: usize,
    pub SetCloseButtonText: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    PrimaryButtonCommand: usize,
    SetPrimaryButtonCommand: usize,
    SecondaryButtonCommand: usize,
    SetSecondaryButtonCommand: usize,
    CloseButtonCommand: usize,
    SetCloseButtonCommand: usize,
    PrimaryButtonCommandParameter: usize,
    SetPrimaryButtonCommandParameter: usize,
    SecondaryButtonCommandParameter: usize,
    SetSecondaryButtonCommandParameter: usize,
    CloseButtonCommandParameter: usize,
    SetCloseButtonCommandParameter: usize,
    IsPrimaryButtonEnabled: usize,
    pub SetIsPrimaryButtonEnabled: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    IsSecondaryButtonEnabled: usize,
    SetIsSecondaryButtonEnabled: usize,
    PrimaryButtonStyle: usize,
    SetPrimaryButtonStyle: usize,
    SecondaryButtonStyle: usize,
    SetSecondaryButtonStyle: usize,
    CloseButtonStyle: usize,
    SetCloseButtonStyle: usize,
    DefaultButton: usize,
    SetDefaultButton: usize,
    Closing: usize,
    RemoveClosing: usize,
    pub Closed: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemoveClosed: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    Opened: usize,
    RemoveOpened: usize,
    PrimaryButtonClick: usize,
    RemovePrimaryButtonClick: usize,
    SecondaryButtonClick: usize,
    RemoveSecondaryButtonClick: usize,
    CloseButtonClick: usize,
    RemoveCloseButtonClick: usize,
    pub Hide: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
    pub ShowAsync: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(
    IContentDialogClosedEventArgs,
    IContentDialogClosedEventArgs_Vtbl,
    0x9b84e681_1ab6_5485_88b2_d0d3c05b29f3
);
impl windows_core::RuntimeType for IContentDialogClosedEventArgs {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IContentDialogClosedEventArgs");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IContentDialogClosedEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Result: unsafe extern "system" fn(*mut core::ffi::c_void, *mut ContentDialogResult) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IContentDialogFactory, IContentDialogFactory_Vtbl, 0xa05b3ad7_c60e_545a_9ee4_f098220ed816);
impl windows_core::RuntimeType for IContentDialogFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IContentDialogFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IContentDialogFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IContentDialogStatics, IContentDialogStatics_Vtbl, 0x397b18dd_6964_5056_97c3_d69cd7716a4b);
impl windows_core::RuntimeType for IContentDialogStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IContentDialogStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IContentDialogStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IControl, IControl_Vtbl, 0x857d6e8a_d45a_5c69_a99c_bf6a5c54fb38);
impl windows_core::RuntimeType for IControl {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IControl");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IControl_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    IsFocusEngagementEnabled: usize,
    SetIsFocusEngagementEnabled: usize,
    IsFocusEngaged: usize,
    SetIsFocusEngaged: usize,
    RequiresPointer: usize,
    SetRequiresPointer: usize,
    FontSize: usize,
    SetFontSize: usize,
    FontFamily: usize,
    SetFontFamily: usize,
    FontWeight: usize,
    SetFontWeight: usize,
    FontStyle: usize,
    SetFontStyle: usize,
    FontStretch: usize,
    SetFontStretch: usize,
    CharacterSpacing: usize,
    SetCharacterSpacing: usize,
    Foreground: usize,
    SetForeground: usize,
    IsTextScaleFactorEnabled: usize,
    SetIsTextScaleFactorEnabled: usize,
    IsEnabled: usize,
    pub SetIsEnabled: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    TabNavigation: usize,
    SetTabNavigation: usize,
    Template: usize,
    SetTemplate: usize,
    Padding: usize,
    SetPadding: usize,
    HorizontalContentAlignment: usize,
    pub SetHorizontalContentAlignment: unsafe extern "system" fn(*mut core::ffi::c_void, HorizontalAlignment) -> windows_core::HRESULT,
    VerticalContentAlignment: usize,
    pub SetVerticalContentAlignment: unsafe extern "system" fn(*mut core::ffi::c_void, VerticalAlignment) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IControlFactory, IControlFactory_Vtbl, 0x25159233_9438_5534_aeb9_00eb059cf73f);
impl windows_core::RuntimeType for IControlFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IControlFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IControlFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IControlStatics, IControlStatics_Vtbl, 0xc3ae388d_aa36_5e10_acac_98415f47bcc7);
impl windows_core::RuntimeType for IControlStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IControlStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IControlStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IDependencyObject, IDependencyObject_Vtbl, 0xe7beaee7_160e_50f7_8789_d63463f979fa);
impl windows_core::RuntimeType for IDependencyObject {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IDependencyObject");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IDependencyObject_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    GetValue: usize,
    SetValue: usize,
    pub ClearValue: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IDependencyObjectFactory, IDependencyObjectFactory_Vtbl, 0x936b614c_475f_5d7d_b3f7_bf1fbea28126);
impl windows_core::RuntimeType for IDependencyObjectFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IDependencyObjectFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IDependencyObjectFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IDependencyProperty, IDependencyProperty_Vtbl, 0x960eab49_9672_58a0_995b_3a42e5ea6278);
impl windows_core::RuntimeType for IDependencyProperty {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IDependencyProperty");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IDependencyProperty_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IDependencyPropertyStatics, IDependencyPropertyStatics_Vtbl, 0x61ddc651_0383_5d6f_98ce_5c046aaaaa8f);
impl windows_core::RuntimeType for IDependencyPropertyStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IDependencyPropertyStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IDependencyPropertyStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IDispatcherQueue, IDispatcherQueue_Vtbl, 0xf6ebf8fa_be1c_5bf6_a467_73da28738ae8);
impl windows_core::RuntimeType for IDispatcherQueue {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Dispatching.IDispatcherQueue");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IDispatcherQueue_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    CreateTimer: usize,
    TryEnqueue: usize,
    pub TryEnqueueWithPriority: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        DispatcherQueuePriority,
        *mut core::ffi::c_void,
        *mut bool,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IDispatcherQueueStatics, IDispatcherQueueStatics_Vtbl, 0xcd3382ea_a455_5124_b63a_ca40d34ca23c);
impl windows_core::RuntimeType for IDispatcherQueueStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Dispatching.IDispatcherQueueStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IDispatcherQueueStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub GetForCurrentThread: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IFontIcon, IFontIcon_Vtbl, 0x6eba5ed9_d233_5f5e_91a8_f5134292658a);
impl windows_core::RuntimeType for IFontIcon {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IFontIcon");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IFontIcon_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Glyph: usize,
    pub SetGlyph: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IFontIconFactory, IFontIconFactory_Vtbl, 0xaa9a24fe_bef8_564a_b200_694cd6f6ba4e);
impl windows_core::RuntimeType for IFontIconFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IFontIconFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IFontIconFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IFontIconStatics, IFontIconStatics_Vtbl, 0x55564030_2f80_5be5_8c2a_ebb6ecba07e8);
impl windows_core::RuntimeType for IFontIconStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IFontIconStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IFontIconStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IFrameworkElement, IFrameworkElement_Vtbl, 0xfe08f13d_dc6a_5495_ad44_c2d8d21863b0);
impl windows_core::RuntimeType for IFrameworkElement {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IFrameworkElement");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IFrameworkElement_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Triggers: usize,
    Resources: usize,
    SetResources: usize,
    pub Tag: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    pub SetTag: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    Language: usize,
    SetLanguage: usize,
    pub ActualWidth: unsafe extern "system" fn(*mut core::ffi::c_void, *mut f64) -> windows_core::HRESULT,
    pub ActualHeight: unsafe extern "system" fn(*mut core::ffi::c_void, *mut f64) -> windows_core::HRESULT,
    Width: usize,
    pub SetWidth: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    Height: usize,
    pub SetHeight: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    MinWidth: usize,
    pub SetMinWidth: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    MaxWidth: usize,
    pub SetMaxWidth: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    MinHeight: usize,
    pub SetMinHeight: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    MaxHeight: usize,
    pub SetMaxHeight: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    HorizontalAlignment: usize,
    pub SetHorizontalAlignment: unsafe extern "system" fn(*mut core::ffi::c_void, HorizontalAlignment) -> windows_core::HRESULT,
    VerticalAlignment: usize,
    pub SetVerticalAlignment: unsafe extern "system" fn(*mut core::ffi::c_void, VerticalAlignment) -> windows_core::HRESULT,
    Margin: usize,
    pub SetMargin: unsafe extern "system" fn(*mut core::ffi::c_void, Thickness) -> windows_core::HRESULT,
    Name: usize,
    SetName: usize,
    BaseUri: usize,
    DataContext: usize,
    SetDataContext: usize,
    AllowFocusOnInteraction: usize,
    SetAllowFocusOnInteraction: usize,
    FocusVisualMargin: usize,
    SetFocusVisualMargin: usize,
    FocusVisualSecondaryThickness: usize,
    SetFocusVisualSecondaryThickness: usize,
    FocusVisualPrimaryThickness: usize,
    SetFocusVisualPrimaryThickness: usize,
    FocusVisualSecondaryBrush: usize,
    SetFocusVisualSecondaryBrush: usize,
    FocusVisualPrimaryBrush: usize,
    SetFocusVisualPrimaryBrush: usize,
    AllowFocusWhenDisabled: usize,
    SetAllowFocusWhenDisabled: usize,
    Style: usize,
    pub SetStyle: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    Parent: usize,
    FlowDirection: usize,
    SetFlowDirection: usize,
    RequestedTheme: usize,
    pub SetRequestedTheme: unsafe extern "system" fn(*mut core::ffi::c_void, ElementTheme) -> windows_core::HRESULT,
    IsLoaded: usize,
    ActualTheme: usize,
    pub Loaded: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemoveLoaded: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IFrameworkElementFactory, IFrameworkElementFactory_Vtbl, 0xbd3f2272_3efa_5f92_b759_90b1cc3e784c);
impl windows_core::RuntimeType for IFrameworkElementFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IFrameworkElementFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IFrameworkElementFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IFrameworkElementStatics, IFrameworkElementStatics_Vtbl, 0x894e2704_14e7_569a_b21e_afc7df7145a1);
impl windows_core::RuntimeType for IFrameworkElementStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IFrameworkElementStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IFrameworkElementStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    TagProperty: usize,
    LanguageProperty: usize,
    ActualWidthProperty: usize,
    ActualHeightProperty: usize,
    WidthProperty: usize,
    HeightProperty: usize,
    MinWidthProperty: usize,
    MaxWidthProperty: usize,
    MinHeightProperty: usize,
    MaxHeightProperty: usize,
    HorizontalAlignmentProperty: usize,
    VerticalAlignmentProperty: usize,
    MarginProperty: usize,
    NameProperty: usize,
    DataContextProperty: usize,
    AllowFocusOnInteractionProperty: usize,
    FocusVisualMarginProperty: usize,
    FocusVisualSecondaryThicknessProperty: usize,
    FocusVisualPrimaryThicknessProperty: usize,
    FocusVisualSecondaryBrushProperty: usize,
    FocusVisualPrimaryBrushProperty: usize,
    AllowFocusWhenDisabledProperty: usize,
    pub StyleProperty: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IGrid, IGrid_Vtbl, 0xc4496219_9014_58a1_b4ad_c5044913a5bb);
impl windows_core::RuntimeType for IGrid {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IGrid");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IGrid_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub RowDefinitions: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    pub ColumnDefinitions: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IGridFactory, IGridFactory_Vtbl, 0xb16bf561_fc6c_57c6_8ebc_0b06ce4513aa);
impl windows_core::RuntimeType for IGridFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IGridFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IGridFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IGridStatics, IGridStatics_Vtbl, 0xef9cf81d_a431_50f4_abf5_3023fe447704);
impl windows_core::RuntimeType for IGridStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IGridStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IGridStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    BackgroundSizingProperty: usize,
    BorderBrushProperty: usize,
    BorderThicknessProperty: usize,
    CornerRadiusProperty: usize,
    PaddingProperty: usize,
    RowSpacingProperty: usize,
    ColumnSpacingProperty: usize,
    RowProperty: usize,
    GetRow: usize,
    pub SetRow: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, i32) -> windows_core::HRESULT,
    ColumnProperty: usize,
    GetColumn: usize,
    pub SetColumn: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, i32) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IIconElement, IIconElement_Vtbl, 0x18f69350_279e_50ea_8d23_138e717ed939);
impl windows_core::RuntimeType for IIconElement {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IIconElement");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IIconElement_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IIconElementStatics, IIconElementStatics_Vtbl, 0x4ccfdc81_9d9d_5a02_b95b_ad1add921750);
impl windows_core::RuntimeType for IIconElementStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IIconElementStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IIconElementStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IImage, IImage_Vtbl, 0x220d3d8d_66de_53a1_a215_ba9c165565ab);
impl windows_core::RuntimeType for IImage {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IImage");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IImage_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Source: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    pub SetSource: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    Stretch: usize,
    pub SetStretch: unsafe extern "system" fn(*mut core::ffi::c_void, Stretch) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IImageSource, IImageSource_Vtbl, 0x6c2038f6_d6d5_55e9_9b9e_082f12dbff60);
impl windows_core::RuntimeType for IImageSource {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.IImageSource");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IImageSource_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IImageStatics, IImageStatics_Vtbl, 0xcb5378a8_996b_547e_9c4c_7bbfd9dbe7dc);
impl windows_core::RuntimeType for IImageStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IImageStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IImageStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IInfoBar, IInfoBar_Vtbl, 0xc1c3a438_dd79_5d22_9e42_5a3cdf8113a9);
impl windows_core::RuntimeType for IInfoBar {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IInfoBar");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IInfoBar_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    IsOpen: usize,
    pub SetIsOpen: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    Title: usize,
    pub SetTitle: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    Message: usize,
    pub SetMessage: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    Severity: usize,
    pub SetSeverity: unsafe extern "system" fn(*mut core::ffi::c_void, InfoBarSeverity) -> windows_core::HRESULT,
    IconSource: usize,
    SetIconSource: usize,
    IsIconVisible: usize,
    SetIsIconVisible: usize,
    IsClosable: usize,
    pub SetIsClosable: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IInfoBarFactory, IInfoBarFactory_Vtbl, 0x60618a60_9be7_5df5_be0d_933d34ddb44c);
impl windows_core::RuntimeType for IInfoBarFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IInfoBarFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IInfoBarFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IInfoBarStatics, IInfoBarStatics_Vtbl, 0x5fb1e80d_ea5d_57ba_9915_b6782b982aca);
impl windows_core::RuntimeType for IInfoBarStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IInfoBarStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IInfoBarStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IItemsControl, IItemsControl_Vtbl, 0xbf1ccb54_83e2_5b98_acbc_736f876c3d35);
impl windows_core::RuntimeType for IItemsControl {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IItemsControl");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IItemsControl_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    ItemsSource: usize,
    pub SetItemsSource: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IItemsControlFactory, IItemsControlFactory_Vtbl, 0xb2686273_2dd3_5454_9f25_247e66270336);
impl windows_core::RuntimeType for IItemsControlFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IItemsControlFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IItemsControlFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IItemsControlStatics, IItemsControlStatics_Vtbl, 0x4fb961e1_5f91_5585_a692_6825cde3ed1e);
impl windows_core::RuntimeType for IItemsControlStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IItemsControlStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IItemsControlStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IIterable<T>(windows_core::IUnknown, core::marker::PhantomData<T>)
where
    T: windows_core::RuntimeType + 'static;
impl<T: windows_core::RuntimeType + 'static> windows_core::imp::CanInto<windows_core::IUnknown> for IIterable<T> {}
impl<T: windows_core::RuntimeType + 'static> windows_core::imp::CanInto<windows_core::IInspectable> for IIterable<T> {}
unsafe impl<T: windows_core::RuntimeType + 'static> windows_core::Interface for IIterable<T> {
    type Vtable = IIterable_Vtbl<T>;

    const IID: windows_core::GUID = windows_core::GUID::from_signature(<Self as windows_core::RuntimeType>::SIGNATURE);
}
impl<T: windows_core::RuntimeType + 'static> windows_core::RuntimeType for IIterable<T> {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::new()
        .push_slice(b"Windows.Foundation.Collections.IIterable`1<")
        .push_other(T::NAME)
        .push_slice(b">");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::new()
        .push_slice(b"pinterface({faa585ea-6214-4217-afda-7f46de5869b3}")
        .push_slice(b";")
        .push_other(T::SIGNATURE)
        .push_slice(b")");
}
impl<T: windows_core::RuntimeType + 'static> IIterable<T> {
    pub fn First(&self) -> windows_core::Result<windows_collections::IIterator<T>> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).First)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }
}
impl<T: windows_core::RuntimeType + 'static> windows_core::RuntimeName for IIterable<T> {
    const NAME: &'static str = "Windows.Foundation.Collections.IIterable";
    const RUNTIME_CLASS_NAME: windows_core::imp::ConstBuffer = <Self as windows_core::RuntimeType>::NAME;
}
pub trait IIterable_Impl<T>: windows_core::IUnknownImpl
where
    T: windows_core::RuntimeType + 'static,
{
    fn First(&self) -> windows_core::Result<windows_collections::IIterator<T>>;
}
impl<T: windows_core::RuntimeType + 'static> IIterable_Vtbl<T> {
    pub const fn new<Identity: IIterable_Impl<T>, const OFFSET: isize>() -> Self {
        unsafe extern "system" fn First<T: windows_core::RuntimeType + 'static, Identity: IIterable_Impl<T>, const OFFSET: isize>(
            this: *mut core::ffi::c_void,
            result__: *mut *mut core::ffi::c_void,
        ) -> windows_core::HRESULT {
            unsafe {
                let this: &Identity = &*((this as *const *const ()).offset(OFFSET) as *const Identity);
                match IIterable_Impl::First(this) {
                    Ok(ok__) => {
                        result__.write(core::mem::transmute_copy(&ok__));
                        core::mem::forget(ok__);
                        windows_core::HRESULT(0)
                    }
                    Err(err) => err.into(),
                }
            }
        }
        Self {
            base__: windows_core::IInspectable_Vtbl::new::<Identity, IIterable<T>, OFFSET>(),
            First: First::<T, Identity, OFFSET>,
            T: core::marker::PhantomData::<T>,
        }
    }

    pub fn matches(iid: &windows_core::GUID) -> bool {
        iid == &<IIterable<T> as windows_core::Interface>::IID
    }
}
#[repr(C)]
pub struct IIterable_Vtbl<T>
where
    T: windows_core::RuntimeType + 'static,
{
    pub base__: windows_core::IInspectable_Vtbl,
    pub First: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    T: core::marker::PhantomData<T>,
}
impl<T: windows_core::RuntimeType> IntoIterator for IIterable<T> {
    type IntoIter = windows_collections::BufferedIterator<Self::Item>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(&self)
    }
}
impl<T: windows_core::RuntimeType> IntoIterator for &IIterable<T> {
    type IntoIter = windows_collections::BufferedIterator<Self::Item>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        windows_collections::BufferedIterator::new(self.First().unwrap())
    }
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IMap<K, V>(windows_core::IUnknown, core::marker::PhantomData<K>, core::marker::PhantomData<V>)
where
    K: windows_core::RuntimeType + 'static,
    V: windows_core::RuntimeType + 'static;
impl<K: windows_core::RuntimeType + 'static, V: windows_core::RuntimeType + 'static> windows_core::imp::CanInto<windows_core::IUnknown>
    for IMap<K, V>
{
}
impl<K: windows_core::RuntimeType + 'static, V: windows_core::RuntimeType + 'static> windows_core::imp::CanInto<windows_core::IInspectable>
    for IMap<K, V>
{
}
unsafe impl<K: windows_core::RuntimeType + 'static, V: windows_core::RuntimeType + 'static> windows_core::Interface for IMap<K, V> {
    type Vtable = IMap_Vtbl<K, V>;

    const IID: windows_core::GUID = windows_core::GUID::from_signature(<Self as windows_core::RuntimeType>::SIGNATURE);
}
impl<K: windows_core::RuntimeType + 'static, V: windows_core::RuntimeType + 'static> windows_core::RuntimeType for IMap<K, V> {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::new()
        .push_slice(b"Windows.Foundation.Collections.IMap`2<")
        .push_other(K::NAME)
        .push_slice(b", ")
        .push_other(V::NAME)
        .push_slice(b">");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::new()
        .push_slice(b"pinterface({3c2925fe-8519-45c1-aa79-197b6718c1c1}")
        .push_slice(b";")
        .push_other(K::SIGNATURE)
        .push_slice(b";")
        .push_other(V::SIGNATURE)
        .push_slice(b")");
}
impl<K: windows_core::RuntimeType + 'static, V: windows_core::RuntimeType + 'static> IMap<K, V> {
    pub fn Lookup<P0>(&self, key: P0) -> windows_core::Result<V>
    where
        P0: windows_core::Param<K>,
    {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Lookup)(windows_core::Interface::as_raw(self), key.param().abi(), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn HasKey<P0>(&self, key: P0) -> windows_core::Result<bool>
    where
        P0: windows_core::Param<K>,
    {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).HasKey)(windows_core::Interface::as_raw(self), key.param().abi(), &mut result__)
                .map(|| result__)
        }
    }

    pub fn Insert<P0, P1>(&self, key: P0, value: P1) -> windows_core::Result<bool>
    where
        P0: windows_core::Param<K>,
        P1: windows_core::Param<V>,
    {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Insert)(
                windows_core::Interface::as_raw(self),
                key.param().abi(),
                value.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }

    pub fn Remove<P0>(&self, key: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<K>,
    {
        unsafe { (windows_core::Interface::vtable(self).Remove)(windows_core::Interface::as_raw(self), key.param().abi()).ok() }
    }

    pub fn First(&self) -> windows_core::Result<windows_collections::IIterator<windows_collections::IKeyValuePair<K, V>>> {
        let this = &windows_core::Interface::cast::<IIterable<windows_collections::IKeyValuePair<K, V>>>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).First)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }
}
impl<K: windows_core::RuntimeType + 'static, V: windows_core::RuntimeType + 'static> IntoIterator for IMap<K, V> {
    type IntoIter = windows_collections::BufferedIterator<Self::Item>;
    type Item = windows_collections::IKeyValuePair<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(&self)
    }
}
impl<K: windows_core::RuntimeType + 'static, V: windows_core::RuntimeType + 'static> IntoIterator for &IMap<K, V> {
    type IntoIter = windows_collections::BufferedIterator<Self::Item>;
    type Item = windows_collections::IKeyValuePair<K, V>;

    fn into_iter(self) -> Self::IntoIter {
        windows_collections::BufferedIterator::new(self.First().unwrap())
    }
}
impl<K: windows_core::RuntimeType + 'static, V: windows_core::RuntimeType + 'static> windows_core::RuntimeName for IMap<K, V> {
    const NAME: &'static str = "Windows.Foundation.Collections.IMap";
    const RUNTIME_CLASS_NAME: windows_core::imp::ConstBuffer = <Self as windows_core::RuntimeType>::NAME;
}
#[repr(C)]
pub struct IMap_Vtbl<K, V>
where
    K: windows_core::RuntimeType + 'static,
    V: windows_core::RuntimeType + 'static,
{
    pub base__: windows_core::IInspectable_Vtbl,
    pub Lookup: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        windows_core::imp::AbiType<K>,
        *mut windows_core::imp::AbiType<V>,
    ) -> windows_core::HRESULT,
    Size: usize,
    pub HasKey: unsafe extern "system" fn(*mut core::ffi::c_void, windows_core::imp::AbiType<K>, *mut bool) -> windows_core::HRESULT,
    GetView: usize,
    pub Insert: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        windows_core::imp::AbiType<K>,
        windows_core::imp::AbiType<V>,
        *mut bool,
    ) -> windows_core::HRESULT,
    pub Remove: unsafe extern "system" fn(*mut core::ffi::c_void, windows_core::imp::AbiType<K>) -> windows_core::HRESULT,
    K: core::marker::PhantomData<K>,
    V: core::marker::PhantomData<V>,
}
windows_core::imp::define_interface!(IMicaBackdrop, IMicaBackdrop_Vtbl, 0xc156a404_3dac_593a_b1f3_7a33c289dc83);
impl windows_core::RuntimeType for IMicaBackdrop {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.IMicaBackdrop");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMicaBackdrop_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Kind: usize,
    pub SetKind: unsafe extern "system" fn(*mut core::ffi::c_void, MicaKind) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IMicaBackdropFactory, IMicaBackdropFactory_Vtbl, 0x774379ce_74bd_59d4_849d_d99c4184d838);
impl windows_core::RuntimeType for IMicaBackdropFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.IMicaBackdropFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMicaBackdropFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IMicaBackdropStatics, IMicaBackdropStatics_Vtbl, 0xa63abdce_c796_5509_9f4d_072bc1e599f1);
impl windows_core::RuntimeType for IMicaBackdropStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.IMicaBackdropStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IMicaBackdropStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(INavigationView, INavigationView_Vtbl, 0xe77a4b36_3dd1_53d9_9f97_65dccaa74a5c);
impl windows_core::RuntimeType for INavigationView {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INavigationView");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INavigationView_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub IsPaneOpen: unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub SetIsPaneOpen: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    CompactModeThresholdWidth: usize,
    SetCompactModeThresholdWidth: usize,
    ExpandedModeThresholdWidth: usize,
    SetExpandedModeThresholdWidth: usize,
    FooterMenuItems: usize,
    FooterMenuItemsSource: usize,
    SetFooterMenuItemsSource: usize,
    PaneFooter: usize,
    pub SetPaneFooter: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    Header: usize,
    SetHeader: usize,
    HeaderTemplate: usize,
    SetHeaderTemplate: usize,
    DisplayMode: usize,
    IsSettingsVisible: usize,
    pub SetIsSettingsVisible: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    IsPaneToggleButtonVisible: usize,
    pub SetIsPaneToggleButtonVisible: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    AlwaysShowHeader: usize,
    SetAlwaysShowHeader: usize,
    CompactPaneLength: usize,
    SetCompactPaneLength: usize,
    OpenPaneLength: usize,
    pub SetOpenPaneLength: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    PaneToggleButtonStyle: usize,
    SetPaneToggleButtonStyle: usize,
    SelectedItem: usize,
    pub SetSelectedItem: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    pub MenuItems: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    MenuItemsSource: usize,
    SetMenuItemsSource: usize,
    SettingsItem: usize,
    AutoSuggestBox: usize,
    SetAutoSuggestBox: usize,
    MenuItemTemplate: usize,
    SetMenuItemTemplate: usize,
    MenuItemTemplateSelector: usize,
    SetMenuItemTemplateSelector: usize,
    MenuItemContainerStyle: usize,
    SetMenuItemContainerStyle: usize,
    MenuItemContainerStyleSelector: usize,
    SetMenuItemContainerStyleSelector: usize,
    MenuItemFromContainer: usize,
    ContainerFromMenuItem: usize,
    pub SelectionChanged: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemoveSelectionChanged: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(INavigationView2, INavigationView2_Vtbl, 0x05b428cf_014c_56dd_896a_a3e7089d73b5);
impl windows_core::RuntimeType for INavigationView2 {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INavigationView2");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INavigationView2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    IsBackButtonVisible: usize,
    pub SetIsBackButtonVisible: unsafe extern "system" fn(*mut core::ffi::c_void, NavigationViewBackButtonVisible) -> windows_core::HRESULT,
    IsBackEnabled: usize,
    SetIsBackEnabled: usize,
    PaneTitle: usize,
    SetPaneTitle: usize,
    BackRequested: usize,
    RemoveBackRequested: usize,
    pub PaneClosed: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemovePaneClosed: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    PaneClosing: usize,
    RemovePaneClosing: usize,
    pub PaneOpened: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemovePaneOpened: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    PaneOpening: usize,
    RemovePaneOpening: usize,
    PaneDisplayMode: usize,
    pub SetPaneDisplayMode: unsafe extern "system" fn(*mut core::ffi::c_void, NavigationViewPaneDisplayMode) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(INavigationViewFactory, INavigationViewFactory_Vtbl, 0xffea1ada_9232_5507_a320_ed2fadbe6127);
impl windows_core::RuntimeType for INavigationViewFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INavigationViewFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INavigationViewFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(INavigationViewItem, INavigationViewItem_Vtbl, 0x3ab3d503_a37c_5836_8adb_2882062e73a1);
impl windows_core::RuntimeType for INavigationViewItem {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INavigationViewItem");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INavigationViewItem_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Icon: usize,
    pub SetIcon: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(INavigationViewItemBase, INavigationViewItemBase_Vtbl, 0x33586494_af48_513f_be4d_f645e8c89005);
impl windows_core::RuntimeType for INavigationViewItemBase {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INavigationViewItemBase");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INavigationViewItemBase_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(
    INavigationViewItemBaseStatics,
    INavigationViewItemBaseStatics_Vtbl,
    0x71a0c438_f0c2_5093_8906_c1c3c02de791
);
impl windows_core::RuntimeType for INavigationViewItemBaseStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INavigationViewItemBaseStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INavigationViewItemBaseStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(INavigationViewItemFactory, INavigationViewItemFactory_Vtbl, 0xde60a001_9385_5535_80e1_2b68f4bfde26);
impl windows_core::RuntimeType for INavigationViewItemFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INavigationViewItemFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INavigationViewItemFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(INavigationViewItemHeader, INavigationViewItemHeader_Vtbl, 0x432bc062_45bc_57ef_a2d3_11851a56a882);
impl windows_core::RuntimeType for INavigationViewItemHeader {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INavigationViewItemHeader");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INavigationViewItemHeader_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(
    INavigationViewItemHeaderFactory,
    INavigationViewItemHeaderFactory_Vtbl,
    0x6a5447cd_2918_5fe3_899b_93d6961285e6
);
impl windows_core::RuntimeType for INavigationViewItemHeaderFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INavigationViewItemHeaderFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INavigationViewItemHeaderFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(INavigationViewItemStatics, INavigationViewItemStatics_Vtbl, 0x7b6198e5_0714_531c_a056_21b3ca40ec1a);
impl windows_core::RuntimeType for INavigationViewItemStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INavigationViewItemStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INavigationViewItemStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(INavigationViewItemStatics2, INavigationViewItemStatics2_Vtbl, 0xd49d016f_abd0_51f3_acf0_118ecceea760);
impl windows_core::RuntimeType for INavigationViewItemStatics2 {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INavigationViewItemStatics2");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INavigationViewItemStatics2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(INavigationViewItemStatics3, INavigationViewItemStatics3_Vtbl, 0x957beb66_d33a_53aa_a518_7d426bb9b177);
impl windows_core::RuntimeType for INavigationViewItemStatics3 {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INavigationViewItemStatics3");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INavigationViewItemStatics3_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(
    INavigationViewSelectionChangedEventArgs,
    INavigationViewSelectionChangedEventArgs_Vtbl,
    0x14a064a5_c79d_5f63_ac6e_1c313fe63566
);
impl windows_core::RuntimeType for INavigationViewSelectionChangedEventArgs {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INavigationViewSelectionChangedEventArgs");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INavigationViewSelectionChangedEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub SelectedItem: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(INavigationViewStatics, INavigationViewStatics_Vtbl, 0xdcd04caf_1904_564b_b0de_babaff9962f5);
impl windows_core::RuntimeType for INavigationViewStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INavigationViewStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INavigationViewStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(INavigationViewStatics2, INavigationViewStatics2_Vtbl, 0x79588150_4a1b_5e7c_9f8a_ebc814ce773d);
impl windows_core::RuntimeType for INavigationViewStatics2 {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INavigationViewStatics2");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INavigationViewStatics2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(INumberBox, INumberBox_Vtbl, 0xc18eb0e9_29fb_525d_abbc_d6b2110f542e);
impl windows_core::RuntimeType for INumberBox {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INumberBox");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INumberBox_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Minimum: usize,
    pub SetMinimum: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    Maximum: usize,
    pub SetMaximum: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    pub Value: unsafe extern "system" fn(*mut core::ffi::c_void, *mut f64) -> windows_core::HRESULT,
    pub SetValue: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    SmallChange: usize,
    pub SetSmallChange: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    LargeChange: usize,
    SetLargeChange: usize,
    Text: usize,
    SetText: usize,
    Header: usize,
    SetHeader: usize,
    HeaderTemplate: usize,
    SetHeaderTemplate: usize,
    PlaceholderText: usize,
    SetPlaceholderText: usize,
    SelectionFlyout: usize,
    SetSelectionFlyout: usize,
    SelectionHighlightColor: usize,
    SetSelectionHighlightColor: usize,
    TextReadingOrder: usize,
    SetTextReadingOrder: usize,
    PreventKeyboardDisplayOnProgrammaticFocus: usize,
    SetPreventKeyboardDisplayOnProgrammaticFocus: usize,
    Description: usize,
    SetDescription: usize,
    ValidationMode: usize,
    SetValidationMode: usize,
    SpinButtonPlacementMode: usize,
    SetSpinButtonPlacementMode: usize,
    IsWrapEnabled: usize,
    SetIsWrapEnabled: usize,
    AcceptsExpression: usize,
    SetAcceptsExpression: usize,
    NumberFormatter: usize,
    SetNumberFormatter: usize,
    pub ValueChanged: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemoveValueChanged: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(INumberBoxFactory, INumberBoxFactory_Vtbl, 0x6b81f3cb_45a4_5d19_9bbb_a9fe4656ac4d);
impl windows_core::RuntimeType for INumberBoxFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INumberBoxFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INumberBoxFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(INumberBoxStatics, INumberBoxStatics_Vtbl, 0x251ec05c_a77c_5540_be39_6053f797cde7);
impl windows_core::RuntimeType for INumberBoxStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INumberBoxStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INumberBoxStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(
    INumberBoxValueChangedEventArgs,
    INumberBoxValueChangedEventArgs_Vtbl,
    0xc66cf16e_7c8a_532e_9d23_058c1c98dd50
);
impl windows_core::RuntimeType for INumberBoxValueChangedEventArgs {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.INumberBoxValueChangedEventArgs");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct INumberBoxValueChangedEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    OldValue: usize,
    pub NewValue: unsafe extern "system" fn(*mut core::ffi::c_void, *mut f64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IPanel, IPanel_Vtbl, 0x27a1b418_56f3_525e_b883_cefed905eed3);
impl windows_core::RuntimeType for IPanel {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IPanel");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IPanel_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Children: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    Background: usize,
    pub SetBackground: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IPanelFactory, IPanelFactory_Vtbl, 0xf5e7e21c_4c97_5d20_bee6_3e4fc6ab14e9);
impl windows_core::RuntimeType for IPanelFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IPanelFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IPanelFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IPanelStatics, IPanelStatics_Vtbl, 0x76a9caa7_a5d4_5061_a325_17c76f66de51);
impl windows_core::RuntimeType for IPanelStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IPanelStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IPanelStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IPasswordBox, IPasswordBox_Vtbl, 0x6d3ccff7_aaee_5adc_8298_33300fa119da);
impl windows_core::RuntimeType for IPasswordBox {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IPasswordBox");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IPasswordBox_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Password: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    pub SetPassword: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    PasswordChar: usize,
    SetPasswordChar: usize,
    IsPasswordRevealButtonEnabled: usize,
    SetIsPasswordRevealButtonEnabled: usize,
    MaxLength: usize,
    SetMaxLength: usize,
    Header: usize,
    SetHeader: usize,
    HeaderTemplate: usize,
    SetHeaderTemplate: usize,
    PlaceholderText: usize,
    SetPlaceholderText: usize,
    SelectionHighlightColor: usize,
    SetSelectionHighlightColor: usize,
    PreventKeyboardDisplayOnProgrammaticFocus: usize,
    SetPreventKeyboardDisplayOnProgrammaticFocus: usize,
    PasswordRevealMode: usize,
    pub SetPasswordRevealMode: unsafe extern "system" fn(*mut core::ffi::c_void, PasswordRevealMode) -> windows_core::HRESULT,
    TextReadingOrder: usize,
    SetTextReadingOrder: usize,
    InputScope: usize,
    SetInputScope: usize,
    CanPasteClipboardContent: usize,
    SelectionFlyout: usize,
    SetSelectionFlyout: usize,
    Description: usize,
    SetDescription: usize,
    pub PasswordChanged: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemovePasswordChanged: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IPasswordBoxStatics, IPasswordBoxStatics_Vtbl, 0x90e6559c_5f74_5cd1_87b6_c0514085ab42);
impl windows_core::RuntimeType for IPasswordBoxStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IPasswordBoxStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IPasswordBoxStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IProgressRing, IProgressRing_Vtbl, 0x2670d03f_e28c_5652_bee2_b5212ebdf7ff);
impl windows_core::RuntimeType for IProgressRing {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IProgressRing");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IProgressRing_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    IsActive: usize,
    pub SetIsActive: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    IsIndeterminate: usize,
    pub SetIsIndeterminate: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IProgressRingFactory, IProgressRingFactory_Vtbl, 0x092fa98c_62a7_5dbc_9a85_3e556ba81f79);
impl windows_core::RuntimeType for IProgressRingFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IProgressRingFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IProgressRingFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IProgressRingStatics, IProgressRingStatics_Vtbl, 0x4ba1166a_cec2_50e2_b26b_66b69dc2e371);
impl windows_core::RuntimeType for IProgressRingStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IProgressRingStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IProgressRingStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IRadioButton, IRadioButton_Vtbl, 0x38f30cee_e75a_5ba1_ae64_4474a3abeac7);
impl windows_core::RuntimeType for IRadioButton {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IRadioButton");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IRadioButton_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    GroupName: usize,
    pub SetGroupName: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IRadioButtonFactory, IRadioButtonFactory_Vtbl, 0x5772c79a_b3eb_5719_8005_2a513429495a);
impl windows_core::RuntimeType for IRadioButtonFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IRadioButtonFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IRadioButtonFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IRadioButtonStatics, IRadioButtonStatics_Vtbl, 0xa8bce4d4_68fe_53d9_a317_74ab409cd4ee);
impl windows_core::RuntimeType for IRadioButtonStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IRadioButtonStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IRadioButtonStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IRangeBase, IRangeBase_Vtbl, 0x540d6d61_8fac_5d5c_b5b0_e172a7dde103);
impl windows_core::RuntimeType for IRangeBase {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.Primitives.IRangeBase");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IRangeBase_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Minimum: usize,
    pub SetMinimum: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    Maximum: usize,
    pub SetMaximum: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    SmallChange: usize,
    pub SetSmallChange: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    LargeChange: usize,
    SetLargeChange: usize,
    pub Value: unsafe extern "system" fn(*mut core::ffi::c_void, *mut f64) -> windows_core::HRESULT,
    pub SetValue: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    pub ValueChanged: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemoveValueChanged: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IRangeBaseFactory, IRangeBaseFactory_Vtbl, 0x41c205e2_4422_5dca_9b49_e31210ea396c);
impl windows_core::RuntimeType for IRangeBaseFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.Primitives.IRangeBaseFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IRangeBaseFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IRangeBaseStatics, IRangeBaseStatics_Vtbl, 0x4aed5e49_64ec_56f1_874d_b8c0f83f9ac8);
impl windows_core::RuntimeType for IRangeBaseStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.Primitives.IRangeBaseStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IRangeBaseStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(
    IRangeBaseValueChangedEventArgs,
    IRangeBaseValueChangedEventArgs_Vtbl,
    0xb0181692_9578_51c7_9d1c_adfcf8945aa9
);
impl windows_core::RuntimeType for IRangeBaseValueChangedEventArgs {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.Primitives.IRangeBaseValueChangedEventArgs");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IRangeBaseValueChangedEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    OldValue: usize,
    pub NewValue: unsafe extern "system" fn(*mut core::ffi::c_void, *mut f64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IResourceDictionary, IResourceDictionary_Vtbl, 0x1b690975_a710_5783_a6e1_15836f6186c2);
impl windows_core::RuntimeType for IResourceDictionary {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IResourceDictionary");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IResourceDictionary_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Source: usize,
    SetSource: usize,
    pub MergedDictionaries: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IResourceDictionaryFactory, IResourceDictionaryFactory_Vtbl, 0xea22a48f_ab71_56f6_a392_d82310c8aa7b);
impl windows_core::RuntimeType for IResourceDictionaryFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IResourceDictionaryFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IResourceDictionaryFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IRoutedEventArgs, IRoutedEventArgs_Vtbl, 0x0908c407_1c7d_5de3_9c50_d971c62ec8ec);
impl windows_core::RuntimeType for IRoutedEventArgs {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IRoutedEventArgs");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IRoutedEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IRoutedEventArgsFactory, IRoutedEventArgsFactory_Vtbl, 0x914b02c7_076b_5b89_98e7_6c373379e9af);
impl windows_core::RuntimeType for IRoutedEventArgsFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IRoutedEventArgsFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IRoutedEventArgsFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IRowDefinition, IRowDefinition_Vtbl, 0xfe870f2f_89ef_5dac_9f33_968d0dc577c3);
impl windows_core::RuntimeType for IRowDefinition {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IRowDefinition");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IRowDefinition_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Height: usize,
    pub SetHeight: unsafe extern "system" fn(*mut core::ffi::c_void, GridLength) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IRowDefinitionStatics, IRowDefinitionStatics_Vtbl, 0x7317c7c5_fd6f_58d9_9b53_4978dbacde23);
impl windows_core::RuntimeType for IRowDefinitionStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IRowDefinitionStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IRowDefinitionStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IScrollViewer, IScrollViewer_Vtbl, 0x1dc28c2e_996c_5394_89c3_4dc656b4ad46);
impl windows_core::RuntimeType for IScrollViewer {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IScrollViewer");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IScrollViewer_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    HorizontalScrollBarVisibility: usize,
    pub SetHorizontalScrollBarVisibility: unsafe extern "system" fn(*mut core::ffi::c_void, ScrollBarVisibility) -> windows_core::HRESULT,
    VerticalScrollBarVisibility: usize,
    pub SetVerticalScrollBarVisibility: unsafe extern "system" fn(*mut core::ffi::c_void, ScrollBarVisibility) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IScrollViewerStatics, IScrollViewerStatics_Vtbl, 0xd971fd86_0a96_50c4_a6e1_9975faa2a142);
impl windows_core::RuntimeType for IScrollViewerStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IScrollViewerStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IScrollViewerStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(ISelectionChangedEventArgs, ISelectionChangedEventArgs_Vtbl, 0xb6c18076_4b76_5416_ad29_e2dc20c46246);
impl windows_core::RuntimeType for ISelectionChangedEventArgs {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ISelectionChangedEventArgs");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ISelectionChangedEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(
    ISelectionChangedEventArgsFactory,
    ISelectionChangedEventArgsFactory_Vtbl,
    0x0fcd041d_465f_5745_9882_0c8348e4135d
);
impl windows_core::RuntimeType for ISelectionChangedEventArgsFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ISelectionChangedEventArgsFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ISelectionChangedEventArgsFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(ISelector, ISelector_Vtbl, 0x8f7e2159_e61d_576f_8476_f83fde3d689e);
impl windows_core::RuntimeType for ISelector {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.Primitives.ISelector");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ISelector_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub SelectedIndex: unsafe extern "system" fn(*mut core::ffi::c_void, *mut i32) -> windows_core::HRESULT,
    pub SetSelectedIndex: unsafe extern "system" fn(*mut core::ffi::c_void, i32) -> windows_core::HRESULT,
    SelectedItem: usize,
    SetSelectedItem: usize,
    SelectedValue: usize,
    SetSelectedValue: usize,
    SelectedValuePath: usize,
    SetSelectedValuePath: usize,
    IsSynchronizedWithCurrentItem: usize,
    SetIsSynchronizedWithCurrentItem: usize,
    pub SelectionChanged: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemoveSelectionChanged: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(ISelectorStatics, ISelectorStatics_Vtbl, 0x569b2234_1ceb_516e_b64e_0d479452e279);
impl windows_core::RuntimeType for ISelectorStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.Primitives.ISelectorStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ISelectorStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(ISlider, ISlider_Vtbl, 0xf7418ecf_7c35_5216_8bf1_d82d47cce5df);
impl windows_core::RuntimeType for ISlider {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ISlider");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ISlider_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    IntermediateValue: usize,
    SetIntermediateValue: usize,
    StepFrequency: usize,
    pub SetStepFrequency: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(ISliderFactory, ISliderFactory_Vtbl, 0x06604d71_34ca_5f39_9656_29d81d3c110c);
impl windows_core::RuntimeType for ISliderFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ISliderFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ISliderFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(ISliderStatics, ISliderStatics_Vtbl, 0xf0541f7b_6d81_59e2_997a_2e65fbcccd21);
impl windows_core::RuntimeType for ISliderStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ISliderStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ISliderStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(ISolidColorBrush, ISolidColorBrush_Vtbl, 0xb3865c31_37c8_55c1_8a72_d41c67642e2a);
impl windows_core::RuntimeType for ISolidColorBrush {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.ISolidColorBrush");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ISolidColorBrush_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Color: usize,
    pub SetColor: unsafe extern "system" fn(*mut core::ffi::c_void, Color) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(ISolidColorBrushFactory, ISolidColorBrushFactory_Vtbl, 0x7b559384_4daa_54f4_91ef_33a23fd816ca);
impl windows_core::RuntimeType for ISolidColorBrushFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.ISolidColorBrushFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ISolidColorBrushFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(ISolidColorBrushStatics, ISolidColorBrushStatics_Vtbl, 0x6bc16da0_c4e6_59b8_995b_b31e48424c07);
impl windows_core::RuntimeType for ISolidColorBrushStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.ISolidColorBrushStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ISolidColorBrushStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IStackPanel, IStackPanel_Vtbl, 0x493ab00b_3a6a_5e4a_9452_407cd5197406);
impl windows_core::RuntimeType for IStackPanel {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IStackPanel");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IStackPanel_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    AreScrollSnapPointsRegular: usize,
    SetAreScrollSnapPointsRegular: usize,
    Orientation: usize,
    pub SetOrientation: unsafe extern "system" fn(*mut core::ffi::c_void, Orientation) -> windows_core::HRESULT,
    BackgroundSizing: usize,
    SetBackgroundSizing: usize,
    BorderBrush: usize,
    SetBorderBrush: usize,
    BorderThickness: usize,
    SetBorderThickness: usize,
    CornerRadius: usize,
    SetCornerRadius: usize,
    Padding: usize,
    SetPadding: usize,
    Spacing: usize,
    pub SetSpacing: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IStackPanelFactory, IStackPanelFactory_Vtbl, 0x64c1d388_47a2_5a74_a75b_559d151ee5ac);
impl windows_core::RuntimeType for IStackPanelFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IStackPanelFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IStackPanelFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IStackPanelStatics, IStackPanelStatics_Vtbl, 0x10bb04e3_eb01_5ea8_9f96_69508479def9);
impl windows_core::RuntimeType for IStackPanelStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IStackPanelStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IStackPanelStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IStyle, IStyle_Vtbl, 0x65e1d164_572f_5b0e_a80f_9c02441fac49);
impl windows_core::RuntimeType for IStyle {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IStyle");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IStyle_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IStyleFactory, IStyleFactory_Vtbl, 0xc2d924a2_3862_517c_b083_9a9120d7302d);
impl windows_core::RuntimeType for IStyleFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IStyleFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IStyleFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(ISymbolIcon, ISymbolIcon_Vtbl, 0xa4322906_0dbe_5eb7_8b64_3e832246eb7f);
impl windows_core::RuntimeType for ISymbolIcon {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ISymbolIcon");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ISymbolIcon_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Symbol: usize,
    pub SetSymbol: unsafe extern "system" fn(*mut core::ffi::c_void, Symbol) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(ISymbolIconFactory, ISymbolIconFactory_Vtbl, 0xd4430447_567c_5aad_996a_a547774e2c3c);
impl windows_core::RuntimeType for ISymbolIconFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ISymbolIconFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ISymbolIconFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(ISymbolIconStatics, ISymbolIconStatics_Vtbl, 0xccaa0c34_31f2_50f1_b328_be6edc291aaa);
impl windows_core::RuntimeType for ISymbolIconStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ISymbolIconStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ISymbolIconStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(ISystemBackdrop, ISystemBackdrop_Vtbl, 0x5aeed5c4_37ac_5852_b73f_1b76ebc3205f);
impl windows_core::RuntimeType for ISystemBackdrop {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.ISystemBackdrop");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ISystemBackdrop_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(ISystemBackdropFactory, ISystemBackdropFactory_Vtbl, 0x1e07656b_fad2_5b29_913f_b6748bc45942);
impl windows_core::RuntimeType for ISystemBackdropFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.ISystemBackdropFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ISystemBackdropFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(ITextBlock, ITextBlock_Vtbl, 0x1ac8d84f_392c_5c7e_83f5_a53e3bf0abb0);
impl windows_core::RuntimeType for ITextBlock {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ITextBlock");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ITextBlock_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    FontSize: usize,
    pub SetFontSize: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    FontFamily: usize,
    SetFontFamily: usize,
    FontWeight: usize,
    pub SetFontWeight: unsafe extern "system" fn(*mut core::ffi::c_void, FontWeight) -> windows_core::HRESULT,
    FontStyle: usize,
    SetFontStyle: usize,
    FontStretch: usize,
    SetFontStretch: usize,
    CharacterSpacing: usize,
    SetCharacterSpacing: usize,
    Foreground: usize,
    SetForeground: usize,
    TextWrapping: usize,
    pub SetTextWrapping: unsafe extern "system" fn(*mut core::ffi::c_void, TextWrapping) -> windows_core::HRESULT,
    TextTrimming: usize,
    pub SetTextTrimming: unsafe extern "system" fn(*mut core::ffi::c_void, TextTrimming) -> windows_core::HRESULT,
    TextAlignment: usize,
    SetTextAlignment: usize,
    Text: usize,
    pub SetText: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    Inlines: usize,
    Padding: usize,
    SetPadding: usize,
    LineHeight: usize,
    SetLineHeight: usize,
    LineStackingStrategy: usize,
    SetLineStackingStrategy: usize,
    IsTextSelectionEnabled: usize,
    pub SetIsTextSelectionEnabled: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    SelectedText: usize,
    ContentStart: usize,
    ContentEnd: usize,
    SelectionStart: usize,
    SelectionEnd: usize,
    BaselineOffset: usize,
    SelectionHighlightColor: usize,
    SetSelectionHighlightColor: usize,
    MaxLines: usize,
    pub SetMaxLines: unsafe extern "system" fn(*mut core::ffi::c_void, i32) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(ITextBlockStatics, ITextBlockStatics_Vtbl, 0x3187104b_65c2_5e53_b889_c8272b1314cd);
impl windows_core::RuntimeType for ITextBlockStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ITextBlockStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ITextBlockStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(ITextBox, ITextBox_Vtbl, 0x873af7c2_ab89_5d76_8dbe_3d6325669df5);
impl windows_core::RuntimeType for ITextBox {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ITextBox");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ITextBox_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Text: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    pub SetText: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    SelectedText: usize,
    SetSelectedText: usize,
    SelectionLength: usize,
    SetSelectionLength: usize,
    SelectionStart: usize,
    SetSelectionStart: usize,
    MaxLength: usize,
    SetMaxLength: usize,
    IsReadOnly: usize,
    SetIsReadOnly: usize,
    AcceptsReturn: usize,
    pub SetAcceptsReturn: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    TextAlignment: usize,
    SetTextAlignment: usize,
    TextWrapping: usize,
    pub SetTextWrapping: unsafe extern "system" fn(*mut core::ffi::c_void, TextWrapping) -> windows_core::HRESULT,
    IsSpellCheckEnabled: usize,
    SetIsSpellCheckEnabled: usize,
    IsTextPredictionEnabled: usize,
    SetIsTextPredictionEnabled: usize,
    InputScope: usize,
    SetInputScope: usize,
    Header: usize,
    SetHeader: usize,
    HeaderTemplate: usize,
    SetHeaderTemplate: usize,
    PlaceholderText: usize,
    pub SetPlaceholderText: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    SelectionHighlightColor: usize,
    SetSelectionHighlightColor: usize,
    PreventKeyboardDisplayOnProgrammaticFocus: usize,
    SetPreventKeyboardDisplayOnProgrammaticFocus: usize,
    IsColorFontEnabled: usize,
    SetIsColorFontEnabled: usize,
    SelectionHighlightColorWhenNotFocused: usize,
    SetSelectionHighlightColorWhenNotFocused: usize,
    HorizontalTextAlignment: usize,
    SetHorizontalTextAlignment: usize,
    CharacterCasing: usize,
    SetCharacterCasing: usize,
    PlaceholderForeground: usize,
    SetPlaceholderForeground: usize,
    CanPasteClipboardContent: usize,
    CanUndo: usize,
    CanRedo: usize,
    SelectionFlyout: usize,
    SetSelectionFlyout: usize,
    ProofingMenuFlyout: usize,
    Description: usize,
    SetDescription: usize,
    pub TextChanged: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemoveTextChanged: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(ITextBoxFactory, ITextBoxFactory_Vtbl, 0xe1d8b82e_bc60_5d27_b646_5ca4c4a69432);
impl windows_core::RuntimeType for ITextBoxFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ITextBoxFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ITextBoxFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(ITextBoxStatics, ITextBoxStatics_Vtbl, 0xa41cf38f_712a_5599_bbed_5a3d9b6bd46e);
impl windows_core::RuntimeType for ITextBoxStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ITextBoxStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ITextBoxStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(ITextChangedEventArgs, ITextChangedEventArgs_Vtbl, 0x71c37e43_7be7_52fc_bf8c_9867f44be5f4);
impl windows_core::RuntimeType for ITextChangedEventArgs {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ITextChangedEventArgs");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ITextChangedEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(ITitleBar, ITitleBar_Vtbl, 0xc552714d_5d30_5a2b_9c7a_d68bea3dde8d);
impl windows_core::RuntimeType for ITitleBar {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ITitleBar");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ITitleBar_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Title: usize,
    pub SetTitle: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    Subtitle: usize,
    SetSubtitle: usize,
    IconSource: usize,
    SetIconSource: usize,
    LeftHeader: usize,
    SetLeftHeader: usize,
    Content: usize,
    pub SetContent: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    RightHeader: usize,
    pub SetRightHeader: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    IsBackButtonVisible: usize,
    pub SetIsBackButtonVisible: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    IsBackButtonEnabled: usize,
    SetIsBackButtonEnabled: usize,
    IsPaneToggleButtonVisible: usize,
    pub SetIsPaneToggleButtonVisible: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    TemplateSettings: usize,
    BackRequested: usize,
    RemoveBackRequested: usize,
    pub PaneToggleRequested: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemovePaneToggleRequested: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(ITitleBarFactory, ITitleBarFactory_Vtbl, 0xc4452799_4606_59ef_9392_a0548d48b82e);
impl windows_core::RuntimeType for ITitleBarFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ITitleBarFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ITitleBarFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(ITitleBarStatics, ITitleBarStatics_Vtbl, 0xf9ada39b_1f24_5955_9b5c_cacbfe9a41cf);
impl windows_core::RuntimeType for ITitleBarStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ITitleBarStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ITitleBarStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(ITitleBarStatics2, ITitleBarStatics2_Vtbl, 0xcd36d484_36f5_510c_9ecd_7e289c97f263);
impl windows_core::RuntimeType for ITitleBarStatics2 {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ITitleBarStatics2");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct ITitleBarStatics2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IToggleButton, IToggleButton_Vtbl, 0x686fbaa4_c866_568b_8f75_481d8d545291);
impl windows_core::RuntimeType for IToggleButton {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.Primitives.IToggleButton");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IToggleButton_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub IsChecked: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    pub SetIsChecked: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    IsThreeState: usize,
    SetIsThreeState: usize,
    pub Checked: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemoveChecked: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IToggleButtonFactory, IToggleButtonFactory_Vtbl, 0x519511bb_d35b_5e2d_966c_8369405a4408);
impl windows_core::RuntimeType for IToggleButtonFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.Primitives.IToggleButtonFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IToggleButtonFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IToggleButtonStatics, IToggleButtonStatics_Vtbl, 0x4b8397e3_76fd_59df_824f_40ae339fb00b);
impl windows_core::RuntimeType for IToggleButtonStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.Primitives.IToggleButtonStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IToggleButtonStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IToggleSwitch, IToggleSwitch_Vtbl, 0x1b17eeb1_74bf_5a83_8161_a86f0fdcdf24);
impl windows_core::RuntimeType for IToggleSwitch {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IToggleSwitch");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IToggleSwitch_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub IsOn: unsafe extern "system" fn(*mut core::ffi::c_void, *mut bool) -> windows_core::HRESULT,
    pub SetIsOn: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    Header: usize,
    SetHeader: usize,
    HeaderTemplate: usize,
    SetHeaderTemplate: usize,
    OnContent: usize,
    SetOnContent: usize,
    OnContentTemplate: usize,
    SetOnContentTemplate: usize,
    OffContent: usize,
    SetOffContent: usize,
    OffContentTemplate: usize,
    SetOffContentTemplate: usize,
    TemplateSettings: usize,
    pub Toggled: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemoveToggled: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IToggleSwitchStatics, IToggleSwitchStatics_Vtbl, 0xc9c203d6_0619_504d_9ed8_5054fe3ca51e);
impl windows_core::RuntimeType for IToggleSwitchStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IToggleSwitchStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IToggleSwitchStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IToolTipService, IToolTipService_Vtbl, 0x01140768_2727_5f89_80e0_5210326a3431);
impl windows_core::RuntimeType for IToolTipService {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IToolTipService");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IToolTipService_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IToolTipServiceStatics, IToolTipServiceStatics_Vtbl, 0x5aa38adc_9874_5e0a_8d8e_1574efc0b88f);
impl windows_core::RuntimeType for IToolTipServiceStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IToolTipServiceStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IToolTipServiceStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    PlacementProperty: usize,
    GetPlacement: usize,
    pub SetPlacement: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, PlacementMode) -> windows_core::HRESULT,
    PlacementTargetProperty: usize,
    GetPlacementTarget: usize,
    SetPlacementTarget: usize,
    ToolTipProperty: usize,
    GetToolTip: usize,
    pub SetToolTip:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IUIElement, IUIElement_Vtbl, 0xc3c01020_320c_5cf6_9d24_d396bbfa4d8b);
impl windows_core::RuntimeType for IUIElement {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IUIElement");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IUIElement_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    DesiredSize: usize,
    AllowDrop: usize,
    SetAllowDrop: usize,
    Opacity: usize,
    pub SetOpacity: unsafe extern "system" fn(*mut core::ffi::c_void, f64) -> windows_core::HRESULT,
    Clip: usize,
    SetClip: usize,
    RenderTransform: usize,
    SetRenderTransform: usize,
    Projection: usize,
    SetProjection: usize,
    Transform3D: usize,
    SetTransform3D: usize,
    RenderTransformOrigin: usize,
    SetRenderTransformOrigin: usize,
    IsHitTestVisible: usize,
    SetIsHitTestVisible: usize,
    pub Visibility: unsafe extern "system" fn(*mut core::ffi::c_void, *mut Visibility) -> windows_core::HRESULT,
    pub SetVisibility: unsafe extern "system" fn(*mut core::ffi::c_void, Visibility) -> windows_core::HRESULT,
    RenderSize: usize,
    UseLayoutRounding: usize,
    SetUseLayoutRounding: usize,
    Transitions: usize,
    SetTransitions: usize,
    CacheMode: usize,
    SetCacheMode: usize,
    IsTapEnabled: usize,
    SetIsTapEnabled: usize,
    IsDoubleTapEnabled: usize,
    SetIsDoubleTapEnabled: usize,
    CanDrag: usize,
    SetCanDrag: usize,
    IsRightTapEnabled: usize,
    SetIsRightTapEnabled: usize,
    IsHoldingEnabled: usize,
    SetIsHoldingEnabled: usize,
    ManipulationMode: usize,
    SetManipulationMode: usize,
    PointerCaptures: usize,
    ContextFlyout: usize,
    SetContextFlyout: usize,
    CompositeMode: usize,
    SetCompositeMode: usize,
    Lights: usize,
    CanBeScrollAnchor: usize,
    SetCanBeScrollAnchor: usize,
    ExitDisplayModeOnAccessKeyInvoked: usize,
    SetExitDisplayModeOnAccessKeyInvoked: usize,
    IsAccessKeyScope: usize,
    SetIsAccessKeyScope: usize,
    AccessKeyScopeOwner: usize,
    SetAccessKeyScopeOwner: usize,
    AccessKey: usize,
    SetAccessKey: usize,
    KeyTipPlacementMode: usize,
    SetKeyTipPlacementMode: usize,
    KeyTipHorizontalOffset: usize,
    SetKeyTipHorizontalOffset: usize,
    KeyTipVerticalOffset: usize,
    SetKeyTipVerticalOffset: usize,
    KeyTipTarget: usize,
    SetKeyTipTarget: usize,
    XYFocusKeyboardNavigation: usize,
    SetXYFocusKeyboardNavigation: usize,
    XYFocusUpNavigationStrategy: usize,
    SetXYFocusUpNavigationStrategy: usize,
    XYFocusDownNavigationStrategy: usize,
    SetXYFocusDownNavigationStrategy: usize,
    XYFocusLeftNavigationStrategy: usize,
    SetXYFocusLeftNavigationStrategy: usize,
    XYFocusRightNavigationStrategy: usize,
    SetXYFocusRightNavigationStrategy: usize,
    KeyboardAccelerators: usize,
    KeyboardAcceleratorPlacementTarget: usize,
    SetKeyboardAcceleratorPlacementTarget: usize,
    KeyboardAcceleratorPlacementMode: usize,
    SetKeyboardAcceleratorPlacementMode: usize,
    HighContrastAdjustment: usize,
    SetHighContrastAdjustment: usize,
    TabFocusNavigation: usize,
    SetTabFocusNavigation: usize,
    OpacityTransition: usize,
    SetOpacityTransition: usize,
    Translation: usize,
    SetTranslation: usize,
    TranslationTransition: usize,
    SetTranslationTransition: usize,
    Rotation: usize,
    SetRotation: usize,
    RotationTransition: usize,
    SetRotationTransition: usize,
    Scale: usize,
    SetScale: usize,
    ScaleTransition: usize,
    SetScaleTransition: usize,
    TransformMatrix: usize,
    SetTransformMatrix: usize,
    CenterPoint: usize,
    SetCenterPoint: usize,
    RotationAxis: usize,
    SetRotationAxis: usize,
    ActualOffset: usize,
    ActualSize: usize,
    pub XamlRoot: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    pub SetXamlRoot: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    Shadow: usize,
    SetShadow: usize,
    RasterizationScale: usize,
    SetRasterizationScale: usize,
    FocusState: usize,
    UseSystemFocusVisuals: usize,
    SetUseSystemFocusVisuals: usize,
    XYFocusLeft: usize,
    SetXYFocusLeft: usize,
    XYFocusRight: usize,
    SetXYFocusRight: usize,
    XYFocusUp: usize,
    SetXYFocusUp: usize,
    XYFocusDown: usize,
    SetXYFocusDown: usize,
    IsTabStop: usize,
    SetIsTabStop: usize,
    TabIndex: usize,
    SetTabIndex: usize,
    KeyUp: usize,
    RemoveKeyUp: usize,
    KeyDown: usize,
    RemoveKeyDown: usize,
    GotFocus: usize,
    RemoveGotFocus: usize,
    LostFocus: usize,
    RemoveLostFocus: usize,
    DragStarting: usize,
    RemoveDragStarting: usize,
    DropCompleted: usize,
    RemoveDropCompleted: usize,
    CharacterReceived: usize,
    RemoveCharacterReceived: usize,
    DragEnter: usize,
    RemoveDragEnter: usize,
    DragLeave: usize,
    RemoveDragLeave: usize,
    DragOver: usize,
    RemoveDragOver: usize,
    Drop: usize,
    RemoveDrop: usize,
    PointerPressed: usize,
    RemovePointerPressed: usize,
    PointerMoved: usize,
    RemovePointerMoved: usize,
    PointerReleased: usize,
    RemovePointerReleased: usize,
    PointerEntered: usize,
    RemovePointerEntered: usize,
    PointerExited: usize,
    RemovePointerExited: usize,
    PointerCaptureLost: usize,
    RemovePointerCaptureLost: usize,
    PointerCanceled: usize,
    RemovePointerCanceled: usize,
    PointerWheelChanged: usize,
    RemovePointerWheelChanged: usize,
    Tapped: usize,
    RemoveTapped: usize,
    DoubleTapped: usize,
    RemoveDoubleTapped: usize,
    Holding: usize,
    RemoveHolding: usize,
    ContextRequested: usize,
    RemoveContextRequested: usize,
    ContextCanceled: usize,
    RemoveContextCanceled: usize,
    RightTapped: usize,
    RemoveRightTapped: usize,
    ManipulationStarting: usize,
    RemoveManipulationStarting: usize,
    ManipulationInertiaStarting: usize,
    RemoveManipulationInertiaStarting: usize,
    ManipulationStarted: usize,
    RemoveManipulationStarted: usize,
    ManipulationDelta: usize,
    RemoveManipulationDelta: usize,
    ManipulationCompleted: usize,
    RemoveManipulationCompleted: usize,
    AccessKeyDisplayRequested: usize,
    RemoveAccessKeyDisplayRequested: usize,
    AccessKeyDisplayDismissed: usize,
    RemoveAccessKeyDisplayDismissed: usize,
    AccessKeyInvoked: usize,
    RemoveAccessKeyInvoked: usize,
    ProcessKeyboardAccelerators: usize,
    RemoveProcessKeyboardAccelerators: usize,
    GettingFocus: usize,
    RemoveGettingFocus: usize,
    LosingFocus: usize,
    RemoveLosingFocus: usize,
    NoFocusCandidateFound: usize,
    RemoveNoFocusCandidateFound: usize,
    PreviewKeyDown: usize,
    RemovePreviewKeyDown: usize,
    PreviewKeyUp: usize,
    RemovePreviewKeyUp: usize,
    BringIntoViewRequested: usize,
    RemoveBringIntoViewRequested: usize,
    Measure: usize,
    Arrange: usize,
    CapturePointer: usize,
    ReleasePointerCapture: usize,
    ReleasePointerCaptures: usize,
    AddHandler: usize,
    RemoveHandler: usize,
    TransformToVisual: usize,
    InvalidateMeasure: usize,
    InvalidateArrange: usize,
    pub UpdateLayout: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IUIElementStatics, IUIElementStatics_Vtbl, 0xd2921d87_3584_5e22_8a3a_c2c78dab4f6e);
impl windows_core::RuntimeType for IUIElementStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IUIElementStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IUIElementStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IVector<T>(windows_core::IUnknown, core::marker::PhantomData<T>)
where
    T: windows_core::RuntimeType + 'static;
impl<T: windows_core::RuntimeType + 'static> windows_core::imp::CanInto<windows_core::IUnknown> for IVector<T> {}
impl<T: windows_core::RuntimeType + 'static> windows_core::imp::CanInto<windows_core::IInspectable> for IVector<T> {}
unsafe impl<T: windows_core::RuntimeType + 'static> windows_core::Interface for IVector<T> {
    type Vtable = IVector_Vtbl<T>;

    const IID: windows_core::GUID = windows_core::GUID::from_signature(<Self as windows_core::RuntimeType>::SIGNATURE);
}
impl<T: windows_core::RuntimeType + 'static> windows_core::RuntimeType for IVector<T> {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::new()
        .push_slice(b"Windows.Foundation.Collections.IVector`1<")
        .push_other(T::NAME)
        .push_slice(b">");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::new()
        .push_slice(b"pinterface({913337e9-11a1-4345-a3a2-4e7f956e222d}")
        .push_slice(b";")
        .push_other(T::SIGNATURE)
        .push_slice(b")");
}
impl<T: windows_core::RuntimeType + 'static> IVector<T> {
    pub fn GetAt(&self, index: u32) -> windows_core::Result<T> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).GetAt)(windows_core::Interface::as_raw(self), index, &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn Size(&self) -> windows_core::Result<u32> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Size)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }

    pub fn IndexOf<P0>(&self, value: P0, index: &mut u32) -> windows_core::Result<bool>
    where
        P0: windows_core::Param<T>,
    {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).IndexOf)(
                windows_core::Interface::as_raw(self),
                value.param().abi(),
                index,
                &mut result__,
            )
            .map(|| result__)
        }
    }

    pub fn InsertAt<P1>(&self, index: u32, value: P1) -> windows_core::Result<()>
    where
        P1: windows_core::Param<T>,
    {
        unsafe { (windows_core::Interface::vtable(self).InsertAt)(windows_core::Interface::as_raw(self), index, value.param().abi()).ok() }
    }

    pub fn RemoveAt(&self, index: u32) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).RemoveAt)(windows_core::Interface::as_raw(self), index).ok() }
    }

    pub fn Append<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<T>,
    {
        unsafe { (windows_core::Interface::vtable(self).Append)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn Clear(&self) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).Clear)(windows_core::Interface::as_raw(self)).ok() }
    }

    pub fn First(&self) -> windows_core::Result<windows_collections::IIterator<T>> {
        let this = &windows_core::Interface::cast::<IIterable<T>>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).First)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }
}
impl<T: windows_core::RuntimeType + 'static> IntoIterator for IVector<T> {
    type IntoIter = windows_collections::BufferedIterator<Self::Item>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(&self)
    }
}
impl<T: windows_core::RuntimeType + 'static> IntoIterator for &IVector<T> {
    type IntoIter = windows_collections::BufferedIterator<Self::Item>;
    type Item = T;

    fn into_iter(self) -> Self::IntoIter {
        windows_collections::BufferedIterator::new(self.First().unwrap())
    }
}
impl<T: windows_core::RuntimeType + 'static> windows_core::RuntimeName for IVector<T> {
    const NAME: &'static str = "Windows.Foundation.Collections.IVector";
    const RUNTIME_CLASS_NAME: windows_core::imp::ConstBuffer = <Self as windows_core::RuntimeType>::NAME;
}
#[repr(C)]
pub struct IVector_Vtbl<T>
where
    T: windows_core::RuntimeType + 'static,
{
    pub base__: windows_core::IInspectable_Vtbl,
    pub GetAt: unsafe extern "system" fn(*mut core::ffi::c_void, u32, *mut windows_core::imp::AbiType<T>) -> windows_core::HRESULT,
    pub Size: unsafe extern "system" fn(*mut core::ffi::c_void, *mut u32) -> windows_core::HRESULT,
    GetView: usize,
    pub IndexOf:
        unsafe extern "system" fn(*mut core::ffi::c_void, windows_core::imp::AbiType<T>, *mut u32, *mut bool) -> windows_core::HRESULT,
    SetAt: usize,
    pub InsertAt: unsafe extern "system" fn(*mut core::ffi::c_void, u32, windows_core::imp::AbiType<T>) -> windows_core::HRESULT,
    pub RemoveAt: unsafe extern "system" fn(*mut core::ffi::c_void, u32) -> windows_core::HRESULT,
    pub Append: unsafe extern "system" fn(*mut core::ffi::c_void, windows_core::imp::AbiType<T>) -> windows_core::HRESULT,
    RemoveAtEnd: usize,
    pub Clear: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
    T: core::marker::PhantomData<T>,
}
windows_core::imp::define_interface!(IViewbox, IViewbox_Vtbl, 0x510683e8_d0fe_5ef4_85bd_e1131076ac22);
impl windows_core::RuntimeType for IViewbox {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IViewbox");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IViewbox_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Child: usize,
    pub SetChild: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    Stretch: usize,
    pub SetStretch: unsafe extern "system" fn(*mut core::ffi::c_void, Stretch) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IViewboxStatics, IViewboxStatics_Vtbl, 0x0466c103_1e17_5e83_a262_8855f216b59f);
impl windows_core::RuntimeType for IViewboxStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IViewboxStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IViewboxStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IWindow, IWindow_Vtbl, 0x61f0ec79_5d52_56b5_86fb_40fa4af288b0);
impl windows_core::RuntimeType for IWindow {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IWindow");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IWindow_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Bounds: usize,
    Visible: usize,
    pub Content: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    pub SetContent: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    CoreWindow: usize,
    Compositor: usize,
    Dispatcher: usize,
    DispatcherQueue: usize,
    Title: usize,
    pub SetTitle: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    ExtendsContentIntoTitleBar: usize,
    pub SetExtendsContentIntoTitleBar: unsafe extern "system" fn(*mut core::ffi::c_void, bool) -> windows_core::HRESULT,
    Activated: usize,
    RemoveActivated: usize,
    pub Closed: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemoveClosed: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
    SizeChanged: usize,
    RemoveSizeChanged: usize,
    VisibilityChanged: usize,
    RemoveVisibilityChanged: usize,
    pub Activate: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
    pub Close: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
    pub SetTitleBar: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IWindow2, IWindow2_Vtbl, 0x42febaa5_1c32_522a_a591_57618c6f665d);
impl windows_core::RuntimeType for IWindow2 {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IWindow2");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IWindow2_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    SystemBackdrop: usize,
    pub SetSystemBackdrop: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void) -> windows_core::HRESULT,
    pub AppWindow: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IWindowEventArgs, IWindowEventArgs_Vtbl, 0x1140827c_fe0a_5268_bc2b_f4492c2ccb49);
impl windows_core::RuntimeType for IWindowEventArgs {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IWindowEventArgs");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IWindowEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IWindowFactory, IWindowFactory_Vtbl, 0xf0441536_afef_5222_918f_324a9b2dec75);
impl windows_core::RuntimeType for IWindowFactory {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IWindowFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IWindowFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstance: unsafe extern "system" fn(
        *mut core::ffi::c_void,
        *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
        *mut *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IWindowStatics, IWindowStatics_Vtbl, 0x8cc985e3_a41a_5df4_b531_d3a1788d86c5);
impl windows_core::RuntimeType for IWindowStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IWindowStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IWindowStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IWriteableBitmap, IWriteableBitmap_Vtbl, 0x78c824a9_0e43_5f1e_93bc_d046cca82b7e);
impl windows_core::RuntimeType for IWriteableBitmap {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.Imaging.IWriteableBitmap");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IWriteableBitmap_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub PixelBuffer: unsafe extern "system" fn(*mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
    pub Invalidate: unsafe extern "system" fn(*mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IWriteableBitmapFactory, IWriteableBitmapFactory_Vtbl, 0x26e861d9_b080_512b_96c4_80050e7e08d1);
impl windows_core::RuntimeType for IWriteableBitmapFactory {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.Imaging.IWriteableBitmapFactory");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IWriteableBitmapFactory_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub CreateInstanceWithDimensions:
        unsafe extern "system" fn(*mut core::ffi::c_void, i32, i32, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IXamlControlsResources, IXamlControlsResources_Vtbl, 0x918ca043_f42c_5805_861b_62d6d1d0c162);
impl windows_core::RuntimeType for IXamlControlsResources {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IXamlControlsResources");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IXamlControlsResources_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(
    IXamlControlsResourcesStatics,
    IXamlControlsResourcesStatics_Vtbl,
    0xcb49196c_1458_5e92_b701_d08d3e816bc5
);
impl windows_core::RuntimeType for IXamlControlsResourcesStatics {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.IXamlControlsResourcesStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IXamlControlsResourcesStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IXamlReader, IXamlReader_Vtbl, 0x54ce54c8_38c6_50d9_ac98_4b03eddbde9f);
impl windows_core::RuntimeType for IXamlReader {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Markup.IXamlReader");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IXamlReader_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
windows_core::imp::define_interface!(IXamlReaderStatics, IXamlReaderStatics_Vtbl, 0x82a4cd9e_435e_5aeb_8c4f_300cece45cae);
impl windows_core::RuntimeType for IXamlReaderStatics {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Markup.IXamlReaderStatics");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IXamlReaderStatics_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    pub Load:
        unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut *mut core::ffi::c_void) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IXamlRoot, IXamlRoot_Vtbl, 0x60cb215a_ad15_520a_8b01_4416824f0441);
impl windows_core::RuntimeType for IXamlRoot {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IXamlRoot");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IXamlRoot_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
    Content: usize,
    Size: usize,
    pub RasterizationScale: unsafe extern "system" fn(*mut core::ffi::c_void, *mut f64) -> windows_core::HRESULT,
    IsHostVisible: usize,
    pub Changed: unsafe extern "system" fn(*mut core::ffi::c_void, *mut core::ffi::c_void, *mut i64) -> windows_core::HRESULT,
    pub RemoveChanged: unsafe extern "system" fn(*mut core::ffi::c_void, i64) -> windows_core::HRESULT,
}
windows_core::imp::define_interface!(IXamlRootChangedEventArgs, IXamlRootChangedEventArgs_Vtbl, 0x61d2c719_f8a1_515a_902c_cfa498ba7a7f);
impl windows_core::RuntimeType for IXamlRootChangedEventArgs {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.IXamlRootChangedEventArgs");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
#[repr(C)]
pub struct IXamlRootChangedEventArgs_Vtbl {
    pub base__: windows_core::IInspectable_Vtbl,
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IconElement(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(IconElement, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(IconElement, FrameworkElement, UIElement, DependencyObject);
impl IconElement {
    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IIconElementStatics<R, F: FnOnce(&IIconElementStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<IconElement, IIconElementStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for IconElement {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IIconElement>();
}
unsafe impl windows_core::Interface for IconElement {
    type Vtable = <IIconElement as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IIconElement as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for IconElement {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.IconElement";
}
unsafe impl Send for IconElement {}
unsafe impl Sync for IconElement {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Image(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(Image, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(Image, FrameworkElement, UIElement, DependencyObject);
impl Image {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }

    fn IActivationFactory<R, F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Image, windows_core::imp::IGenericFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn Source(&self) -> windows_core::Result<ImageSource> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Source)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetSource<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<ImageSource>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetSource)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn SetStretch(&self, value: Stretch) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetStretch)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IImageStatics<R, F: FnOnce(&IImageStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Image, IImageStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for Image {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IImage>();
}
unsafe impl windows_core::Interface for Image {
    type Vtable = <IImage as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IImage as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for Image {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.Image";
}
unsafe impl Send for Image {}
unsafe impl Sync for Image {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ImageSource(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(ImageSource, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(ImageSource, DependencyObject);
impl ImageSource {
    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }
}
impl windows_core::RuntimeType for ImageSource {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IImageSource>();
}
unsafe impl windows_core::Interface for ImageSource {
    type Vtable = <IImageSource as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IImageSource as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ImageSource {
    const NAME: &'static str = "Microsoft.UI.Xaml.Media.ImageSource";
}
unsafe impl Send for ImageSource {}
unsafe impl Sync for ImageSource {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InfoBar(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(InfoBar, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(InfoBar, Control, FrameworkElement, UIElement, DependencyObject);
impl InfoBar {
    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetIsOpen(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetIsOpen)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetTitle(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetTitle)(windows_core::Interface::as_raw(self), core::mem::transmute_copy(value)).ok()
        }
    }

    pub fn SetMessage(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetMessage)(windows_core::Interface::as_raw(self), core::mem::transmute_copy(value)).ok()
        }
    }

    pub fn SetSeverity(&self, value: InfoBarSeverity) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetSeverity)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetIsClosable(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetIsClosable)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::IInfoBarFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::IInfoBarFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IInfoBarFactory<R, F: FnOnce(&IInfoBarFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<InfoBar, IInfoBarFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IInfoBarStatics<R, F: FnOnce(&IInfoBarStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<InfoBar, IInfoBarStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for InfoBar {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IInfoBar>();
}
unsafe impl windows_core::Interface for InfoBar {
    type Vtable = <IInfoBar as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IInfoBar as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for InfoBar {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.InfoBar";
}
unsafe impl Send for InfoBar {}
unsafe impl Sync for InfoBar {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct InfoBarSeverity(pub i32);
impl InfoBarSeverity {
    pub const Error: Self = Self(3);
    pub const Informational: Self = Self(0);
    pub const Success: Self = Self(1);
    pub const Warning: Self = Self(2);
}
impl windows_core::imp::TypeKind for InfoBarSeverity {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for InfoBarSeverity {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.InfoBarSeverity");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.Controls.InfoBarSeverity;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ItemsControl(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(ItemsControl, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(ItemsControl, Control, FrameworkElement, UIElement, DependencyObject);
impl ItemsControl {
    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetItemsSource<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetItemsSource)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IItemsControlFactory<R, F: FnOnce(&IItemsControlFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ItemsControl, IItemsControlFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IItemsControlStatics<R, F: FnOnce(&IItemsControlStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ItemsControl, IItemsControlStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for ItemsControl {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IItemsControl>();
}
unsafe impl windows_core::Interface for ItemsControl {
    type Vtable = <IItemsControl as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IItemsControl as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ItemsControl {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.ItemsControl";
}
unsafe impl Send for ItemsControl {}
unsafe impl Sync for ItemsControl {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MicaBackdrop(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(MicaBackdrop, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(MicaBackdrop, SystemBackdrop, DependencyObject);
impl MicaBackdrop {
    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn SetKind(&self, value: MicaKind) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetKind)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::IMicaBackdropFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::IMicaBackdropFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    fn IMicaBackdropFactory<R, F: FnOnce(&IMicaBackdropFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<MicaBackdrop, IMicaBackdropFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IMicaBackdropStatics<R, F: FnOnce(&IMicaBackdropStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<MicaBackdrop, IMicaBackdropStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for MicaBackdrop {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IMicaBackdrop>();
}
unsafe impl windows_core::Interface for MicaBackdrop {
    type Vtable = <IMicaBackdrop as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IMicaBackdrop as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for MicaBackdrop {
    const NAME: &'static str = "Microsoft.UI.Xaml.Media.MicaBackdrop";
}
unsafe impl Send for MicaBackdrop {}
unsafe impl Sync for MicaBackdrop {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct MicaKind(pub i32);
impl MicaKind {
    pub const Base: Self = Self(0);
    pub const BaseAlt: Self = Self(1);
}
impl windows_core::imp::TypeKind for MicaKind {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for MicaKind {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Composition.SystemBackdrops.MicaKind");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Composition.SystemBackdrops.MicaKind;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationView(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(NavigationView, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(NavigationView, ContentControl, Control, FrameworkElement, UIElement, DependencyObject);
impl NavigationView {
    pub fn SetContent<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IContentControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetContent)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn IsPaneOpen(&self) -> windows_core::Result<bool> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).IsPaneOpen)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }

    pub fn SetIsPaneOpen(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetIsPaneOpen)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetPaneFooter<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<UIElement>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetPaneFooter)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn SetIsSettingsVisible(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetIsSettingsVisible)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetIsPaneToggleButtonVisible(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetIsPaneToggleButtonVisible)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetOpenPaneLength(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetOpenPaneLength)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetSelectedItem<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetSelectedItem)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn MenuItems(&self) -> windows_core::Result<IVector<windows_core::IInspectable>> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).MenuItems)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SelectionChanged<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<Self>, windows_core::Ref<NavigationViewSelectionChangedEventArgs>) + Send + 'static,
    {
        let handler = <TypedEventHandler<Self, NavigationViewSelectionChangedEventArgs>>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).SelectionChanged)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemoveSelectionChanged))
        }
    }

    pub fn SetIsBackButtonVisible(&self, value: NavigationViewBackButtonVisible) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<INavigationView2>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsBackButtonVisible)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn PaneClosed<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<Self>, windows_core::Ref<windows_core::IInspectable>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<INavigationView2>(self)?;
        let handler = <TypedEventHandler<Self, windows_core::IInspectable>>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).PaneClosed)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemovePaneClosed))
        }
    }

    pub fn PaneOpened<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<Self>, windows_core::Ref<windows_core::IInspectable>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<INavigationView2>(self)?;
        let handler = <TypedEventHandler<Self, windows_core::IInspectable>>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).PaneOpened)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemovePaneOpened))
        }
    }

    pub fn SetPaneDisplayMode(&self, value: NavigationViewPaneDisplayMode) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<INavigationView2>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetPaneDisplayMode)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::INavigationViewFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::INavigationViewFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn INavigationViewFactory<R, F: FnOnce(&INavigationViewFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<NavigationView, INavigationViewFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn INavigationViewStatics<R, F: FnOnce(&INavigationViewStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<NavigationView, INavigationViewStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn INavigationViewStatics2<R, F: FnOnce(&INavigationViewStatics2) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<NavigationView, INavigationViewStatics2> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for NavigationView {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, INavigationView>();
}
unsafe impl windows_core::Interface for NavigationView {
    type Vtable = <INavigationView as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <INavigationView as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for NavigationView {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.NavigationView";
}
unsafe impl Send for NavigationView {}
unsafe impl Sync for NavigationView {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NavigationViewBackButtonVisible(pub i32);
impl NavigationViewBackButtonVisible {
    pub const Auto: Self = Self(2);
    pub const Collapsed: Self = Self(0);
    pub const Visible: Self = Self(1);
}
impl windows_core::imp::TypeKind for NavigationViewBackButtonVisible {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for NavigationViewBackButtonVisible {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.NavigationViewBackButtonVisible");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.Controls.NavigationViewBackButtonVisible;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationViewItem(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(NavigationViewItem, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(
    NavigationViewItem,
    NavigationViewItemBase,
    ContentControl,
    Control,
    FrameworkElement,
    UIElement,
    DependencyObject
);
impl NavigationViewItem {
    pub fn SetContent<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IContentControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetContent)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetIcon<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<IconElement>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetIcon)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::INavigationViewItemFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::INavigationViewItemFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn INavigationViewItemFactory<R, F: FnOnce(&INavigationViewItemFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<NavigationViewItem, INavigationViewItemFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn INavigationViewItemStatics<R, F: FnOnce(&INavigationViewItemStatics) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<NavigationViewItem, INavigationViewItemStatics> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn INavigationViewItemStatics2<R, F: FnOnce(&INavigationViewItemStatics2) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<NavigationViewItem, INavigationViewItemStatics2> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn INavigationViewItemStatics3<R, F: FnOnce(&INavigationViewItemStatics3) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<NavigationViewItem, INavigationViewItemStatics3> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for NavigationViewItem {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, INavigationViewItem>();
}
unsafe impl windows_core::Interface for NavigationViewItem {
    type Vtable = <INavigationViewItem as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <INavigationViewItem as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for NavigationViewItem {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.NavigationViewItem";
}
unsafe impl Send for NavigationViewItem {}
unsafe impl Sync for NavigationViewItem {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationViewItemBase(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(NavigationViewItemBase, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(NavigationViewItemBase, ContentControl, Control, FrameworkElement, UIElement, DependencyObject);
impl NavigationViewItemBase {
    pub fn SetContent<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IContentControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetContent)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn INavigationViewItemBaseStatics<R, F: FnOnce(&INavigationViewItemBaseStatics) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<NavigationViewItemBase, INavigationViewItemBaseStatics> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for NavigationViewItemBase {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, INavigationViewItemBase>();
}
unsafe impl windows_core::Interface for NavigationViewItemBase {
    type Vtable = <INavigationViewItemBase as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <INavigationViewItemBase as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for NavigationViewItemBase {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.NavigationViewItemBase";
}
unsafe impl Send for NavigationViewItemBase {}
unsafe impl Sync for NavigationViewItemBase {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationViewItemHeader(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(NavigationViewItemHeader, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(
    NavigationViewItemHeader,
    NavigationViewItemBase,
    ContentControl,
    Control,
    FrameworkElement,
    UIElement,
    DependencyObject
);
impl NavigationViewItemHeader {
    pub fn SetContent<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IContentControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetContent)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::INavigationViewItemHeaderFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::INavigationViewItemHeaderFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn INavigationViewItemHeaderFactory<R, F: FnOnce(&INavigationViewItemHeaderFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<NavigationViewItemHeader, INavigationViewItemHeaderFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for NavigationViewItemHeader {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, INavigationViewItemHeader>();
}
unsafe impl windows_core::Interface for NavigationViewItemHeader {
    type Vtable = <INavigationViewItemHeader as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <INavigationViewItemHeader as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for NavigationViewItemHeader {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.NavigationViewItemHeader";
}
unsafe impl Send for NavigationViewItemHeader {}
unsafe impl Sync for NavigationViewItemHeader {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct NavigationViewPaneDisplayMode(pub i32);
impl NavigationViewPaneDisplayMode {
    pub const Auto: Self = Self(0);
    pub const Left: Self = Self(1);
    pub const LeftCompact: Self = Self(3);
    pub const LeftMinimal: Self = Self(4);
    pub const Top: Self = Self(2);
}
impl windows_core::imp::TypeKind for NavigationViewPaneDisplayMode {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for NavigationViewPaneDisplayMode {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.NavigationViewPaneDisplayMode");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.Controls.NavigationViewPaneDisplayMode;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NavigationViewSelectionChangedEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(NavigationViewSelectionChangedEventArgs, windows_core::IUnknown, windows_core::IInspectable);
impl NavigationViewSelectionChangedEventArgs {
    pub fn SelectedItem(&self) -> windows_core::Result<windows_core::IInspectable> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).SelectedItem)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }
}
impl windows_core::RuntimeType for NavigationViewSelectionChangedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::for_class::<Self, INavigationViewSelectionChangedEventArgs>();
}
unsafe impl windows_core::Interface for NavigationViewSelectionChangedEventArgs {
    type Vtable = <INavigationViewSelectionChangedEventArgs as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <INavigationViewSelectionChangedEventArgs as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for NavigationViewSelectionChangedEventArgs {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.NavigationViewSelectionChangedEventArgs";
}
unsafe impl Send for NavigationViewSelectionChangedEventArgs {}
unsafe impl Sync for NavigationViewSelectionChangedEventArgs {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumberBox(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(NumberBox, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(NumberBox, Control, FrameworkElement, UIElement, DependencyObject);
impl NumberBox {
    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetMinimum(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetMinimum)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetMaximum(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetMaximum)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn Value(&self) -> windows_core::Result<f64> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Value)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }

    pub fn SetValue(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetValue)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetSmallChange(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetSmallChange)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn ValueChanged<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<Self>, windows_core::Ref<NumberBoxValueChangedEventArgs>) + Send + 'static,
    {
        let handler = <TypedEventHandler<Self, NumberBoxValueChangedEventArgs>>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).ValueChanged)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemoveValueChanged))
        }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::INumberBoxFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::INumberBoxFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn INumberBoxFactory<R, F: FnOnce(&INumberBoxFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<NumberBox, INumberBoxFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn INumberBoxStatics<R, F: FnOnce(&INumberBoxStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<NumberBox, INumberBoxStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for NumberBox {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, INumberBox>();
}
unsafe impl windows_core::Interface for NumberBox {
    type Vtable = <INumberBox as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <INumberBox as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for NumberBox {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.NumberBox";
}
unsafe impl Send for NumberBox {}
unsafe impl Sync for NumberBox {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NumberBoxValueChangedEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(NumberBoxValueChangedEventArgs, windows_core::IUnknown, windows_core::IInspectable);
impl NumberBoxValueChangedEventArgs {
    pub fn NewValue(&self) -> windows_core::Result<f64> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).NewValue)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }
}
impl windows_core::RuntimeType for NumberBoxValueChangedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, INumberBoxValueChangedEventArgs>();
}
unsafe impl windows_core::Interface for NumberBoxValueChangedEventArgs {
    type Vtable = <INumberBoxValueChangedEventArgs as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <INumberBoxValueChangedEventArgs as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for NumberBoxValueChangedEventArgs {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.NumberBoxValueChangedEventArgs";
}
unsafe impl Send for NumberBoxValueChangedEventArgs {}
unsafe impl Sync for NumberBoxValueChangedEventArgs {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Orientation(pub i32);
impl Orientation {
    pub const Horizontal: Self = Self(1);
    pub const Vertical: Self = Self(0);
}
impl windows_core::imp::TypeKind for Orientation {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for Orientation {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.Orientation");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.Controls.Orientation;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Panel(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(Panel, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(Panel, FrameworkElement, UIElement, DependencyObject);
impl Panel {
    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn Children(&self) -> windows_core::Result<UIElementCollection> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Children)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetBackground<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Brush>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetBackground)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IPanelFactory<R, F: FnOnce(&IPanelFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Panel, IPanelFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IPanelStatics<R, F: FnOnce(&IPanelStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Panel, IPanelStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for Panel {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IPanel>();
}
unsafe impl windows_core::Interface for Panel {
    type Vtable = <IPanel as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IPanel as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for Panel {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.Panel";
}
unsafe impl Send for Panel {}
unsafe impl Sync for Panel {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PasswordBox(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(PasswordBox, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(PasswordBox, Control, FrameworkElement, UIElement, DependencyObject);
impl PasswordBox {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }

    fn IActivationFactory<R, F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<PasswordBox, windows_core::imp::IGenericFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn Password(&self) -> windows_core::Result<windows_core::HSTRING> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Password)(windows_core::Interface::as_raw(self), &mut result__)
                .map(|| core::mem::transmute(result__))
        }
    }

    pub fn SetPassword(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetPassword)(windows_core::Interface::as_raw(self), core::mem::transmute_copy(value))
                .ok()
        }
    }

    pub fn SetPasswordRevealMode(&self, value: PasswordRevealMode) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetPasswordRevealMode)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn PasswordChanged<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).PasswordChanged)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemovePasswordChanged))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IPasswordBoxStatics<R, F: FnOnce(&IPasswordBoxStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<PasswordBox, IPasswordBoxStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for PasswordBox {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IPasswordBox>();
}
unsafe impl windows_core::Interface for PasswordBox {
    type Vtable = <IPasswordBox as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IPasswordBox as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for PasswordBox {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.PasswordBox";
}
unsafe impl Send for PasswordBox {}
unsafe impl Sync for PasswordBox {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PasswordRevealMode(pub i32);
impl PasswordRevealMode {
    pub const Hidden: Self = Self(1);
    pub const Peek: Self = Self(0);
    pub const Visible: Self = Self(2);
}
impl windows_core::imp::TypeKind for PasswordRevealMode {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for PasswordRevealMode {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.PasswordRevealMode");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.Controls.PasswordRevealMode;i4)");
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct PlacementMode(pub i32);
impl PlacementMode {
    pub const Bottom: Self = Self(2);
    pub const Left: Self = Self(9);
    pub const Mouse: Self = Self(7);
    pub const Right: Self = Self(4);
    pub const Top: Self = Self(10);
}
impl windows_core::imp::TypeKind for PlacementMode {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for PlacementMode {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.Primitives.PlacementMode");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.Controls.Primitives.PlacementMode;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProgressRing(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(ProgressRing, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(ProgressRing, Control, FrameworkElement, UIElement, DependencyObject);
impl ProgressRing {
    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetIsActive(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetIsActive)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetIsIndeterminate(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetIsIndeterminate)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::IProgressRingFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::IProgressRingFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IProgressRingFactory<R, F: FnOnce(&IProgressRingFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ProgressRing, IProgressRingFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IProgressRingStatics<R, F: FnOnce(&IProgressRingStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ProgressRing, IProgressRingStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for ProgressRing {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IProgressRing>();
}
unsafe impl windows_core::Interface for ProgressRing {
    type Vtable = <IProgressRing as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IProgressRing as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ProgressRing {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.ProgressRing";
}
unsafe impl Send for ProgressRing {}
unsafe impl Sync for ProgressRing {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RadioButton(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(RadioButton, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(
    RadioButton,
    ToggleButton,
    ButtonBase,
    ContentControl,
    Control,
    FrameworkElement,
    UIElement,
    DependencyObject
);
impl RadioButton {
    pub fn Click<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IButtonBase>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Click)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveClick))
        }
    }

    pub fn SetContent<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IContentControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetContent)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetGroupName(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetGroupName)(windows_core::Interface::as_raw(self), core::mem::transmute_copy(value))
                .ok()
        }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::IRadioButtonFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::IRadioButtonFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn IsChecked(&self) -> windows_core::Result<bool> {
        let this = &windows_core::Interface::cast::<IToggleButton>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).IsChecked)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
                .and_then(|r__: windows_reference::IReference<bool>| r__.Value())
        }
    }

    pub fn SetIsChecked(&self, value: Option<bool>) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IToggleButton>(self)?;
        let value__ = value.map(<windows_reference::IReference<bool> as From<_>>::from);
        unsafe {
            (windows_core::Interface::vtable(this).SetIsChecked)(
                windows_core::Interface::as_raw(this),
                windows_core::Param::param(value__.as_ref()).abi(),
            )
            .ok()
        }
    }

    pub fn Checked<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IToggleButton>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Checked)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveChecked))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IRadioButtonFactory<R, F: FnOnce(&IRadioButtonFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<RadioButton, IRadioButtonFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IRadioButtonStatics<R, F: FnOnce(&IRadioButtonStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<RadioButton, IRadioButtonStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for RadioButton {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IRadioButton>();
}
unsafe impl windows_core::Interface for RadioButton {
    type Vtable = <IRadioButton as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IRadioButton as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for RadioButton {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.RadioButton";
}
unsafe impl Send for RadioButton {}
unsafe impl Sync for RadioButton {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RangeBase(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(RangeBase, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(RangeBase, Control, FrameworkElement, UIElement, DependencyObject);
impl RangeBase {
    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetMinimum(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetMinimum)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetMaximum(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetMaximum)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetSmallChange(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetSmallChange)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn Value(&self) -> windows_core::Result<f64> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Value)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }

    pub fn SetValue(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetValue)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn ValueChanged<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RangeBaseValueChangedEventArgs>) + Send + 'static,
    {
        let handler = <RangeBaseValueChangedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).ValueChanged)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemoveValueChanged))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IRangeBaseFactory<R, F: FnOnce(&IRangeBaseFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<RangeBase, IRangeBaseFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IRangeBaseStatics<R, F: FnOnce(&IRangeBaseStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<RangeBase, IRangeBaseStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for RangeBase {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IRangeBase>();
}
unsafe impl windows_core::Interface for RangeBase {
    type Vtable = <IRangeBase as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IRangeBase as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for RangeBase {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.Primitives.RangeBase";
}
unsafe impl Send for RangeBase {}
unsafe impl Sync for RangeBase {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RangeBaseValueChangedEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(RangeBaseValueChangedEventArgs, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(RangeBaseValueChangedEventArgs, RoutedEventArgs);
impl RangeBaseValueChangedEventArgs {
    pub fn NewValue(&self) -> windows_core::Result<f64> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).NewValue)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }
}
impl windows_core::RuntimeType for RangeBaseValueChangedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IRangeBaseValueChangedEventArgs>();
}
unsafe impl windows_core::Interface for RangeBaseValueChangedEventArgs {
    type Vtable = <IRangeBaseValueChangedEventArgs as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IRangeBaseValueChangedEventArgs as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for RangeBaseValueChangedEventArgs {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.Primitives.RangeBaseValueChangedEventArgs";
}
unsafe impl Send for RangeBaseValueChangedEventArgs {}
unsafe impl Sync for RangeBaseValueChangedEventArgs {}
windows_core::imp::define_interface!(
    RangeBaseValueChangedEventHandler,
    RangeBaseValueChangedEventHandler_Vtbl,
    0x23f0e209_9455_54cb_b8bc_0b49553c7dcc
);
impl windows_core::RuntimeType for RangeBaseValueChangedEventHandler {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
impl RangeBaseValueChangedEventHandler {
    pub fn new<
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RangeBaseValueChangedEventArgs>) -> windows_core::Result<()>
            + Send
            + 'static,
    >(
        invoke: F,
    ) -> Self {
        let com = windows_core::imp::DelegateBox::<Self, F>::new(&RangeBaseValueChangedEventHandlerBox::<F>::VTABLE, invoke);
        unsafe { core::mem::transmute(windows_core::imp::box_new(com)) }
    }

    pub fn Invoke<P0, P1>(&self, sender: P0, e: P1) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
        P1: windows_core::Param<RangeBaseValueChangedEventArgs>,
    {
        unsafe {
            (windows_core::Interface::vtable(self).Invoke)(windows_core::Interface::as_raw(self), sender.param().abi(), e.param().abi())
                .ok()
        }
    }
}
#[repr(C)]
pub struct RangeBaseValueChangedEventHandler_Vtbl {
    base__: windows_core::IUnknown_Vtbl,
    Invoke: unsafe extern "system" fn(
        this: *mut core::ffi::c_void,
        sender: *mut core::ffi::c_void,
        e: *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
struct RangeBaseValueChangedEventHandlerBox<
    F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RangeBaseValueChangedEventArgs>) -> windows_core::Result<()>
        + Send
        + 'static,
>(core::marker::PhantomData<(fn() -> F,)>);
impl<
    F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RangeBaseValueChangedEventArgs>) -> windows_core::Result<()>
        + Send
        + 'static,
> RangeBaseValueChangedEventHandlerBox<F>
{
    const VTABLE: RangeBaseValueChangedEventHandler_Vtbl = RangeBaseValueChangedEventHandler_Vtbl {
        base__: windows_core::IUnknown_Vtbl {
            QueryInterface: windows_core::imp::DelegateBox::<RangeBaseValueChangedEventHandler, F>::QueryInterface,
            AddRef: windows_core::imp::DelegateBox::<RangeBaseValueChangedEventHandler, F>::AddRef,
            Release: windows_core::imp::DelegateBox::<RangeBaseValueChangedEventHandler, F>::Release,
        },
        Invoke: Self::Invoke,
    };

    unsafe extern "system" fn Invoke(
        this: *mut core::ffi::c_void,
        sender: *mut core::ffi::c_void,
        e: *mut core::ffi::c_void,
    ) -> windows_core::HRESULT {
        unsafe {
            let this =
                &mut *(this as *mut *mut core::ffi::c_void as *mut windows_core::imp::DelegateBox<RangeBaseValueChangedEventHandler, F>);
            (this.invoke)(core::mem::transmute_copy(&sender), core::mem::transmute_copy(&e)).into()
        }
    }
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceDictionary(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(ResourceDictionary, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(ResourceDictionary, IIterable < windows_collections::IKeyValuePair < windows_core::IInspectable, windows_core::IInspectable > >, IMap < windows_core::IInspectable, windows_core::IInspectable >, DependencyObject);
impl ResourceDictionary {
    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn First(
        &self,
    ) -> windows_core::Result<
        windows_collections::IIterator<windows_collections::IKeyValuePair<windows_core::IInspectable, windows_core::IInspectable>>,
    > {
        let this = &windows_core::Interface::cast::<
            IIterable<windows_collections::IKeyValuePair<windows_core::IInspectable, windows_core::IInspectable>>,
        >(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).First)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn Lookup<P0>(&self, key: P0) -> windows_core::Result<windows_core::IInspectable>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IMap<windows_core::IInspectable, windows_core::IInspectable>>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Lookup)(windows_core::Interface::as_raw(this), key.param().abi(), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn HasKey<P0>(&self, key: P0) -> windows_core::Result<bool>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IMap<windows_core::IInspectable, windows_core::IInspectable>>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).HasKey)(windows_core::Interface::as_raw(this), key.param().abi(), &mut result__)
                .map(|| result__)
        }
    }

    pub fn Insert<P0, P1>(&self, key: P0, value: P1) -> windows_core::Result<bool>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
        P1: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IMap<windows_core::IInspectable, windows_core::IInspectable>>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Insert)(
                windows_core::Interface::as_raw(this),
                key.param().abi(),
                value.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }

    pub fn Remove<P0>(&self, key: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IMap<windows_core::IInspectable, windows_core::IInspectable>>(self)?;
        unsafe { (windows_core::Interface::vtable(this).Remove)(windows_core::Interface::as_raw(this), key.param().abi()).ok() }
    }

    pub fn MergedDictionaries(&self) -> windows_core::Result<IVector<Self>> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).MergedDictionaries)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    fn IResourceDictionaryFactory<R, F: FnOnce(&IResourceDictionaryFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ResourceDictionary, IResourceDictionaryFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for ResourceDictionary {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IResourceDictionary>();
}
unsafe impl windows_core::Interface for ResourceDictionary {
    type Vtable = <IResourceDictionary as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IResourceDictionary as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ResourceDictionary {
    const NAME: &'static str = "Microsoft.UI.Xaml.ResourceDictionary";
}
unsafe impl Send for ResourceDictionary {}
unsafe impl Sync for ResourceDictionary {}
impl IntoIterator for ResourceDictionary {
    type IntoIter = windows_collections::BufferedIterator<Self::Item>;
    type Item = windows_collections::IKeyValuePair<windows_core::IInspectable, windows_core::IInspectable>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(&self)
    }
}
impl IntoIterator for &ResourceDictionary {
    type IntoIter = windows_collections::BufferedIterator<Self::Item>;
    type Item = windows_collections::IKeyValuePair<windows_core::IInspectable, windows_core::IInspectable>;

    fn into_iter(self) -> Self::IntoIter {
        windows_collections::BufferedIterator::new(self.First().unwrap())
    }
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RoutedEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(RoutedEventArgs, windows_core::IUnknown, windows_core::IInspectable);
impl RoutedEventArgs {
    fn IRoutedEventArgsFactory<R, F: FnOnce(&IRoutedEventArgsFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<RoutedEventArgs, IRoutedEventArgsFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for RoutedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IRoutedEventArgs>();
}
unsafe impl windows_core::Interface for RoutedEventArgs {
    type Vtable = <IRoutedEventArgs as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IRoutedEventArgs as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for RoutedEventArgs {
    const NAME: &'static str = "Microsoft.UI.Xaml.RoutedEventArgs";
}
unsafe impl Send for RoutedEventArgs {}
unsafe impl Sync for RoutedEventArgs {}
windows_core::imp::define_interface!(RoutedEventHandler, RoutedEventHandler_Vtbl, 0xdae23d85_69ca_5bdf_805b_6161a3a215cc);
impl windows_core::RuntimeType for RoutedEventHandler {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
impl RoutedEventHandler {
    pub fn new<
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) -> windows_core::Result<()> + Send + 'static,
    >(
        invoke: F,
    ) -> Self {
        let com = windows_core::imp::DelegateBox::<Self, F>::new(&RoutedEventHandlerBox::<F>::VTABLE, invoke);
        unsafe { core::mem::transmute(windows_core::imp::box_new(com)) }
    }

    pub fn Invoke<P0, P1>(&self, sender: P0, e: P1) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
        P1: windows_core::Param<RoutedEventArgs>,
    {
        unsafe {
            (windows_core::Interface::vtable(self).Invoke)(windows_core::Interface::as_raw(self), sender.param().abi(), e.param().abi())
                .ok()
        }
    }
}
#[repr(C)]
pub struct RoutedEventHandler_Vtbl {
    base__: windows_core::IUnknown_Vtbl,
    Invoke: unsafe extern "system" fn(
        this: *mut core::ffi::c_void,
        sender: *mut core::ffi::c_void,
        e: *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
struct RoutedEventHandlerBox<
    F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) -> windows_core::Result<()> + Send + 'static,
>(core::marker::PhantomData<(fn() -> F,)>);
impl<F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) -> windows_core::Result<()> + Send + 'static>
    RoutedEventHandlerBox<F>
{
    const VTABLE: RoutedEventHandler_Vtbl = RoutedEventHandler_Vtbl {
        base__: windows_core::IUnknown_Vtbl {
            QueryInterface: windows_core::imp::DelegateBox::<RoutedEventHandler, F>::QueryInterface,
            AddRef: windows_core::imp::DelegateBox::<RoutedEventHandler, F>::AddRef,
            Release: windows_core::imp::DelegateBox::<RoutedEventHandler, F>::Release,
        },
        Invoke: Self::Invoke,
    };

    unsafe extern "system" fn Invoke(
        this: *mut core::ffi::c_void,
        sender: *mut core::ffi::c_void,
        e: *mut core::ffi::c_void,
    ) -> windows_core::HRESULT {
        unsafe {
            let this = &mut *(this as *mut *mut core::ffi::c_void as *mut windows_core::imp::DelegateBox<RoutedEventHandler, F>);
            (this.invoke)(core::mem::transmute_copy(&sender), core::mem::transmute_copy(&e)).into()
        }
    }
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RowDefinition(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(RowDefinition, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(RowDefinition, DependencyObject);
impl RowDefinition {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }

    fn IActivationFactory<R, F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<RowDefinition, windows_core::imp::IGenericFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn SetHeight(&self, value: GridLength) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetHeight)(windows_core::Interface::as_raw(self), value).ok() }
    }

    fn IRowDefinitionStatics<R, F: FnOnce(&IRowDefinitionStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<RowDefinition, IRowDefinitionStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for RowDefinition {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IRowDefinition>();
}
unsafe impl windows_core::Interface for RowDefinition {
    type Vtable = <IRowDefinition as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IRowDefinition as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for RowDefinition {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.RowDefinition";
}
unsafe impl Send for RowDefinition {}
unsafe impl Sync for RowDefinition {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RowDefinitionCollection(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(
    RowDefinitionCollection,
    windows_core::IUnknown,
    windows_core::IInspectable,
    IVector<RowDefinition>
);
windows_core::imp::required_hierarchy!(RowDefinitionCollection, IIterable<RowDefinition>);
impl RowDefinitionCollection {
    pub fn First(&self) -> windows_core::Result<windows_collections::IIterator<RowDefinition>> {
        let this = &windows_core::Interface::cast::<IIterable<RowDefinition>>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).First)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn GetAt(&self, index: u32) -> windows_core::Result<RowDefinition> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).GetAt)(windows_core::Interface::as_raw(self), index, &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn Size(&self) -> windows_core::Result<u32> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Size)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }

    pub fn IndexOf<P0>(&self, value: P0, index: &mut u32) -> windows_core::Result<bool>
    where
        P0: windows_core::Param<RowDefinition>,
    {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).IndexOf)(
                windows_core::Interface::as_raw(self),
                value.param().abi(),
                index,
                &mut result__,
            )
            .map(|| result__)
        }
    }

    pub fn InsertAt<P1>(&self, index: u32, value: P1) -> windows_core::Result<()>
    where
        P1: windows_core::Param<RowDefinition>,
    {
        unsafe { (windows_core::Interface::vtable(self).InsertAt)(windows_core::Interface::as_raw(self), index, value.param().abi()).ok() }
    }

    pub fn RemoveAt(&self, index: u32) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).RemoveAt)(windows_core::Interface::as_raw(self), index).ok() }
    }

    pub fn Append<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<RowDefinition>,
    {
        unsafe { (windows_core::Interface::vtable(self).Append)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn Clear(&self) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).Clear)(windows_core::Interface::as_raw(self)).ok() }
    }
}
impl windows_core::RuntimeType for RowDefinitionCollection {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IVector<RowDefinition>>();
}
unsafe impl windows_core::Interface for RowDefinitionCollection {
    type Vtable = <IVector<RowDefinition> as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IVector<RowDefinition> as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for RowDefinitionCollection {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.RowDefinitionCollection";
}
unsafe impl Send for RowDefinitionCollection {}
unsafe impl Sync for RowDefinitionCollection {}
impl IntoIterator for RowDefinitionCollection {
    type IntoIter = windows_collections::BufferedIterator<Self::Item>;
    type Item = RowDefinition;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(&self)
    }
}
impl IntoIterator for &RowDefinitionCollection {
    type IntoIter = windows_collections::BufferedIterator<Self::Item>;
    type Item = RowDefinition;

    fn into_iter(self) -> Self::IntoIter {
        windows_collections::BufferedIterator::new(self.First().unwrap())
    }
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ScrollBarVisibility(pub i32);
impl ScrollBarVisibility {
    pub const Auto: Self = Self(1);
    pub const Disabled: Self = Self(0);
    pub const Hidden: Self = Self(2);
    pub const Visible: Self = Self(3);
}
impl windows_core::imp::TypeKind for ScrollBarVisibility {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for ScrollBarVisibility {
    const NAME: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.ScrollBarVisibility");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.Controls.ScrollBarVisibility;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScrollViewer(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(ScrollViewer, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(ScrollViewer, ContentControl, Control, FrameworkElement, UIElement, DependencyObject);
impl ScrollViewer {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }

    fn IActivationFactory<R, F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ScrollViewer, windows_core::imp::IGenericFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    pub fn SetContent<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IContentControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetContent)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetHorizontalScrollBarVisibility(&self, value: ScrollBarVisibility) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetHorizontalScrollBarVisibility)(windows_core::Interface::as_raw(self), value).ok()
        }
    }

    pub fn SetVerticalScrollBarVisibility(&self, value: ScrollBarVisibility) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetVerticalScrollBarVisibility)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IScrollViewerStatics<R, F: FnOnce(&IScrollViewerStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ScrollViewer, IScrollViewerStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for ScrollViewer {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IScrollViewer>();
}
unsafe impl windows_core::Interface for ScrollViewer {
    type Vtable = <IScrollViewer as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IScrollViewer as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ScrollViewer {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.ScrollViewer";
}
unsafe impl Send for ScrollViewer {}
unsafe impl Sync for ScrollViewer {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SelectionChangedEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(SelectionChangedEventArgs, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(SelectionChangedEventArgs, RoutedEventArgs);
impl SelectionChangedEventArgs {
    fn ISelectionChangedEventArgsFactory<R, F: FnOnce(&ISelectionChangedEventArgsFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<SelectionChangedEventArgs, ISelectionChangedEventArgsFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for SelectionChangedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, ISelectionChangedEventArgs>();
}
unsafe impl windows_core::Interface for SelectionChangedEventArgs {
    type Vtable = <ISelectionChangedEventArgs as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <ISelectionChangedEventArgs as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for SelectionChangedEventArgs {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.SelectionChangedEventArgs";
}
unsafe impl Send for SelectionChangedEventArgs {}
unsafe impl Sync for SelectionChangedEventArgs {}
windows_core::imp::define_interface!(
    SelectionChangedEventHandler,
    SelectionChangedEventHandler_Vtbl,
    0xa232390d_0e34_595e_8931_fa928a9909f4
);
impl windows_core::RuntimeType for SelectionChangedEventHandler {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
impl SelectionChangedEventHandler {
    pub fn new<
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<SelectionChangedEventArgs>) -> windows_core::Result<()>
            + Send
            + 'static,
    >(
        invoke: F,
    ) -> Self {
        let com = windows_core::imp::DelegateBox::<Self, F>::new(&SelectionChangedEventHandlerBox::<F>::VTABLE, invoke);
        unsafe { core::mem::transmute(windows_core::imp::box_new(com)) }
    }

    pub fn Invoke<P0, P1>(&self, sender: P0, e: P1) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
        P1: windows_core::Param<SelectionChangedEventArgs>,
    {
        unsafe {
            (windows_core::Interface::vtable(self).Invoke)(windows_core::Interface::as_raw(self), sender.param().abi(), e.param().abi())
                .ok()
        }
    }
}
#[repr(C)]
pub struct SelectionChangedEventHandler_Vtbl {
    base__: windows_core::IUnknown_Vtbl,
    Invoke: unsafe extern "system" fn(
        this: *mut core::ffi::c_void,
        sender: *mut core::ffi::c_void,
        e: *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
struct SelectionChangedEventHandlerBox<
    F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<SelectionChangedEventArgs>) -> windows_core::Result<()>
        + Send
        + 'static,
>(core::marker::PhantomData<(fn() -> F,)>);
impl<
    F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<SelectionChangedEventArgs>) -> windows_core::Result<()>
        + Send
        + 'static,
> SelectionChangedEventHandlerBox<F>
{
    const VTABLE: SelectionChangedEventHandler_Vtbl = SelectionChangedEventHandler_Vtbl {
        base__: windows_core::IUnknown_Vtbl {
            QueryInterface: windows_core::imp::DelegateBox::<SelectionChangedEventHandler, F>::QueryInterface,
            AddRef: windows_core::imp::DelegateBox::<SelectionChangedEventHandler, F>::AddRef,
            Release: windows_core::imp::DelegateBox::<SelectionChangedEventHandler, F>::Release,
        },
        Invoke: Self::Invoke,
    };

    unsafe extern "system" fn Invoke(
        this: *mut core::ffi::c_void,
        sender: *mut core::ffi::c_void,
        e: *mut core::ffi::c_void,
    ) -> windows_core::HRESULT {
        unsafe {
            let this = &mut *(this as *mut *mut core::ffi::c_void as *mut windows_core::imp::DelegateBox<SelectionChangedEventHandler, F>);
            (this.invoke)(core::mem::transmute_copy(&sender), core::mem::transmute_copy(&e)).into()
        }
    }
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Selector(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(Selector, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(Selector, ItemsControl, Control, FrameworkElement, UIElement, DependencyObject);
impl Selector {
    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetItemsSource<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IItemsControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetItemsSource)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SelectedIndex(&self) -> windows_core::Result<i32> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).SelectedIndex)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }

    pub fn SetSelectedIndex(&self, value: i32) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetSelectedIndex)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SelectionChanged<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<SelectionChangedEventArgs>) + Send + 'static,
    {
        let handler = <SelectionChangedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).SelectionChanged)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemoveSelectionChanged))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn ISelectorStatics<R, F: FnOnce(&ISelectorStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Selector, ISelectorStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for Selector {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, ISelector>();
}
unsafe impl windows_core::Interface for Selector {
    type Vtable = <ISelector as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <ISelector as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for Selector {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.Primitives.Selector";
}
unsafe impl Send for Selector {}
unsafe impl Sync for Selector {}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SizeInt32 {
    pub Width: i32,
    pub Height: i32,
}
impl windows_core::imp::TypeKind for SizeInt32 {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for SizeInt32 {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Windows.Graphics.SizeInt32");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"struct(Windows.Graphics.SizeInt32;i4;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Slider(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(Slider, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(Slider, RangeBase, Control, FrameworkElement, UIElement, DependencyObject);
impl Slider {
    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetMinimum(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IRangeBase>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinimum)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaximum(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IRangeBase>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaximum)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetSmallChange(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IRangeBase>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetSmallChange)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Value(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IRangeBase>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Value)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetValue(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IRangeBase>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetValue)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ValueChanged<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RangeBaseValueChangedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IRangeBase>(self)?;
        let handler = <RangeBaseValueChangedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).ValueChanged)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveValueChanged))
        }
    }

    pub fn SetStepFrequency(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetStepFrequency)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::ISliderFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::ISliderFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn ISliderFactory<R, F: FnOnce(&ISliderFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Slider, ISliderFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn ISliderStatics<R, F: FnOnce(&ISliderStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Slider, ISliderStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for Slider {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, ISlider>();
}
unsafe impl windows_core::Interface for Slider {
    type Vtable = <ISlider as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <ISlider as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for Slider {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.Slider";
}
unsafe impl Send for Slider {}
unsafe impl Sync for Slider {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SolidColorBrush(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(SolidColorBrush, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(SolidColorBrush, Brush, DependencyObject);
impl SolidColorBrush {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }

    fn IActivationFactory<R, F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<SolidColorBrush, windows_core::imp::IGenericFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn SetColor(&self, value: Color) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetColor)(windows_core::Interface::as_raw(self), value).ok() }
    }

    fn ISolidColorBrushFactory<R, F: FnOnce(&ISolidColorBrushFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<SolidColorBrush, ISolidColorBrushFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn ISolidColorBrushStatics<R, F: FnOnce(&ISolidColorBrushStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<SolidColorBrush, ISolidColorBrushStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for SolidColorBrush {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, ISolidColorBrush>();
}
unsafe impl windows_core::Interface for SolidColorBrush {
    type Vtable = <ISolidColorBrush as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <ISolidColorBrush as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for SolidColorBrush {
    const NAME: &'static str = "Microsoft.UI.Xaml.Media.SolidColorBrush";
}
unsafe impl Send for SolidColorBrush {}
unsafe impl Sync for SolidColorBrush {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StackPanel(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(StackPanel, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(StackPanel, Panel, FrameworkElement, UIElement, DependencyObject);
impl StackPanel {
    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn Children(&self) -> windows_core::Result<UIElementCollection> {
        let this = &windows_core::Interface::cast::<IPanel>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Children)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetBackground<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Brush>,
    {
        let this = &windows_core::Interface::cast::<IPanel>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetBackground)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetOrientation(&self, value: Orientation) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetOrientation)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetSpacing(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetSpacing)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::IStackPanelFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::IStackPanelFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IStackPanelFactory<R, F: FnOnce(&IStackPanelFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<StackPanel, IStackPanelFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IStackPanelStatics<R, F: FnOnce(&IStackPanelStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<StackPanel, IStackPanelStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for StackPanel {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IStackPanel>();
}
unsafe impl windows_core::Interface for StackPanel {
    type Vtable = <IStackPanel as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IStackPanel as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for StackPanel {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.StackPanel";
}
unsafe impl Send for StackPanel {}
unsafe impl Sync for StackPanel {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Stretch(pub i32);
impl Stretch {
    pub const Fill: Self = Self(1);
    pub const None: Self = Self(0);
    pub const Uniform: Self = Self(2);
    pub const UniformToFill: Self = Self(3);
}
impl windows_core::imp::TypeKind for Stretch {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for Stretch {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Media.Stretch");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.Media.Stretch;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Style(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(Style, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(Style, DependencyObject);
impl Style {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }

    fn IActivationFactory<R, F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Style, windows_core::imp::IGenericFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    fn IStyleFactory<R, F: FnOnce(&IStyleFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Style, IStyleFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for Style {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IStyle>();
}
unsafe impl windows_core::Interface for Style {
    type Vtable = <IStyle as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IStyle as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for Style {
    const NAME: &'static str = "Microsoft.UI.Xaml.Style";
}
unsafe impl Send for Style {}
unsafe impl Sync for Style {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Symbol(pub i32);
impl Symbol {
    pub const Accept: Self = Self(57611);
    pub const Account: Self = Self(57704);
    pub const Add: Self = Self(57609);
    pub const AddFriend: Self = Self(57826);
    pub const Admin: Self = Self(57767);
    pub const AlignCenter: Self = Self(57761);
    pub const AlignLeft: Self = Self(57762);
    pub const AlignRight: Self = Self(57760);
    pub const AllApps: Self = Self(57721);
    pub const Attach: Self = Self(57708);
    pub const AttachCamera: Self = Self(57645);
    pub const Audio: Self = Self(57737);
    pub const Back: Self = Self(57618);
    pub const BackToWindow: Self = Self(57816);
    pub const BlockContact: Self = Self(57824);
    pub const Bold: Self = Self(57755);
    pub const Bookmarks: Self = Self(57647);
    pub const BrowsePhotos: Self = Self(57685);
    pub const Bullets: Self = Self(57651);
    pub const Calculator: Self = Self(57808);
    pub const Calendar: Self = Self(57699);
    pub const CalendarDay: Self = Self(57697);
    pub const CalendarReply: Self = Self(57819);
    pub const CalendarWeek: Self = Self(57698);
    pub const Camera: Self = Self(57620);
    pub const Cancel: Self = Self(57610);
    pub const Caption: Self = Self(57690);
    pub const CellPhone: Self = Self(57801);
    pub const Character: Self = Self(57700);
    pub const Clear: Self = Self(57606);
    pub const ClearSelection: Self = Self(57797);
    pub const Clock: Self = Self(57633);
    pub const ClosePane: Self = Self(57639);
    pub const ClosedCaption: Self = Self(57744);
    pub const Comment: Self = Self(57652);
    pub const Contact: Self = Self(57661);
    pub const Contact2: Self = Self(57735);
    pub const ContactInfo: Self = Self(57654);
    pub const ContactPresence: Self = Self(57729);
    pub const Copy: Self = Self(57711);
    pub const Crop: Self = Self(57635);
    pub const Cut: Self = Self(57707);
    pub const Delete: Self = Self(57607);
    pub const Directions: Self = Self(57809);
    pub const DisableUpdates: Self = Self(57748);
    pub const DisconnectDrive: Self = Self(57722);
    pub const Dislike: Self = Self(57758);
    pub const DockBottom: Self = Self(57671);
    pub const DockLeft: Self = Self(57669);
    pub const DockRight: Self = Self(57670);
    pub const Document: Self = Self(57648);
    pub const Download: Self = Self(57624);
    pub const Edit: Self = Self(57604);
    pub const Emoji: Self = Self(57629);
    pub const Emoji2: Self = Self(57712);
    pub const Favorite: Self = Self(57619);
    pub const Filter: Self = Self(57710);
    pub const Find: Self = Self(57626);
    pub const Flag: Self = Self(57641);
    pub const Folder: Self = Self(57736);
    pub const Font: Self = Self(57733);
    pub const FontColor: Self = Self(57734);
    pub const FontDecrease: Self = Self(57798);
    pub const FontIncrease: Self = Self(57799);
    pub const FontSize: Self = Self(57800);
    pub const Forward: Self = Self(57617);
    pub const FourBars: Self = Self(57833);
    pub const FullScreen: Self = Self(57817);
    pub const GlobalNavigationButton: Self = Self(59136);
    pub const Globe: Self = Self(57643);
    pub const Go: Self = Self(57667);
    pub const GoToStart: Self = Self(57828);
    pub const GoToToday: Self = Self(57732);
    pub const HangUp: Self = Self(57655);
    pub const Help: Self = Self(57627);
    pub const HideBcc: Self = Self(57706);
    pub const Highlight: Self = Self(57747);
    pub const Home: Self = Self(57615);
    pub const Import: Self = Self(57680);
    pub const ImportAll: Self = Self(57681);
    pub const Important: Self = Self(57713);
    pub const Italic: Self = Self(57753);
    pub const Keyboard: Self = Self(57668);
    pub const LeaveChat: Self = Self(57631);
    pub const Library: Self = Self(57811);
    pub const Like: Self = Self(57759);
    pub const LikeDislike: Self = Self(57757);
    pub const Link: Self = Self(57703);
    pub const List: Self = Self(57676);
    pub const Mail: Self = Self(57625);
    pub const MailFilled: Self = Self(57653);
    pub const MailForward: Self = Self(57632);
    pub const MailReply: Self = Self(57714);
    pub const MailReplyAll: Self = Self(57701);
    pub const Manage: Self = Self(57720);
    pub const Map: Self = Self(57796);
    pub const MapDrive: Self = Self(57723);
    pub const MapPin: Self = Self(57657);
    pub const Memo: Self = Self(57813);
    pub const Message: Self = Self(57695);
    pub const Microphone: Self = Self(57814);
    pub const More: Self = Self(57612);
    pub const MoveToFolder: Self = Self(57756);
    pub const MusicInfo: Self = Self(57666);
    pub const Mute: Self = Self(57752);
    pub const NewFolder: Self = Self(57818);
    pub const NewWindow: Self = Self(57724);
    pub const Next: Self = Self(57601);
    pub const OneBar: Self = Self(57830);
    pub const OpenFile: Self = Self(57765);
    pub const OpenLocal: Self = Self(57751);
    pub const OpenPane: Self = Self(57638);
    pub const OpenWith: Self = Self(57725);
    pub const Orientation: Self = Self(57679);
    pub const OtherUser: Self = Self(57766);
    pub const OutlineStar: Self = Self(57806);
    pub const Page: Self = Self(57650);
    pub const Page2: Self = Self(57696);
    pub const Paste: Self = Self(57709);
    pub const Pause: Self = Self(57603);
    pub const People: Self = Self(57637);
    pub const Permissions: Self = Self(57746);
    pub const Phone: Self = Self(57658);
    pub const PhoneBook: Self = Self(57812);
    pub const Pictures: Self = Self(57688);
    pub const Pin: Self = Self(57665);
    pub const Placeholder: Self = Self(57738);
    pub const Play: Self = Self(57602);
    pub const PostUpdate: Self = Self(57815);
    pub const Preview: Self = Self(58005);
    pub const PreviewLink: Self = Self(57642);
    pub const Previous: Self = Self(57600);
    pub const Print: Self = Self(59209);
    pub const Priority: Self = Self(57730);
    pub const ProtectedDocument: Self = Self(57649);
    pub const ReShare: Self = Self(57802);
    pub const Read: Self = Self(57702);
    pub const Redo: Self = Self(57613);
    pub const Refresh: Self = Self(57673);
    pub const Remote: Self = Self(57672);
    pub const Remove: Self = Self(57608);
    pub const Rename: Self = Self(57662);
    pub const Repair: Self = Self(57694);
    pub const RepeatAll: Self = Self(57805);
    pub const RepeatOne: Self = Self(57804);
    pub const ReportHacked: Self = Self(57822);
    pub const Rotate: Self = Self(57674);
    pub const RotateCamera: Self = Self(57636);
    pub const Save: Self = Self(57605);
    pub const SaveLocal: Self = Self(57689);
    pub const Scan: Self = Self(58004);
    pub const SelectAll: Self = Self(57678);
    pub const Send: Self = Self(57634);
    pub const SetLockScreen: Self = Self(57740);
    pub const SetTile: Self = Self(57741);
    pub const Setting: Self = Self(57621);
    pub const Share: Self = Self(59181);
    pub const Shop: Self = Self(57677);
    pub const ShowBcc: Self = Self(57705);
    pub const ShowResults: Self = Self(57692);
    pub const Shuffle: Self = Self(57675);
    pub const SlideShow: Self = Self(57715);
    pub const SolidStar: Self = Self(57807);
    pub const Sort: Self = Self(57716);
    pub const Stop: Self = Self(57691);
    pub const StopSlideShow: Self = Self(57745);
    pub const Street: Self = Self(57795);
    pub const Switch: Self = Self(57660);
    pub const SwitchApps: Self = Self(57825);
    pub const Sync: Self = Self(57623);
    pub const SyncFolder: Self = Self(57823);
    pub const Tag: Self = Self(57803);
    pub const Target: Self = Self(57810);
    pub const ThreeBars: Self = Self(57832);
    pub const TouchPointer: Self = Self(57827);
    pub const Trim: Self = Self(57644);
    pub const TwoBars: Self = Self(57831);
    pub const TwoPage: Self = Self(57630);
    pub const UnFavorite: Self = Self(57749);
    pub const UnPin: Self = Self(57750);
    pub const UnSyncFolder: Self = Self(57821);
    pub const Underline: Self = Self(57754);
    pub const Undo: Self = Self(57614);
    pub const Up: Self = Self(57616);
    pub const Upload: Self = Self(57628);
    pub const Video: Self = Self(57622);
    pub const VideoChat: Self = Self(57659);
    pub const View: Self = Self(57739);
    pub const ViewAll: Self = Self(57656);
    pub const Volume: Self = Self(57693);
    pub const WebCam: Self = Self(57686);
    pub const World: Self = Self(57640);
    pub const XboxOneConsole: Self = Self(59792);
    pub const ZeroBars: Self = Self(57829);
    pub const Zoom: Self = Self(57763);
    pub const ZoomIn: Self = Self(57646);
    pub const ZoomOut: Self = Self(57764);
}
impl windows_core::imp::TypeKind for Symbol {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for Symbol {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Controls.Symbol");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.Controls.Symbol;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SymbolIcon(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(SymbolIcon, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(SymbolIcon, IconElement, FrameworkElement, UIElement, DependencyObject);
impl SymbolIcon {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }

    fn IActivationFactory<R, F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<SymbolIcon, windows_core::imp::IGenericFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetSymbol(&self, value: Symbol) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetSymbol)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn ISymbolIconFactory<R, F: FnOnce(&ISymbolIconFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<SymbolIcon, ISymbolIconFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn ISymbolIconStatics<R, F: FnOnce(&ISymbolIconStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<SymbolIcon, ISymbolIconStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for SymbolIcon {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, ISymbolIcon>();
}
unsafe impl windows_core::Interface for SymbolIcon {
    type Vtable = <ISymbolIcon as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <ISymbolIcon as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for SymbolIcon {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.SymbolIcon";
}
unsafe impl Send for SymbolIcon {}
unsafe impl Sync for SymbolIcon {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SystemBackdrop(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(SystemBackdrop, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(SystemBackdrop, DependencyObject);
impl SystemBackdrop {
    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    fn ISystemBackdropFactory<R, F: FnOnce(&ISystemBackdropFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<SystemBackdrop, ISystemBackdropFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for SystemBackdrop {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, ISystemBackdrop>();
}
unsafe impl windows_core::Interface for SystemBackdrop {
    type Vtable = <ISystemBackdrop as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <ISystemBackdrop as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for SystemBackdrop {
    const NAME: &'static str = "Microsoft.UI.Xaml.Media.SystemBackdrop";
}
unsafe impl Send for SystemBackdrop {}
unsafe impl Sync for SystemBackdrop {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextBlock(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(TextBlock, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(TextBlock, FrameworkElement, UIElement, DependencyObject);
impl TextBlock {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }

    fn IActivationFactory<R, F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<TextBlock, windows_core::imp::IGenericFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetFontSize(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetFontSize)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetFontWeight(&self, value: FontWeight) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetFontWeight)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetTextWrapping(&self, value: TextWrapping) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetTextWrapping)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetTextTrimming(&self, value: TextTrimming) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetTextTrimming)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetText(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetText)(windows_core::Interface::as_raw(self), core::mem::transmute_copy(value)).ok()
        }
    }

    pub fn SetIsTextSelectionEnabled(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetIsTextSelectionEnabled)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetMaxLines(&self, value: i32) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetMaxLines)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn ITextBlockStatics<R, F: FnOnce(&ITextBlockStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<TextBlock, ITextBlockStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for TextBlock {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, ITextBlock>();
}
unsafe impl windows_core::Interface for TextBlock {
    type Vtable = <ITextBlock as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <ITextBlock as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for TextBlock {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.TextBlock";
}
unsafe impl Send for TextBlock {}
unsafe impl Sync for TextBlock {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextBox(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(TextBox, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(TextBox, Control, FrameworkElement, UIElement, DependencyObject);
impl TextBox {
    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn Text(&self) -> windows_core::Result<windows_core::HSTRING> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Text)(windows_core::Interface::as_raw(self), &mut result__)
                .map(|| core::mem::transmute(result__))
        }
    }

    pub fn SetText(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetText)(windows_core::Interface::as_raw(self), core::mem::transmute_copy(value)).ok()
        }
    }

    pub fn SetAcceptsReturn(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetAcceptsReturn)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetTextWrapping(&self, value: TextWrapping) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetTextWrapping)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetPlaceholderText(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetPlaceholderText)(
                windows_core::Interface::as_raw(self),
                core::mem::transmute_copy(value),
            )
            .ok()
        }
    }

    pub fn TextChanged<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<TextChangedEventArgs>) + Send + 'static,
    {
        let handler = <TextChangedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).TextChanged)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemoveTextChanged))
        }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::ITextBoxFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::ITextBoxFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn ITextBoxFactory<R, F: FnOnce(&ITextBoxFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<TextBox, ITextBoxFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn ITextBoxStatics<R, F: FnOnce(&ITextBoxStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<TextBox, ITextBoxStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for TextBox {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, ITextBox>();
}
unsafe impl windows_core::Interface for TextBox {
    type Vtable = <ITextBox as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <ITextBox as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for TextBox {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.TextBox";
}
unsafe impl Send for TextBox {}
unsafe impl Sync for TextBox {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextChangedEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(TextChangedEventArgs, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(TextChangedEventArgs, RoutedEventArgs);
impl windows_core::RuntimeType for TextChangedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, ITextChangedEventArgs>();
}
unsafe impl windows_core::Interface for TextChangedEventArgs {
    type Vtable = <ITextChangedEventArgs as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <ITextChangedEventArgs as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for TextChangedEventArgs {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.TextChangedEventArgs";
}
unsafe impl Send for TextChangedEventArgs {}
unsafe impl Sync for TextChangedEventArgs {}
windows_core::imp::define_interface!(TextChangedEventHandler, TextChangedEventHandler_Vtbl, 0x5d8ddcff_45d8_5e7c_9b8b_c41d2893c6a1);
impl windows_core::RuntimeType for TextChangedEventHandler {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_interface::<Self>();
}
impl TextChangedEventHandler {
    pub fn new<
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<TextChangedEventArgs>) -> windows_core::Result<()>
            + Send
            + 'static,
    >(
        invoke: F,
    ) -> Self {
        let com = windows_core::imp::DelegateBox::<Self, F>::new(&TextChangedEventHandlerBox::<F>::VTABLE, invoke);
        unsafe { core::mem::transmute(windows_core::imp::box_new(com)) }
    }

    pub fn Invoke<P0, P1>(&self, sender: P0, e: P1) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
        P1: windows_core::Param<TextChangedEventArgs>,
    {
        unsafe {
            (windows_core::Interface::vtable(self).Invoke)(windows_core::Interface::as_raw(self), sender.param().abi(), e.param().abi())
                .ok()
        }
    }
}
#[repr(C)]
pub struct TextChangedEventHandler_Vtbl {
    base__: windows_core::IUnknown_Vtbl,
    Invoke: unsafe extern "system" fn(
        this: *mut core::ffi::c_void,
        sender: *mut core::ffi::c_void,
        e: *mut core::ffi::c_void,
    ) -> windows_core::HRESULT,
}
struct TextChangedEventHandlerBox<
    F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<TextChangedEventArgs>) -> windows_core::Result<()> + Send + 'static,
>(core::marker::PhantomData<(fn() -> F,)>);
impl<
    F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<TextChangedEventArgs>) -> windows_core::Result<()> + Send + 'static,
> TextChangedEventHandlerBox<F>
{
    const VTABLE: TextChangedEventHandler_Vtbl = TextChangedEventHandler_Vtbl {
        base__: windows_core::IUnknown_Vtbl {
            QueryInterface: windows_core::imp::DelegateBox::<TextChangedEventHandler, F>::QueryInterface,
            AddRef: windows_core::imp::DelegateBox::<TextChangedEventHandler, F>::AddRef,
            Release: windows_core::imp::DelegateBox::<TextChangedEventHandler, F>::Release,
        },
        Invoke: Self::Invoke,
    };

    unsafe extern "system" fn Invoke(
        this: *mut core::ffi::c_void,
        sender: *mut core::ffi::c_void,
        e: *mut core::ffi::c_void,
    ) -> windows_core::HRESULT {
        unsafe {
            let this = &mut *(this as *mut *mut core::ffi::c_void as *mut windows_core::imp::DelegateBox<TextChangedEventHandler, F>);
            (this.invoke)(core::mem::transmute_copy(&sender), core::mem::transmute_copy(&e)).into()
        }
    }
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TextTrimming(pub i32);
impl TextTrimming {
    pub const CharacterEllipsis: Self = Self(1);
    pub const Clip: Self = Self(3);
    pub const None: Self = Self(0);
    pub const WordEllipsis: Self = Self(2);
}
impl windows_core::imp::TypeKind for TextTrimming {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for TextTrimming {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.TextTrimming");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.TextTrimming;i4)");
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TextWrapping(pub i32);
impl TextWrapping {
    pub const NoWrap: Self = Self(1);
    pub const Wrap: Self = Self(2);
    pub const WrapWholeWords: Self = Self(3);
}
impl windows_core::imp::TypeKind for TextWrapping {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for TextWrapping {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.TextWrapping");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.TextWrapping;i4)");
}
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Thickness {
    pub Left: f64,
    pub Top: f64,
    pub Right: f64,
    pub Bottom: f64,
}
impl windows_core::imp::TypeKind for Thickness {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for Thickness {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Thickness");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"struct(Microsoft.UI.Xaml.Thickness;f8;f8;f8;f8)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TitleBar(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(TitleBar, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(TitleBar, Control, FrameworkElement, UIElement, DependencyObject);
impl TitleBar {
    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetTitle(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetTitle)(windows_core::Interface::as_raw(self), core::mem::transmute_copy(value)).ok()
        }
    }

    pub fn SetContent<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<UIElement>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetContent)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn SetRightHeader<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<UIElement>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetRightHeader)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn SetIsBackButtonVisible(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetIsBackButtonVisible)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn SetIsPaneToggleButtonVisible(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetIsPaneToggleButtonVisible)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn PaneToggleRequested<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<Self>, windows_core::Ref<windows_core::IInspectable>) + Send + 'static,
    {
        let handler = <TypedEventHandler<Self, windows_core::IInspectable>>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).PaneToggleRequested)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemovePaneToggleRequested))
        }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::ITitleBarFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::ITitleBarFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn ITitleBarFactory<R, F: FnOnce(&ITitleBarFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<TitleBar, ITitleBarFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn ITitleBarStatics<R, F: FnOnce(&ITitleBarStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<TitleBar, ITitleBarStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn ITitleBarStatics2<R, F: FnOnce(&ITitleBarStatics2) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<TitleBar, ITitleBarStatics2> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for TitleBar {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, ITitleBar>();
}
unsafe impl windows_core::Interface for TitleBar {
    type Vtable = <ITitleBar as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <ITitleBar as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for TitleBar {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.TitleBar";
}
unsafe impl Send for TitleBar {}
unsafe impl Sync for TitleBar {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TitleBarHeightOption(pub i32);
impl TitleBarHeightOption {
    pub const Collapsed: Self = Self(2);
    pub const Standard: Self = Self(0);
    pub const Tall: Self = Self(1);
}
impl windows_core::imp::TypeKind for TitleBarHeightOption {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for TitleBarHeightOption {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Windowing.TitleBarHeightOption");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Windowing.TitleBarHeightOption;i4)");
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TitleBarTheme(pub i32);
impl TitleBarTheme {
    pub const Dark: Self = Self(3);
    pub const Legacy: Self = Self(0);
    pub const Light: Self = Self(2);
    pub const UseDefaultAppMode: Self = Self(1);
}
impl windows_core::imp::TypeKind for TitleBarTheme {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for TitleBarTheme {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Windowing.TitleBarTheme");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Windowing.TitleBarTheme;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToggleButton(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(ToggleButton, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(ToggleButton, ButtonBase, ContentControl, Control, FrameworkElement, UIElement, DependencyObject);
impl ToggleButton {
    pub fn Click<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IButtonBase>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Click)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveClick))
        }
    }

    pub fn SetContent<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IContentControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetContent)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn IsChecked(&self) -> windows_core::Result<bool> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).IsChecked)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
                .and_then(|r__: windows_reference::IReference<bool>| r__.Value())
        }
    }

    pub fn SetIsChecked(&self, value: Option<bool>) -> windows_core::Result<()> {
        let value__ = value.map(<windows_reference::IReference<bool> as From<_>>::from);
        unsafe {
            (windows_core::Interface::vtable(self).SetIsChecked)(
                windows_core::Interface::as_raw(self),
                windows_core::Param::param(value__.as_ref()).abi(),
            )
            .ok()
        }
    }

    pub fn Checked<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).Checked)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemoveChecked))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IToggleButtonFactory<R, F: FnOnce(&IToggleButtonFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ToggleButton, IToggleButtonFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IToggleButtonStatics<R, F: FnOnce(&IToggleButtonStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ToggleButton, IToggleButtonStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for ToggleButton {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IToggleButton>();
}
unsafe impl windows_core::Interface for ToggleButton {
    type Vtable = <IToggleButton as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IToggleButton as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ToggleButton {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.Primitives.ToggleButton";
}
unsafe impl Send for ToggleButton {}
unsafe impl Sync for ToggleButton {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToggleSwitch(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(ToggleSwitch, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(ToggleSwitch, Control, FrameworkElement, UIElement, DependencyObject);
impl ToggleSwitch {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }

    fn IActivationFactory<R, F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ToggleSwitch, windows_core::imp::IGenericFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    pub fn SetIsEnabled(&self, value: bool) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetIsEnabled)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalContentAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalContentAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IControl>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalContentAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn IsOn(&self) -> windows_core::Result<bool> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).IsOn)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }

    pub fn SetIsOn(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetIsOn)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn Toggled<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).Toggled)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemoveToggled))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    fn IToggleSwitchStatics<R, F: FnOnce(&IToggleSwitchStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ToggleSwitch, IToggleSwitchStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for ToggleSwitch {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IToggleSwitch>();
}
unsafe impl windows_core::Interface for ToggleSwitch {
    type Vtable = <IToggleSwitch as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IToggleSwitch as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ToggleSwitch {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.ToggleSwitch";
}
unsafe impl Send for ToggleSwitch {}
unsafe impl Sync for ToggleSwitch {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ToolTipService(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(ToolTipService, windows_core::IUnknown, windows_core::IInspectable);
impl ToolTipService {
    pub fn SetPlacement<P0>(element: P0, value: PlacementMode) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyObject>,
    {
        Self::IToolTipServiceStatics(|this| unsafe {
            (windows_core::Interface::vtable(this).SetPlacement)(windows_core::Interface::as_raw(this), element.param().abi(), value).ok()
        })
    }

    pub fn SetToolTip<P0, P1>(element: P0, value: P1) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyObject>,
        P1: windows_core::Param<windows_core::IInspectable>,
    {
        Self::IToolTipServiceStatics(|this| unsafe {
            (windows_core::Interface::vtable(this).SetToolTip)(
                windows_core::Interface::as_raw(this),
                element.param().abi(),
                value.param().abi(),
            )
            .ok()
        })
    }

    fn IToolTipServiceStatics<R, F: FnOnce(&IToolTipServiceStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<ToolTipService, IToolTipServiceStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for ToolTipService {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IToolTipService>();
}
unsafe impl windows_core::Interface for ToolTipService {
    type Vtable = <IToolTipService as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IToolTipService as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for ToolTipService {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.ToolTipService";
}
unsafe impl Send for ToolTipService {}
unsafe impl Sync for ToolTipService {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TypedEventHandler<TSender, TResult>(
    windows_core::IUnknown,
    core::marker::PhantomData<TSender>,
    core::marker::PhantomData<TResult>,
)
where
    TSender: windows_core::RuntimeType + 'static,
    TResult: windows_core::RuntimeType + 'static;
unsafe impl<TSender: windows_core::RuntimeType + 'static, TResult: windows_core::RuntimeType + 'static> windows_core::Interface
    for TypedEventHandler<TSender, TResult>
{
    type Vtable = TypedEventHandler_Vtbl<TSender, TResult>;

    const IID: windows_core::GUID = windows_core::GUID::from_signature(<Self as windows_core::RuntimeType>::SIGNATURE);
}
impl<TSender: windows_core::RuntimeType + 'static, TResult: windows_core::RuntimeType + 'static> windows_core::RuntimeType
    for TypedEventHandler<TSender, TResult>
{
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::new()
        .push_slice(b"pinterface({9de1c534-6ae1-11e0-84e1-18a905bcc53f}")
        .push_slice(b";")
        .push_other(TSender::SIGNATURE)
        .push_slice(b";")
        .push_other(TResult::SIGNATURE)
        .push_slice(b")");
}
impl<TSender: windows_core::RuntimeType + 'static, TResult: windows_core::RuntimeType + 'static> TypedEventHandler<TSender, TResult> {
    pub fn new<F: Fn(windows_core::Ref<TSender>, windows_core::Ref<TResult>) -> windows_core::Result<()> + Send + 'static>(
        invoke: F,
    ) -> Self {
        let com = windows_core::imp::DelegateBox::<Self, F>::new(&TypedEventHandlerBox::<TSender, TResult, F>::VTABLE, invoke);
        unsafe { core::mem::transmute(windows_core::imp::box_new(com)) }
    }

    pub fn Invoke<P0, P1>(&self, sender: P0, args: P1) -> windows_core::Result<()>
    where
        P0: windows_core::Param<TSender>,
        P1: windows_core::Param<TResult>,
    {
        unsafe {
            (windows_core::Interface::vtable(self).Invoke)(windows_core::Interface::as_raw(self), sender.param().abi(), args.param().abi())
                .ok()
        }
    }
}
#[repr(C)]
pub struct TypedEventHandler_Vtbl<TSender, TResult>
where
    TSender: windows_core::RuntimeType + 'static,
    TResult: windows_core::RuntimeType + 'static,
{
    base__: windows_core::IUnknown_Vtbl,
    Invoke: unsafe extern "system" fn(
        this: *mut core::ffi::c_void,
        sender: windows_core::imp::AbiType<TSender>,
        args: windows_core::imp::AbiType<TResult>,
    ) -> windows_core::HRESULT,
    TSender: core::marker::PhantomData<TSender>,
    TResult: core::marker::PhantomData<TResult>,
}
struct TypedEventHandlerBox<
    TSender,
    TResult,
    F: Fn(windows_core::Ref<TSender>, windows_core::Ref<TResult>) -> windows_core::Result<()> + Send + 'static,
>(core::marker::PhantomData<(TSender, TResult, fn() -> F)>)
where
    TSender: windows_core::RuntimeType + 'static,
    TResult: windows_core::RuntimeType + 'static;
impl<
    TSender: windows_core::RuntimeType + 'static,
    TResult: windows_core::RuntimeType + 'static,
    F: Fn(windows_core::Ref<TSender>, windows_core::Ref<TResult>) -> windows_core::Result<()> + Send + 'static,
> TypedEventHandlerBox<TSender, TResult, F>
{
    const VTABLE: TypedEventHandler_Vtbl<TSender, TResult> = TypedEventHandler_Vtbl::<TSender, TResult> {
        base__: windows_core::IUnknown_Vtbl {
            QueryInterface: windows_core::imp::DelegateBox::<TypedEventHandler<TSender, TResult>, F>::QueryInterface,
            AddRef: windows_core::imp::DelegateBox::<TypedEventHandler<TSender, TResult>, F>::AddRef,
            Release: windows_core::imp::DelegateBox::<TypedEventHandler<TSender, TResult>, F>::Release,
        },
        Invoke: Self::Invoke,
        TSender: core::marker::PhantomData::<TSender>,
        TResult: core::marker::PhantomData::<TResult>,
    };

    unsafe extern "system" fn Invoke(
        this: *mut core::ffi::c_void,
        sender: windows_core::imp::AbiType<TSender>,
        args: windows_core::imp::AbiType<TResult>,
    ) -> windows_core::HRESULT {
        unsafe {
            let this =
                &mut *(this as *mut *mut core::ffi::c_void as *mut windows_core::imp::DelegateBox<TypedEventHandler<TSender, TResult>, F>);
            (this.invoke)(core::mem::transmute_copy(&sender), core::mem::transmute_copy(&args)).into()
        }
    }
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UIElement(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(UIElement, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(UIElement, DependencyObject);
impl UIElement {
    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetOpacity)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Visibility)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetVisibility)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).XamlRoot)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetXamlRoot)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).UpdateLayout)(windows_core::Interface::as_raw(self)).ok() }
    }

    fn IUIElementStatics<R, F: FnOnce(&IUIElementStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<UIElement, IUIElementStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for UIElement {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IUIElement>();
}
unsafe impl windows_core::Interface for UIElement {
    type Vtable = <IUIElement as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IUIElement as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for UIElement {
    const NAME: &'static str = "Microsoft.UI.Xaml.UIElement";
}
unsafe impl Send for UIElement {}
unsafe impl Sync for UIElement {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UIElementCollection(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(UIElementCollection, windows_core::IUnknown, windows_core::IInspectable, IVector<UIElement>);
windows_core::imp::required_hierarchy!(UIElementCollection, IIterable<UIElement>);
impl UIElementCollection {
    pub fn First(&self) -> windows_core::Result<windows_collections::IIterator<UIElement>> {
        let this = &windows_core::Interface::cast::<IIterable<UIElement>>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).First)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn GetAt(&self, index: u32) -> windows_core::Result<UIElement> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).GetAt)(windows_core::Interface::as_raw(self), index, &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn Size(&self) -> windows_core::Result<u32> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Size)(windows_core::Interface::as_raw(self), &mut result__).map(|| result__)
        }
    }

    pub fn IndexOf<P0>(&self, value: P0, index: &mut u32) -> windows_core::Result<bool>
    where
        P0: windows_core::Param<UIElement>,
    {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).IndexOf)(
                windows_core::Interface::as_raw(self),
                value.param().abi(),
                index,
                &mut result__,
            )
            .map(|| result__)
        }
    }

    pub fn InsertAt<P1>(&self, index: u32, value: P1) -> windows_core::Result<()>
    where
        P1: windows_core::Param<UIElement>,
    {
        unsafe { (windows_core::Interface::vtable(self).InsertAt)(windows_core::Interface::as_raw(self), index, value.param().abi()).ok() }
    }

    pub fn RemoveAt(&self, index: u32) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).RemoveAt)(windows_core::Interface::as_raw(self), index).ok() }
    }

    pub fn Append<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<UIElement>,
    {
        unsafe { (windows_core::Interface::vtable(self).Append)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn Clear(&self) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).Clear)(windows_core::Interface::as_raw(self)).ok() }
    }
}
impl windows_core::RuntimeType for UIElementCollection {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IVector<UIElement>>();
}
unsafe impl windows_core::Interface for UIElementCollection {
    type Vtable = <IVector<UIElement> as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IVector<UIElement> as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for UIElementCollection {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.UIElementCollection";
}
unsafe impl Send for UIElementCollection {}
unsafe impl Sync for UIElementCollection {}
impl IntoIterator for UIElementCollection {
    type IntoIter = windows_collections::BufferedIterator<Self::Item>;
    type Item = UIElement;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(&self)
    }
}
impl IntoIterator for &UIElementCollection {
    type IntoIter = windows_collections::BufferedIterator<Self::Item>;
    type Item = UIElement;

    fn into_iter(self) -> Self::IntoIter {
        windows_collections::BufferedIterator::new(self.First().unwrap())
    }
}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct VerticalAlignment(pub i32);
impl VerticalAlignment {
    pub const Bottom: Self = Self(2);
    pub const Center: Self = Self(1);
    pub const Stretch: Self = Self(3);
    pub const Top: Self = Self(0);
}
impl windows_core::imp::TypeKind for VerticalAlignment {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for VerticalAlignment {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.VerticalAlignment");
    const SIGNATURE: windows_core::imp::ConstBuffer =
        windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.VerticalAlignment;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Viewbox(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(Viewbox, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(Viewbox, FrameworkElement, UIElement, DependencyObject);
impl Viewbox {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }

    fn IActivationFactory<R, F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Viewbox, windows_core::imp::IGenericFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn Tag(&self) -> windows_core::Result<windows_core::IInspectable> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Tag)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetTag<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetTag)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn ActualWidth(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualWidth)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn ActualHeight(&self) -> windows_core::Result<f64> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).ActualHeight)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxWidth(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxWidth)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMinHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMinHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMaxHeight(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMaxHeight)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetHorizontalAlignment(&self, value: HorizontalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetHorizontalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetVerticalAlignment(&self, value: VerticalAlignment) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVerticalAlignment)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetMargin(&self, value: Thickness) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetMargin)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn SetStyle<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<Style>,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetStyle)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn SetRequestedTheme(&self, value: ElementTheme) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetRequestedTheme)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Loaded<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<RoutedEventArgs>) + Send + 'static,
    {
        let this = &windows_core::Interface::cast::<IFrameworkElement>(self)?;
        let handler = <RoutedEventHandler>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(this).Loaded)(
                windows_core::Interface::as_raw(this),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(this.clone(), token__, windows_core::Interface::vtable(this).RemoveLoaded))
        }
    }

    pub fn SetOpacity(&self, value: f64) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetOpacity)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn Visibility(&self) -> windows_core::Result<Visibility> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Visibility)(windows_core::Interface::as_raw(this), &mut result__).map(|| result__)
        }
    }

    pub fn SetVisibility(&self, value: Visibility) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetVisibility)(windows_core::Interface::as_raw(this), value).ok() }
    }

    pub fn XamlRoot(&self) -> windows_core::Result<XamlRoot> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).XamlRoot)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetXamlRoot<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<XamlRoot>,
    {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).SetXamlRoot)(windows_core::Interface::as_raw(this), value.param().abi()).ok() }
    }

    pub fn UpdateLayout(&self) -> windows_core::Result<()> {
        let this = &windows_core::Interface::cast::<IUIElement>(self)?;
        unsafe { (windows_core::Interface::vtable(this).UpdateLayout)(windows_core::Interface::as_raw(this)).ok() }
    }

    pub fn SetChild<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<UIElement>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetChild)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn SetStretch(&self, value: Stretch) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetStretch)(windows_core::Interface::as_raw(self), value).ok() }
    }

    fn IViewboxStatics<R, F: FnOnce(&IViewboxStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Viewbox, IViewboxStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for Viewbox {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IViewbox>();
}
unsafe impl windows_core::Interface for Viewbox {
    type Vtable = <IViewbox as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IViewbox as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for Viewbox {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.Viewbox";
}
unsafe impl Send for Viewbox {}
unsafe impl Sync for Viewbox {}
#[repr(transparent)]
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct Visibility(pub i32);
impl Visibility {
    pub const Collapsed: Self = Self(1);
    pub const Visible: Self = Self(0);
}
impl windows_core::imp::TypeKind for Visibility {
    type TypeKind = windows_core::imp::CopyType;
}
impl windows_core::RuntimeType for Visibility {
    const NAME: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"Microsoft.UI.Xaml.Visibility");
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::from_slice(b"enum(Microsoft.UI.Xaml.Visibility;i4)");
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Window(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(Window, windows_core::IUnknown, windows_core::IInspectable);
impl Window {
    pub fn Content(&self) -> windows_core::Result<UIElement> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).Content)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn SetContent<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<UIElement>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetContent)(windows_core::Interface::as_raw(self), value.param().abi()).ok() }
    }

    pub fn SetTitle(&self, value: &windows_core::HSTRING) -> windows_core::Result<()> {
        unsafe {
            (windows_core::Interface::vtable(self).SetTitle)(windows_core::Interface::as_raw(self), core::mem::transmute_copy(value)).ok()
        }
    }

    pub fn SetExtendsContentIntoTitleBar(&self, value: bool) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).SetExtendsContentIntoTitleBar)(windows_core::Interface::as_raw(self), value).ok() }
    }

    pub fn Closed<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<windows_core::IInspectable>, windows_core::Ref<WindowEventArgs>) + Send + 'static,
    {
        let handler = <TypedEventHandler<windows_core::IInspectable, WindowEventArgs>>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).Closed)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemoveClosed))
        }
    }

    pub fn Activate(&self) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).Activate)(windows_core::Interface::as_raw(self)).ok() }
    }

    pub fn Close(&self) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).Close)(windows_core::Interface::as_raw(self)).ok() }
    }

    pub fn SetTitleBar<P0>(&self, titlebar: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<UIElement>,
    {
        unsafe { (windows_core::Interface::vtable(self).SetTitleBar)(windows_core::Interface::as_raw(self), titlebar.param().abi()).ok() }
    }

    pub fn SetSystemBackdrop<P0>(&self, value: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<SystemBackdrop>,
    {
        let this = &windows_core::Interface::cast::<IWindow2>(self)?;
        unsafe {
            (windows_core::Interface::vtable(this).SetSystemBackdrop)(windows_core::Interface::as_raw(this), value.param().abi()).ok()
        }
    }

    pub fn AppWindow(&self) -> windows_core::Result<AppWindow> {
        let this = &windows_core::Interface::cast::<IWindow2>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).AppWindow)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn new() -> windows_core::Result<Self> {
        Self::IWindowFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    pub fn compose<T>(compose: T) -> windows_core::Result<Self>
    where
        T: windows_core::Compose,
    {
        Self::IWindowFactory(|this| unsafe {
            let (derived__, base__) = windows_core::Compose::compose(compose);
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstance)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(&derived__),
                base__ as *mut _ as _,
                &mut result__,
            )
            .ok()?;
            let _ = &derived__;
            windows_core::imp::Type::from_abi(result__)
        })
    }

    fn IWindowFactory<R, F: FnOnce(&IWindowFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Window, IWindowFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    fn IWindowStatics<R, F: FnOnce(&IWindowStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<Window, IWindowStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for Window {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IWindow>();
}
unsafe impl windows_core::Interface for Window {
    type Vtable = <IWindow as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IWindow as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for Window {
    const NAME: &'static str = "Microsoft.UI.Xaml.Window";
}
unsafe impl Send for Window {}
unsafe impl Sync for Window {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WindowEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(WindowEventArgs, windows_core::IUnknown, windows_core::IInspectable);
impl windows_core::RuntimeType for WindowEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IWindowEventArgs>();
}
unsafe impl windows_core::Interface for WindowEventArgs {
    type Vtable = <IWindowEventArgs as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IWindowEventArgs as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for WindowEventArgs {
    const NAME: &'static str = "Microsoft.UI.Xaml.WindowEventArgs";
}
unsafe impl Send for WindowEventArgs {}
unsafe impl Sync for WindowEventArgs {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WriteableBitmap(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(WriteableBitmap, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(WriteableBitmap, BitmapSource, ImageSource, DependencyObject);
impl WriteableBitmap {
    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn PixelBuffer(&self) -> windows_core::Result<IBuffer> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).PixelBuffer)(windows_core::Interface::as_raw(self), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn Invalidate(&self) -> windows_core::Result<()> {
        unsafe { (windows_core::Interface::vtable(self).Invalidate)(windows_core::Interface::as_raw(self)).ok() }
    }

    pub fn CreateInstanceWithDimensions(pixelwidth: i32, pixelheight: i32) -> windows_core::Result<Self> {
        Self::IWriteableBitmapFactory(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).CreateInstanceWithDimensions)(
                windows_core::Interface::as_raw(this),
                pixelwidth,
                pixelheight,
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    fn IWriteableBitmapFactory<R, F: FnOnce(&IWriteableBitmapFactory) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<WriteableBitmap, IWriteableBitmapFactory> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for WriteableBitmap {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IWriteableBitmap>();
}
unsafe impl windows_core::Interface for WriteableBitmap {
    type Vtable = <IWriteableBitmap as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IWriteableBitmap as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for WriteableBitmap {
    const NAME: &'static str = "Microsoft.UI.Xaml.Media.Imaging.WriteableBitmap";
}
unsafe impl Send for WriteableBitmap {}
unsafe impl Sync for WriteableBitmap {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XamlControlsResources(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(XamlControlsResources, windows_core::IUnknown, windows_core::IInspectable);
windows_core::imp::required_hierarchy!(XamlControlsResources, IIterable < windows_collections::IKeyValuePair < windows_core::IInspectable, windows_core::IInspectable > >, IMap < windows_core::IInspectable, windows_core::IInspectable >, ResourceDictionary, DependencyObject);
impl XamlControlsResources {
    pub fn new() -> windows_core::Result<Self> {
        Self::IActivationFactory(|f| f.ActivateInstance::<Self>())
    }

    fn IActivationFactory<R, F: FnOnce(&windows_core::imp::IGenericFactory) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<XamlControlsResources, windows_core::imp::IGenericFactory> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }

    pub fn ClearValue<P0>(&self, dp: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<DependencyProperty>,
    {
        let this = &windows_core::Interface::cast::<IDependencyObject>(self)?;
        unsafe { (windows_core::Interface::vtable(this).ClearValue)(windows_core::Interface::as_raw(this), dp.param().abi()).ok() }
    }

    pub fn First(
        &self,
    ) -> windows_core::Result<
        windows_collections::IIterator<windows_collections::IKeyValuePair<windows_core::IInspectable, windows_core::IInspectable>>,
    > {
        let this = &windows_core::Interface::cast::<
            IIterable<windows_collections::IKeyValuePair<windows_core::IInspectable, windows_core::IInspectable>>,
        >(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).First)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn Lookup<P0>(&self, key: P0) -> windows_core::Result<windows_core::IInspectable>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IMap<windows_core::IInspectable, windows_core::IInspectable>>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Lookup)(windows_core::Interface::as_raw(this), key.param().abi(), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    pub fn HasKey<P0>(&self, key: P0) -> windows_core::Result<bool>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IMap<windows_core::IInspectable, windows_core::IInspectable>>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).HasKey)(windows_core::Interface::as_raw(this), key.param().abi(), &mut result__)
                .map(|| result__)
        }
    }

    pub fn Insert<P0, P1>(&self, key: P0, value: P1) -> windows_core::Result<bool>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
        P1: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IMap<windows_core::IInspectable, windows_core::IInspectable>>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Insert)(
                windows_core::Interface::as_raw(this),
                key.param().abi(),
                value.param().abi(),
                &mut result__,
            )
            .map(|| result__)
        }
    }

    pub fn Remove<P0>(&self, key: P0) -> windows_core::Result<()>
    where
        P0: windows_core::Param<windows_core::IInspectable>,
    {
        let this = &windows_core::Interface::cast::<IMap<windows_core::IInspectable, windows_core::IInspectable>>(self)?;
        unsafe { (windows_core::Interface::vtable(this).Remove)(windows_core::Interface::as_raw(this), key.param().abi()).ok() }
    }

    pub fn MergedDictionaries(&self) -> windows_core::Result<IVector<ResourceDictionary>> {
        let this = &windows_core::Interface::cast::<IResourceDictionary>(self)?;
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).MergedDictionaries)(windows_core::Interface::as_raw(this), &mut result__)
                .and_then(|| windows_core::imp::Type::from_abi(result__))
        }
    }

    fn IXamlControlsResourcesStatics<R, F: FnOnce(&IXamlControlsResourcesStatics) -> windows_core::Result<R>>(
        callback: F,
    ) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<XamlControlsResources, IXamlControlsResourcesStatics> =
            windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for XamlControlsResources {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IXamlControlsResources>();
}
unsafe impl windows_core::Interface for XamlControlsResources {
    type Vtable = <IXamlControlsResources as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IXamlControlsResources as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for XamlControlsResources {
    const NAME: &'static str = "Microsoft.UI.Xaml.Controls.XamlControlsResources";
}
unsafe impl Send for XamlControlsResources {}
unsafe impl Sync for XamlControlsResources {}
impl IntoIterator for XamlControlsResources {
    type IntoIter = windows_collections::BufferedIterator<Self::Item>;
    type Item = windows_collections::IKeyValuePair<windows_core::IInspectable, windows_core::IInspectable>;

    fn into_iter(self) -> Self::IntoIter {
        IntoIterator::into_iter(&self)
    }
}
impl IntoIterator for &XamlControlsResources {
    type IntoIter = windows_collections::BufferedIterator<Self::Item>;
    type Item = windows_collections::IKeyValuePair<windows_core::IInspectable, windows_core::IInspectable>;

    fn into_iter(self) -> Self::IntoIter {
        windows_collections::BufferedIterator::new(self.First().unwrap())
    }
}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XamlReader(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(XamlReader, windows_core::IUnknown, windows_core::IInspectable);
impl XamlReader {
    pub fn Load(xaml: &windows_core::HSTRING) -> windows_core::Result<windows_core::IInspectable> {
        Self::IXamlReaderStatics(|this| unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(this).Load)(
                windows_core::Interface::as_raw(this),
                core::mem::transmute_copy(xaml),
                &mut result__,
            )
            .and_then(|| windows_core::imp::Type::from_abi(result__))
        })
    }

    fn IXamlReaderStatics<R, F: FnOnce(&IXamlReaderStatics) -> windows_core::Result<R>>(callback: F) -> windows_core::Result<R> {
        static SHARED: windows_core::imp::FactoryCache<XamlReader, IXamlReaderStatics> = windows_core::imp::FactoryCache::new();
        SHARED.call(callback)
    }
}
impl windows_core::RuntimeType for XamlReader {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IXamlReader>();
}
unsafe impl windows_core::Interface for XamlReader {
    type Vtable = <IXamlReader as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IXamlReader as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for XamlReader {
    const NAME: &'static str = "Microsoft.UI.Xaml.Markup.XamlReader";
}
unsafe impl Send for XamlReader {}
unsafe impl Sync for XamlReader {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XamlRoot(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(XamlRoot, windows_core::IUnknown, windows_core::IInspectable);
impl XamlRoot {
    pub fn RasterizationScale(&self) -> windows_core::Result<f64> {
        unsafe {
            let mut result__ = core::mem::zeroed();
            (windows_core::Interface::vtable(self).RasterizationScale)(windows_core::Interface::as_raw(self), &mut result__)
                .map(|| result__)
        }
    }

    pub fn Changed<F>(&self, handler: F) -> windows_core::Result<windows_core::EventRevoker>
    where
        F: Fn(windows_core::Ref<Self>, windows_core::Ref<XamlRootChangedEventArgs>) + Send + 'static,
    {
        let handler = <TypedEventHandler<Self, XamlRootChangedEventArgs>>::new(move |a0, a1| {
            handler(a0, a1);
            Ok(())
        });
        unsafe {
            let mut result__ = core::mem::zeroed();
            let token__ = (windows_core::Interface::vtable(self).Changed)(
                windows_core::Interface::as_raw(self),
                windows_core::Interface::as_raw(&handler),
                &mut result__,
            )
            .map(|| result__)?;
            Ok(windows_core::EventRevoker::new(self.clone(), token__, windows_core::Interface::vtable(self).RemoveChanged))
        }
    }
}
impl windows_core::RuntimeType for XamlRoot {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IXamlRoot>();
}
unsafe impl windows_core::Interface for XamlRoot {
    type Vtable = <IXamlRoot as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IXamlRoot as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for XamlRoot {
    const NAME: &'static str = "Microsoft.UI.Xaml.XamlRoot";
}
unsafe impl Send for XamlRoot {}
unsafe impl Sync for XamlRoot {}
#[repr(transparent)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct XamlRootChangedEventArgs(windows_core::IUnknown);
windows_core::imp::interface_hierarchy!(XamlRootChangedEventArgs, windows_core::IUnknown, windows_core::IInspectable);
impl windows_core::RuntimeType for XamlRootChangedEventArgs {
    const SIGNATURE: windows_core::imp::ConstBuffer = windows_core::imp::ConstBuffer::for_class::<Self, IXamlRootChangedEventArgs>();
}
unsafe impl windows_core::Interface for XamlRootChangedEventArgs {
    type Vtable = <IXamlRootChangedEventArgs as windows_core::Interface>::Vtable;

    const IID: windows_core::GUID = <IXamlRootChangedEventArgs as windows_core::Interface>::IID;
}
impl windows_core::RuntimeName for XamlRootChangedEventArgs {
    const NAME: &'static str = "Microsoft.UI.Xaml.XamlRootChangedEventArgs";
}
unsafe impl Send for XamlRootChangedEventArgs {}
unsafe impl Sync for XamlRootChangedEventArgs {}

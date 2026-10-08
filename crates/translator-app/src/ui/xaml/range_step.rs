//! The WinUI Slider keyboard step uses `SmallChange`, which defaults to 1, not `StepFrequency`.
//! windows-reactor 0.100 does not expose it, so patch the native RangeBase and NumberBox over COM.
//! [`crate::ui::xaml::apply`] runs [`sync`] again after focus changes and for a few frames after each view rebuild.

#![allow(non_snake_case)]

use std::ffi::c_void;

use windows_core::{HRESULT, IInspectable, IInspectable_Vtbl, Interface, Result};

use crate::ui::xaml::{IVisualTreeHelperStatics, visual_tree};

windows_core::imp::define_interface!(IRangeBase, IRangeBase_Vtbl, 0x540d6d61_8fac_5d5c_b5b0_e172a7dde103);
#[repr(C)]
pub struct IRangeBase_Vtbl {
    base__: IInspectable_Vtbl,
    _bounds: [usize; 4],
    _small_change: usize,
    SetSmallChange: unsafe extern "system" fn(*mut c_void, f64) -> HRESULT,
    _large_change: [usize; 2],
}

impl IRangeBase {
    fn SetSmallChange(&self, value: f64) -> Result<()> {
        unsafe {
            // SAFETY: `self` is a live `IRangeBase`; `value` is a finite step.
            (Interface::vtable(self).SetSmallChange)(Interface::as_raw(self), value).ok()
        }
    }
}

windows_core::imp::define_interface!(ISlider, ISlider_Vtbl, 0xf7418ecf_7c35_5216_8bf1_d82d47cce5df);
#[repr(C)]
pub struct ISlider_Vtbl {
    base__: IInspectable_Vtbl,
    _intermediate: usize,
    _set_intermediate: usize,
    StepFrequency: unsafe extern "system" fn(*mut c_void, *mut f64) -> HRESULT,
}

impl ISlider {
    fn StepFrequency(&self) -> Result<f64> {
        let mut value = 0.0;
        unsafe {
            // SAFETY: `value` is a stack f64; `self` is a live `ISlider`.
            (Interface::vtable(self).StepFrequency)(Interface::as_raw(self), &mut value).map(|| value)
        }
    }
}

windows_core::imp::define_interface!(INumberBox, INumberBox_Vtbl, 0xc18eb0e9_29fb_525d_abbc_d6b2110f542e);
#[repr(C)]
pub struct INumberBox_Vtbl {
    base__: IInspectable_Vtbl,
    _before_small_change: [usize; 7],
    SetSmallChange: unsafe extern "system" fn(*mut c_void, f64) -> HRESULT,
    _large_change: [usize; 2],
}

impl INumberBox {
    fn SetSmallChange(&self, value: f64) -> Result<()> {
        unsafe {
            // SAFETY: `self` is a live `INumberBox`; `value` matches the sibling slider step.
            (Interface::vtable(self).SetSmallChange)(Interface::as_raw(self), value).ok()
        }
    }
}

fn patch_slider(statics: &IVisualTreeHelperStatics, node: &IInspectable) -> Result<()> {
    let slider: ISlider = node.cast()?;
    let step = slider.StepFrequency()?;
    if !(step.is_finite() && step > 0.0) {
        return Ok(());
    }
    node.cast::<IRangeBase>()?.SetSmallChange(step)?;
    let parent = statics.GetParent(node)?;
    let n = statics.GetChildrenCount(&parent)?;
    for i in 0..n {
        if let Ok(child) = statics.GetChild(&parent, i)
            && let Ok(nb) = child.cast::<INumberBox>()
        {
            let _ = nb.SetSmallChange(step);
        }
    }
    Ok(())
}

pub(crate) fn sync(root: &IInspectable) {
    let Ok(statics) = visual_tree() else {
        return;
    };
    let mut stack = vec![root.clone()];
    while let Some(node) = stack.pop() {
        if node.cast::<ISlider>().is_ok() {
            let _ = patch_slider(&statics, &node);
        }
        let Ok(n) = statics.GetChildrenCount(&node) else {
            continue;
        };
        for i in 0..n {
            if let Ok(child) = statics.GetChild(&node, i) {
                stack.push(child);
            }
        }
    }
}

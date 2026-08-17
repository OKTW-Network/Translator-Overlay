//! Overlay thread IPC: commands in, events out.

use translator_core::{NormRect, OverlayConfig, TranslatedBlock};

pub enum OverlayCommand {
    Attach {
        target_hwnd: isize,
    },
    Detach,
    SetBlocks {
        blocks: Vec<TranslatedBlock>,
        content_width: u32,
        content_height: u32,
    },
    Clear,
    UpdateConfig(OverlayConfig),
    BeginRegionSelect {
        regions: Vec<NormRect>,
    },
    CancelRegionSelect,
    ConfirmRegionSelect,
    ClearRegionSelect,
    Shutdown,
}

/// Overlay thread → pipeline (picker results).
#[derive(Debug, Clone)]
pub enum OverlayEvent {
    RegionsCommitted(Vec<NormRect>),
    RegionSelectCancelled,
    RegionSelectUpdated(Vec<NormRect>),
}

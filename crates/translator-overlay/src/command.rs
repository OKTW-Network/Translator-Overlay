//! Overlay thread IPC: commands in, events out.

use translator_core::{NormRect, OverlayConfig, TranslatedBlock};

/// Icon on the HUD primary button (start / resume vs pause).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudPrimary {
    Play,
    Pause,
}

/// Localized snapshot pushed to the always-on-top control bar.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudSnapshot {
    pub label: String,
    pub primary: HudPrimary,
    pub primary_enabled: bool,
    pub stop_enabled: bool,
}

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
    /// Hide or restore in-place captions without dropping stored blocks.
    SetCaptionsVisible(bool),
    SetHud(HudSnapshot),
    /// Localized text the translation window shows while it has nothing to show.
    SetReaderPlaceholder(String),
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
    HudPrimary,
    HudStop,
}

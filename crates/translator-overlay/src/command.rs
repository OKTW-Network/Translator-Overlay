//! Overlay thread IPC: commands in, events out.

use translator_core::{NormRect, OverlayConfig, TranslatedBlock};

pub enum OverlayCommand {
    Attach {
        target_hwnd: isize,
    },
    Detach,
    SetBlocks {
        blocks: Vec<TranslatedBlock>,
        /// BCP-47 tag of the translations (target language): picks the CJK font and
        /// vertical punctuation for column captions.
        locale: String,
        content_width: u32,
        content_height: u32,
    },
    Clear,
    UpdateConfig(OverlayConfig),
    /// Hide or restore in-place captions without dropping stored blocks.
    SetCaptionsVisible(bool),
    /// Pipeline heartbeat: re-assert the reader's always-on-top.
    RestoreTopmost,
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
}

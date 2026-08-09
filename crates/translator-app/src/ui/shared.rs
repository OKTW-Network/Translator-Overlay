//! Shared UI state, snapshot, and config draft helpers.

use std::sync::{Arc, Mutex};

use translator_capture::{WindowInfo, list_windows};
use translator_core::{AppConfig, ModelTier};

use crate::pipeline::{CmdTx, SharedState};

/// Pending destructive settings action awaiting ContentDialog confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfirmAction {
    #[default]
    None,
    Reload,
    Discard,
}

pub struct UiShared {
    pub state: SharedState,
    pub cmd_tx: CmdTx,
    pub windows: Vec<WindowInfo>,
    pub selected_idx: usize,
    /// Editable draft of settings (committed on Save).
    pub draft: AppConfig,
    pub settings_dirty: bool,
    /// Optional API numbers (kept while toggle is off so re-enable restores).
    pub temp_val: f64,
    pub top_p_val: f64,
    pub max_tokens_val: f64,
    pub reasoning_str: String,
    pub temp_enabled: bool,
    pub top_p_enabled: bool,
    pub max_tokens_enabled: bool,
    pub reasoning_enabled: bool,
    pub text_argb_str: String,
    pub bg_argb_str: String,
    /// ColorPicker popup open (text / background). Only one should be true.
    pub text_color_picker_open: bool,
    pub bg_color_picker_open: bool,
    /// Show API key as plain text (PasswordRevealMode::Visible).
    pub api_key_revealed: bool,
    /// Pending Reload / Discard confirmation dialog.
    pub confirm: ConfirmAction,
    /// Teaching tip for optional API params (shown once per session).
    pub optional_tip_seen: bool,
    /// Inline form validation message (blocks Save until fixed).
    pub form_error: Option<String>,
    /// OCR settings expanders (session UI chrome, not persisted).
    pub expand_line_merge: bool,
    pub expand_line_merge_adv: bool,
}

pub fn make_shared() -> Arc<Mutex<UiShared>> {
    let (state, cmd_tx) = crate::APP_HANDLES.get().expect("APP_HANDLES must be set before UI starts").clone();
    let draft = state.read().config.clone();
    let optional = optional_api_state(&draft);
    let (text_argb_str, bg_argb_str) = overlay_color_strings(&draft);
    Arc::new(Mutex::new(UiShared {
        state,
        cmd_tx,
        windows: list_windows().unwrap_or_default(),
        selected_idx: 0,
        draft,
        settings_dirty: false,
        temp_val: optional.temp_val,
        top_p_val: optional.top_p_val,
        max_tokens_val: optional.max_tokens_val,
        reasoning_str: optional.reasoning_str,
        temp_enabled: optional.temp_enabled,
        top_p_enabled: optional.top_p_enabled,
        max_tokens_enabled: optional.max_tokens_enabled,
        reasoning_enabled: optional.reasoning_enabled,
        text_argb_str,
        bg_argb_str,
        text_color_picker_open: false,
        bg_color_picker_open: false,
        api_key_revealed: false,
        confirm: ConfirmAction::None,
        optional_tip_seen: false,
        expand_line_merge: false,
        expand_line_merge_adv: false,
        form_error: None,
    }))
}

struct OptionalApiState {
    temp_val: f64,
    top_p_val: f64,
    max_tokens_val: f64,
    reasoning_str: String,
    temp_enabled: bool,
    top_p_enabled: bool,
    max_tokens_enabled: bool,
    reasoning_enabled: bool,
}

fn optional_api_state(cfg: &AppConfig) -> OptionalApiState {
    OptionalApiState {
        temp_enabled: cfg.api.temperature.is_some(),
        top_p_enabled: cfg.api.top_p.is_some(),
        max_tokens_enabled: cfg.api.max_tokens.is_some(),
        reasoning_enabled: cfg.api.reasoning_effort.as_ref().is_some_and(|s| !s.trim().is_empty()),
        // Always keep a valid number (toggle off = omit on save, not empty field).
        temp_val: f64::from(cfg.api.temperature.unwrap_or(0.7)),
        top_p_val: f64::from(cfg.api.top_p.unwrap_or(0.9)),
        max_tokens_val: f64::from(cfg.api.max_tokens.unwrap_or(2048)),
        reasoning_str: cfg.api.reasoning_effort.clone().unwrap_or_else(|| "medium".into()),
    }
}

pub fn overlay_color_strings(cfg: &AppConfig) -> (String, String) {
    (format!("{:08X}", cfg.overlay.text_color_argb), format!("{:08X}", cfg.overlay.background_color_argb))
}

pub fn reload_draft_from_state(ui: &mut UiShared) {
    ui.draft = ui.state.read().config.clone();
    apply_optional_from_config(ui);
    let (ta, ba) = overlay_color_strings(&ui.draft);
    ui.text_argb_str = ta;
    ui.bg_argb_str = ba;
    ui.settings_dirty = false;
}

pub fn apply_optional_from_config(ui: &mut UiShared) {
    let o = optional_api_state(&ui.draft);
    ui.temp_val = o.temp_val;
    ui.top_p_val = o.top_p_val;
    ui.max_tokens_val = o.max_tokens_val;
    ui.reasoning_str = o.reasoning_str;
    ui.temp_enabled = o.temp_enabled;
    ui.top_p_enabled = o.top_p_enabled;
    ui.max_tokens_enabled = o.max_tokens_enabled;
    ui.reasoning_enabled = o.reasoning_enabled;
}

/// Parse ARGB hex for settings fields (delegates to core).
pub fn parse_hex_u32(s: &str) -> Option<u32> {
    translator_core::parse_argb_hex(s)
}

/// Merge optional / overlay free-form fields into a config snapshot (pure).
pub fn effective_draft(ui: &UiShared) -> AppConfig {
    let mut cfg = ui.draft.clone();
    cfg.api.temperature = if ui.temp_enabled {
        Some(ui.temp_val.clamp(0.0, 2.0) as f32)
    } else {
        None
    };
    cfg.api.top_p = if ui.top_p_enabled {
        Some(ui.top_p_val.clamp(0.0, 1.0) as f32)
    } else {
        None
    };
    cfg.api.max_tokens = if ui.max_tokens_enabled {
        Some(ui.max_tokens_val.round().clamp(1.0, 1_000_000.0) as u32)
    } else {
        None
    };
    cfg.api.reasoning_effort = if ui.reasoning_enabled {
        let r = ui.reasoning_str.trim();
        if r.is_empty() { None } else { Some(r.to_string()) }
    } else {
        None
    };
    if let Some(v) = parse_hex_u32(&ui.text_argb_str) {
        cfg.overlay.text_color_argb = v;
    }
    if let Some(v) = parse_hex_u32(&ui.bg_argb_str) {
        cfg.overlay.background_color_argb = v;
    }
    cfg
}

/// Write free-text fields into `ui.draft` before ApplyConfig / Save.
pub fn commit_optional_fields(ui: &mut UiShared) {
    ui.draft = effective_draft(ui);
}

/// True when the form (draft + free-text fields) differs from `live`.
///
/// Prefer this when the caller already holds `state.read()` — nested
/// `is_settings_dirty` → `state.read()` deadlocks under parking_lot's fair
/// policy once a writer (pipeline) is waiting.
pub fn draft_differs_from(ui: &UiShared, live: &AppConfig) -> bool {
    effective_draft(ui) != *live
}

/// True only when the form actually differs from the running config.
///
/// Do not trust a sticky dirty flag: Slider/NumberBox/TextBox often fire
/// change events when re-bound on the 250ms UI tick, which would mark dirty
/// even when nothing changed.
///
/// Acquires `state` once; safe to call without an existing state lock.
pub fn is_settings_dirty(ui: &UiShared) -> bool {
    let live = ui.state.read().config.clone();
    draft_differs_from(ui, &live)
}

/// Call after a real user edit. Clears the success banner so it does not stack.
pub fn mark_dirty(ui: &mut UiShared) {
    ui.settings_dirty = true;
    ui.form_error = None;
    ui.state.write().settings_message = None;
}

/// Soft validation issues that should block Save.
pub fn form_validation_error(ui: &UiShared) -> Option<String> {
    if ui.draft.api.model.trim().is_empty() {
        return Some("Model name is required.".into());
    }
    if ui.draft.api.base_url.trim().is_empty() {
        return Some("Base URL is required.".into());
    }
    // Optional numbers always have a value; toggle off = omit. No empty checks.
    if ui.reasoning_enabled && ui.reasoning_str.trim().is_empty() {
        return Some("Reasoning effort is on but empty.".into());
    }
    let text = ui.text_argb_str.trim();
    if !text.is_empty() && parse_hex_u32(text).is_none() {
        return Some("Text color must be 8-digit ARGB hex (e.g. FFFFFFFF).".into());
    }
    let bg = ui.bg_argb_str.trim();
    if !bg.is_empty() && parse_hex_u32(bg).is_none() {
        return Some("Background color must be 8-digit ARGB hex (e.g. C8000000).".into());
    }
    None
}

pub fn do_reload_from_disk(ui: &mut UiShared) {
    match AppConfig::load_or_create_default() {
        Ok(cfg) => {
            let _ = ui.cmd_tx.send(crate::pipeline::PipelineCommand::ApplyConfig(Box::new(cfg)));
            if let Ok(c) = AppConfig::load_or_create_default() {
                ui.draft = c;
                apply_optional_from_config(ui);
                let (ta, ba) = overlay_color_strings(&ui.draft);
                ui.text_argb_str = ta;
                ui.bg_argb_str = ba;
                ui.settings_dirty = false;
                ui.form_error = None;
                ui.confirm = ConfirmAction::None;
            }
        }
        Err(e) => {
            ui.state.write().set_error(format!("Could not reload config: {e}"));
            ui.confirm = ConfirmAction::None;
        }
    }
}

pub fn do_discard(ui: &mut UiShared) {
    reload_draft_from_state(ui);
    ui.form_error = None;
    ui.confirm = ConfirmAction::None;
}

pub fn argb_u32_to_parts(v: u32) -> (u8, u8, u8, u8) {
    let (a, r, g, b) = translator_overlay::argb_channels(v);
    (a, r, g, b)
}

pub fn parts_to_argb_u32(a: u8, r: u8, g: u8, b: u8) -> u32 {
    (u32::from(a) << 24) | (u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)
}

pub fn truncate(s: &str, max: usize) -> String {
    let t = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if t.chars().count() <= max {
        t
    } else {
        let cut: String = t.chars().take(max.saturating_sub(1)).collect();
        format!("{cut}…")
    }
}

pub struct Snapshot {
    pub status: String,
    pub target: String,
    pub preview: String,
    pub show_preview: bool,
    pub ocr_text: String,
    pub translation: String,
    pub model: String,
    pub target_lang: String,
    pub source_lang: String,
    pub api_ready: bool,
    pub tier: String,
    pub auto_running: bool,
    pub frame_count: u64,
    /// Last OCR inference time in ms (`None` → show "—").
    pub last_ocr_ms: Option<u64>,
    pub last_ocr_block_count: u32,
    pub history_len: usize,
    pub history_preview: String,
    pub selected_window_idx: i32,
    pub window_count: usize,
    pub window_labels: Vec<String>,
    pub translate_in_flight: bool,
    pub can_retry: bool,
    pub last_error: String,
    pub settings_message: String,
    pub settings_dirty: bool,
    pub form_error: String,
    pub confirm: ConfirmAction,
    pub optional_tip_seen: bool,
    pub expand_line_merge: bool,
    pub expand_line_merge_adv: bool,
    pub temp_val: f64,
    pub top_p_val: f64,
    pub max_tokens_val: f64,
    pub reasoning_str: String,
    pub temp_enabled: bool,
    pub top_p_enabled: bool,
    pub max_tokens_enabled: bool,
    pub reasoning_enabled: bool,
    pub text_argb_str: String,
    pub bg_argb_str: String,
    pub text_color_picker_open: bool,
    pub bg_color_picker_open: bool,
    pub api_key_revealed: bool,
    pub base_url: String,
    pub api_key: String,
    pub draft_model: String,
    pub timeout_secs: f64,
    pub max_retries: f64,
    pub retry_backoff: f64,
    pub source_lang_draft: String,
    pub target_lang_draft: String,
    pub history_max: f64,
    pub conv_max: f64,
    pub system_prompt: String,
    pub model_tier_idx: i32,
    pub confidence: f64,
    pub stable_ms: f64,
    pub max_unstable_ms: f64,
    pub interval_ms: f64,
    pub filter_single: bool,
    pub persist_ms: f64,
    pub max_miss_ms: f64,
    pub merge_enabled: bool,
    // Line-merge geometry (mirrors LineMergeConfig; f64 for NumberBox/slider).
    pub merge_min_gap: f64,
    pub merge_max_gap: f64,
    pub merge_gap_slack: f64,
    pub merge_list_gap_min: f64,
    pub merge_wrap_width: f64,
    pub merge_height_ratio: f64,
    pub merge_left_align: f64,
    pub merge_short_max_gap: f64,
    pub merge_list_min_peers: f64,
    pub merge_compact_aspect: f64,
    pub merge_keep_nameplate: bool,
    pub merge_nameplate_body: f64,
}

pub fn take_snapshot(shared: &Arc<Mutex<UiShared>>) -> Snapshot {
    let ui = shared.lock().unwrap();
    let s = ui.state.read();
    let history_preview = s
        .history
        .iter()
        .take(5)
        .map(|h| {
            let src = truncate(&h.source_text, 40);
            let dst = truncate(&h.translated_text, 40);
            format!("• {src} → {dst}")
        })
        .collect::<Vec<_>>()
        .join("\n");
    let history_preview = if history_preview.is_empty() {
        "(no history yet)".into()
    } else {
        history_preview
    };

    let tier_idx = match ui.draft.ocr.model_tier {
        ModelTier::Tiny => 0,
        ModelTier::Small => 1,
        ModelTier::Medium => 2,
    };

    Snapshot {
        status: s.status.label(),
        target: s.target_window_title.clone().unwrap_or_else(|| "(none)".into()),
        preview: if s.config.capture.show_preview {
            format!("{}x{} seq={} {}", s.preview.width, s.preview.height, s.preview.sequence, s.preview.path.as_deref().unwrap_or(""))
        } else {
            "(preview off)".into()
        },
        show_preview: s.config.capture.show_preview,
        ocr_text: if s.latest_ocr_text.is_empty() {
            "(no OCR yet)".into()
        } else {
            s.latest_ocr_text.clone()
        },
        translation: if s.latest_translated_text.is_empty() {
            "(no translation yet)".into()
        } else {
            s.latest_translated_text.clone()
        },
        model: s.config.api.model.clone(),
        target_lang: s.config.translation.target_lang.clone(),
        source_lang: s.config.translation.source_lang.clone(),
        api_ready: !s.config.api.api_key.trim().is_empty(),
        tier: s.config.ocr.model_tier.to_string(),
        auto_running: s.auto_running,
        frame_count: s.frame_count,
        last_ocr_ms: s.last_ocr_ms,
        last_ocr_block_count: s.last_ocr_block_count,
        history_len: s.history.len(),
        history_preview,
        selected_window_idx: if ui.windows.is_empty() {
            -1
        } else {
            ui.selected_idx.min(ui.windows.len().saturating_sub(1)) as i32
        },
        window_count: ui.windows.len(),
        window_labels: ui.windows.iter().map(|w| truncate(&w.title, 72)).collect(),
        translate_in_flight: s.translate_in_flight,
        can_retry: s.can_retry_translate,
        last_error: s.last_error.clone().unwrap_or_default(),
        settings_message: s.settings_message.clone().unwrap_or_default(),
        // Compare draft↔live config (not a sticky flag) so re-bind events
        // from Slider/NumberBox do not show false "Unsaved changes".
        // Use already-held `s.config` — do not call is_settings_dirty (nested read).
        settings_dirty: draft_differs_from(&ui, &s.config),
        form_error: ui.form_error.clone().unwrap_or_default(),
        confirm: ui.confirm,
        optional_tip_seen: ui.optional_tip_seen,
        expand_line_merge: ui.expand_line_merge,
        expand_line_merge_adv: ui.expand_line_merge_adv,
        temp_val: ui.temp_val,
        top_p_val: ui.top_p_val,
        max_tokens_val: ui.max_tokens_val,
        reasoning_str: ui.reasoning_str.clone(),
        temp_enabled: ui.temp_enabled,
        top_p_enabled: ui.top_p_enabled,
        max_tokens_enabled: ui.max_tokens_enabled,
        reasoning_enabled: ui.reasoning_enabled,
        text_argb_str: ui.text_argb_str.clone(),
        bg_argb_str: ui.bg_argb_str.clone(),
        text_color_picker_open: ui.text_color_picker_open,
        bg_color_picker_open: ui.bg_color_picker_open,
        api_key_revealed: ui.api_key_revealed,
        base_url: ui.draft.api.base_url.clone(),
        api_key: ui.draft.api.api_key.clone(),
        draft_model: ui.draft.api.model.clone(),
        timeout_secs: ui.draft.api.request_timeout_secs as f64,
        max_retries: ui.draft.api.max_retries as f64,
        retry_backoff: ui.draft.api.retry_backoff_ms as f64,
        source_lang_draft: ui.draft.translation.source_lang.clone(),
        target_lang_draft: ui.draft.translation.target_lang.clone(),
        history_max: ui.draft.translation.history_max_items as f64,
        conv_max: ui.draft.translation.conversation_max_turns as f64,
        system_prompt: ui.draft.translation.system_prompt.clone().unwrap_or_default(),
        model_tier_idx: tier_idx,
        confidence: ui.draft.ocr.confidence_threshold as f64,
        stable_ms: ui.draft.ocr.stable_duration_ms as f64,
        max_unstable_ms: ui.draft.ocr.max_unstable_ms as f64,
        interval_ms: ui.draft.capture.min_interval_ms as f64,
        filter_single: ui.draft.ocr.filter_single_char,
        persist_ms: ui.draft.ocr.block_persist_ms as f64,
        max_miss_ms: ui.draft.ocr.block_max_miss_ms as f64,
        merge_enabled: ui.draft.ocr.line_merge.enabled,
        merge_min_gap: ui.draft.ocr.line_merge.min_gap_ratio as f64,
        merge_max_gap: ui.draft.ocr.line_merge.max_gap_ratio as f64,
        merge_gap_slack: ui.draft.ocr.line_merge.gap_slack as f64,
        merge_list_gap_min: ui.draft.ocr.line_merge.list_gap_min_ratio as f64,
        merge_wrap_width: ui.draft.ocr.line_merge.wrap_width_ratio as f64,
        merge_height_ratio: ui.draft.ocr.line_merge.height_ratio_min as f64,
        merge_left_align: ui.draft.ocr.line_merge.left_align_ratio as f64,
        merge_short_max_gap: ui.draft.ocr.line_merge.short_max_gap_ratio as f64,
        merge_list_min_peers: ui.draft.ocr.line_merge.list_min_peers as f64,
        merge_compact_aspect: ui.draft.ocr.line_merge.compact_aspect_max as f64,
        merge_keep_nameplate: ui.draft.ocr.line_merge.keep_speaker_separate,
        merge_nameplate_body: ui.draft.ocr.line_merge.nameplate_body_width_ratio as f64,
    }
}

//! Shared UI state, ChromeSnap, and config draft helpers.

use std::{borrow::Cow, sync::Arc};

use parking_lot::Mutex;
use rust_i18n::t;
use translator_capture::{WindowInfo, list_windows};
use translator_core::{
    ApiConfig, ApiProfile, ApiProfileFile, AppConfig, NormRect, PipelineStatus, RegionPreset, RegionPresetFile, UiLanguage,
    api_profiles_path, config_path, format_argb_hex, parse_argb_hex, region_presets_path,
};
use windows_reactor::LocalSender;

use crate::{
    APP_HANDLES,
    pipeline::{CmdTx, PipelineCommand, SharedState},
    ui::locale::apply_ui_locale,
};

/// Apply overlay / reader / HUD visibility immediately (live config + disk), and keep the draft in sync.
pub fn send_overlay_display(ui: &mut UiShared, enabled: bool, reader_enabled: bool, hud_enabled: bool) {
    ui.draft.overlay.enabled = enabled;
    ui.draft.overlay.reader_enabled = reader_enabled;
    ui.draft.overlay.hud_enabled = hud_enabled;
    let _ = ui.cmd_tx.send(PipelineCommand::SetOverlayDisplay {
        enabled,
        reader_enabled,
        hud_enabled,
    });
}

/// Apply the control-window language immediately and persist it on the live config.
pub fn send_ui_language(ui: &mut UiShared, language: UiLanguage) {
    ui.draft.ui.language = Some(language);
    ui.state.write().config.ui.language = Some(language);
    apply_ui_locale(Some(language));
    let _ = ui.cmd_tx.send(PipelineCommand::SetUiLanguage { language });
}

/// Messages the root `AppRoot` component accepts.
#[derive(Clone, Debug)]
pub enum AppMsg {
    /// UI-thread refresh (do not re-arm the pipeline waiter).
    Refresh,
    /// Background pipeline requested a rerender; re-arm the waiter.
    PipelineWake,
    SelectPage(String),
    PaneOpen(bool),
    TogglePane,
    /// Load model ids for the current API draft (`debounce` waits 400ms first).
    FetchModelList {
        debounce: bool,
    },
    ModelListDone {
        generation: u64,
        ids: Vec<String>,
    },
}

/// Shared UI handle for event closures. Clone once per handler (cheap Arc bumps).
#[derive(Clone)]
pub struct UiCx {
    pub shared: Arc<Mutex<UiShared>>,
    pub bump: LocalSender<AppMsg>,
}

impl UiCx {
    pub fn new(shared: &Arc<Mutex<UiShared>>, bump: &LocalSender<AppMsg>) -> Self {
        Self {
            shared: shared.clone(),
            bump: bump.clone(),
        }
    }

    /// Re-render after a UI mutation.
    pub fn refresh(&self) {
        let _ = self.bump.send(AppMsg::Refresh);
    }

    /// Mutate shared UI state, then refresh.
    pub fn with_mut(&self, f: impl FnOnce(&mut UiShared)) {
        f(&mut self.shared.lock());
        self.refresh();
    }

    /// Send a pipeline command and refresh.
    pub fn send_cmd(&self, cmd: PipelineCommand) {
        let _ = self.shared.lock().cmd_tx.send(cmd);
        self.refresh();
    }
}

/// Pending destructive settings action awaiting ContentDialog confirmation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfirmAction {
    #[default]
    None,
    Reload,
    Discard,
}

/// Name row and confirmation dialogs for a list of saved named items.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum NamedDialog {
    #[default]
    None,
    /// Compact inline name row under the combo.
    SaveName,
    Overwrite {
        name: String,
    },
    Delete {
        name: String,
    },
}

/// Combo selection, name draft, and dialog for one list of saved named items.
#[derive(Debug, Clone, Default)]
pub struct NamedPick {
    pub selected: Option<usize>,
    pub name_draft: String,
    pub dialog: NamedDialog,
}

/// Storage hooks that let region presets and API profiles share one save, overwrite,
/// and delete flow.
pub struct NamedStore {
    pub pick: fn(&mut UiShared) -> &mut NamedPick,
    pub names: fn(&UiShared) -> Vec<String>,
    /// Store the current form under the name, replacing an item with that name, and write the file.
    pub commit: fn(&mut UiShared, &str) -> Result<(), String>,
    /// Remove the named item and write the file.
    pub delete: fn(&mut UiShared, &str) -> Result<(), String>,
    /// Extra state to drop when the name row is cancelled.
    pub on_cancel: fn(&mut UiShared),
    /// Translation keys: empty-name error, then overwrite and delete dialog title and body.
    pub name_required: &'static str,
    pub overwrite_title: &'static str,
    pub overwrite_body: &'static str,
    pub delete_title: &'static str,
    pub delete_body: &'static str,
}

pub const REGION_PRESETS: NamedStore = NamedStore {
    pick: |ui| &mut ui.presets,
    names: |ui| ui.region_presets.presets.iter().map(|p| p.name.clone()).collect(),
    commit: |ui, name| {
        // The Save button opens the name row only with sanitized regions.
        ui.region_presets.upsert(name.to_string(), ui.pending_save_regions.clone());
        save_region_presets(ui)?;
        ui.pending_save_regions.clear();
        Ok(())
    },
    delete: |ui, name| {
        ui.region_presets.presets.retain(|p| p.name != name);
        save_region_presets(ui)
    },
    on_cancel: |ui| ui.pending_save_regions.clear(),
    name_required: "err.preset_name_required",
    overwrite_title: "dash.overwrite_preset_title",
    overwrite_body: "dash.overwrite_preset_body",
    delete_title: "dash.delete_preset_title",
    delete_body: "dash.delete_preset_body",
};

pub const API_PROFILES: NamedStore = NamedStore {
    pick: |ui| &mut ui.profiles,
    names: |ui| ui.api_profiles.iter().map(|p| p.name.clone()).collect(),
    commit: |ui, name| {
        let api = effective_draft(ui).api;
        match ui.api_profiles.iter_mut().find(|p| p.name == name) {
            Some(p) => p.api = api,
            None => ui.api_profiles.push(ApiProfile {
                name: name.to_string(),
                api,
            }),
        }
        save_api_profiles(ui)
    },
    delete: |ui, name| {
        ui.api_profiles.retain(|p| p.name != name);
        save_api_profiles(ui)
    },
    on_cancel: |_| {},
    name_required: "err.profile_name_required",
    overwrite_title: "api.overwrite_title",
    overwrite_body: "api.overwrite_body",
    delete_title: "api.delete_title",
    delete_body: "api.delete_body",
};

/// Store the form under `name`, then select it and close the name row. Errors go to the status bar.
pub fn commit_named(ui: &mut UiShared, store: &NamedStore, name: String) {
    if let Err(e) = (store.commit)(ui, &name) {
        ui.state.write().set_error(e);
        return;
    }
    let selected = (store.names)(ui).iter().position(|n| *n == name);
    let pick = (store.pick)(ui);
    pick.selected = selected;
    pick.name_draft = name;
    pick.dialog = NamedDialog::None;
}

pub struct UiShared {
    pub state: SharedState,
    pub cmd_tx: CmdTx,
    pub windows: Vec<WindowInfo>,
    /// `None` until the user picks a window (ComboBox placeholder).
    pub selected_idx: Option<usize>,
    /// Editable draft of settings (committed on Save).
    pub draft: AppConfig,
    /// Optional API numbers (kept while toggle is off so re-enable restores).
    pub optional: OptionalApiState,
    pub text_argb_str: String,
    pub bg_argb_str: String,
    /// ColorPicker panel open (text / background). Only one should be true.
    pub text_color_picker_open: bool,
    pub bg_color_picker_open: bool,
    /// Show API key as plain text (PasswordRevealMode::Visible).
    pub api_key_revealed: bool,
    /// Pending Reload / Discard confirmation dialog.
    pub confirm: ConfirmAction,
    /// Inline form validation message (blocks Save until fixed).
    pub form_error: Option<Cow<'static, str>>,
    /// Named OCR region presets (`region-presets.toml`).
    pub region_presets: RegionPresetFile,
    pub presets: NamedPick,
    /// Regions captured when the preset name row opened.
    pub pending_save_regions: Vec<NormRect>,
    /// Named API connection profiles (`api-profiles.toml`).
    pub api_profiles: Vec<ApiProfile>,
    pub profiles: NamedPick,
    /// Cached model ids for the API settings AutoSuggestBox.
    pub model_catalog: Vec<String>,
    pub model_list_loading: bool,
    pub model_list_fp: String,
    pub model_list_gen: u64,
}

pub fn make_shared() -> Arc<Mutex<UiShared>> {
    let (state, cmd_tx) = APP_HANDLES.get().expect("APP_HANDLES must be set before UI starts").clone();
    let draft = state.read().config.clone();
    let optional = optional_api_state(&draft);
    let (text_argb_str, bg_argb_str) = overlay_color_strings(&draft);
    let (region_presets, preset_load_error) = region_presets_path()
        .map_err(|e| t!("err.load_region_presets", error = e.to_string()))
        .and_then(|path| RegionPresetFile::load_or_empty_at(&path).map_err(|e| t!("err.load_region_presets", error = e.to_string())))
        .map_or_else(|message| (RegionPresetFile::default(), Some(message)), |file| (file, None));
    let (api_profiles, api_profile_load_error) = api_profiles_path()
        .map_err(|e| t!("err.load_api_profiles", error = e.to_string()))
        .and_then(|path| ApiProfileFile::load_or_empty_at(&path).map_err(|e| t!("err.load_api_profiles", error = e.to_string())))
        .map(|file| file.profiles)
        .map_or_else(|message| (Vec::new(), Some(message)), |profiles| (profiles, None));
    let shared = Arc::new(Mutex::new(UiShared {
        state,
        cmd_tx,
        windows: list_windows().unwrap_or_default(),
        selected_idx: None,
        draft,
        optional,
        text_argb_str,
        bg_argb_str,
        text_color_picker_open: false,
        bg_color_picker_open: false,
        api_key_revealed: false,
        confirm: ConfirmAction::None,
        form_error: None,
        region_presets,
        presets: NamedPick::default(),
        pending_save_regions: Vec::new(),
        api_profiles,
        profiles: NamedPick::default(),
        model_catalog: Vec::new(),
        model_list_loading: false,
        model_list_fp: String::new(),
        model_list_gen: 0,
    }));
    if let Some(msg) = preset_load_error.or(api_profile_load_error) {
        shared.lock().state.write().set_error(msg);
    }
    shared
}

fn model_list_fingerprint(api: &ApiConfig) -> String {
    format!("{:?}", api.provider)
}

fn start_model_list(ui: &mut UiShared, bump: &LocalSender<AppMsg>, fp: String, debounce: bool) {
    ui.model_list_gen = ui.model_list_gen.saturating_add(1);
    ui.model_list_fp = fp;
    ui.model_catalog.clear();
    ui.model_list_loading = true;
    let _ = bump.send(AppMsg::FetchModelList { debounce });
}

pub fn schedule_model_list_if_needed(ui: &mut UiShared, bump: &LocalSender<AppMsg>) {
    let fp = model_list_fingerprint(&ui.draft.api);
    if ui.model_list_fp == fp {
        return;
    }
    start_model_list(ui, bump, fp, true);
}

pub fn request_model_list(ui: &mut UiShared, bump: &LocalSender<AppMsg>) {
    start_model_list(ui, bump, model_list_fingerprint(&ui.draft.api), false);
}

/// Selected Dashboard preset, if the combo index is in range.
pub fn selected_preset(ui: &UiShared) -> Option<&RegionPreset> {
    ui.presets.selected.and_then(|i| ui.region_presets.presets.get(i))
}

/// Persist current in-memory presets to `region-presets.toml`.
fn save_region_presets(ui: &UiShared) -> Result<(), String> {
    let path = region_presets_path().map_err(|e| t!("err.save_region_presets", error = e.to_string()).into_owned())?;
    ui.region_presets
        .save(&path)
        .map_err(|e| t!("err.save_region_presets", error = e.to_string()).into_owned())
}

/// Selected API profile, if the combo index is in range.
pub fn selected_api_profile(ui: &UiShared) -> Option<&ApiProfile> {
    ui.profiles.selected.and_then(|i| ui.api_profiles.get(i))
}

/// Persist current in-memory API profiles to `api-profiles.toml`.
fn save_api_profiles(ui: &mut UiShared) -> Result<(), String> {
    let mut file = ApiProfileFile {
        profiles: ui.api_profiles.clone(),
    };
    let path = api_profiles_path().map_err(|e| t!("err.save_api_profiles", error = e.to_string()).into_owned())?;
    file.save(&path)
        .map_err(|e| t!("err.save_api_profiles", error = e.to_string()).into_owned())?;
    ui.api_profiles = file.profiles;
    Ok(())
}

/// Copy the selected profile into the API form. Does not apply until Settings Save.
pub fn load_api_profile(ui: &mut UiShared) -> Result<(), String> {
    let profile = selected_api_profile(ui)
        .cloned()
        .ok_or_else(|| t!("err.select_profile").into_owned())?;
    ui.draft.api = profile.api;
    apply_optional_from_config(ui);
    ui.profiles.dialog = NamedDialog::None;
    mark_dirty(ui);
    Ok(())
}

#[derive(Clone)]
pub struct OptionalApiState {
    pub temp_val: f64,
    pub top_p_val: f64,
    pub max_tokens_val: f64,
    pub reasoning_str: String,
    pub temp_enabled: bool,
    pub top_p_enabled: bool,
    pub max_tokens_enabled: bool,
    pub reasoning_enabled: bool,
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
    (format_argb_hex(cfg.overlay.text_color_argb), format_argb_hex(cfg.overlay.background_color_argb))
}

pub fn reload_draft_from_state(ui: &mut UiShared) {
    ui.draft = ui.state.read().config.clone();
    apply_optional_from_config(ui);
    let (ta, ba) = overlay_color_strings(&ui.draft);
    ui.text_argb_str = ta;
    ui.bg_argb_str = ba;
}

pub fn apply_optional_from_config(ui: &mut UiShared) {
    ui.optional = optional_api_state(&ui.draft);
}

/// Merge optional / overlay free-form fields into a config snapshot (pure).
pub fn effective_draft(ui: &UiShared) -> AppConfig {
    let mut cfg = ui.draft.clone();
    let o = &ui.optional;
    cfg.api.temperature = if o.temp_enabled {
        Some(o.temp_val.clamp(0.0, 2.0) as f32)
    } else {
        None
    };
    cfg.api.top_p = if o.top_p_enabled {
        Some(o.top_p_val.clamp(0.0, 1.0) as f32)
    } else {
        None
    };
    cfg.api.max_tokens = if o.max_tokens_enabled {
        Some(o.max_tokens_val.round().clamp(1.0, 1_000_000.0) as u32)
    } else {
        None
    };
    cfg.api.reasoning_effort = if o.reasoning_enabled {
        let r = o.reasoning_str.trim();
        if r.is_empty() { None } else { Some(r.to_string()) }
    } else {
        None
    };
    if let Some(v) = parse_argb_hex(&ui.text_argb_str) {
        cfg.overlay.text_color_argb = v;
    }
    if let Some(v) = parse_argb_hex(&ui.bg_argb_str) {
        cfg.overlay.background_color_argb = v;
    }
    cfg
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
/// change events when re-bound on a snapshot-driven rerender, which would
/// mark dirty even when nothing changed.
///
/// Acquires `state` once; safe to call without an existing state lock.
pub fn is_settings_dirty(ui: &UiShared) -> bool {
    let live = ui.state.read().config.clone();
    draft_differs_from(ui, &live)
}

/// Call after a real user edit. Clears the success banner so it does not stack.
pub fn mark_dirty(ui: &mut UiShared) {
    ui.form_error = None;
    ui.state.write().settings_message = None;
}

/// Soft validation issues that should block Save.
pub fn form_validation_error(ui: &UiShared) -> Option<Cow<'static, str>> {
    // Optional numbers always have a value; toggle off = omit. No empty checks.
    if ui.optional.reasoning_enabled && ui.optional.reasoning_str.trim().is_empty() {
        return Some(t!("err.reasoning_empty"));
    }
    let text = ui.text_argb_str.trim();
    if !text.is_empty() && parse_argb_hex(text).is_none() {
        return Some(t!("err.text_color"));
    }
    let bg = ui.bg_argb_str.trim();
    if !bg.is_empty() && parse_argb_hex(bg).is_none() {
        return Some(t!("err.bg_color"));
    }
    None
}

pub fn do_reload_from_disk(ui: &mut UiShared) {
    let path = match config_path() {
        Ok(p) => p,
        Err(e) => {
            ui.state.write().set_error(t!("err.reload_config", error = e.to_string()));
            ui.confirm = ConfirmAction::None;
            return;
        }
    };
    match AppConfig::load_or_create(&path) {
        Ok(mut cfg) => {
            if cfg.ui.language.is_none() {
                cfg.ui.language = Some(UiLanguage::from_locale_name(&crate::ui::system_locale_name()));
            }
            apply_ui_locale(cfg.ui.language);
            ui.draft = cfg.clone();
            let _ = ui.cmd_tx.send(PipelineCommand::ApplyConfig(Box::new(cfg)));
            apply_optional_from_config(ui);
            let (ta, ba) = overlay_color_strings(&ui.draft);
            ui.text_argb_str = ta;
            ui.bg_argb_str = ba;
            ui.form_error = None;
            ui.confirm = ConfirmAction::None;
        }
        Err(e) => {
            ui.state.write().set_error(t!("err.reload_config", error = e.to_string()));
            ui.confirm = ConfirmAction::None;
        }
    }
}

pub fn do_discard(ui: &mut UiShared) {
    reload_draft_from_state(ui);
    apply_ui_locale(ui.draft.ui.language);
    ui.form_error = None;
    ui.confirm = ConfirmAction::None;
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

/// Title bar, nav Start/Stop, settings Save bar, dashboard InfoBar.
pub struct ChromeSnap {
    pub status: PipelineStatus,
    pub target: Option<String>,
    pub last_error: Option<String>,
    pub auto_running: bool,
    pub capture_paused: bool,
    pub capture_busy: bool,
    pub selected_hwnd: Option<isize>,
    pub settings_dirty: bool,
    pub form_error: Option<Cow<'static, str>>,
    pub settings_message: Option<String>,
    pub confirm: ConfirmAction,
}

pub fn take_chrome(shared: &Arc<Mutex<UiShared>>) -> ChromeSnap {
    let ui = shared.lock();
    let s = ui.state.read();
    ChromeSnap {
        status: s.status.clone(),
        target: s.target_window_title.clone(),
        last_error: s.last_error.clone(),
        auto_running: s.auto_running,
        capture_paused: s.capture_paused,
        capture_busy: s.capture_busy,
        selected_hwnd: ui.selected_idx.and_then(|i| ui.windows.get(i).map(|w| w.hwnd)),
        // Use already-held `s.config` — do not call is_settings_dirty (nested read).
        settings_dirty: draft_differs_from(&ui, &s.config),
        form_error: ui.form_error.clone(),
        settings_message: s.settings_message.clone(),
        confirm: ui.confirm,
    }
}

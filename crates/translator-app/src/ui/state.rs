//! Shared UI state, ChromeSnap, and config draft helpers.

use std::{borrow::Cow, sync::Arc};

use parking_lot::Mutex;
use rust_i18n::t;
use translator_capture::{WindowInfo, list_windows};
use translator_core::{
    ApiConfig, ApiProfile, ApiProfileFile, AppConfig, DictionaryPair, HotkeyAction, NormRect, OVERLAY_FONT_PX_DEFAULT, OVERLAY_FONT_PX_MAX,
    OVERLAY_FONT_PX_MIN, PipelineStatus, RegionPreset, RegionPresetFile, UiLanguage, api_profiles_path, config_path, delete_profile,
    dictionaries_dir, format_argb_hex, is_valid_profile_name, list_dictionary_names_default, load_profile_pairs, parse_argb_hex,
    region_presets_path, sanitize_pairs, save_profile_pairs,
};

use crate::{
    APP_HANDLES,
    hotkey::{self, Captured},
    pipeline::{CmdTx, PipelineCommand, SharedState, ping_ui},
    ui::{
        app::{fetch_model_list, refresh},
        locale::{apply_ui_locale, system_locale_name},
    },
};

/// Apply the draft's overlay / reader / HUD visibility immediately (live config + disk).
pub fn send_overlay_display(ui: &UiShared) {
    let o = &ui.draft.overlay;
    let _ = ui.cmd_tx.send(PipelineCommand::SetOverlayDisplay {
        enabled: o.enabled,
        reader_enabled: o.reader_enabled,
        hud_enabled: o.hud_enabled,
    });
}

/// Apply the control-window language immediately and persist it on the live config.
pub fn send_ui_language(ui: &mut UiShared, language: UiLanguage) {
    ui.draft.ui.language = Some(language);
    ui.state.write().config.ui.language = Some(language);
    apply_ui_locale(Some(language));
    let _ = ui.cmd_tx.send(PipelineCommand::SetUiLanguage { language });
}

/// Control-window pages, in navigation order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Page {
    #[default]
    Dashboard,
    Api,
    Translation,
    Ocr,
    Overlay,
    Hotkeys,
}

/// Shared UI handle for event closures. Clone once per handler (cheap Arc bump).
#[derive(Clone)]
pub struct UiCx {
    pub shared: Arc<Mutex<UiShared>>,
}

impl UiCx {
    pub fn new(shared: &Arc<Mutex<UiShared>>) -> Self {
        Self { shared: shared.clone() }
    }

    /// Re-sync the window after a UI mutation.
    pub fn refresh(&self) {
        refresh();
    }

    /// Mutate shared UI state, then refresh.
    pub fn with_mut(&self, f: impl FnOnce(&mut UiShared)) {
        f(&mut self.shared.lock());
        self.refresh();
    }

    /// Change handler for a settings control: applies the new value, then marks the form dirty.
    pub fn edit<T>(&self, f: impl Fn(&mut UiShared, T) + Clone + Send + 'static) -> impl Fn(T) + Clone + Send + 'static {
        let cx = self.clone();
        move |v| {
            cx.with_mut(|ui| {
                f(ui, v);
                mark_dirty(ui);
            });
        }
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

/// Storage hooks that let region presets, glossaries, and API profiles share one save,
/// overwrite, and delete flow.
pub struct NamedStore {
    pub pick: fn(&mut UiShared) -> &mut NamedPick,
    pub view: fn(&UiShared) -> &NamedPick,
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
    view: |ui| &ui.presets,
    names: |ui| ui.region_presets.presets.iter().map(|p| p.name.clone()).collect(),
    commit: |ui, name| {
        // The Save button fills this with sanitized regions. An empty list would save a preset the next load drops.
        if ui.pending_save_regions.is_empty() {
            return Err(t!("err.regions_required").into_owned());
        }
        let mut file = ui.region_presets.clone();
        file.upsert(name.to_string(), ui.pending_save_regions.clone());
        save_region_presets(ui, file)?;
        ui.pending_save_regions.clear();
        Ok(())
    },
    delete: |ui, name| {
        let mut file = ui.region_presets.clone();
        file.presets.retain(|p| p.name != name);
        save_region_presets(ui, file)
    },
    on_cancel: |ui| ui.pending_save_regions.clear(),
    name_required: "err.preset_name_required",
    overwrite_title: "dash.overwrite_preset_title",
    overwrite_body: "dash.overwrite_preset_body",
    delete_title: "dash.delete_preset_title",
    delete_body: "dash.delete_preset_body",
};

/// Glossary editor and the settings apply combo. Live pairs for translate live on `AppState`.
#[derive(Clone, Default)]
pub struct DictionaryUi {
    pub names: Vec<String>,
    pub pick: NamedPick,
    pub pairs: Vec<DictionaryPair>,
    pub saved_pairs: Vec<DictionaryPair>,
    /// File currently in the editor buffer.
    pub loaded: Option<String>,
    pub editor_open: bool,
}

impl DictionaryUi {
    pub fn dirty(&self) -> bool {
        sanitize_pairs(&self.pairs) != self.saved_pairs
    }

    pub fn selected_name(&self) -> Option<String> {
        self.pick.selected.and_then(|i| self.names.get(i)).cloned()
    }

    pub fn revert(&mut self) {
        self.pairs.clone_from(&self.saved_pairs);
    }

    pub fn close_editor(&mut self) {
        self.editor_open = false;
        self.pick.dialog = NamedDialog::None;
    }
}

pub const DICTIONARIES: NamedStore = NamedStore {
    pick: |ui| &mut ui.dictionary.pick,
    view: |ui| &ui.dictionary.pick,
    names: |ui| ui.dictionary.names.clone(),
    commit: |ui, name| {
        if !is_valid_profile_name(name) {
            return Err(t!("err.dictionary_name_invalid").into_owned());
        }
        let dir = dictionaries_dir().map_err(|e| t!("err.save_dictionary", error = e.to_string()).into_owned())?;
        let pairs = sanitize_pairs(&ui.dictionary.pairs);
        save_profile_pairs(&dir, name, &pairs).map_err(|e| t!("err.save_dictionary", error = e.to_string()).into_owned())?;
        ui.dictionary.pairs = pairs.clone();
        ui.dictionary.saved_pairs = pairs;
        ui.dictionary.loaded = Some(name.to_string());
        refresh_dictionary_names(ui);
        if ui.draft.translation.dictionary_profile.as_deref() == Some(name) {
            mark_dirty(ui);
        }
        ping_ui();
        Ok(())
    },
    delete: |ui, name| {
        let dir = dictionaries_dir().map_err(|e| t!("err.delete_dictionary", error = e.to_string()).into_owned())?;
        delete_profile(&dir, name).map_err(|e| t!("err.delete_dictionary", error = e.to_string()).into_owned())?;
        if ui.dictionary.loaded.as_deref() == Some(name) {
            ui.dictionary.pairs.clear();
            ui.dictionary.saved_pairs.clear();
            ui.dictionary.loaded = None;
        }
        refresh_dictionary_names(ui);
        if ui.draft.translation.dictionary_profile.as_deref() == Some(name) {
            ui.draft.translation.dictionary_profile = None;
            mark_dirty(ui);
        }
        // Discard restores live config. Drop the name there too so a deleted
        // in-use profile cannot come back, and persist so restart matches.
        let apply_live = {
            let mut s = ui.state.write();
            if s.config.translation.dictionary_profile.as_deref() == Some(name) {
                s.config.translation.dictionary_profile = None;
                Some(s.config.clone())
            } else {
                None
            }
        };
        if let Some(cfg) = apply_live {
            let _ = ui.cmd_tx.send(PipelineCommand::ApplyConfig(Box::new(cfg)));
        }
        ping_ui();
        Ok(())
    },
    on_cancel: |_| {},
    name_required: "err.dictionary_name_required",
    overwrite_title: "tr.overwrite_dictionary_title",
    overwrite_body: "tr.overwrite_dictionary_body",
    delete_title: "tr.delete_dictionary_title",
    delete_body: "tr.delete_dictionary_body",
};

pub const API_PROFILES: NamedStore = NamedStore {
    pick: |ui| &mut ui.profiles,
    view: |ui| &ui.profiles,
    names: |ui| ui.api_profiles.iter().map(|p| p.name.clone()).collect(),
    commit: |ui, name| {
        let api = effective_draft(ui).api;
        let mut profiles = ui.api_profiles.clone();
        match profiles.iter_mut().find(|p| p.name == name) {
            Some(p) => p.api = api,
            None => profiles.push(ApiProfile {
                name: name.to_string(),
                api,
            }),
        }
        save_api_profiles(ui, profiles)
    },
    delete: |ui, name| {
        let profiles = ui.api_profiles.iter().filter(|p| p.name != name).cloned().collect();
        save_api_profiles(ui, profiles)
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
    pub draft: Box<AppConfig>,
    /// Optional API numbers (kept while toggle is off so re-enable restores).
    pub optional: Box<OptionalApiState>,
    pub text_argb_str: String,
    pub bg_argb_str: String,
    /// Overlay caption size slider (kept while the override toggle is off).
    pub overlay_font_val: f64,
    pub overlay_font_enabled: bool,
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
    pub dictionary: Box<DictionaryUi>,
    /// Cached model ids for the API settings AutoSuggestBox.
    pub model_catalog: Vec<String>,
    pub model_list_loading: bool,
    pub model_list_fp: String,
    pub model_list_gen: u64,
    /// Row waiting for one key. `hotkey_listen_gen` must match the hook before a chord is kept.
    pub hotkey_listen: Option<HotkeyAction>,
    pub hotkey_listen_gen: u64,
    /// Duplicate-chord message shown under one row.
    pub hotkey_error: Option<(HotkeyAction, String)>,
    pub page: Page,
    pub pane_open: bool,
}

pub fn make_shared() -> Arc<Mutex<UiShared>> {
    let (state, cmd_tx) = APP_HANDLES.get().expect("APP_HANDLES must be set before UI starts").clone();
    let draft = state.read().config.clone();
    let (region_presets, preset_load_error) = region_presets_path()
        .map_err(|e| t!("err.load_region_presets", error = e.to_string()))
        .and_then(|path| RegionPresetFile::load_or_empty_at(&path).map_err(|e| t!("err.load_region_presets", error = e.to_string())))
        .map_or_else(|message| (RegionPresetFile::default(), Some(message)), |file| (file, None));
    let (api_profiles, api_profile_load_error) = api_profiles_path()
        .map_err(|e| t!("err.load_api_profiles", error = e.to_string()))
        .and_then(|path| ApiProfileFile::load_or_empty_at(&path).map_err(|e| t!("err.load_api_profiles", error = e.to_string())))
        .map(|file| file.profiles)
        .map_or_else(|message| (Vec::new(), Some(message)), |profiles| (profiles, None));
    let (dictionary_names, dictionary_load_error) = list_dictionary_names_default()
        .map_err(|e| t!("err.load_dictionary", error = e.to_string()))
        .map_or_else(|message| (Vec::new(), Some(message)), |names| (names, None));
    // `load_form` below fills the draft and the free-form fields.
    let mut ui = UiShared {
        state,
        cmd_tx,
        windows: list_windows().unwrap_or_default(),
        selected_idx: None,
        draft: Box::default(),
        optional: Box::new(optional_api_state(&draft)),
        text_argb_str: String::new(),
        bg_argb_str: String::new(),
        overlay_font_val: 0.0,
        overlay_font_enabled: false,
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
        dictionary: Box::new(DictionaryUi {
            names: dictionary_names,
            ..DictionaryUi::default()
        }),
        model_catalog: Vec::new(),
        model_list_loading: false,
        model_list_fp: String::new(),
        model_list_gen: 0,
        hotkey_listen: None,
        hotkey_listen_gen: 0,
        hotkey_error: None,
        page: Page::Dashboard,
        pane_open: true,
    };
    load_form(&mut ui, draft);
    if let Some(msg) = preset_load_error.or(api_profile_load_error).or(dictionary_load_error) {
        ui.state.write().set_error(msg);
    }
    Arc::new(Mutex::new(ui))
}

fn model_list_fingerprint(api: &ApiConfig) -> String {
    format!("{:?}", api.provider)
}

fn start_model_list(ui: &mut UiShared, fp: String, debounce: bool) {
    ui.model_list_gen = ui.model_list_gen.saturating_add(1);
    ui.model_list_fp = fp;
    ui.model_catalog.clear();
    ui.model_list_loading = true;
    fetch_model_list(ui.model_list_gen, debounce);
}

pub fn schedule_model_list_if_needed(ui: &mut UiShared) {
    let fp = model_list_fingerprint(&ui.draft.api);
    if ui.model_list_fp == fp {
        return;
    }
    start_model_list(ui, fp, true);
}

pub fn request_model_list(ui: &mut UiShared) {
    start_model_list(ui, model_list_fingerprint(&ui.draft.api), false);
}

/// Opens the glossary editor, loading the profile the settings combo points at.
pub fn open_dictionary_editor(ui: &mut UiShared) {
    if !ui.dictionary.editor_open {
        prepare_dictionary_editor(ui);
        ui.dictionary.editor_open = true;
    }
}

/// Selected Dashboard preset, if the combo index is in range.
pub fn selected_preset(ui: &UiShared) -> Option<&RegionPreset> {
    ui.presets.selected.and_then(|i| ui.region_presets.presets.get(i))
}

/// Write `file` to `region-presets.toml`. It replaces the in-memory list only when the write
/// succeeds, so a failed save does not show a preset that is not on disk.
fn save_region_presets(ui: &mut UiShared, file: RegionPresetFile) -> Result<(), String> {
    let path = region_presets_path().map_err(|e| t!("err.save_region_presets", error = e.to_string()).into_owned())?;
    file.save(&path)
        .map_err(|e| t!("err.save_region_presets", error = e.to_string()).into_owned())?;
    ui.region_presets = file;
    Ok(())
}

/// Selected API profile, if the combo index is in range.
pub fn selected_api_profile(ui: &UiShared) -> Option<&ApiProfile> {
    ui.profiles.selected.and_then(|i| ui.api_profiles.get(i))
}

/// Write `profiles` to `api-profiles.toml`. They replace the in-memory list only when the
/// write succeeds, so a failed save does not show a profile that is not on disk.
fn save_api_profiles(ui: &mut UiShared, profiles: Vec<ApiProfile>) -> Result<(), String> {
    let mut file = ApiProfileFile { profiles };
    let path = api_profiles_path().map_err(|e| t!("err.save_api_profiles", error = e.to_string()).into_owned())?;
    file.save(&path)
        .map_err(|e| t!("err.save_api_profiles", error = e.to_string()).into_owned())?;
    ui.api_profiles = file.profiles;
    Ok(())
}

pub fn refresh_dictionary_names(ui: &mut UiShared) {
    match list_dictionary_names_default() {
        Ok(names) => {
            let keep = ui
                .dictionary
                .loaded
                .clone()
                .or_else(|| ui.dictionary.pick.selected.and_then(|i| ui.dictionary.names.get(i)).cloned());
            ui.dictionary.names = names;
            if let Some(name) = keep {
                ui.dictionary.pick.selected = ui.dictionary.names.iter().position(|n| *n == name);
            }
        }
        Err(e) => ui.state.write().set_error(t!("err.load_dictionary", error = e.to_string())),
    }
}

/// Combo pick: save the buffer into the loaded profile, then load the new one.
pub fn select_dictionary_profile(ui: &mut UiShared, idx: Option<usize>) {
    let count = ui.dictionary.names.len();
    let Some(idx) = idx.filter(|&i| i < count) else {
        return;
    };
    let next_name = ui.dictionary.names.get(idx).cloned();
    if next_name.as_deref() == ui.dictionary.loaded.as_deref() {
        ui.dictionary.pick.selected = Some(idx);
        return;
    }
    if let Some(loaded) = ui.dictionary.loaded.clone()
        && ui.dictionary.dirty()
    {
        commit_named(ui, &DICTIONARIES, loaded);
        if ui.dictionary.dirty() {
            return;
        }
    }
    ui.dictionary.pick.selected = Some(idx);
    if let Some(name) = next_name
        && let Err(e) = load_dictionary_into_editor(ui, &name)
    {
        ui.state.write().set_error(e);
    }
}

/// Save the loaded profile, then open the 新建 name row.
pub fn start_new_dictionary(ui: &mut UiShared) {
    if let Some(loaded) = ui.dictionary.loaded.clone() {
        commit_named(ui, &DICTIONARIES, loaded);
        if ui.dictionary.dirty() {
            return;
        }
    }
    ui.dictionary.pick.name_draft.clear();
    ui.dictionary.pick.dialog = NamedDialog::SaveName;
}

pub fn load_dictionary_into_editor(ui: &mut UiShared, name: &str) -> Result<(), String> {
    let dir = dictionaries_dir().map_err(|e| t!("err.load_dictionary", error = e.to_string()).into_owned())?;
    let pairs = load_profile_pairs(&dir, name).map_err(|e| t!("err.load_dictionary", error = e.to_string()).into_owned())?;
    ui.dictionary.pairs = pairs.clone();
    ui.dictionary.saved_pairs = pairs;
    ui.dictionary.loaded = Some(name.to_string());
    ui.dictionary.pick.selected = ui.dictionary.names.iter().position(|n| n == name);
    ui.dictionary.pick.dialog = NamedDialog::None;
    Ok(())
}

/// Fill the editor from the settings combo when there is nothing unsaved.
pub fn prepare_dictionary_editor(ui: &mut UiShared) {
    refresh_dictionary_names(ui);
    if ui.dictionary.dirty() {
        return;
    }
    let Some(name) = ui.draft.translation.dictionary_profile.clone() else {
        ui.dictionary.pairs.clear();
        ui.dictionary.saved_pairs.clear();
        ui.dictionary.loaded = None;
        ui.dictionary.pick.selected = None;
        ui.dictionary.pick.name_draft.clear();
        if ui.dictionary.names.is_empty() {
            ui.dictionary.pick.dialog = NamedDialog::SaveName;
        }
        return;
    };
    if let Err(e) = load_dictionary_into_editor(ui, &name) {
        ui.state.write().set_error(e);
    }
}

/// Copy the selected profile into the API form. Does not apply until Settings Save.
pub fn load_api_profile(ui: &mut UiShared) -> Result<(), String> {
    let profile = selected_api_profile(ui)
        .cloned()
        .ok_or_else(|| t!("err.select_profile").into_owned())?;
    ui.draft.api = profile.api;
    *ui.optional = optional_api_state(&ui.draft);
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

/// Point the form at `cfg`: the draft plus the free-form fields kept beside it.
fn load_form(ui: &mut UiShared, cfg: AppConfig) {
    *ui.optional = optional_api_state(&cfg);
    ui.text_argb_str = format_argb_hex(cfg.overlay.text_color_argb);
    ui.bg_argb_str = format_argb_hex(cfg.overlay.background_color_argb);
    ui.overlay_font_enabled = cfg.overlay.font_px.is_some();
    ui.overlay_font_val = f64::from(cfg.overlay.font_px.unwrap_or(OVERLAY_FONT_PX_DEFAULT));
    *ui.draft = cfg;
    ui.form_error = None;
}

/// Merge optional / overlay free-form fields into a config snapshot (pure).
pub fn effective_draft(ui: &UiShared) -> AppConfig {
    let mut cfg = ui.draft.as_ref().clone();
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
    cfg.overlay.font_px = if ui.overlay_font_enabled {
        Some(
            ui.overlay_font_val
                .round()
                .clamp(f64::from(OVERLAY_FONT_PX_MIN), f64::from(OVERLAY_FONT_PX_MAX)) as u32,
        )
    } else {
        None
    };
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

/// Label for a shortcut row. Reuses the dashboard and action names.
pub fn hotkey_action_label(action: HotkeyAction) -> Cow<'static, str> {
    match action {
        HotkeyAction::StartPause => t!("hotkeys.start_pause"),
        HotkeyAction::Stop => t!("action.stop"),
        HotkeyAction::SelectRegion => t!("dash.select_regions"),
        HotkeyAction::Retry => t!("action.retry"),
        HotkeyAction::ClearHistory => t!("dash.clear_chat"),
    }
}

/// Apply a chord the hook captured, when it still belongs to the open listen.
pub fn apply_hotkey_capture(ui: &mut UiShared) {
    let Some(captured) = hotkey::take_capture() else {
        return;
    };
    let (generation, bind) = match captured {
        Captured::Bind { action, chord, generation } => (generation, Some((action, chord))),
        Captured::Cancel { generation } => (generation, None),
    };
    if ui.hotkey_listen.is_none() || generation != ui.hotkey_listen_gen {
        return;
    }
    let listening = ui.hotkey_listen.take();
    let Some((action, chord)) = bind else {
        return;
    };
    if listening != Some(action) {
        return;
    }
    match ui.draft.hotkeys.assign(action, chord) {
        Ok(()) => {
            ui.hotkey_error = None;
            mark_dirty(ui);
        }
        Err(other) => {
            ui.hotkey_error = Some((action, t!("hotkeys.duplicate", action = hotkey_action_label(other)).into_owned()));
        }
    }
}

/// Drop an in-progress bind. Chords already in the draft stay.
pub fn stop_hotkey_edit(ui: &mut UiShared) {
    ui.hotkey_listen = None;
    ui.hotkey_error = None;
    hotkey::cancel_listen();
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
    stop_hotkey_edit(ui);
    ui.confirm = ConfirmAction::None;
    let loaded = config_path()
        .map_err(|e| e.to_string())
        .and_then(|path| AppConfig::load_or_create(&path).map_err(|e| e.to_string()));
    match loaded {
        Ok(mut cfg) => {
            if cfg.ui.language.is_none() {
                cfg.ui.language = Some(UiLanguage::from_locale_name(&system_locale_name()));
            }
            apply_ui_locale(cfg.ui.language);
            let _ = ui.cmd_tx.send(PipelineCommand::ApplyConfig(Box::new(cfg.clone())));
            load_form(ui, cfg);
        }
        Err(e) => ui.state.write().set_error(t!("err.reload_config", error = e)),
    }
}

pub fn do_discard(ui: &mut UiShared) {
    stop_hotkey_edit(ui);
    ui.confirm = ConfirmAction::None;
    let cfg = ui.state.read().config.clone();
    apply_ui_locale(cfg.ui.language);
    load_form(ui, cfg);
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

pub fn take_chrome(ui: &UiShared) -> ChromeSnap {
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
        settings_dirty: draft_differs_from(ui, &s.config),
        form_error: ui.form_error.clone(),
        settings_message: s.settings_message.clone(),
        confirm: ui.confirm,
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn ui_shared_stays_under_1kb() {
        let n = std::mem::size_of::<super::UiShared>();
        assert!(n <= 1024, "UiShared is {n} bytes; box another large field");
    }
}

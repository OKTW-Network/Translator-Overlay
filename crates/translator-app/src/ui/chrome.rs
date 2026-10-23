//! Window chrome: status, Start/Stop, sticky Save bar, Reload / Discard confirmation.

use std::borrow::Cow;

use rust_i18n::t;
use translator_core::{PipelineStatus, SETTINGS_SAVED, UiLanguage};
use translator_winui::{
    Border, Button, ComboBox, Grid, HorizontalAlignment, InfoBar, InfoBarSeverity, Interface, Layout, Result, TextBlock, TextTrimming,
    TextWrapping, UIElement, VerticalAlignment, auto, grid, star, thickness,
};

use crate::{
    children,
    pipeline::PipelineCommand,
    ui::{
        locale::UI_LANGUAGES,
        state::{
            ChromeSnap, ConfirmAction, UiCx, UiShared, do_discard, do_reload_from_disk, effective_draft, form_validation_error,
            is_settings_dirty, send_ui_language, stop_hotkey_edit,
        },
        widgets::{
            Binds, ButtonKind, Prop, Theme, button, button_style, cards::BOLD, click, dialog::Dialog, fields::Combo, foreground, glyph_box,
            hs, muted, row, text,
        },
    },
};

/// Pipeline InfoBar for the Dashboard only. Always open (Informational when idle).
pub fn status_infobar<S>(b: &mut Binds<S>) -> Result<InfoBar> {
    let bar = InfoBar::new()?;
    bar.SetIsOpen(true)?;
    bar.SetIsClosable(false)?;
    let el = bar.clone();
    let mut shown = Prop::default();
    b.add(move |_, snap| {
        let target = snap.target.as_deref().map(Cow::Borrowed).unwrap_or_else(|| t!("target.none"));
        let (title, message, severity) = match (&snap.status, snap.last_error.as_deref().filter(|s| !s.is_empty())) {
            (PipelineStatus::RetryingTranslate { attempt, max_retries, .. }, Some(err)) => (
                err.to_string(),
                t!("status.retrying_short", attempt = attempt, max_retries = max_retries).into_owned(),
                InfoBarSeverity::Warning,
            ),
            (_, Some(err)) => (err.to_string(), String::new(), InfoBarSeverity::Error),
            _ => (status_text(&snap.status).into_owned(), target.into_owned(), InfoBarSeverity::Informational),
        };
        shown.set((title, message, severity.0), |(title, message, severity)| {
            el.SetTitle(&hs(title))?;
            el.SetMessage(&hs(message))?;
            el.SetSeverity(InfoBarSeverity(*severity))
        })
    });
    Ok(bar)
}

/// Compact title-bar status: pipeline label and target window only.
pub fn app_status_strip(b: &mut Binds<UiShared>) -> Result<TextBlock> {
    let tb = muted("")?;
    tb.SetTextWrapping(TextWrapping::NoWrap)?;
    tb.SetTextTrimming(TextTrimming::CharacterEllipsis)?;
    tb.SetMaxLines(1)?;
    tb.valign(VerticalAlignment::Center)?;
    let el = tb.clone();
    let mut shown = Prop::default();
    b.add(move |_, snap| {
        let line = format!("{}  ·  {}", status_text(&snap.status), snap.target.as_deref().unwrap_or(&t!("target.none")));
        shown.set(line, |line| el.SetText(&hs(line)))
    });
    Ok(tb)
}

/// Language combo for the title bar `RightHeader` (fixed next to caption buttons).
pub fn ui_language_combo(b: &mut Binds<UiShared>, cx: &UiCx) -> Result<ComboBox> {
    let cx = cx.clone();
    let mut combo = Combo::new("", move |idx| {
        let Some(&(language, _)) = idx.and_then(|i| UI_LANGUAGES.get(i)) else {
            return;
        };
        cx.with_mut(|ui| {
            if ui.draft.ui.language != Some(language) {
                send_ui_language(ui, language);
            }
        });
    })?;
    combo
        .cb
        .min_width(132.0)?
        .valign(VerticalAlignment::Center)?
        .margin(8.0, 0.0, 0.0, 0.0)?;
    let el = combo.cb.clone();
    b.add(move |ui, _| {
        let selected = ui.draft.ui.language.unwrap_or(UiLanguage::En);
        let items = UI_LANGUAGES.map(|(_, label)| label.to_string()).to_vec();
        combo.set(items, UI_LANGUAGES.iter().position(|(lang, _)| *lang == selected), true)
    });
    Ok(el)
}

const NAV_ITEM_HEIGHT: f64 = 36.0;
const NAV_ICON_BOX: f64 = 40.0;

/// Start/Pause (same slot) plus Stop above it in the NavigationView pane footer.
///
/// Rebuilt when the button set changes shape (style, compact geometry, stop enabled);
/// otherwise only enabled state and tooltips update.
pub struct Footer {
    pub root: Grid,
    key: Option<String>,
    stop: Option<Button>,
    primary: Option<Button>,
    tips: Prop<(bool, bool, String, String)>,
}

impl Footer {
    pub fn new() -> Result<Self> {
        let root = grid(&[auto(), auto()], &[])?;
        root.valign(VerticalAlignment::Bottom)?;
        Ok(Self {
            root,
            key: None,
            stop: None,
            primary: None,
            tips: Prop::default(),
        })
    }

    pub fn sync(&mut self, cx: &UiCx, snap: &ChromeSnap, pane_open: bool) -> Result<()> {
        let has_window = snap.selected_hwnd.is_some();
        let paused = snap.capture_paused;
        let running = snap.auto_running && !paused;
        let session = snap.auto_running;
        let busy = snap.capture_busy;
        let primary_enabled = !busy && (session || has_window);
        let stop_enabled = !busy && session;
        let key = format!(
            "cap-{}-{}-{}-{}",
            if running {
                "pause"
            } else if paused {
                "resume"
            } else {
                "start"
            },
            if pane_open { "wide" } else { "compact" },
            if stop_enabled { "stop-on" } else { "stop-off" },
            &*rust_i18n::locale(),
        );
        if self.key.as_deref() != Some(&key) {
            self.rebuild(cx, running, paused, pane_open)?;
            self.key = Some(key);
            self.tips = Prop::default();
        }
        let primary_tip = if busy {
            t!("capture.waiting_stop")
        } else if running {
            t!("capture.pause")
        } else if paused {
            t!("capture.resume")
        } else if has_window {
            t!("capture.start")
        } else {
            t!("capture.select_window_first")
        };
        let stop_tip = if busy {
            t!("capture.waiting_stop")
        } else if session {
            t!("capture.stop")
        } else {
            t!("capture.stop_idle")
        };
        let (Some(stop), Some(primary)) = (&self.stop, &self.primary) else {
            return Ok(());
        };
        self.tips
            .set((primary_enabled, stop_enabled, primary_tip.into_owned(), stop_tip.into_owned()), |(pe, se, pt, st)| {
                primary.SetIsEnabled(*pe)?;
                stop.SetIsEnabled(*se)?;
                primary.tooltip(pt)?;
                stop.tooltip(st)?;
                Ok(())
            })
    }

    fn rebuild(&mut self, cx: &UiCx, running: bool, paused: bool, pane_open: bool) -> Result<()> {
        let primary_label = if running { t!("action.pause") } else { t!("action.start") };
        let primary_glyph = if running { "\u{E769}" } else { "\u{F5B0}" };
        let stop_label = t!("action.stop");
        let content = |glyph: &str, label: &str| -> Result<UIElement> {
            if pane_open {
                let panel = row(8.0)?;
                panel.valign(VerticalAlignment::Center)?;
                let tb = text(label)?;
                tb.valign(VerticalAlignment::Center)?;
                children!(panel; glyph_box(glyph)?, tb);
                panel.cast()
            } else {
                glyph_box(glyph)?.cast()
            }
        };

        let stop = Button::new()?;
        stop.valign(VerticalAlignment::Bottom)?
            .height(NAV_ITEM_HEIGHT)?
            .automation_name(&stop_label)?
            .automation_id("btn-stop")?;
        click(&stop, {
            let cx = cx.clone();
            move || {
                {
                    let ui = cx.shared.lock();
                    let mut s = ui.state.write();
                    if !s.capture_busy && s.auto_running {
                        s.capture_busy = true;
                        drop(s);
                        let _ = ui.cmd_tx.send(PipelineCommand::StopCapture);
                    }
                }
                cx.refresh();
            }
        })?;
        if !pane_open {
            button_style(&stop, ButtonKind::Subtle)?;
        }

        let primary = Button::new()?;
        primary
            .valign(VerticalAlignment::Bottom)?
            .height(NAV_ITEM_HEIGHT)?
            .automation_name(&primary_label)?
            .automation_id("btn-start-pause")?;
        click(&primary, {
            let cx = cx.clone();
            move || {
                {
                    let ui = cx.shared.lock();
                    let s = ui.state.write();
                    if !s.capture_busy {
                        if s.auto_running && !s.capture_paused {
                            drop(s);
                            let _ = ui.cmd_tx.send(PipelineCommand::PauseCapture);
                        } else if s.capture_paused {
                            drop(s);
                            let _ = ui.cmd_tx.send(PipelineCommand::ResumeCapture);
                        } else {
                            drop(s);
                            if let Some(w) = ui.selected_idx.and_then(|i| ui.windows.get(i)).cloned() {
                                let _ = ui.cmd_tx.send(PipelineCommand::StartCapture {
                                    hwnd: w.hwnd,
                                    title: w.title,
                                });
                            }
                        }
                    }
                }
                cx.refresh();
            }
        })?;
        if running || paused {
            button_style(&primary, ButtonKind::Accent)?;
        } else if !pane_open {
            button_style(&primary, ButtonKind::Subtle)?;
        }

        if pane_open {
            stop.halign(HorizontalAlignment::Stretch)?.margin(8.0, 8.0, 8.0, 4.0)?;
            primary.halign(HorizontalAlignment::Stretch)?.margin(8.0, 4.0, 8.0, 8.0)?;
            self.root.halign(HorizontalAlignment::Stretch)?;
            self.root.SetWidth(f64::NAN)?;
        } else {
            stop.width(NAV_ICON_BOX)?
                .halign(HorizontalAlignment::Center)?
                .margin(0.0, 4.0, 0.0, 0.0)?;
            primary
                .width(NAV_ICON_BOX)?
                .halign(HorizontalAlignment::Center)?
                .margin(0.0, 4.0, 0.0, 8.0)?;
            self.root.width(48.0)?.halign(HorizontalAlignment::Center)?;
        }
        stop.SetContent(&content("\u{EE95}", &stop_label)?)?;
        primary.SetContent(&content(primary_glyph, &primary_label)?)?;

        let stop_cell = Border::new()?;
        stop_cell.row(0)?;
        stop_cell.SetChild(&stop)?;
        let primary_cell = Border::new()?;
        primary_cell.row(1)?;
        primary_cell.SetChild(&primary)?;
        let children = self.root.Children()?;
        children.Clear()?;
        children.Append(&stop_cell)?;
        children.Append(&primary_cell)?;
        self.stop = Some(stop);
        self.primary = Some(primary);
        Ok(())
    }
}

/// Sticky top bar for settings pages: page title + Save / Reload / Discard (outside scroll).
pub fn settings_sticky_chrome(b: &mut Binds<UiShared>, cx: &UiCx, title: &str) -> Result<Border> {
    let title_el = text(title)?;
    title_el.SetFontSize(28.0)?;
    title_el.SetFontWeight(BOLD)?;

    let status_hint = TextBlock::new()?;
    status_hint.SetFontSize(12.0)?;
    status_hint.valign(VerticalAlignment::Center)?;
    let save = button(&t!("action.save"), {
        let cx = cx.clone();
        move || {
            {
                let mut ui = cx.shared.lock();
                stop_hotkey_edit(&mut ui);
                if let Some(err) = form_validation_error(&ui) {
                    ui.form_error = Some(err);
                    drop(ui);
                    cx.refresh();
                    return;
                }
                *ui.draft = effective_draft(&ui);
                let cfg = ui.draft.as_ref().clone();
                ui.form_error = None;
                {
                    let mut s = ui.state.write();
                    s.config = cfg.clone();
                    s.settings_message = None;
                    s.last_error = None;
                }
                let _ = ui.cmd_tx.send(PipelineCommand::ApplyConfig(Box::new(cfg)));
            }
            cx.refresh();
        }
    })?;
    let reload = button(&t!("action.reload"), {
        let cx = cx.clone();
        move || {
            cx.with_mut(|ui| {
                stop_hotkey_edit(ui);
                if is_settings_dirty(ui) {
                    ui.confirm = ConfirmAction::Reload;
                } else {
                    do_reload_from_disk(ui);
                }
            });
        }
    })?;
    reload.tooltip(&t!("save.reload_tip"))?;
    let discard = button(&t!("action.discard"), {
        let cx = cx.clone();
        move || {
            cx.with_mut(|ui| {
                stop_hotkey_edit(ui);
                if is_settings_dirty(ui) {
                    ui.confirm = ConfirmAction::Discard;
                } else {
                    do_discard(ui);
                }
            });
        }
    })?;
    discard.tooltip(&t!("save.discard_tip"))?;
    let actions = row(8.0)?;
    children!(actions; status_hint, save, reload, discard);

    let bar = grid(&[auto()], &[star(1.0), auto()])?;
    let title_cell = Border::new()?;
    title_cell
        .row(0)?
        .column(0)?
        .valign(VerticalAlignment::Center)?
        .margin(0.0, 0.0, 16.0, 0.0)?;
    title_cell.SetChild(&title_el)?;
    let actions_cell = Border::new()?;
    actions_cell
        .row(0)?
        .column(1)?
        .valign(VerticalAlignment::Center)?
        .halign(HorizontalAlignment::Right)?;
    actions_cell.SetChild(&actions)?;
    children!(bar; title_cell, actions_cell);
    let root = Border::new()?;
    root.SetPadding(thickness(24.0, 16.0, 24.0, 12.0))?;
    root.SetChild(&bar)?;

    let mut accent = Prop::default();
    let mut tip = Prop::default();
    let mut hint = Prop::default();
    b.add(move |_, snap| {
        let dirty = snap.settings_dirty;
        accent
            .set(dirty || snap.form_error.is_some(), |&a| button_style(&save, if a { ButtonKind::Accent } else { ButtonKind::Default }))?;
        let tooltip = if let Some(err) = snap.form_error.as_deref() {
            t!("save.cannot", error = err)
        } else if dirty {
            t!("save.unsaved")
        } else {
            t!("save.idle_tip")
        };
        tip.set(tooltip.into_owned(), |t| save.tooltip(t).map(drop))?;
        let (message, brush) = if let Some(err) = snap.form_error.as_ref() {
            (err.to_string(), Some(Theme::SystemCritical))
        } else if let Some(msg) = snap.settings_message.as_ref().filter(|s| !s.is_empty()) {
            let msg = if msg == SETTINGS_SAVED {
                t!("saved").into_owned()
            } else {
                msg.clone()
            };
            (msg, Some(Theme::Accent))
        } else {
            (String::new(), None)
        };
        hint.set((message, brush.map(|b| b as u8)), |(message, _)| {
            status_hint.SetText(&hs(message))?;
            match brush {
                Some(b) => foreground(&status_hint, b),
                None => status_hint.style(None).map(drop),
            }
        })
    });
    Ok(root)
}

/// Reload / Discard confirmation for settings pages.
pub fn confirm_dialog(b: &mut Binds<UiShared>, cx: &UiCx) -> Result<()> {
    let cx = cx.clone();
    let mut dialog = Dialog::new(move |primary| {
        cx.with_mut(|ui| {
            let action = ui.confirm;
            ui.confirm = ConfirmAction::None;
            if primary {
                match action {
                    ConfirmAction::Reload => do_reload_from_disk(ui),
                    ConfirmAction::Discard => do_discard(ui),
                    ConfirmAction::None => {}
                }
            }
        });
    })?;
    b.add(move |_, snap| {
        let (title, body, primary) = match snap.confirm {
            ConfirmAction::Reload => (t!("confirm.reload_title"), t!("confirm.reload_body"), t!("action.reload")),
            ConfirmAction::Discard => (t!("confirm.discard_title"), t!("confirm.discard_body"), t!("action.discard")),
            ConfirmAction::None => (Cow::Borrowed(""), Cow::Borrowed(""), t!("action.ok")),
        };
        let open = snap.confirm != ConfirmAction::None;
        if open {
            dialog.labels(&title, &primary, &t!("action.cancel"))?;
            dialog.body(&body)?;
        }
        dialog.show(open)
    });
    Ok(())
}

pub(crate) fn status_text(status: &PipelineStatus) -> Cow<'static, str> {
    match status {
        PipelineStatus::Idle => t!("status.idle"),
        PipelineStatus::Capturing => t!("status.capturing"),
        PipelineStatus::RunningOcr => t!("status.running_ocr"),
        PipelineStatus::WaitingForStable { elapsed_ms } => t!("status.waiting_stable", elapsed_ms = elapsed_ms),
        PipelineStatus::DownloadingModels {
            file,
            file_index,
            file_count,
            percent,
        } => {
            t!("status.downloading", file = file, file_index = file_index, file_count = file_count, percent = percent)
        }
        PipelineStatus::LoadingModels => t!("status.loading_models"),
        PipelineStatus::Translating => t!("status.translating"),
        PipelineStatus::RetryingTranslate { attempt, max_retries, .. } => {
            t!("status.retrying", attempt = attempt, max_retries = max_retries)
        }
        PipelineStatus::Cancelled => t!("status.cancelled"),
        PipelineStatus::OverlayActive => t!("status.overlay_active"),
        PipelineStatus::Paused => t!("status.paused"),
        PipelineStatus::Error { message } => t!("status.error", message = message),
    }
}

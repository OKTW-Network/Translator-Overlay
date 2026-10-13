//! Glossary editor as a ContentDialog on the control window.

use std::sync::Arc;

use parking_lot::Mutex;
use rust_i18n::t;
use translator_core::DictionaryPair;
use windows_reactor::{
    AutomationExt, Border, Button, ButtonStyle, ChildrenControl, ComboBox, ContentControl, ContentDialog, ContentDialogResult, FontIcon,
    FontWeight, Grid, GridChildExt, GridLength, HorizontalAlignment, InfoBar, InfoBarSeverity, KeyedView, LayoutControl, LocalSender,
    Orientation, ScrollBarVisibility, ScrollViewer, SlotsControl, StackPanel, Stretch, TextBlock, TextBox, Thickness, VerticalAlignment,
    View, Viewbox, ViewboxSlot,
};

use crate::ui::{
    named::{NamedSnap, name_entry_row, named_confirm_inline},
    shared::{AppMsg, DICTIONARIES, NamedDialog, UiCx, UiShared, commit_named, select_dictionary_profile, start_new_dictionary},
};

pub fn dictionary_editor_dialog(shared: &Arc<Mutex<UiShared>>, bump: &LocalSender<AppMsg>) -> View {
    let cx = UiCx::new(shared, bump);
    let (open, snap, pairs, last_error, can_edit, combo_settle) = {
        let mut ui = shared.lock();
        let combo_settle = if ui.dictionary.combo_armed {
            ui.dictionary.suppress_combo = false;
            ui.dictionary.combo_armed = false;
            false
        } else if ui.dictionary.suppress_combo {
            ui.dictionary.combo_armed = true;
            true
        } else {
            false
        };
        (
            ui.dictionary.editor_open,
            NamedSnap::take(&mut ui, &DICTIONARIES),
            ui.dictionary.pairs.clone(),
            ui.state.read().last_error.clone(),
            ui.dictionary.loaded.is_some(),
            combo_settle,
        )
    };
    if combo_settle {
        let _ = bump.send(AppMsg::Refresh);
    }

    // reactor forbids Tooltip/Flyout inside ContentDialog (`StructureUnsupported`).
    let dialog = ContentDialog::new()
        .title(t!("tr.editor_title"))
        .primary_button_text(t!("action.save"))
        .close_button_text(t!("action.cancel"))
        .is_primary_button_enabled(can_edit)
        .is_open(open)
        .on_closed({
            let cx = cx.clone();
            move |result: ContentDialogResult| {
                let mut reopen = false;
                cx.with_mut(|ui| {
                    if !ui.dictionary.editor_open {
                        return;
                    }
                    if result == ContentDialogResult::Primary {
                        if let Some(name) = ui.dictionary.loaded.clone() {
                            commit_named(ui, &DICTIONARIES, name);
                        }
                        if ui.dictionary.dirty() {
                            ui.dictionary.close_editor();
                            reopen = true;
                            return;
                        }
                    } else {
                        ui.dictionary.revert();
                    }
                    ui.dictionary.close_editor();
                });
                if reopen {
                    let _ = cx.bump.send(AppMsg::OpenDictionaryEditor);
                }
            }
        });

    dialog.content(editor_body(&cx, &snap, &pairs, last_error, can_edit))
}

fn editor_body(cx: &UiCx, snap: &NamedSnap, pairs: &[DictionaryPair], last_error: Option<String>, can_edit: bool) -> View {
    let combo = ComboBox::new()
        .items_source(snap.names.clone())
        .selected_index(snap.selected())
        .placeholder_text(t!("tr.select_dictionary"))
        .is_enabled(!snap.names.is_empty())
        .on_selection_changed({
            let cx = cx.clone();
            move |idx: Option<usize>| {
                cx.with_mut(|ui| select_dictionary_profile(ui, idx));
            }
        })
        .automation_id("cmb-dictionary-editor")
        .width(180.0)
        .min_width(140.0)
        .vertical_alignment(VerticalAlignment::Center);
    let toolbar = StackPanel::new()
        .orientation(Orientation::Horizontal)
        .spacing(8.0)
        .vertical_alignment(VerticalAlignment::Center)
        .horizontal_alignment(HorizontalAlignment::Left)
        .children((
            combo,
            Button::new()
                .on_click({
                    let cx = cx.clone();
                    move || {
                        cx.with_mut(start_new_dictionary);
                    }
                })
                .automation_id("btn-dictionary-new")
                .content(t!("action.new").as_ref()),
            Button::new()
                .is_enabled(snap.selected().is_some())
                .on_click({
                    let cx = cx.clone();
                    move || {
                        cx.with_mut(|ui| {
                            if let Some(name) = ui.dictionary.selected_name() {
                                ui.dictionary.pick.dialog = NamedDialog::Delete { name };
                            }
                        });
                    }
                })
                .content(t!("action.delete").as_ref()),
        ));

    let error_bar = InfoBar::new()
        .title(last_error.clone().unwrap_or_default())
        .severity(InfoBarSeverity::Error)
        .is_open(last_error.as_ref().is_some_and(|s| !s.is_empty()))
        .is_closable(false);

    // Pixel columns so ContentScrollViewer's infinite measure cannot stretch
    // the row past the default ContentDialog (max 548, ~24 padding each side).
    // Subtle + FontIcon clips if the cell is locked to 36×36; min 36 + Viewbox 16.
    const FIELD: f64 = 220.0;
    const BTN: f64 = 40.0;
    const ROW: f64 = FIELD * 2.0 + BTN;
    let pair_cols = [GridLength::Pixel(FIELD), GridLength::Pixel(FIELD), GridLength::Pixel(BTN)];
    let pair_row = || {
        Grid::new()
            .columns(pair_cols)
            .width(ROW)
            .horizontal_alignment(HorizontalAlignment::Left)
    };
    let glyph_icon = |glyph: &str| {
        Viewbox::new()
            .width(16.0)
            .height(16.0)
            .stretch(Stretch::Uniform)
            .horizontal_alignment(HorizontalAlignment::Center)
            .vertical_alignment(VerticalAlignment::Center)
            .slot(ViewboxSlot::Child, FontIcon::new().glyph(glyph))
    };
    let headers = pair_row().children((
        TextBlock::new()
            .text(t!("tr.term_source"))
            .font_weight(FontWeight::SEMI_BOLD)
            .grid_column(0)
            .margin(Thickness::new(0.0, 0.0, 8.0, 0.0)),
        TextBlock::new()
            .text(t!("tr.term_target"))
            .font_weight(FontWeight::SEMI_BOLD)
            .grid_column(1)
            .margin(Thickness::new(0.0, 0.0, 8.0, 0.0)),
        Border::new().width(BTN).grid_column(2),
    ));

    let mut pair_rows: Vec<KeyedView> = pairs
        .iter()
        .enumerate()
        .map(|(i, pair)| {
            KeyedView::new(
                i,
                pair_row().children((
                    Border::new()
                        .grid_column(0)
                        .horizontal_alignment(HorizontalAlignment::Stretch)
                        .margin(Thickness::new(0.0, 0.0, 8.0, 0.0))
                        .content(
                            TextBox::new()
                                .text(pair.source.clone())
                                .placeholder_text(t!("tr.term_source"))
                                .is_enabled(can_edit)
                                .automation_id(format!("txt-dictionary-source-{i}"))
                                .on_text_changed({
                                    let cx = cx.clone();
                                    move |text: String| {
                                        cx.with_mut(|ui| {
                                            if let Some(row) = ui.dictionary.pairs.get_mut(i) {
                                                row.source = text;
                                            }
                                        });
                                    }
                                })
                                .horizontal_alignment(HorizontalAlignment::Stretch),
                        ),
                    Border::new()
                        .grid_column(1)
                        .horizontal_alignment(HorizontalAlignment::Stretch)
                        .margin(Thickness::new(0.0, 0.0, 8.0, 0.0))
                        .content(
                            TextBox::new()
                                .text(pair.target.clone())
                                .placeholder_text(t!("tr.term_target"))
                                .is_enabled(can_edit)
                                .automation_id(format!("txt-dictionary-target-{i}"))
                                .on_text_changed({
                                    let cx = cx.clone();
                                    move |text: String| {
                                        cx.with_mut(|ui| {
                                            if let Some(row) = ui.dictionary.pairs.get_mut(i) {
                                                row.target = text;
                                            }
                                        });
                                    }
                                })
                                .horizontal_alignment(HorizontalAlignment::Stretch),
                        ),
                    Border::new()
                        .grid_column(2)
                        .horizontal_alignment(HorizontalAlignment::Center)
                        .vertical_alignment(VerticalAlignment::Center)
                        .content(
                            Button::new()
                                .style(ButtonStyle::Subtle)
                                .min_width(36.0)
                                .min_height(36.0)
                                .is_enabled(can_edit)
                                .horizontal_content_alignment(HorizontalAlignment::Center)
                                .vertical_content_alignment(VerticalAlignment::Center)
                                .on_click({
                                    let cx = cx.clone();
                                    move || {
                                        cx.with_mut(|ui| {
                                            if i < ui.dictionary.pairs.len() {
                                                ui.dictionary.pairs.remove(i);
                                            }
                                        });
                                    }
                                })
                                .automation_name(t!("action.delete"))
                                .content(glyph_icon("\u{E74D}")),
                        ),
                )),
            )
        })
        .collect();

    pair_rows.push(KeyedView::new(
        "add",
        pair_row().children((
            Border::new().height(BTN).grid_column(0),
            Border::new().height(BTN).grid_column(1),
            Border::new()
                .grid_column(2)
                .horizontal_alignment(HorizontalAlignment::Center)
                .vertical_alignment(VerticalAlignment::Center)
                .content(
                    Button::new()
                        .style(ButtonStyle::Subtle)
                        .min_width(36.0)
                        .min_height(36.0)
                        .is_enabled(can_edit)
                        .horizontal_content_alignment(HorizontalAlignment::Center)
                        .vertical_content_alignment(VerticalAlignment::Center)
                        .on_click({
                            let cx = cx.clone();
                            move || {
                                cx.with_mut(|ui| ui.dictionary.pairs.push(DictionaryPair::default()));
                            }
                        })
                        .automation_id("btn-dictionary-add")
                        .automation_name(t!("action.add"))
                        .content(glyph_icon("\u{E710}")),
                ),
        )),
    ));

    // Default VerticalScrollBarVisibility is Visible. Pin height to the rows so
    // Auto does not paint a bar when ContentDialog measures this with infinite height.
    const PAIR_ROW: f64 = 40.0;
    const PAIR_GAP: f64 = 8.0;
    const PAIR_MAX: f64 = 320.0;
    let n = (pairs.len() + 1) as f64;
    let content_h = n * PAIR_ROW + (n - 1.0).max(0.0) * PAIR_GAP;
    let pairs_list = ScrollViewer::new()
        .width(ROW)
        .height(content_h.min(PAIR_MAX))
        .horizontal_scroll_bar_visibility(ScrollBarVisibility::Disabled)
        .vertical_scroll_bar_visibility(ScrollBarVisibility::Auto)
        .content(
            StackPanel::new()
                .spacing(PAIR_GAP)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .keyed_children(pair_rows),
        );

    StackPanel::new()
        .width(ROW)
        .spacing(8.0)
        .horizontal_alignment(HorizontalAlignment::Left)
        .children((
            toolbar,
            name_entry_row(cx, &DICTIONARIES, snap),
            named_confirm_inline(cx, &DICTIONARIES, snap),
            error_bar,
            headers,
            pairs_list,
        ))
}

//! Settings card layout: section labels and header/control rows.

use translator_winui::{
    Border, FontWeight, HorizontalAlignment, Interface, Layout, Result, StackPanel, TextBlock, TextWrapping, UIElement, VerticalAlignment,
    auto, corners, grid, star, uniform,
};

use crate::{
    children,
    ui::widgets::{Theme, column, muted, text, themed_border},
};

pub const SEMI_BOLD: FontWeight = FontWeight { Weight: 600 };
pub const BOLD: FontWeight = FontWeight { Weight: 700 };

/// A card plus its description line, for descriptions that change with state.
pub struct Card {
    pub root: Border,
    pub desc: Option<TextBlock>,
}

fn header_text(s: &str) -> Result<TextBlock> {
    let tb = text(s)?;
    tb.SetFontWeight(SEMI_BOLD)?;
    tb.SetFontSize(14.0)?;
    Ok(tb)
}

fn labeled_stack(header: &TextBlock, description: Option<&str>) -> Result<(StackPanel, Option<TextBlock>)> {
    let panel = column(0.0)?;
    panel.valign(VerticalAlignment::Center)?;
    let desc = match description {
        Some(d) if !d.is_empty() => {
            panel.SetSpacing(2.0)?;
            let tb = muted(d)?;
            tb.SetTextWrapping(TextWrapping::WrapWholeWords)?;
            children!(panel; *header, tb);
            Some(tb)
        }
        _ => {
            children!(panel; *header);
            None
        }
    };
    Ok((panel, desc))
}

/// Section label above a group of cards.
pub fn section_header(title: &str) -> Result<TextBlock> {
    let tb = text(title)?;
    tb.SetFontSize(14.0)?;
    tb.SetFontWeight(SEMI_BOLD)?;
    tb.margin(0.0, 12.0, 0.0, 4.0)?;
    Ok(tb)
}

/// Label for a cluster of cards inside a section (not a peer of [`section_header`]).
pub fn subsection_header(title: &str) -> Result<TextBlock> {
    let tb = muted(title)?;
    tb.SetFontWeight(SEMI_BOLD)?;
    tb.margin(0.0, 8.0, 0.0, 2.0)?;
    Ok(tb)
}

fn card_border(key: &str) -> Result<Border> {
    let border = themed_border(Some(Theme::CardBackground), None)?;
    border.automation_id(key)?;
    border.SetCornerRadius(corners(8.0))?;
    border.SetPadding(uniform(16.0))?;
    Ok(border)
}

/// Header + Description on the left, control flush-right; `below` (if any) under that row.
pub fn settings_card_with_below(
    key: &str,
    header: &str,
    description: Option<&str>,
    control: &impl Interface,
    below: Option<&UIElement>,
) -> Result<Card> {
    let (left, desc) = labeled_stack(&header_text(header)?, description)?;
    let body = grid(&[auto()], &[star(1.0), auto()])?;
    let left_cell = Border::new()?;
    left_cell
        .row(0)?
        .column(0)?
        .valign(VerticalAlignment::Center)?
        .margin(0.0, 0.0, 16.0, 0.0)?;
    left_cell.SetChild(&left)?;
    let right_cell = Border::new()?;
    right_cell
        .row(0)?
        .column(1)?
        .halign(HorizontalAlignment::Right)?
        .valign(VerticalAlignment::Center)?;
    right_cell.SetChild(&control.cast::<UIElement>()?)?;
    children!(body; left_cell, right_cell);
    let root = card_border(key)?;
    match below {
        Some(below) => {
            let stack = column(10.0)?;
            children!(stack; body, *below);
            root.SetChild(&stack)?;
        }
        None => root.SetChild(&body)?,
    }
    Ok(Card { root, desc })
}

/// Standalone settings card: Header + Description on the left, control flush-right.
pub fn settings_card(key: &str, header: &str, description: Option<&str>, control: &impl Interface) -> Result<Card> {
    settings_card_with_below(key, header, description, control, None)
}

/// Card with header/description on top and full-width content below (multiline text).
pub fn settings_card_stack(key: &str, header: &str, description: Option<&str>, content: &impl Interface) -> Result<Card> {
    let (head, desc) = labeled_stack(&header_text(header)?, description)?;
    let stack = column(12.0)?;
    children!(stack; head, content.cast::<UIElement>()?);
    let root = card_border(key)?;
    root.SetChild(&stack)?;
    Ok(Card { root, desc })
}

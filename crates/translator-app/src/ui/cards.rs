//! Settings card layout, with section labels and rows of a header and a control.

use windows_reactor::{
    AutomationExt, Border, ChildrenControl, ContentControl, FontWeight, Grid, GridChildExt, GridLength, HorizontalAlignment, LayoutControl,
    StackPanel, TextBlock, TextWrapping, ThemeBrush, Thickness, VerticalAlignment, View,
};

fn labeled_stack(header: TextBlock, description: Option<&str>) -> View {
    match description {
        Some(d) if !d.is_empty() => StackPanel::new()
            .spacing(2.0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Center)
            .children((
                header,
                TextBlock::new()
                    .text(d)
                    .font_size(12.0)
                    .foreground(ThemeBrush::PrimaryText)
                    .opacity(0.72)
                    .text_wrapping(TextWrapping::WrapWholeWords),
            )),
        _ => StackPanel::new()
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .vertical_alignment(VerticalAlignment::Center)
            .children((header,)),
    }
}

/// Section label above a group of cards.
pub fn section_header(title: impl Into<String>) -> TextBlock {
    TextBlock::new()
        .text(title)
        .font_size(14.0)
        .font_weight(FontWeight::SEMI_BOLD)
        .margin(Thickness::new(0.0, 12.0, 0.0, 4.0))
}

/// Label for a group of cards inside a section. It sits below [`section_header`], not beside it.
pub fn subsection_header(title: impl Into<String>) -> TextBlock {
    TextBlock::new()
        .text(title)
        .font_size(12.0)
        .font_weight(FontWeight::SEMI_BOLD)
        .foreground(ThemeBrush::PrimaryText)
        .opacity(0.72)
        .margin(Thickness::new(0.0, 8.0, 0.0, 2.0))
}

fn settings_row_body(header: impl Into<String>, description: Option<&str>, control: impl Into<View>) -> View {
    let left = labeled_stack(TextBlock::new().text(header).font_weight(FontWeight::SEMI_BOLD).font_size(14.0), description);

    Grid::new()
        .rows([GridLength::Auto])
        .columns([GridLength::Star(1.0), GridLength::Auto])
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .children((
            Border::new()
                .grid_row(0)
                .grid_column(0)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .vertical_alignment(VerticalAlignment::Center)
                .margin(Thickness::new(0.0, 0.0, 16.0, 0.0))
                .content(left),
            Border::new()
                .grid_row(0)
                .grid_column(1)
                .horizontal_alignment(HorizontalAlignment::Right)
                .vertical_alignment(VerticalAlignment::Center)
                .content(control),
        ))
}

/// Standalone settings card with the header and description on the left and the control aligned right.
pub fn settings_card(key: &str, header: impl Into<String>, description: Option<&str>, control: impl Into<View>) -> View {
    settings_card_with_below(key, header, description, control, None)
}

/// [`settings_card`] with optional extra content under the header and control row.
pub fn settings_card_with_below(
    key: &str,
    header: impl Into<String>,
    description: Option<&str>,
    control: impl Into<View>,
    below: Option<View>,
) -> View {
    let row = settings_row_body(header, description, control);
    let content = if let Some(below) = below {
        StackPanel::new()
            .spacing(10.0)
            .horizontal_alignment(HorizontalAlignment::Stretch)
            .children((row, below))
    } else {
        row
    };
    Border::new()
        .automation_id(key)
        .background(ThemeBrush::CardBackground)
        .corner_radius(8.0)
        .padding(Thickness::uniform(16.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .content(content)
}

/// Card with the header and description on top and full-width content below, such as sliders or multiline text.
pub fn settings_card_stack(key: &str, header: impl Into<String>, description: Option<&str>, content: impl Into<View>) -> View {
    let head = labeled_stack(TextBlock::new().text(header).font_weight(FontWeight::SEMI_BOLD).font_size(14.0), description);

    Border::new()
        .automation_id(key)
        .background(ThemeBrush::CardBackground)
        .corner_radius(8.0)
        .padding(Thickness::uniform(16.0))
        .horizontal_alignment(HorizontalAlignment::Stretch)
        .content(
            StackPanel::new()
                .spacing(12.0)
                .horizontal_alignment(HorizontalAlignment::Stretch)
                .children((head, content.into())),
        )
}

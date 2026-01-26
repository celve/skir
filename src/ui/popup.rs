//! Install input popup rendering.

use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Clear, Padding, Paragraph},
};

use crate::app::App;
use super::theme;

/// Draw the install input popup.
pub fn draw_install_input(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Calculate popup dimensions
    let popup_width = 60.min(area.width.saturating_sub(4));
    let popup_height = 5;
    let popup_x = (area.width.saturating_sub(popup_width)) / 2;
    let popup_y = (area.height.saturating_sub(popup_height)) / 2;

    let popup_area = Rect::new(popup_x, popup_y, popup_width, popup_height);

    // Clear the area behind the popup
    frame.render_widget(Clear, popup_area);

    // Draw the input box
    let input_text = format!("{}_", app.input);
    let input = Paragraph::new(input_text)
        .style(Style::default().fg(theme::TEXT))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(theme::ACCENT))
                .title(" Enter Git URL ")
                .title_style(Style::default().fg(theme::ACCENT))
                .title_bottom(Line::from(" Esc to cancel ").centered())
                .padding(Padding::horizontal(1)),
        );
    frame.render_widget(input, popup_area);
}

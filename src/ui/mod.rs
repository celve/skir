//! UI rendering for the Silk TUI.

mod theme;
mod lists;
mod popup;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{App, View};

/// Main draw function.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // Title
            Constraint::Min(5),    // Content
            Constraint::Length(2), // Status
            Constraint::Length(2), // Help
        ])
        .split(area);

    draw_title(frame, chunks[0]);
    draw_content(frame, chunks[1], app);
    draw_status_bar(frame, chunks[2], app);
    draw_help_bar(frame, chunks[3], app);

    if app.view == View::InstallInput {
        popup::draw_install_input(frame, app);
    }
}

/// Draw the title bar.
fn draw_title(frame: &mut Frame, area: Rect) {
    let title = Paragraph::new("silk - Plugin Manager")
        .style(Style::default().fg(theme::ACCENT).bold())
        .alignment(Alignment::Center);
    frame.render_widget(title, area);
}

/// Draw the main content area based on current view.
fn draw_content(frame: &mut Frame, area: Rect, app: &mut App) {
    match app.view {
        View::PluginList | View::InstallInput => lists::draw_plugin_list(frame, area, app),
        View::SkillList => lists::draw_skill_list(frame, area, app),
    }
}

/// Draw the status bar.
fn draw_status_bar(frame: &mut Frame, area: Rect, app: &App) {
    let status_text = app.status.as_deref().unwrap_or("Ready");

    let color = if status_text.starts_with("Error") || status_text.contains("failed") {
        theme::ERROR
    } else if status_text == "Ready" {
        theme::SUCCESS
    } else {
        theme::ACCENT
    };

    let status = Paragraph::new(format!(" {}", status_text))
        .style(Style::default().fg(color))
        .block(
            Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(theme::BORDER)),
        );
    frame.render_widget(status, area);
}

/// Draw the help bar.
fn draw_help_bar(frame: &mut Frame, area: Rect, app: &App) {
    let help_text = match app.view {
        View::PluginList => "i:install  d:delete  r:refresh  l:view  q:quit",
        View::SkillList => "j/k:navigate  l:link  h:back  q:quit",
        View::InstallInput => "enter:submit  esc:cancel",
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(theme::TEXT_DIM))
        .alignment(Alignment::Center);
    frame.render_widget(help, area);
}

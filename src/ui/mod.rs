//! UI rendering for the Silk TUI.

mod theme;
mod lists;

use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph},
};

use crate::app::{App, View};
use crate::status::StatusKind;

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
    let status_text = app.status.get_display();

    let color = match app.status.display_kind() {
        StatusKind::Error => theme::ERROR,
        StatusKind::Success => theme::SUCCESS,
        StatusKind::Progress => theme::ACCENT,
        StatusKind::Info => theme::TEXT_DIM,
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
    // Show search bar instead of help when searching
    if app.search_active {
        draw_search_bar(frame, area, app);
        return;
    }

    // Show install bar when in install input mode
    if app.view == View::InstallInput {
        draw_install_bar(frame, area, app);
        return;
    }

    let help_text = match app.view {
        View::PluginList => "/:search  i:install  d:delete  r:refresh  u:update  l:view  q:quit",
        View::SkillList => "/:search  j/k:navigate  l:link  h:back  q:quit",
        View::InstallInput => unreachable!(),
    };

    let help = Paragraph::new(help_text)
        .style(Style::default().fg(theme::TEXT_DIM))
        .alignment(Alignment::Center);
    frame.render_widget(help, area);
}

/// Draw the search bar.
fn draw_search_bar(frame: &mut Frame, area: Rect, app: &App) {
    let text = format!("/{}_", app.search_query);
    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(theme::ACCENT));
    frame.render_widget(paragraph, area);
}

/// Draw the install input bar.
fn draw_install_bar(frame: &mut Frame, area: Rect, app: &App) {
    let text = format!("git url: {}_", app.input);
    let paragraph = Paragraph::new(text)
        .style(Style::default().fg(theme::ACCENT));
    frame.render_widget(paragraph, area);
}

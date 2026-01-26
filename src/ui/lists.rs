//! Plugin and skill list rendering.

use ratatui::{
    prelude::*,
    widgets::{List, ListItem, ListState, Paragraph},
};

use crate::app::App;
use super::theme;

/// Create a selection indicator span.
fn selection_indicator(is_selected: bool) -> Span<'static> {
    let (text, color) = if is_selected {
        ("> ", theme::ACCENT)
    } else {
        ("  ", theme::TEXT_DIM)
    };
    Span::styled(text, Style::default().fg(color))
}

/// Draw the plugin list.
pub fn draw_plugin_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let total_count = app.plugins.len() + app.installing.len();
    let filtered_indices = app.filtered_plugin_indices();
    let filtered_count = filtered_indices.len() + if app.search_query.is_empty() { app.installing.len() } else { 0 };

    let header_text = if app.search_active && !app.search_query.is_empty() {
        format!("Plugins ({} of {})", filtered_count, total_count)
    } else {
        format!("Plugins ({})", total_count)
    };

    // Split area for header and list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    // Draw header
    let header = Paragraph::new(header_text)
        .style(Style::default().fg(theme::TEXT_DIM));
    frame.render_widget(header, chunks[0]);

    if app.plugins.is_empty() && app.installing.is_empty() {
        let message = Paragraph::new("No plugins installed. Press 'i' to install a plugin.")
            .style(Style::default().fg(theme::TEXT_DIM));
        frame.render_widget(message, chunks[1]);
        return;
    }

    // Build filtered list items
    let mut items: Vec<ListItem> = filtered_indices
        .iter()
        .map(|&i| {
            let plugin = &app.plugins[i];
            let is_selected = i == app.selected_plugin;
            let skills = plugin.skills();
            let total = skills.len();
            let linked = skills.iter().filter(|s| s.is_linked()).count();

            let line = Line::from(vec![
                selection_indicator(is_selected),
                Span::styled(
                    format!("{}/{}", plugin.owner, plugin.name()),
                    Style::default().fg(if is_selected { theme::ACCENT } else { theme::TEXT }),
                ),
                Span::styled(
                    format!("  [{}/{} linked]", linked, total),
                    Style::default().fg(theme::TEXT_DIM),
                ),
            ]);

            ListItem::new(line)
        })
        .collect();

    // Add installing entries after regular plugins (only when not filtering)
    if app.search_query.is_empty() {
        for (i, (url, _)) in app.installing.iter().enumerate() {
            let idx = app.plugins.len() + i;
            let is_selected = idx == app.selected_plugin;

            let line = Line::from(vec![
                selection_indicator(is_selected),
                Span::styled(
                    url.clone(),
                    Style::default().fg(if is_selected { theme::ACCENT } else { theme::TEXT }),
                ),
                Span::styled("  [installing]", Style::default().fg(theme::ACCENT)),
            ]);

            items.push(ListItem::new(line));
        }
    }

    // Find the position of selected item in the filtered list for proper scrolling
    let selected_position = filtered_indices
        .iter()
        .position(|&i| i == app.selected_plugin);

    let mut list_state = ListState::default().with_selected(selected_position);
    let list = List::new(items);
    frame.render_stateful_widget(list, chunks[1], &mut list_state);
}

/// Draw the skill list for the selected plugin.
pub fn draw_skill_list(frame: &mut Frame, area: Rect, app: &mut App) {
    let Some(plugin) = app.selected_plugin() else {
        return;
    };

    let skills = plugin.skills();
    let filtered_indices = app.filtered_skill_indices();

    let header_text = if app.search_active && !app.search_query.is_empty() {
        format!("{}/{} ({} of {} skills)", plugin.owner, plugin.name(), filtered_indices.len(), skills.len())
    } else {
        format!("{}/{}", plugin.owner, plugin.name())
    };

    // Split area for header and list
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(area);

    // Draw header
    let header = Paragraph::new(header_text)
        .style(Style::default().fg(theme::TEXT_DIM));
    frame.render_widget(header, chunks[0]);

    if skills.is_empty() {
        let message = Paragraph::new("No skills in this plugin.")
            .style(Style::default().fg(theme::TEXT_DIM));
        frame.render_widget(message, chunks[1]);
        return;
    }

    // Build filtered list items
    let items: Vec<ListItem> = filtered_indices
        .iter()
        .map(|&i| {
            let skill = &skills[i];
            let is_selected = i == app.selected_skill;
            let is_linked = skill.is_linked();

            let mut spans = vec![
                selection_indicator(is_selected),
                Span::styled(
                    skill.name.clone(),
                    Style::default().fg(if is_selected { theme::ACCENT } else { theme::TEXT }),
                ),
            ];

            if is_linked {
                spans.push(Span::styled("  [linked]", Style::default().fg(theme::SUCCESS)));
            }

            // Show description for selected skill
            if is_selected {
                if let Some(desc) = &skill.description {
                    spans.push(Span::styled(
                        format!("  {}", desc),
                        Style::default().fg(theme::TEXT_DIM),
                    ));
                }
            }

            ListItem::new(Line::from(spans))
        })
        .collect();

    // Find the position of selected item in the filtered list for proper scrolling
    let selected_position = filtered_indices
        .iter()
        .position(|&i| i == app.selected_skill);

    let mut list_state = ListState::default().with_selected(selected_position);
    let list = List::new(items);
    frame.render_stateful_widget(list, chunks[1], &mut list_state);
}

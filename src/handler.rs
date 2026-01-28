use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::app::{App, View};

/// Handle a key event and update app state.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match app.view {
        View::PluginList => handle_plugin_list_key(app, key),
        View::SkillList => handle_skill_list_key(app, key),
        View::LinkTargetSelect => handle_link_target_key(app, key),
        View::InstallInput => handle_install_input_key(app, key),
    }
}

/// Handle keys in the plugin list view.
fn handle_plugin_list_key(app: &mut App, key: KeyEvent) {
    if app.search_active {
        handle_search_input(app, key);
        return;
    }

    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) => app.should_quit = true,
        (KeyCode::Char('j'), _) | (KeyCode::Down, _) => app.select_next(),
        (KeyCode::Char('k'), _) | (KeyCode::Up, _) => app.select_prev(),
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => app.scroll_down(),
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => app.scroll_up(),
        (KeyCode::Enter, _) | (KeyCode::Char('l'), _) => app.enter_skill_list(),
        (KeyCode::Char('i'), _) => app.enter_install_input(),
        (KeyCode::Char('d'), _) => app.delete_selected(),
        (KeyCode::Char('r'), _) => app.refresh(),
        (KeyCode::Char('u'), _) => app.update_selected(),
        (KeyCode::Char('/'), _) => app.enter_search(),
        _ => {}
    }
}

/// Handle keys in the skill list view.
fn handle_skill_list_key(app: &mut App, key: KeyEvent) {
    if app.search_active {
        handle_search_input(app, key);
        return;
    }

    match (key.code, key.modifiers) {
        (KeyCode::Char('q'), _) => app.should_quit = true,
        (KeyCode::Esc, _) | (KeyCode::Char('h'), _) => app.back_to_plugin_list(),
        (KeyCode::Char('j'), _) | (KeyCode::Down, _) => app.select_next(),
        (KeyCode::Char('k'), _) | (KeyCode::Up, _) => app.select_prev(),
        (KeyCode::Char('d'), KeyModifiers::CONTROL) => app.scroll_down(),
        (KeyCode::Char('u'), KeyModifiers::CONTROL) => app.scroll_up(),
        (KeyCode::Char('l'), _) | (KeyCode::Enter, _) => app.enter_link_target_view(),
        (KeyCode::Char('L'), _) => app.link_to_all_targets(),
        (KeyCode::Char('/'), _) => app.enter_search(),
        _ => {}
    }
}

/// Handle keys in the link target selection view.
fn handle_link_target_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc | KeyCode::Char('h') => app.back_to_skill_list(),
        KeyCode::Char('j') | KeyCode::Down => app.select_next(),
        KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
        KeyCode::Char('l') | KeyCode::Enter => app.toggle_selected_link_target(),
        _ => {}
    }
}

/// Handle keys in the install input view.
fn handle_install_input_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.back_to_plugin_list(),
        KeyCode::Enter => app.start_install(),
        KeyCode::Backspace => {
            if app.input.is_empty() {
                app.back_to_plugin_list();
            } else {
                app.input.pop();
            }
        }
        KeyCode::Char(c) => {
            app.input.push(c);
        }
        _ => {}
    }
}

/// Handle keys in search mode.
fn handle_search_input(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.exit_search();
        }
        KeyCode::Enter => {
            app.exit_search();
            // Also enter selection like 'l' would
            match app.view {
                View::PluginList => app.enter_skill_list(),
                View::SkillList => app.enter_link_target_view(),
                View::LinkTargetSelect | View::InstallInput => {}
            }
        }
        KeyCode::Backspace => {
            if app.search_query.is_empty() {
                app.exit_search();
            } else {
                app.search_backspace();
            }
        }
        KeyCode::Up => app.select_prev_filtered(),
        KeyCode::Down => app.select_next_filtered(),
        KeyCode::Char(c) => app.search_input(c),
        _ => {}
    }
}

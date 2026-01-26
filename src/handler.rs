use crossterm::event::{KeyCode, KeyEvent};

use crate::app::{App, View};

/// Handle a key event and update app state.
pub fn handle_key(app: &mut App, key: KeyEvent) {
    match app.view {
        View::PluginList => handle_plugin_list_key(app, key),
        View::SkillList => handle_skill_list_key(app, key),
        View::InstallInput => handle_install_input_key(app, key),
    }
}

/// Handle keys in the plugin list view.
fn handle_plugin_list_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Char('j') | KeyCode::Down => app.select_next(),
        KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
        KeyCode::Enter | KeyCode::Char('l') => app.enter_skill_list(),
        KeyCode::Char('i') => app.enter_install_input(),
        KeyCode::Char('d') => app.delete_selected(),
        KeyCode::Char('r') => app.refresh(),
        KeyCode::Char('u') => app.update_selected(),
        _ => {}
    }
}

/// Handle keys in the skill list view.
fn handle_skill_list_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Char('q') => app.should_quit = true,
        KeyCode::Esc | KeyCode::Char('h') => app.back_to_plugin_list(),
        KeyCode::Char('j') | KeyCode::Down => app.select_next(),
        KeyCode::Char('k') | KeyCode::Up => app.select_prev(),
        KeyCode::Char('l') => app.toggle_skill_link(),
        _ => {}
    }
}

/// Handle keys in the install input view.
fn handle_install_input_key(app: &mut App, key: KeyEvent) {
    match key.code {
        KeyCode::Esc => app.back_to_plugin_list(),
        KeyCode::Enter => app.start_install(),
        KeyCode::Backspace => {
            app.input.pop();
        }
        KeyCode::Char(c) => {
            app.input.push(c);
        }
        _ => {}
    }
}

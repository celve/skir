use std::sync::mpsc::Receiver;
use std::sync::Arc;

use ratatui::widgets::ListState;

use crate::plugin::{Plugin, PluginError, PluginManager};

/// The current view in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    PluginList,
    SkillList,
    InstallInput,
}

/// Application state.
pub struct App {
    pub manager: PluginManager,
    pub plugins: Vec<Arc<Plugin>>,
    pub installing: Vec<(String, Receiver<Result<Arc<Plugin>, PluginError>>)>,
    pub selected_plugin: usize,
    pub selected_skill: usize,
    pub plugin_list_state: ListState,
    pub skill_list_state: ListState,
    pub view: View,
    pub input: String,
    pub status: Option<String>,
    pub should_quit: bool,
    pub search_active: bool,
    pub search_query: String,
}

impl App {
    /// Create a new App instance.
    pub fn new() -> Result<Self, PluginError> {
        let manager = PluginManager::new()?;
        let plugins = manager.list_installed()?;

        Ok(Self {
            manager,
            plugins,
            installing: Vec::new(),
            selected_plugin: 0,
            selected_skill: 0,
            plugin_list_state: ListState::default().with_selected(Some(0)),
            skill_list_state: ListState::default().with_selected(Some(0)),
            view: View::PluginList,
            input: String::new(),
            status: None,
            should_quit: false,
            search_active: false,
            search_query: String::new(),
        })
    }

    /// Refresh the plugin list.
    pub fn refresh(&mut self) {
        match self.manager.list_installed() {
            Ok(plugins) => {
                self.plugins = plugins;
                self.selected_plugin = self.selected_plugin.min(self.plugins.len().saturating_sub(1));
                self.status = Some("Refreshed plugin list".to_string());
            }
            Err(e) => {
                self.status = Some(format!("Error: {}", e));
            }
        }
    }

    /// Start installing a plugin from the current input URL in the background.
    pub fn start_install(&mut self) {
        let url = self.input.trim().to_string();
        if url.is_empty() {
            self.status = Some("URL cannot be empty".to_string());
            return;
        }

        self.input.clear();
        self.view = View::PluginList;
        self.status = Some(format!("Installing {}...", url));

        let manager = self.manager.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let url_clone = url.clone();

        std::thread::spawn(move || {
            let result = manager.install(&url_clone);
            let _ = tx.send(result);
        });

        self.installing.push((url, rx));
    }

    /// Poll for completed background installations.
    pub fn poll_installs(&mut self) {
        let mut completed = Vec::new();

        for (i, (url, rx)) in self.installing.iter().enumerate() {
            if let Ok(result) = rx.try_recv() {
                completed.push((i, url.clone(), result));
            }
        }

        // Remove completed in reverse order to preserve indices
        for (i, url, result) in completed.into_iter().rev() {
            self.installing.remove(i);
            match result {
                Ok(plugin) => {
                    self.status = Some(format!("Installed: {}/{}", plugin.owner, plugin.name()));
                    self.plugins.push(plugin);
                }
                Err(e) => {
                    self.status = Some(format!("Install failed ({}): {}", url, e));
                }
            }
        }
    }

    /// Check if the selected item is a plugin being installed.
    pub fn is_selected_installing(&self) -> bool {
        self.selected_plugin >= self.plugins.len()
    }

    /// Delete the currently selected plugin.
    pub fn delete_selected(&mut self) {
        if self.plugins.is_empty() {
            self.status = Some("No plugin selected".to_string());
            return;
        }

        if self.is_selected_installing() {
            self.status = Some("Plugin is still installing".to_string());
            return;
        }

        let plugin = &self.plugins[self.selected_plugin];
        let name = format!("{}/{}", plugin.owner, plugin.name());

        match plugin.remove() {
            Ok(()) => {
                self.plugins.remove(self.selected_plugin);
                self.selected_plugin = self.selected_plugin.min(self.plugins.len().saturating_sub(1));
                self.status = Some(format!("Deleted: {}", name));
            }
            Err(e) => {
                self.status = Some(format!("Delete failed: {}", e));
            }
        }
    }

    /// Get the currently selected plugin.
    pub fn selected_plugin(&self) -> Option<&Arc<Plugin>> {
        self.plugins.get(self.selected_plugin)
    }

    /// Move selection up.
    pub fn select_prev(&mut self) {
        match self.view {
            View::PluginList => {
                if self.selected_plugin > 0 {
                    self.selected_plugin -= 1;
                    self.plugin_list_state.select(Some(self.selected_plugin));
                }
            }
            View::SkillList => {
                if self.selected_skill > 0 {
                    self.selected_skill -= 1;
                    self.skill_list_state.select(Some(self.selected_skill));
                }
            }
            View::InstallInput => {}
        }
    }

    /// Move selection down.
    pub fn select_next(&mut self) {
        match self.view {
            View::PluginList => {
                let total = self.plugins.len() + self.installing.len();
                if total > 0 && self.selected_plugin < total - 1 {
                    self.selected_plugin += 1;
                    self.plugin_list_state.select(Some(self.selected_plugin));
                }
            }
            View::SkillList => {
                if let Some(plugin) = self.selected_plugin() {
                    let skill_count = plugin.skills().len();
                    if skill_count > 0 && self.selected_skill < skill_count - 1 {
                        self.selected_skill += 1;
                        self.skill_list_state.select(Some(self.selected_skill));
                    }
                }
            }
            View::InstallInput => {}
        }
    }

    /// Scroll down by half a page (10 items).
    pub fn scroll_down(&mut self) {
        const SCROLL_AMOUNT: usize = 10;
        match self.view {
            View::PluginList => {
                let total = self.plugins.len() + self.installing.len();
                if total > 0 {
                    self.selected_plugin = (self.selected_plugin + SCROLL_AMOUNT).min(total - 1);
                    self.plugin_list_state.select(Some(self.selected_plugin));
                }
            }
            View::SkillList => {
                if let Some(plugin) = self.selected_plugin() {
                    let skill_count = plugin.skills().len();
                    if skill_count > 0 {
                        self.selected_skill = (self.selected_skill + SCROLL_AMOUNT).min(skill_count - 1);
                        self.skill_list_state.select(Some(self.selected_skill));
                    }
                }
            }
            View::InstallInput => {}
        }
    }

    /// Scroll up by half a page (10 items).
    pub fn scroll_up(&mut self) {
        const SCROLL_AMOUNT: usize = 10;
        match self.view {
            View::PluginList => {
                self.selected_plugin = self.selected_plugin.saturating_sub(SCROLL_AMOUNT);
                self.plugin_list_state.select(Some(self.selected_plugin));
            }
            View::SkillList => {
                self.selected_skill = self.selected_skill.saturating_sub(SCROLL_AMOUNT);
                self.skill_list_state.select(Some(self.selected_skill));
            }
            View::InstallInput => {}
        }
    }

    /// Enter skill list view for selected plugin.
    pub fn enter_skill_list(&mut self) {
        if self.is_selected_installing() {
            self.status = Some("Plugin is still installing".to_string());
            return;
        }
        if self.selected_plugin().is_some() {
            self.selected_skill = 0;
            self.skill_list_state.select(Some(0));
            self.view = View::SkillList;
        }
    }

    /// Enter install input view.
    pub fn enter_install_input(&mut self) {
        self.input.clear();
        self.view = View::InstallInput;
    }

    /// Go back to plugin list view.
    pub fn back_to_plugin_list(&mut self) {
        self.view = View::PluginList;
        self.input.clear();
    }

    /// Update the currently selected plugin.
    pub fn update_selected(&mut self) {
        if self.plugins.is_empty() {
            self.status = Some("No plugin selected".to_string());
            return;
        }

        if self.is_selected_installing() {
            self.status = Some("Plugin is still installing".to_string());
            return;
        }

        let plugin = &self.plugins[self.selected_plugin];
        let name = format!("{}/{}", plugin.owner, plugin.name());

        match plugin.update() {
            Ok(updated_plugin) => {
                self.plugins[self.selected_plugin] = Arc::new(updated_plugin);
                self.status = Some(format!("Updated: {}", name));
            }
            Err(e) => {
                self.status = Some(format!("Update failed: {}", e));
            }
        }
    }

    /// Toggle link/unlink for the currently selected skill.
    pub fn toggle_skill_link(&mut self) {
        let Some(plugin) = self.selected_plugin() else {
            return;
        };
        let skills = plugin.skills();
        if skills.is_empty() || self.selected_skill >= skills.len() {
            return;
        }

        let skill = &skills[self.selected_skill];
        if skill.is_linked() {
            match skill.unlink() {
                Ok(()) => self.status = Some(format!("Unlinked: {}", skill.name)),
                Err(e) => self.status = Some(format!("Unlink failed: {}", e)),
            }
        } else {
            match skill.link() {
                Ok(()) => self.status = Some(format!("Linked: {}", skill.name)),
                Err(e) => self.status = Some(format!("Link failed: {}", e)),
            }
        }
    }

    /// Enter search mode.
    pub fn enter_search(&mut self) {
        self.search_active = true;
        self.search_query.clear();
    }

    /// Exit search mode.
    pub fn exit_search(&mut self) {
        self.search_active = false;
        self.search_query.clear();
    }

    /// Add a character to the search query.
    pub fn search_input(&mut self, c: char) {
        self.search_query.push(c);
        self.select_first_filtered();
    }

    /// Remove the last character from the search query.
    pub fn search_backspace(&mut self) {
        self.search_query.pop();
        self.select_first_filtered();
    }

    /// Select the first item in filtered results.
    fn select_first_filtered(&mut self) {
        match self.view {
            View::PluginList => {
                let filtered = self.filtered_plugin_indices();
                if let Some(&first) = filtered.first() {
                    self.selected_plugin = first;
                    self.plugin_list_state.select(Some(first));
                }
            }
            View::SkillList => {
                let filtered = self.filtered_skill_indices();
                if let Some(&first) = filtered.first() {
                    self.selected_skill = first;
                    self.skill_list_state.select(Some(first));
                }
            }
            View::InstallInput => {}
        }
    }

    /// Get filtered plugin indices matching the search query.
    pub fn filtered_plugin_indices(&self) -> Vec<usize> {
        if self.search_query.is_empty() {
            return (0..self.plugins.len()).collect();
        }

        let query = self.search_query.to_lowercase();
        self.plugins
            .iter()
            .enumerate()
            .filter(|(_, plugin)| {
                let name = format!("{}/{}", plugin.owner, plugin.name()).to_lowercase();
                name.contains(&query)
            })
            .map(|(i, _)| i)
            .collect()
    }

    /// Get filtered skill indices matching the search query.
    pub fn filtered_skill_indices(&self) -> Vec<usize> {
        let Some(plugin) = self.selected_plugin() else {
            return Vec::new();
        };

        let skills = plugin.skills();
        if self.search_query.is_empty() {
            return (0..skills.len()).collect();
        }

        let query = self.search_query.to_lowercase();
        skills
            .iter()
            .enumerate()
            .filter(|(_, skill)| skill.name.to_lowercase().contains(&query))
            .map(|(i, _)| i)
            .collect()
    }

    /// Move selection up in filtered results.
    pub fn select_prev_filtered(&mut self) {
        match self.view {
            View::PluginList => {
                let filtered = self.filtered_plugin_indices();
                if filtered.is_empty() {
                    return;
                }
                let current_pos = filtered.iter().position(|&i| i == self.selected_plugin);
                if let Some(pos) = current_pos {
                    if pos > 0 {
                        self.selected_plugin = filtered[pos - 1];
                        self.plugin_list_state.select(Some(self.selected_plugin));
                    }
                } else if !filtered.is_empty() {
                    self.selected_plugin = filtered[0];
                    self.plugin_list_state.select(Some(self.selected_plugin));
                }
            }
            View::SkillList => {
                let filtered = self.filtered_skill_indices();
                if filtered.is_empty() {
                    return;
                }
                let current_pos = filtered.iter().position(|&i| i == self.selected_skill);
                if let Some(pos) = current_pos {
                    if pos > 0 {
                        self.selected_skill = filtered[pos - 1];
                        self.skill_list_state.select(Some(self.selected_skill));
                    }
                } else if !filtered.is_empty() {
                    self.selected_skill = filtered[0];
                    self.skill_list_state.select(Some(self.selected_skill));
                }
            }
            View::InstallInput => {}
        }
    }

    /// Move selection down in filtered results.
    pub fn select_next_filtered(&mut self) {
        match self.view {
            View::PluginList => {
                let filtered = self.filtered_plugin_indices();
                if filtered.is_empty() {
                    return;
                }
                let current_pos = filtered.iter().position(|&i| i == self.selected_plugin);
                if let Some(pos) = current_pos {
                    if pos < filtered.len() - 1 {
                        self.selected_plugin = filtered[pos + 1];
                        self.plugin_list_state.select(Some(self.selected_plugin));
                    }
                } else if !filtered.is_empty() {
                    self.selected_plugin = filtered[0];
                    self.plugin_list_state.select(Some(self.selected_plugin));
                }
            }
            View::SkillList => {
                let filtered = self.filtered_skill_indices();
                if filtered.is_empty() {
                    return;
                }
                let current_pos = filtered.iter().position(|&i| i == self.selected_skill);
                if let Some(pos) = current_pos {
                    if pos < filtered.len() - 1 {
                        self.selected_skill = filtered[pos + 1];
                        self.skill_list_state.select(Some(self.selected_skill));
                    }
                } else if !filtered.is_empty() {
                    self.selected_skill = filtered[0];
                    self.skill_list_state.select(Some(self.selected_skill));
                }
            }
            View::InstallInput => {}
        }
    }
}

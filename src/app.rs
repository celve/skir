use std::sync::mpsc::Receiver;
use std::sync::Arc;

use ratatui::widgets::ListState;

use crate::plugin::{GitSource, LinkTarget, Plugin, PluginError, PluginManager};
use crate::status::{StatusKind, StatusManager};

/// The current view in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    PluginList,
    SkillList,
    LinkTargetSelect,
    InstallInput,
}

/// Application state.
pub struct App {
    pub manager: PluginManager,
    pub plugins: Vec<Arc<Plugin>>,
    pub installing: Vec<(String, Receiver<Result<Arc<Plugin>, PluginError>>)>,
    pub updating: Vec<(usize, String, Receiver<Result<Plugin, PluginError>>)>,
    pub selected_plugin: usize,
    pub selected_skill: usize,
    pub plugin_list_state: ListState,
    pub skill_list_state: ListState,
    pub view: View,
    pub input: String,
    pub status: StatusManager,
    pub should_quit: bool,
    pub search_active: bool,
    pub search_query: String,
    pub link_target_selection: usize,
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
            updating: Vec::new(),
            selected_plugin: 0,
            selected_skill: 0,
            plugin_list_state: ListState::default().with_selected(Some(0)),
            skill_list_state: ListState::default().with_selected(Some(0)),
            view: View::PluginList,
            input: String::new(),
            status: StatusManager::new(),
            should_quit: false,
            search_active: false,
            search_query: String::new(),
            link_target_selection: 0,
        })
    }

    /// Refresh the plugin list.
    pub fn refresh(&mut self) {
        match self.manager.list_installed() {
            Ok(plugins) => {
                self.plugins = plugins;
                self.selected_plugin = self.selected_plugin.min(self.plugins.len().saturating_sub(1));
                self.status.add("refresh", "Refreshed plugin list", StatusKind::Success);
            }
            Err(e) => {
                self.status.add("refresh", format!("Error: {}", e), StatusKind::Error);
            }
        }
    }

    /// Start installing a plugin from the current input URL in the background.
    pub fn start_install(&mut self) {
        let url = self.input.trim().to_string();
        if url.is_empty() {
            self.status.add("install:error", "URL cannot be empty", StatusKind::Error);
            return;
        }

        // Parse URL to check if already installed
        let source = match GitSource::parse(&url) {
            Ok(s) => s,
            Err(e) => {
                self.status.add("install:error", format!("Invalid URL: {}", e), StatusKind::Error);
                return;
            }
        };

        // Check if already installed
        if self.manager.is_installed(&source) {
            self.input.clear();
            self.view = View::PluginList;
            self.status.add(
                format!("install:{}", url),
                format!("Already installed: {}/{}", source.owner, source.repo),
                StatusKind::Info,
            );
            return;
        }

        self.input.clear();
        self.view = View::PluginList;
        self.status.add(format!("install:{}", url), format!("Installing {}...", url), StatusKind::Progress);

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
            let status_id = format!("install:{}", url);
            match result {
                Ok(plugin) => {
                    self.status.add(&status_id, format!("Installed: {}/{}", plugin.owner, plugin.name()), StatusKind::Success);
                    self.plugins.push(plugin);
                }
                Err(e) => {
                    self.status.add(&status_id, format!("Install failed ({}): {}", url, e), StatusKind::Error);
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
            self.status.add("delete:error", "No plugin selected", StatusKind::Error);
            return;
        }

        if self.is_selected_installing() {
            self.status.add("delete:error", "Plugin is still installing", StatusKind::Error);
            return;
        }

        let plugin = &self.plugins[self.selected_plugin];
        let name = format!("{}/{}", plugin.owner, plugin.name());
        let status_id = format!("delete:{}", name);

        match plugin.remove() {
            Ok(()) => {
                self.plugins.remove(self.selected_plugin);
                self.selected_plugin = self.selected_plugin.min(self.plugins.len().saturating_sub(1));
                self.status.add(&status_id, format!("Deleted: {}", name), StatusKind::Success);
            }
            Err(e) => {
                self.status.add(&status_id, format!("Delete failed: {}", e), StatusKind::Error);
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
            View::LinkTargetSelect => {
                if self.link_target_selection > 0 {
                    self.link_target_selection -= 1;
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
            View::LinkTargetSelect => {
                let target_count = LinkTarget::all().len();
                if self.link_target_selection < target_count - 1 {
                    self.link_target_selection += 1;
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
            View::LinkTargetSelect | View::InstallInput => {}
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
            View::LinkTargetSelect | View::InstallInput => {}
        }
    }

    /// Enter skill list view for selected plugin.
    pub fn enter_skill_list(&mut self) {
        if self.is_selected_installing() {
            self.status.add("view:error", "Plugin is still installing", StatusKind::Error);
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
            self.status.add("update:error", "No plugin selected", StatusKind::Error);
            return;
        }

        if self.is_selected_installing() {
            self.status.add("update:error", "Plugin is still installing", StatusKind::Error);
            return;
        }

        let plugin = Arc::clone(&self.plugins[self.selected_plugin]);
        let name = format!("{}/{}", plugin.owner, plugin.name());
        let status_id = format!("update:{}", name);
        self.status.add(&status_id, format!("Updating {}...", name), StatusKind::Progress);

        let (tx, rx) = std::sync::mpsc::channel();
        let idx = self.selected_plugin;

        std::thread::spawn(move || {
            let result = plugin.update();
            let _ = tx.send(result);
        });

        self.updating.push((idx, name, rx));
    }

    /// Poll for completed background updates.
    pub fn poll_updates(&mut self) {
        let mut completed = Vec::new();

        for (i, (idx, name, rx)) in self.updating.iter().enumerate() {
            if let Ok(result) = rx.try_recv() {
                completed.push((i, *idx, name.clone(), result));
            }
        }

        // Remove completed in reverse order to preserve indices
        for (i, idx, name, result) in completed.into_iter().rev() {
            self.updating.remove(i);
            let status_id = format!("update:{}", name);
            match result {
                Ok(updated_plugin) => {
                    if idx < self.plugins.len() {
                        self.plugins[idx] = Arc::new(updated_plugin);
                    }
                    self.status.add(&status_id, format!("Updated: {}", name), StatusKind::Success);
                }
                Err(e) => {
                    self.status.add(&status_id, format!("Update failed: {}", e), StatusKind::Error);
                }
            }
        }
    }

    /// Enter the link target selection view for the currently selected skill.
    pub fn enter_link_target_view(&mut self) {
        let Some(plugin) = self.selected_plugin() else {
            return;
        };
        let skills = plugin.skills();
        if skills.is_empty() || self.selected_skill >= skills.len() {
            return;
        }

        self.link_target_selection = 0;
        self.view = View::LinkTargetSelect;
    }

    /// Toggle link/unlink for the currently selected link target.
    pub fn toggle_selected_link_target(&mut self) {
        let Some(plugin) = self.selected_plugin() else {
            return;
        };
        let skills = plugin.skills();
        if skills.is_empty() || self.selected_skill >= skills.len() {
            return;
        }

        let targets = LinkTarget::all();
        if self.link_target_selection >= targets.len() {
            return;
        }

        let target = targets[self.link_target_selection];
        let skill = &skills[self.selected_skill];
        let status_id = format!("link:{}:{}", target.display_name(), skill.name);

        if skill.is_linked_to(target) {
            match skill.unlink_from(target) {
                Ok(()) => self.status.add(
                    &status_id,
                    format!("Unlinked {} from {}", skill.name, target.display_name()),
                    StatusKind::Success,
                ),
                Err(e) => self.status.add(&status_id, format!("Unlink failed: {}", e), StatusKind::Error),
            }
        } else {
            match skill.link_to(target) {
                Ok(()) => self.status.add(
                    &status_id,
                    format!("Linked {} to {}", skill.name, target.display_name()),
                    StatusKind::Success,
                ),
                Err(e) => self.status.add(&status_id, format!("Link failed: {}", e), StatusKind::Error),
            }
        }
    }

    /// Go back to skill list from link target selection view.
    pub fn back_to_skill_list(&mut self) {
        self.view = View::SkillList;
    }

    /// Link or unlink the currently selected skill to/from all targets.
    /// If any target is not linked, links to all. If all are linked, unlinks from all.
    pub fn link_to_all_targets(&mut self) {
        let Some(plugin) = self.selected_plugin() else {
            return;
        };
        let skills = plugin.skills();
        if skills.is_empty() || self.selected_skill >= skills.len() {
            return;
        }

        let skill = &skills[self.selected_skill];
        let targets = LinkTarget::all();

        // Check if all targets are linked
        let all_linked = targets.iter().all(|t| skill.is_linked_to(*t));

        if all_linked {
            // Unlink from all
            for target in targets {
                if let Err(e) = skill.unlink_from(*target) {
                    self.status.add(
                        format!("link:all:{}", skill.name),
                        format!("Unlink from {} failed: {}", target.display_name(), e),
                        StatusKind::Error,
                    );
                    return;
                }
            }
            self.status.add(
                format!("link:all:{}", skill.name),
                format!("Unlinked {} from all targets", skill.name),
                StatusKind::Success,
            );
        } else {
            // Link to all unlinked targets
            for target in targets {
                if !skill.is_linked_to(*target) {
                    if let Err(e) = skill.link_to(*target) {
                        self.status.add(
                            format!("link:all:{}", skill.name),
                            format!("Link to {} failed: {}", target.display_name(), e),
                            StatusKind::Error,
                        );
                        return;
                    }
                }
            }
            self.status.add(
                format!("link:all:{}", skill.name),
                format!("Linked {} to all targets", skill.name),
                StatusKind::Success,
            );
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
            View::LinkTargetSelect | View::InstallInput => {}
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
            View::LinkTargetSelect | View::InstallInput => {}
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
            View::LinkTargetSelect | View::InstallInput => {}
        }
    }
}

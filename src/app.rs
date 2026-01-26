use std::sync::mpsc::Receiver;
use std::sync::Arc;

use ratatui::widgets::ListState;

use crate::plugin::{Installer, Plugin, PluginError};

/// The current view in the TUI.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    PluginList,
    SkillList,
    InstallInput,
}

/// Application state.
pub struct App {
    pub installer: Installer,
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
}

impl App {
    /// Create a new App instance.
    pub fn new() -> Result<Self, PluginError> {
        let installer = Installer::new()?;
        let plugins = installer.list_installed()?;

        Ok(Self {
            installer,
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
        })
    }

    /// Refresh the plugin list.
    pub fn refresh(&mut self) {
        match self.installer.list_installed() {
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

        let installer = self.installer.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        let url_clone = url.clone();

        std::thread::spawn(move || {
            let result = installer.install(&url_clone);
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

        match self.installer.remove(plugin) {
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

        match self.installer.update(plugin) {
            Ok(updated_plugin) => {
                self.plugins[self.selected_plugin] = updated_plugin;
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
}

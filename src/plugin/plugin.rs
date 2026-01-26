use std::path::PathBuf;

use super::skill::Skill;

/// A plugin with its identifier and discovered skills.
#[derive(Debug)]
pub struct Plugin {
    /// The git host, e.g., "github.com".
    pub host: String,
    /// The repository owner, e.g., "anthropics".
    pub owner: String,
    /// The repository name, e.g., "claude-code".
    pub repo: String,
    /// The local path where the plugin is installed.
    pub path: PathBuf,
    /// Skills discovered in this plugin (populated after Arc creation).
    skills: Vec<Skill>,
}

impl Plugin {
    /// Create a new plugin (without skills - they are added later).
    pub(crate) fn new(host: String, owner: String, repo: String, path: PathBuf) -> Self {
        Self {
            host,
            owner,
            repo,
            path,
            skills: Vec::new(),
        }
    }

    /// The plugin name (derived from the repository name).
    pub fn name(&self) -> &str {
        &self.repo
    }

    /// Get the skills discovered in this plugin.
    pub fn skills(&self) -> &[Skill] {
        &self.skills
    }

    /// Set the skills for this plugin.
    pub(crate) fn set_skills(&mut self, skills: Vec<Skill>) {
        self.skills = skills;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_name() {
        let plugin = Plugin::new(
            "github.com".to_string(),
            "anthropics".to_string(),
            "claude-code".to_string(),
            PathBuf::from("/test/path"),
        );
        assert_eq!(plugin.name(), "claude-code");
    }

    #[test]
    fn test_plugin_fields() {
        let plugin = Plugin::new(
            "github.com".to_string(),
            "anthropics".to_string(),
            "claude-code".to_string(),
            PathBuf::from("/test/path"),
        );
        assert_eq!(plugin.host, "github.com");
        assert_eq!(plugin.owner, "anthropics");
        assert_eq!(plugin.repo, "claude-code");
        assert_eq!(plugin.path, PathBuf::from("/test/path"));
        assert!(plugin.skills().is_empty());
    }
}

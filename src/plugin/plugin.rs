use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::error::PluginError;
use super::git::{git_clone, git_pull, is_git_repo};
use super::skill::{self, Skill};
use super::source::GitSource;

/// Extract the directory name from a path as a String.
fn dir_name(path: &Path) -> Option<String> {
    path.file_name()?.to_str().map(String::from)
}

/// Scan a directory for SKILL.md files.
///
/// Returns a list of (skill_name, skill_path) pairs.
pub(crate) fn scan_for_skills(root: &Path) -> Result<Vec<(String, PathBuf)>, PluginError> {
    let mut skills = Vec::new();
    scan_directory(root, root, &mut skills)?;
    Ok(skills)
}

/// Recursively scan for SKILL.md files.
fn scan_directory(
    root: &Path,
    current: &Path,
    skills: &mut Vec<(String, PathBuf)>,
) -> Result<(), PluginError> {
    for entry in fs::read_dir(current)? {
        let path = entry?.path();

        if path.is_dir() {
            // Skip VCS directories only (not all hidden directories)
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name != ".git" && name != ".svn" && name != ".hg" {
                    scan_directory(root, &path, skills)?;
                }
            }
        } else if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name == "SKILL.md" {
                    let skill_name = derive_skill_name(root, &path);
                    skills.push((skill_name, path));
                }
            }
        }
    }

    Ok(())
}

/// Derive the skill name from a SKILL.md file path.
///
/// - If SKILL.md is in a subdirectory, use the parent directory name.
/// - If SKILL.md is at the root, use the root directory name.
fn derive_skill_name(root: &Path, skill_path: &Path) -> String {
    if let Some(parent) = skill_path.parent() {
        if parent == root {
            // SKILL.md is at root, use root directory name
            dir_name(root).unwrap_or_else(|| "unknown".to_string())
        } else {
            // Use the parent directory name
            dir_name(parent).unwrap_or_else(|| "unknown".to_string())
        }
    } else {
        "unknown".to_string()
    }
}

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
    fn new(host: String, owner: String, repo: String, path: PathBuf) -> Self {
        Self {
            host,
            owner,
            repo,
            path,
            skills: Vec::new(),
        }
    }

    /// Build a new Plugin by scanning for skills at the given path.
    pub(crate) fn build(
        host: String,
        owner: String,
        repo: String,
        path: PathBuf,
    ) -> Result<Plugin, PluginError> {
        let skill_paths = scan_for_skills(&path)?;
        let skills: Vec<Skill> = skill_paths
            .into_iter()
            .map(|(name, skill_path)| Skill::new(name, skill_path, owner.clone(), repo.clone()))
            .collect();

        let mut plugin = Plugin::new(host, owner, repo, path);
        plugin.set_skills(skills);
        Ok(plugin)
    }

    /// Install a plugin by cloning (or updating) the repository and scanning for skills.
    ///
    /// If the path already contains a git repo, pulls latest changes instead of cloning.
    pub fn install(source: GitSource, path: PathBuf) -> Result<Plugin, PluginError> {
        if is_git_repo(&path) {
            // Already installed, update instead
            git_pull(&path)?;
        } else {
            // Clone the repository
            git_clone(&source.url, &path)?;
        }

        Plugin::build(source.host, source.owner, source.repo, path)
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

    /// Update this plugin by pulling latest changes and rescanning skills.
    /// Returns a new Plugin with refreshed skill list.
    pub fn update(&self) -> Result<Plugin, PluginError> {
        if !is_git_repo(&self.path) {
            return Err(PluginError::UpdateFailed {
                path: self.path.clone(),
                stderr: "plugin is not installed".to_string(),
            });
        }

        // Collect qualified names and paths of currently linked skills
        let linked_before: Vec<(String, PathBuf)> = self
            .skills
            .iter()
            .filter(|s| s.is_linked())
            .map(|s| (s.qualified_name(), s.path.clone()))
            .collect();

        // Pull latest changes
        git_pull(&self.path)?;

        // Build new plugin with rescanned skills
        let new_plugin = Plugin::build(
            self.host.clone(),
            self.owner.clone(),
            self.repo.clone(),
            self.path.clone(),
        )?;

        // Build a map of new skill paths by qualified name
        let new_skill_paths: HashMap<String, PathBuf> = new_plugin
            .skills
            .iter()
            .map(|s| (s.qualified_name(), s.path.clone()))
            .collect();

        // Handle removed or relocated skills
        for (name, old_path) in &linked_before {
            match new_skill_paths.get(name) {
                None => {
                    // Skill was removed - delete symlink
                    let _ = skill::remove_skill_symlink(name);
                }
                Some(new_path) if new_path != old_path => {
                    // Skill was moved - relink to new location
                    let _ = skill::remove_skill_symlink(name);
                    if let Some(skill) = new_plugin.skills.iter().find(|s| &s.qualified_name() == name) {
                        let _ = skill.link();
                    }
                }
                _ => {
                    // Path unchanged - nothing to do
                }
            }
        }

        Ok(new_plugin)
    }

    /// Remove this plugin from disk and unlink all skills.
    pub fn remove(&self) -> Result<(), PluginError> {
        if !self.path.exists() {
            return Err(PluginError::NotInstalled {
                name: self.name().to_string(),
            });
        }

        // Unlink all skills before removing the plugin directory
        for skill in &self.skills {
            let _ = skill.unlink(); // Ignore errors (may already be unlinked)
        }

        fs::remove_dir_all(&self.path)?;

        // Clean up empty parent directories
        if let Some(owner_dir) = self.path.parent() {
            if owner_dir.exists() && fs::read_dir(owner_dir)?.next().is_none() {
                let _ = fs::remove_dir(owner_dir);
                if let Some(host_dir) = owner_dir.parent() {
                    if host_dir.exists() && fs::read_dir(host_dir)?.next().is_none() {
                        let _ = fs::remove_dir(host_dir);
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_plugin_name() {
        let dir = tempdir().unwrap();
        let plugin = Plugin::build(
            "github.com".to_string(),
            "anthropics".to_string(),
            "claude-code".to_string(),
            dir.path().to_path_buf(),
        )
        .unwrap();
        assert_eq!(plugin.name(), "claude-code");
    }

    #[test]
    fn test_plugin_fields() {
        let dir = tempdir().unwrap();
        let plugin = Plugin::build(
            "github.com".to_string(),
            "anthropics".to_string(),
            "claude-code".to_string(),
            dir.path().to_path_buf(),
        )
        .unwrap();
        assert_eq!(plugin.host, "github.com");
        assert_eq!(plugin.owner, "anthropics");
        assert_eq!(plugin.repo, "claude-code");
        assert_eq!(plugin.path, dir.path());
        assert!(plugin.skills().is_empty());
    }

    #[test]
    fn test_scan_for_skills_empty() {
        let dir = tempdir().unwrap();
        let skills = scan_for_skills(dir.path()).unwrap();
        assert!(skills.is_empty());
    }

    #[test]
    fn test_scan_for_skills_at_root() {
        let dir = tempdir().unwrap();
        File::create(dir.path().join("SKILL.md")).unwrap();

        let skills = scan_for_skills(dir.path()).unwrap();

        assert_eq!(skills.len(), 1);
        // The skill name should be the temp directory name
        let expected_name = dir
            .path()
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap();
        assert_eq!(skills[0].0, expected_name);
    }

    #[test]
    fn test_scan_for_skills_in_subdirectory() {
        let dir = tempdir().unwrap();
        let skills_dir = dir.path().join("skills").join("foo");
        fs::create_dir_all(&skills_dir).unwrap();
        File::create(skills_dir.join("SKILL.md")).unwrap();

        let skills = scan_for_skills(dir.path()).unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].0, "foo");
    }

    #[test]
    fn test_scan_for_skills_multiple() {
        let dir = tempdir().unwrap();

        // Create skills/foo/SKILL.md
        let foo_dir = dir.path().join("skills").join("foo");
        fs::create_dir_all(&foo_dir).unwrap();
        File::create(foo_dir.join("SKILL.md")).unwrap();

        // Create skills/bar/SKILL.md
        let bar_dir = dir.path().join("skills").join("bar");
        fs::create_dir_all(&bar_dir).unwrap();
        File::create(bar_dir.join("SKILL.md")).unwrap();

        let skills = scan_for_skills(dir.path()).unwrap();

        assert_eq!(skills.len(), 2);
        let names: Vec<&str> = skills.iter().map(|(n, _)| n.as_str()).collect();
        assert!(names.contains(&"foo"));
        assert!(names.contains(&"bar"));
    }

    #[test]
    fn test_scan_for_skills_git_skipped() {
        let dir = tempdir().unwrap();

        // Create .git/SKILL.md (should be skipped)
        let git_dir = dir.path().join(".git");
        fs::create_dir_all(&git_dir).unwrap();
        File::create(git_dir.join("SKILL.md")).unwrap();

        // Create .system/my-skill/SKILL.md (should be found)
        let system_dir = dir.path().join(".system").join("my-skill");
        fs::create_dir_all(&system_dir).unwrap();
        File::create(system_dir.join("SKILL.md")).unwrap();

        let skills = scan_for_skills(dir.path()).unwrap();

        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].0, "my-skill");
    }
}

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::error::PluginError;
use super::git::{git_clone, git_pull, is_git_repo};
use super::plugin::Plugin;
use super::skill::{self, Skill};
use super::source::GitSource;

/// Extract the directory name from a path as a String.
fn dir_name(path: &Path) -> Option<String> {
    path.file_name()?.to_str().map(String::from)
}

/// Central installer service for plugin operations.
#[derive(Clone)]
pub struct Installer {
    cache_dir: PathBuf,
}

impl Installer {
    /// Create a new installer with the default cache directory.
    ///
    /// The cache directory is `~/.cache/silk/repos` on Unix systems.
    pub fn new() -> Result<Self, PluginError> {
        let cache_dir = dirs::home_dir()
            .ok_or(PluginError::CacheDirectoryNotFound)?
            .join(".cache")
            .join("silk")
            .join("repos");
        Ok(Self { cache_dir })
    }

    /// Create an installer with a custom cache directory.
    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        Self { cache_dir }
    }

    /// Install a plugin from a git URL.
    ///
    /// Clones the repository and scans for skills.
    /// If already installed, this will update instead.
    pub fn install(&self, url: &str) -> Result<Arc<Plugin>, PluginError> {
        let source = GitSource::parse(url)?;
        let path = self.local_path(&source);

        if is_git_repo(&path) {
            // Already installed, update instead
            git_pull(&path)?;
        } else {
            // Clone the repository
            git_clone(&source.url, &path)?;
        }

        self.build_plugin(source, path)
    }

    /// Update an installed plugin.
    ///
    /// Returns an error if the plugin is not installed.
    /// Also cleans up symlinks for skills that were removed from the repository.
    pub fn update(&self, plugin: &Plugin) -> Result<Arc<Plugin>, PluginError> {
        if !is_git_repo(&plugin.path) {
            return Err(PluginError::UpdateFailed {
                path: plugin.path.clone(),
                stderr: "plugin is not installed".to_string(),
            });
        }

        // Collect qualified names and paths of currently linked skills
        let linked_before: Vec<(String, PathBuf)> = plugin
            .skills()
            .iter()
            .filter(|s| s.is_linked())
            .map(|s| (s.qualified_name(), s.path.clone()))
            .collect();

        // Pull latest changes
        git_pull(&plugin.path)?;

        // Rebuild plugin with new skill list
        let source = GitSource {
            host: plugin.host.clone(),
            owner: plugin.owner.clone(),
            repo: plugin.repo.clone(),
            url: format!(
                "https://{}/{}/{}",
                plugin.host, plugin.owner, plugin.repo
            ),
        };
        let new_plugin = self.build_plugin(source, plugin.path.clone())?;

        // Build a map of new skill paths by qualified name
        let new_skill_paths: HashMap<String, PathBuf> = new_plugin
            .skills()
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
                    if let Some(skill) =
                        new_plugin.skills().iter().find(|s| &s.qualified_name() == name)
                    {
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

    /// Check if a plugin is installed.
    pub fn is_installed(&self, source: &GitSource) -> bool {
        is_git_repo(&self.local_path(source))
    }

    /// List all installed plugins by scanning the cache directory.
    ///
    /// Scans host/owner/repo directories and builds Plugin objects for each.
    pub fn list_installed(&self) -> Result<Vec<Arc<Plugin>>, PluginError> {
        let mut plugins = Vec::new();

        // Check if cache directory exists
        if !self.cache_dir.exists() {
            return Ok(plugins);
        }

        // Scan host directories (e.g., github.com)
        for host_entry in fs::read_dir(&self.cache_dir)? {
            let host_path = host_entry?.path();
            if !host_path.is_dir() {
                continue;
            }
            let Some(host) = dir_name(&host_path) else { continue };

            // Scan owner directories (e.g., anthropics)
            for owner_entry in fs::read_dir(&host_path)? {
                let owner_path = owner_entry?.path();
                if !owner_path.is_dir() {
                    continue;
                }
                let Some(owner) = dir_name(&owner_path) else { continue };

                // Scan repo directories (e.g., claude-code)
                for repo_entry in fs::read_dir(&owner_path)? {
                    let repo_path = repo_entry?.path();
                    if !repo_path.is_dir() || !is_git_repo(&repo_path) {
                        continue;
                    }
                    let Some(repo) = dir_name(&repo_path) else { continue };

                    // Build the plugin
                    let source = GitSource {
                        host: host.clone(),
                        owner: owner.clone(),
                        repo: repo.clone(),
                        url: format!("https://{}/{}/{}", host, owner, repo),
                    };
                    let plugin = self.build_plugin(source, repo_path)?;
                    plugins.push(plugin);
                }
            }
        }

        Ok(plugins)
    }

    /// Remove an installed plugin by deleting its directory.
    ///
    /// Also unlinks any skills that were linked to Claude Code's skills directory.
    pub fn remove(&self, plugin: &Plugin) -> Result<(), PluginError> {
        if !plugin.path.exists() {
            return Err(PluginError::NotInstalled {
                name: plugin.name().to_string(),
            });
        }

        // Unlink all skills before removing the plugin directory
        for skill in plugin.skills() {
            let _ = skill.unlink(); // Ignore errors (may already be unlinked)
        }

        fs::remove_dir_all(&plugin.path)?;

        // Clean up empty parent directories
        if let Some(owner_dir) = plugin.path.parent() {
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

    /// Get the local path for a source.
    pub fn local_path(&self, source: &GitSource) -> PathBuf {
        self.cache_dir
            .join(&source.host)
            .join(&source.owner)
            .join(&source.repo)
    }

    /// Build a Plugin with discovered skills.
    fn build_plugin(&self, source: GitSource, path: PathBuf) -> Result<Arc<Plugin>, PluginError> {
        // Scan for skills
        let skill_paths = scan_for_skills(&path)?;

        // Create skills with owner/repo info
        let skills: Vec<Skill> = skill_paths
            .into_iter()
            .map(|(name, skill_path)| {
                Skill::new(name, skill_path, source.owner.clone(), source.repo.clone())
            })
            .collect();

        let mut plugin = Plugin::new(source.host, source.owner, source.repo, path);
        plugin.set_skills(skills);

        Ok(Arc::new(plugin))
    }
}

/// Scan a directory for SKILL.md files.
///
/// Returns a list of (skill_name, skill_path) pairs.
fn scan_for_skills(root: &Path) -> Result<Vec<(String, PathBuf)>, PluginError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::tempdir;

    #[test]
    fn test_installer_with_cache_dir() {
        let dir = tempdir().unwrap();
        let installer = Installer::with_cache_dir(dir.path().to_path_buf());
        assert_eq!(installer.cache_dir, dir.path());
    }

    #[test]
    fn test_local_path() {
        let dir = tempdir().unwrap();
        let installer = Installer::with_cache_dir(dir.path().to_path_buf());
        let source = GitSource::parse("https://github.com/anthropics/claude-code").unwrap();

        let path = installer.local_path(&source);
        assert_eq!(
            path,
            dir.path().join("github.com/anthropics/claude-code")
        );
    }

    #[test]
    fn test_is_installed_false() {
        let dir = tempdir().unwrap();
        let installer = Installer::with_cache_dir(dir.path().to_path_buf());
        let source = GitSource::parse("https://github.com/anthropics/claude-code").unwrap();

        assert!(!installer.is_installed(&source));
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

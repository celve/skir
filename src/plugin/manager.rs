use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::error::PluginError;
use super::git::is_git_repo;
use super::plugin::Plugin;
use super::source::GitSource;

/// Extract the directory name from a path as a String.
fn dir_name(path: &Path) -> Option<String> {
    path.file_name()?.to_str().map(String::from)
}

/// Central manager service for plugin operations.
#[derive(Clone)]
pub struct PluginManager {
    cache_dir: PathBuf,
}

impl PluginManager {
    /// Create a new plugin manager with the default cache directory.
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

    /// Create a plugin manager with a custom cache directory.
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

        let plugin = Plugin::install(source, path)?;
        Ok(Arc::new(plugin))
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
                    let plugin = Plugin::build(
                        host.clone(),
                        owner.clone(),
                        repo.clone(),
                        repo_path,
                    )?;
                    plugins.push(Arc::new(plugin));
                }
            }
        }

        Ok(plugins)
    }

    /// Get the local path for a source.
    pub fn local_path(&self, source: &GitSource) -> PathBuf {
        self.cache_dir
            .join(&source.host)
            .join(&source.owner)
            .join(&source.repo)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_manager_with_cache_dir() {
        let dir = tempdir().unwrap();
        let manager = PluginManager::with_cache_dir(dir.path().to_path_buf());
        assert_eq!(manager.cache_dir, dir.path());
    }

    #[test]
    fn test_local_path() {
        let dir = tempdir().unwrap();
        let manager = PluginManager::with_cache_dir(dir.path().to_path_buf());
        let source = GitSource::parse("https://github.com/anthropics/claude-code").unwrap();

        let path = manager.local_path(&source);
        assert_eq!(
            path,
            dir.path().join("github.com/anthropics/claude-code")
        );
    }

    #[test]
    fn test_is_installed_false() {
        let dir = tempdir().unwrap();
        let manager = PluginManager::with_cache_dir(dir.path().to_path_buf());
        let source = GitSource::parse("https://github.com/anthropics/claude-code").unwrap();

        assert!(!manager.is_installed(&source));
    }
}

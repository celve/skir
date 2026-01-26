use std::fs;
use std::path::{Path, PathBuf};

use super::error::PluginError;

/// Get the Claude Code skills directory (~/.claude/skills).
fn claude_skills_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join(".claude").join("skills"))
}

/// Check if a symlink exists at the given path (even if broken).
fn symlink_exists(path: &Path) -> bool {
    path.symlink_metadata().is_ok()
}

/// Remove a symlink from the Claude skills directory by qualified name.
/// Works even when the symlink target no longer exists (broken symlink).
pub fn remove_skill_symlink(qualified_name: &str) -> Result<(), PluginError> {
    let skills_dir = claude_skills_dir().ok_or(PluginError::LinkFailed {
        name: qualified_name.to_string(),
        reason: "cannot determine home directory".to_string(),
    })?;

    let link_path = skills_dir.join(qualified_name);

    if symlink_exists(&link_path) {
        fs::remove_file(&link_path)?;
    }

    Ok(())
}

/// A skill discovered within a plugin.
#[derive(Debug)]
pub struct Skill {
    /// The skill name (derived from the directory containing SKILL.md).
    pub name: String,
    /// The path to the SKILL.md file.
    pub path: PathBuf,
    /// The owner (username/org) of the parent plugin.
    owner: String,
    /// The repository name of the parent plugin.
    repo: String,
}

impl Skill {
    /// Create a new skill with owner and repo information from its parent plugin.
    pub(crate) fn new(name: String, path: PathBuf, owner: String, repo: String) -> Self {
        Self {
            name,
            path,
            owner,
            repo,
        }
    }

    /// Get the qualified name for this skill (owner:repo:skillname).
    ///
    /// This format ensures unique symlink names across different plugins,
    /// avoiding collisions when multiple plugins have skills with the same name.
    pub fn qualified_name(&self) -> String {
        format!("{}:{}:{}", self.owner, self.repo, self.name)
    }

    /// Get the link path for this skill.
    ///
    /// Uses the qualified name (owner:repo:skillname) to avoid collisions.
    pub fn link_path(&self) -> Option<PathBuf> {
        let skills_dir = claude_skills_dir()?;
        Some(skills_dir.join(self.qualified_name()))
    }

    /// Check if this skill is linked to Claude Code.
    ///
    /// Uses `exists()` which follows the symlink and checks if the target exists,
    /// correctly detecting broken symlinks as "not linked".
    pub fn is_linked(&self) -> bool {
        self.link_path()
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    /// Link this skill to Claude Code's skills directory.
    pub fn link(&self) -> Result<(), PluginError> {
        let link_path = self.link_path().ok_or(PluginError::LinkFailed {
            name: self.name.clone(),
            reason: "cannot determine home directory".to_string(),
        })?;

        if symlink_exists(&link_path) {
            return Err(PluginError::AlreadyLinked {
                name: self.qualified_name(),
            });
        }

        // Ensure ~/.claude/skills exists
        if let Some(parent) = link_path.parent() {
            fs::create_dir_all(parent)?;
        }

        // Get the skill directory (parent of SKILL.md)
        let skill_dir = self.path.parent().ok_or(PluginError::LinkFailed {
            name: self.name.clone(),
            reason: "invalid skill path".to_string(),
        })?;

        // Create symlink
        #[cfg(unix)]
        std::os::unix::fs::symlink(skill_dir, &link_path)?;

        #[cfg(windows)]
        std::os::windows::fs::symlink_dir(skill_dir, &link_path)?;

        Ok(())
    }

    /// Unlink this skill from Claude Code's skills directory.
    pub fn unlink(&self) -> Result<(), PluginError> {
        let link_path = self.link_path().ok_or(PluginError::NotLinked {
            name: self.name.clone(),
        })?;

        if !symlink_exists(&link_path) {
            return Err(PluginError::NotLinked {
                name: self.name.clone(),
            });
        }

        fs::remove_file(&link_path)?; // remove_file works on symlinks
        Ok(())
    }
}

use std::fs;
use std::path::{Path, PathBuf};

use super::error::PluginError;

/// Target for skill linking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LinkTarget {
    ClaudeCode,
    Codex,
}

impl LinkTarget {
    /// Get the skills directory for this target.
    pub fn skills_dir(&self) -> Option<PathBuf> {
        dirs::home_dir().map(|h| match self {
            LinkTarget::ClaudeCode => h.join(".claude").join("skills"),
            LinkTarget::Codex => h.join(".codex").join("skills"),
        })
    }

    /// Get the display name for this target.
    pub fn display_name(&self) -> &'static str {
        match self {
            LinkTarget::ClaudeCode => "Claude Code",
            LinkTarget::Codex => "Codex",
        }
    }

    /// Get all available link targets.
    pub fn all() -> &'static [LinkTarget] {
        &[LinkTarget::ClaudeCode, LinkTarget::Codex]
    }
}

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

/// Parse the description from YAML frontmatter in a SKILL.md file.
fn parse_description(path: &Path) -> Option<String> {
    let content = fs::read_to_string(path).ok()?;
    let content = content.trim_start();

    // Check for YAML frontmatter delimiter
    if !content.starts_with("---") {
        return None;
    }

    // Find the closing delimiter
    let rest = &content[3..];
    let end = rest.find("---")?;
    let frontmatter = &rest[..end];

    // Look for description field
    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some(value) = line.strip_prefix("description:") {
            let value = value.trim();
            // Handle quoted strings
            let value = value
                .strip_prefix('"')
                .and_then(|s| s.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|s| s.strip_suffix('\'')))
                .unwrap_or(value);
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }

    None
}

/// A skill discovered within a plugin.
#[derive(Debug)]
pub struct Skill {
    /// The skill name (derived from the directory containing SKILL.md).
    pub name: String,
    /// The path to the SKILL.md file.
    pub path: PathBuf,
    /// The description from SKILL.md frontmatter.
    pub description: Option<String>,
    /// The owner (username/org) of the parent plugin.
    owner: String,
    /// The repository name of the parent plugin.
    repo: String,
}

impl Skill {
    /// Create a new skill with owner and repo information from its parent plugin.
    pub(crate) fn new(name: String, path: PathBuf, owner: String, repo: String) -> Self {
        let description = parse_description(&path);
        Self {
            name,
            path,
            description,
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

    /// Get the link path for this skill for a specific target.
    pub fn link_path_for(&self, target: LinkTarget) -> Option<PathBuf> {
        let skills_dir = target.skills_dir()?;
        Some(skills_dir.join(self.qualified_name()))
    }

    /// Get the link path for this skill (Claude Code).
    ///
    /// Uses the qualified name (owner:repo:skillname) to avoid collisions.
    pub fn link_path(&self) -> Option<PathBuf> {
        self.link_path_for(LinkTarget::ClaudeCode)
    }

    /// Check if this skill is linked to a specific target.
    ///
    /// Uses `exists()` which follows the symlink and checks if the target exists,
    /// correctly detecting broken symlinks as "not linked".
    pub fn is_linked_to(&self, target: LinkTarget) -> bool {
        self.link_path_for(target)
            .map(|p| p.exists())
            .unwrap_or(false)
    }

    /// Check if this skill is linked to Claude Code.
    pub fn is_linked(&self) -> bool {
        self.is_linked_to(LinkTarget::ClaudeCode)
    }

    /// Link this skill to a specific target's skills directory.
    pub fn link_to(&self, target: LinkTarget) -> Result<(), PluginError> {
        let link_path = self.link_path_for(target).ok_or(PluginError::LinkFailed {
            name: self.name.clone(),
            reason: "cannot determine home directory".to_string(),
        })?;

        if symlink_exists(&link_path) {
            return Err(PluginError::AlreadyLinked {
                name: self.qualified_name(),
            });
        }

        // Ensure skills directory exists
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

    /// Link this skill to Claude Code's skills directory.
    pub fn link(&self) -> Result<(), PluginError> {
        self.link_to(LinkTarget::ClaudeCode)
    }

    /// Unlink this skill from a specific target's skills directory.
    pub fn unlink_from(&self, target: LinkTarget) -> Result<(), PluginError> {
        let link_path = self.link_path_for(target).ok_or(PluginError::NotLinked {
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

    /// Unlink this skill from Claude Code's skills directory.
    pub fn unlink(&self) -> Result<(), PluginError> {
        self.unlink_from(LinkTarget::ClaudeCode)
    }
}

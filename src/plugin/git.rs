use std::path::Path;
use std::process::Command;

use super::error::PluginError;

/// Clone a git repository to the specified destination.
pub fn git_clone(url: &str, dest: &Path) -> Result<(), PluginError> {
    // Create parent directories if they don't exist
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let output = Command::new("git")
        .args(["clone", "--depth", "1", url])
        .arg(dest)
        .output()?;

    if !output.status.success() {
        return Err(PluginError::CloneFailed {
            url: url.to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(())
}

/// Pull the latest changes in a git repository.
pub fn git_pull(path: &Path) -> Result<(), PluginError> {
    let output = Command::new("git")
        .args(["pull", "--ff-only"])
        .current_dir(path)
        .output()?;

    if !output.status.success() {
        return Err(PluginError::UpdateFailed {
            path: path.to_path_buf(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        });
    }

    Ok(())
}

/// Check if a path is a git repository.
pub fn is_git_repo(path: &Path) -> bool {
    path.join(".git").is_dir()
}

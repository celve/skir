use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("invalid URL: {url}")]
    InvalidUrl { url: String },

    #[error("clone failed for {url}: {stderr}")]
    CloneFailed { url: String, stderr: String },

    #[error("update failed for {}: {stderr}", path.display())]
    UpdateFailed { path: PathBuf, stderr: String },

    #[error("plugin not installed: {name}")]
    NotInstalled { name: String },

    #[error("failed to link skill {name}: {reason}")]
    LinkFailed { name: String, reason: String },

    #[error("skill already linked: {name}")]
    AlreadyLinked { name: String },

    #[error("skill not linked: {name}")]
    NotLinked { name: String },

    #[error("cache directory not found")]
    CacheDirectoryNotFound,

    #[error(transparent)]
    Io(#[from] std::io::Error),
}

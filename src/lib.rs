pub mod plugin;
pub mod status;

pub use plugin::{GitSource, LinkTarget, Plugin, PluginError, PluginManager, Skill};
pub use status::{StatusKind, StatusManager};

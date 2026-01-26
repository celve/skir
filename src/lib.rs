pub mod plugin;
pub mod status;

pub use plugin::{GitSource, Plugin, PluginError, PluginManager, Skill};
pub use status::{StatusKind, StatusManager};

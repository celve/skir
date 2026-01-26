//! Status management for concurrent notifications.

use std::time::{Duration, Instant};

/// Duration before non-progress statuses auto-clear.
const STATUS_DISPLAY_DURATION: Duration = Duration::from_secs(3);

/// The kind of status notification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StatusKind {
    /// General information (e.g., "Ready")
    Info,
    /// In-progress operation (e.g., "Installing...", "Updating...")
    Progress,
    /// Completed successfully
    Success,
    /// Failed operation
    Error,
}

/// A single status entry.
#[derive(Debug, Clone)]
pub struct StatusEntry {
    /// Unique identifier (e.g., "install:owner/repo")
    pub id: String,
    /// Display message
    pub message: String,
    /// Status kind
    pub kind: StatusKind,
    /// When this entry was created/updated
    pub created_at: Instant,
}

/// Manages multiple concurrent status notifications.
#[derive(Debug, Default)]
pub struct StatusManager {
    entries: Vec<StatusEntry>,
}

impl StatusManager {
    /// Create a new empty StatusManager.
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Add or update a status entry by ID.
    pub fn add(&mut self, id: impl Into<String>, message: impl Into<String>, kind: StatusKind) {
        let id = id.into();
        let message = message.into();
        let now = Instant::now();

        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.message = message;
            entry.kind = kind;
            entry.created_at = now;
        } else {
            self.entries.push(StatusEntry {
                id,
                message,
                kind,
                created_at: now,
            });
        }
    }

    /// Remove a status entry by ID.
    pub fn remove(&mut self, id: &str) {
        self.entries.retain(|e| e.id != id);
    }

    /// Remove all non-Progress entries.
    pub fn clear_completed(&mut self) {
        self.entries.retain(|e| e.kind == StatusKind::Progress);
    }

    /// Remove expired non-progress entries (older than STATUS_DISPLAY_DURATION).
    pub fn clear_expired(&mut self) {
        let now = Instant::now();
        self.entries.retain(|e| {
            // Keep progress entries (they clear when operation completes)
            // Keep recent non-progress entries
            e.kind == StatusKind::Progress
                || now.duration_since(e.created_at) < STATUS_DISPLAY_DURATION
        });
    }

    /// Check if there are any entries.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Get the combined status string for UI display.
    pub fn get_display(&self) -> String {
        if self.entries.is_empty() {
            return "Ready".to_string();
        }

        // Show all entries, sorted by priority: Progress > Error > Success > Info
        let mut sorted_entries: Vec<_> = self.entries.iter().collect();
        sorted_entries.sort_by_key(|e| match e.kind {
            StatusKind::Progress => 0,
            StatusKind::Error => 1,
            StatusKind::Success => 2,
            StatusKind::Info => 3,
        });

        if !sorted_entries.is_empty() {
            return sorted_entries
                .iter()
                .map(|e| e.message.as_str())
                .collect::<Vec<_>>()
                .join(" | ");
        }

        "Ready".to_string()
    }

    /// Check if there are any error entries.
    pub fn has_error(&self) -> bool {
        self.entries.iter().any(|e| e.kind == StatusKind::Error)
    }

    /// Check if there are any progress entries.
    pub fn has_progress(&self) -> bool {
        self.entries.iter().any(|e| e.kind == StatusKind::Progress)
    }

    /// Get the kind of the most relevant status for coloring.
    pub fn display_kind(&self) -> StatusKind {
        // Priority: Progress > Error > Success > Info
        if self.has_progress() {
            StatusKind::Progress
        } else if self.has_error() {
            StatusKind::Error
        } else if self.entries.iter().any(|e| e.kind == StatusKind::Success) {
            StatusKind::Success
        } else if self.entries.is_empty() {
            StatusKind::Success // "Ready" state
        } else {
            StatusKind::Info
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_manager_shows_ready() {
        let manager = StatusManager::new();
        assert_eq!(manager.get_display(), "Ready");
        assert!(manager.is_empty());
    }

    #[test]
    fn test_add_single_status() {
        let mut manager = StatusManager::new();
        manager.add("test:1", "Installing foo...", StatusKind::Progress);
        assert_eq!(manager.get_display(), "Installing foo...");
        assert!(!manager.is_empty());
    }

    #[test]
    fn test_multiple_progress_entries() {
        let mut manager = StatusManager::new();
        manager.add("install:foo", "Installing foo...", StatusKind::Progress);
        manager.add("install:bar", "Installing bar...", StatusKind::Progress);
        assert_eq!(
            manager.get_display(),
            "Installing foo... | Installing bar..."
        );
    }

    #[test]
    fn test_update_existing_entry() {
        let mut manager = StatusManager::new();
        manager.add("install:foo", "Installing foo...", StatusKind::Progress);
        manager.add("install:foo", "Installed foo", StatusKind::Success);
        assert_eq!(manager.get_display(), "Installed foo");
    }

    #[test]
    fn test_remove_entry() {
        let mut manager = StatusManager::new();
        manager.add("install:foo", "Installing foo...", StatusKind::Progress);
        manager.add("install:bar", "Installing bar...", StatusKind::Progress);
        manager.remove("install:foo");
        assert_eq!(manager.get_display(), "Installing bar...");
    }

    #[test]
    fn test_clear_completed() {
        let mut manager = StatusManager::new();
        manager.add("install:foo", "Installing foo...", StatusKind::Progress);
        manager.add("done:bar", "Installed bar", StatusKind::Success);
        manager.add("error:baz", "Failed baz", StatusKind::Error);
        manager.clear_completed();
        assert_eq!(manager.get_display(), "Installing foo...");
    }

    #[test]
    fn test_multiple_success_entries() {
        let mut manager = StatusManager::new();
        manager.add("update:foo", "Updated: foo", StatusKind::Success);
        manager.add("update:bar", "Updated: bar", StatusKind::Success);
        // Both success entries should be displayed
        assert!(manager.get_display().contains("Updated: foo"));
        assert!(manager.get_display().contains("Updated: bar"));
    }

    #[test]
    fn test_mixed_entries_priority_order() {
        let mut manager = StatusManager::new();
        manager.add("info:1", "Info message", StatusKind::Info);
        manager.add("success:1", "Success message", StatusKind::Success);
        manager.add("progress:1", "Progress message", StatusKind::Progress);
        manager.add("error:1", "Error message", StatusKind::Error);

        let display = manager.get_display();
        // Progress should come first, then error, then success, then info
        let progress_pos = display.find("Progress message").unwrap();
        let error_pos = display.find("Error message").unwrap();
        let success_pos = display.find("Success message").unwrap();
        let info_pos = display.find("Info message").unwrap();

        assert!(progress_pos < error_pos);
        assert!(error_pos < success_pos);
        assert!(success_pos < info_pos);
    }

    #[test]
    fn test_display_kind_priority() {
        let mut manager = StatusManager::new();
        assert_eq!(manager.display_kind(), StatusKind::Success); // Empty = Ready

        manager.add("info:1", "Info message", StatusKind::Info);
        assert_eq!(manager.display_kind(), StatusKind::Info);

        manager.add("success:1", "Success message", StatusKind::Success);
        assert_eq!(manager.display_kind(), StatusKind::Success);

        manager.add("error:1", "Error message", StatusKind::Error);
        assert_eq!(manager.display_kind(), StatusKind::Error);

        manager.add("progress:1", "Progress message", StatusKind::Progress);
        assert_eq!(manager.display_kind(), StatusKind::Progress);
    }

    #[test]
    fn test_clear_expired_keeps_progress() {
        let mut manager = StatusManager::new();
        manager.add("progress:1", "Installing...", StatusKind::Progress);
        manager.add("success:1", "Done", StatusKind::Success);

        // Immediately after adding, nothing should be expired
        manager.clear_expired();
        // Both entries are displayed, with progress first
        assert_eq!(manager.get_display(), "Installing... | Done");

        // Progress entry should always be kept (regardless of time)
        // Note: Testing actual expiration would require sleeping 3+ seconds
    }

    #[test]
    fn test_clear_expired_keeps_recent() {
        let mut manager = StatusManager::new();
        manager.add("success:1", "Action completed", StatusKind::Success);

        // Recently added entries should be kept
        manager.clear_expired();
        assert_eq!(manager.get_display(), "Action completed");
    }
}

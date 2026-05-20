use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::services::git_service::{GitSnapshot, GitWatcher};

/// A debounced diff snapshot manager that polls git working tree changes
/// and emits structured diff data to registered listeners.
///
/// Debouncing prevents rapid-fire git commands when multiple file changes
/// occur in quick succession (e.g., during `Write` tool calls).
pub struct DiffManager {
    /// Active watches keyed by working directory path.
    watches: Arc<Mutex<HashMap<PathBuf, WatchState>>>,
}

struct WatchState {
    /// The git watcher handle.
    _watcher: GitWatcher,
    /// Last debounce timestamp.
    last_notified: Instant,
    /// Current snapshot cache.
    snapshot: Option<GitSnapshot>,
    /// Listeners to notify on change.
    listeners: Vec<Box<dyn Fn(GitSnapshot) + Send + 'static>>,
}

impl DiffManager {
    /// Debounce window in milliseconds. Changes within this window are coalesced.
    pub const DEBOUNCE_MS: u64 = 150;

    pub fn new() -> Self {
        Self {
            watches: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Start watching a git working tree for changes.
    /// Calls `listener` with a debounced snapshot whenever git state changes.
    pub fn watch<F>(&self, cwd: PathBuf, listener: F)
    where
        F: Fn(GitSnapshot) + Send + 'static,
    {
        let mut watches = self.watches.lock().unwrap_or_else(|e| e.into_inner());

        let debounce_ms = Self::DEBOUNCE_MS;
        let watches_clone = Arc::clone(&self.watches);
        let cwd_clone = cwd.clone();

        let on_change: Arc<dyn Fn(GitSnapshot) + Send + Sync + 'static> = {
            let watches = watches_clone;
            let cwd = cwd_clone;
            Arc::new(move |snapshot: GitSnapshot| {
                if let Ok(mut w) = watches.lock() {
                    if let Some(state) = w.get_mut(&cwd) {
                        let now = Instant::now();
                        let elapsed = now.duration_since(state.last_notified);
                        if elapsed < Duration::from_millis(debounce_ms) {
                            // Debounce: update snapshot but don't notify yet
                            state.snapshot = Some(snapshot);
                            return;
                        }
                        state.last_notified = now;
                        state.snapshot = Some(snapshot.clone());
                        for listener in &state.listeners {
                            listener(snapshot.clone());
                        }
                    }
                }
            })
        };

        let watcher = GitWatcher::watch(cwd.clone(), on_change);
        watches.insert(
            cwd,
            WatchState {
                _watcher: watcher,
                last_notified: Instant::now(),
                snapshot: None,
                listeners: vec![Box::new(listener)],
            },
        );
    }

    /// Get the latest cached snapshot for a working directory, or poll fresh.
    pub fn get_snapshot(&self, cwd: &PathBuf) -> GitSnapshot {
        let watches = self.watches.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(state) = watches.get(cwd) {
            if let Some(ref snap) = state.snapshot {
                return snap.clone();
            }
        }
        drop(watches);
        // No cached snapshot, poll directly
        GitWatcher::poll_snapshot(cwd)
    }

    /// Stop watching a directory. Returns true if a watch was active.
    pub fn unwatch(&self, cwd: &PathBuf) -> bool {
        let mut watches = self.watches.lock().unwrap_or_else(|e| e.into_inner());
        watches.remove(cwd).is_some()
    }

    /// Return count of active watches.
    pub fn active_watches(&self) -> usize {
        let watches = self.watches.lock().unwrap_or_else(|e| e.into_inner());
        watches.len()
    }
}

impl Default for DiffManager {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn should_create_diff_manager() {
        let dm = DiffManager::new();
        assert_eq!(dm.active_watches(), 0);
    }

    #[test]
    fn should_poll_snapshot_fresh_when_no_watch_active() {
        let tmp = std::env::temp_dir().join("orbit-test-diff-fresh");
        std::fs::create_dir_all(&tmp).ok();
        GitWatcher::run_git(&tmp, &["init"]).ok();

        let dm = DiffManager::new();
        let snap = dm.get_snapshot(&tmp);
        assert_eq!(snap.cwd, tmp.to_string_lossy());

        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn should_unwatch_directory() {
        let tmp = std::env::temp_dir().join("orbit-test-diff-unwatch");
        std::fs::create_dir_all(&tmp).ok();
        GitWatcher::run_git(&tmp, &["init"]).ok();

        let dm = DiffManager::new();
        dm.watch(tmp.clone(), |_| {});
        assert_eq!(dm.active_watches(), 1);

        let removed = dm.unwatch(&tmp);
        assert!(removed);
        assert_eq!(dm.active_watches(), 0);

        std::fs::remove_dir_all(&tmp).ok();
    }
}

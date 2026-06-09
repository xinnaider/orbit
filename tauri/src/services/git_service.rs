use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::time::Duration;

/// A snapshot of git state at a point in time.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GitSnapshot {
    pub cwd: String,
    pub branch: Option<String>,
    pub upstream: Option<String>,
    pub is_dirty: bool,
    pub file_count: usize,
    pub files: Vec<String>,
    pub status_output: Option<String>,
    pub error: Option<String>,
}

/// Polling-based git working tree watcher.
///
/// Spawns a background thread that runs `git status` at a configurable interval
/// and emits change callbacks when the state differs from the previous snapshot.
pub struct GitWatcher {
    running: Arc<AtomicBool>,
}

impl GitWatcher {
    /// Poll interval between git status checks.
    const POLL_INTERVAL: Duration = Duration::from_secs(5);

    /// Start watching a git repository. Calls `on_change(new_snapshot)` whenever
    /// the git state changes. Returns a handle that can be `stop()`ped.
    pub fn watch<F>(cwd: PathBuf, on_change: Arc<F>) -> Self
    where
        F: Fn(GitSnapshot) + Send + Sync + 'static + ?Sized,
    {
        let running = Arc::new(AtomicBool::new(true));
        let running_clone = Arc::clone(&running);

        // Store previous snapshot for change detection
        let prev = Arc::new(Mutex::new(None::<GitSnapshot>));

        std::thread::spawn(move || {
            while running_clone.load(Ordering::Relaxed) {
                let snapshot = Self::poll_snapshot(&cwd);
                let mut prev_guard = prev.lock().unwrap_or_else(|e| e.into_inner());
                let changed = match (prev_guard.as_ref(), &snapshot) {
                    (None, _) => true,
                    (Some(old), new) => {
                        old.branch != new.branch
                            || old.is_dirty != new.is_dirty
                            || old.file_count != new.file_count
                    }
                };
                if changed {
                    *prev_guard = Some(snapshot.clone());
                    on_change(snapshot);
                }
                std::thread::sleep(Self::POLL_INTERVAL);
            }
        });

        Self { running }
    }

    /// Take a one-shot snapshot of the current git state.
    pub fn poll_snapshot(cwd: &PathBuf) -> GitSnapshot {
        let cwd_str = cwd.to_string_lossy().to_string();
        if !cwd.join(".git").exists() {
            return GitSnapshot {
                cwd: cwd_str,
                branch: None,
                upstream: None,
                is_dirty: false,
                file_count: 0,
                files: vec![],
                status_output: None,
                error: Some("Not a git repository".to_string()),
            };
        }

        let branch = Self::run_git(cwd, &["rev-parse", "--abbrev-ref", "HEAD"])
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && s != "HEAD");

        let upstream = branch.as_ref().and_then(|b| {
            Self::run_git(
                cwd,
                &[
                    "rev-parse",
                    "--abbrev-ref",
                    "--symbolic-full-name",
                    &format!("{}@{{upstream}}", b),
                ],
            )
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
        });

        // `--untracked-files=all` lists files inside new subfolders individually
        // instead of collapsing them into a single `?? subdir/` entry.
        let status_output =
            Self::run_git(cwd, &["status", "--porcelain", "--untracked-files=all"]).ok();
        let files: Vec<String> = status_output
            .as_ref()
            .map(|s| {
                s.lines()
                    .filter(|l| !l.is_empty())
                    .map(|l| l.to_string())
                    .collect()
            })
            .unwrap_or_default();
        let file_count = files.len();
        let is_dirty = file_count > 0;

        GitSnapshot {
            cwd: cwd_str,
            branch,
            upstream,
            is_dirty,
            file_count,
            files,
            status_output,
            error: None,
        }
    }

    pub fn run_git(cwd: &PathBuf, args: &[&str]) -> Result<String, String> {
        let mut cmd = std::process::Command::new("git");
        cmd.current_dir(cwd).args(args);
        crate::services::process_util::apply_silent(&mut cmd);
        let output = cmd
            .output()
            .map_err(|e| format!("Failed to run git: {}", e))?;
        if output.status.success() {
            String::from_utf8(output.stdout)
                .map(|s| s.trim().to_string())
                .map_err(|e| format!("Invalid UTF-8: {}", e))
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(stderr.trim().to_string())
        }
    }

    /// Stop the watcher. The background thread will exit on the next poll cycle.
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

impl Drop for GitWatcher {
    fn drop(&mut self) {
        self.stop();
    }
}

// ─── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn should_poll_snapshot_for_non_git_dir() {
        let tmp = std::env::temp_dir().join("orbit-test-non-git");
        std::fs::create_dir_all(&tmp).ok();
        let snapshot = GitWatcher::poll_snapshot(&tmp);
        assert_eq!(snapshot.cwd, tmp.to_string_lossy());
        assert!(snapshot.branch.is_none());
        assert!(snapshot.is_error());
        // Cleanup
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn should_detect_git_dirty_state() {
        // Create temp git repo
        let tmp = std::env::temp_dir().join("orbit-test-git-dirty");
        std::fs::create_dir_all(&tmp).ok();
        GitWatcher::run_git(&tmp, &["init"]).ok();
        GitWatcher::run_git(&tmp, &["config", "user.email", "test@test.com"]).ok();
        GitWatcher::run_git(&tmp, &["config", "user.name", "Test"]).ok();

        // Create a file to make repo dirty
        std::fs::write(tmp.join("test.txt"), "hello").ok();

        let snapshot = GitWatcher::poll_snapshot(&tmp);
        assert!(snapshot.is_dirty);
        assert!(!snapshot.files.is_empty());

        // Cleanup
        std::fs::remove_dir_all(&tmp).ok();
    }

    #[test]
    fn should_stop_watcher_gracefully() {
        let tmp = std::env::temp_dir().join("orbit-test-watcher-stop");
        std::fs::create_dir_all(&tmp).ok();
        GitWatcher::run_git(&tmp, &["init"]).ok();

        let changed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let changed_clone = Arc::clone(&changed);
        let on_change = Arc::new(move |_: GitSnapshot| {
            changed_clone.store(true, Ordering::Relaxed);
        });

        let watcher = GitWatcher::watch(tmp.clone(), on_change);
        // Give the watcher thread time to start and complete its first poll
        std::thread::sleep(Duration::from_millis(500));
        watcher.stop();
        // Wait for thread to see the stop signal
        std::thread::sleep(Duration::from_millis(100));
        assert!(!watcher.running.load(Ordering::Relaxed));

        std::fs::remove_dir_all(&tmp).ok();
    }
}

impl GitSnapshot {
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }
}

use std::path::{Path, PathBuf};
use std::process::Command;

/// Converts a session name into a valid git branch slug.
/// "hammerhead · orbit" → "hammerhead-orbit"
pub fn generate_branch_slug(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' {
                c
            } else {
                '-'
            }
        })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
}

/// Creates a git worktree at `<project_path>/.worktrees/<slug>` on branch `orbit/<slug>`.
/// Returns the absolute path to the created worktree.
pub fn create_worktree(project_path: &Path, slug: &str) -> Result<PathBuf, String> {
    let worktree_path = project_path.join(".worktrees").join(slug);
    let branch_name = format!("orbit/{slug}");

    #[cfg(windows)]
    let output = {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        Command::new("git")
            .args([
                "-C",
                project_path.to_str().unwrap_or("."),
                "worktree",
                "add",
                worktree_path.to_str().unwrap_or(""),
                "-b",
                &branch_name,
            ])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| format!("git não encontrado: {e}"))?
    };

    #[cfg(not(windows))]
    let output = Command::new("git")
        .args([
            "-C",
            project_path.to_str().unwrap_or("."),
            "worktree",
            "add",
            worktree_path.to_str().unwrap_or(""),
            "-b",
            &branch_name,
        ])
        .output()
        .map_err(|e| format!("git não encontrado: {e}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    Ok(worktree_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slug_lowercase() {
        assert_eq!(generate_branch_slug("My Session"), "my-session");
    }

    #[test]
    fn test_slug_special_chars() {
        // The · character from Android names becomes a hyphen
        assert_eq!(
            generate_branch_slug("hammerhead · orbit"),
            "hammerhead-orbit"
        );
    }

    #[test]
    fn test_slug_collapses_dashes() {
        assert_eq!(generate_branch_slug("  spaces  "), "spaces");
    }

    #[test]
    fn test_slug_preserves_hyphens() {
        assert_eq!(generate_branch_slug("abc-def"), "abc-def");
    }

    /// Integration test: creates a real git worktree in a temporary git repo.
    /// Requires the `git` binary (available in CI/Windows).
    #[test]
    fn test_create_worktree_in_real_git_repo() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let repo = dir.path();

        // Initialize a clean git repo
        let init = Command::new("git")
            .args(["init"])
            .current_dir(repo)
            .output()
            .unwrap();
        assert!(init.status.success(), "git init failed");

        // Empty commit (required for worktree)
        for cmd in [
            vec!["config", "user.email", "test@test.com"],
            vec!["config", "user.name", "Test"],
            vec!["commit", "--allow-empty", "-m", "init"],
        ] {
            let out = Command::new("git")
                .args(&cmd)
                .current_dir(repo)
                .output()
                .unwrap();
            assert!(out.status.success(), "git {:?} failed: {:?}", cmd, out);
        }

        let result = create_worktree(repo, "minha-sessao");
        assert!(result.is_ok(), "create_worktree failed: {:?}", result.err());

        let wt_path = result.unwrap();
        assert!(
            wt_path.exists(),
            "worktree path does not exist: {:?}",
            wt_path
        );
        assert!(wt_path.join(".git").exists(), "worktree missing .git");
    }
}

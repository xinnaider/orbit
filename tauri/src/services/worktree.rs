use std::path::{Path, PathBuf};
use std::process::Command;

/// Creates a git worktree on a remote host via SSH.
/// Returns the remote path to the created worktree.
pub fn create_worktree_remote(
    host: &str,
    user: &str,
    ssh_key_path: Option<&str>,
    remote_project_path: &str,
    slug: &str,
) -> Result<String, String> {
    let worktree_path = format!("{remote_project_path}/.worktrees/{slug}");
    let branch_name = format!("orbit/{slug}");
    let script =
        format!("git -C {remote_project_path} worktree add {worktree_path} -b {branch_name}");

    let (child, _guard) = super::ssh::spawn_via_ssh(host, user, ssh_key_path, &script)
        .map_err(|e| format!("failed to spawn ssh: {e}"))?;

    let output = child
        .wait_with_output()
        .map_err(|e| format!("ssh command failed: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(stderr.trim().to_string());
    }

    Ok(worktree_path)
}

/// Converts a session name into a valid git branch slug.
/// "hammerhead · orbit" -> "hammerhead-orbit"
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
            .map_err(|e| format!("git nao encontrado: {e}"))?
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
        .map_err(|e| format!("git nao encontrado: {e}"))?;

    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_string());
    }

    Ok(worktree_path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestCase;

    #[test]
    fn should_lowercase_slug() {
        let mut t = TestCase::new("should_lowercase_slug");
        t.phase("Act");
        let result = generate_branch_slug("My Session");
        t.phase("Assert");
        t.eq("lowercased with hyphen", result.as_str(), "my-session");
    }

    #[test]
    fn should_replace_middle_dot_with_hyphen() {
        let mut t = TestCase::new("should_replace_middle_dot_with_hyphen");
        t.phase("Act");
        let result = generate_branch_slug("hammerhead · orbit");
        t.phase("Assert");
        t.eq(
            "middle dot becomes hyphen",
            result.as_str(),
            "hammerhead-orbit",
        );
    }

    #[test]
    fn should_collapse_consecutive_separators_to_single_hyphen() {
        let mut t = TestCase::new("should_collapse_consecutive_separators_to_single_hyphen");
        t.phase("Act");
        let result = generate_branch_slug("  spaces  ");
        t.phase("Assert");
        t.eq(
            "leading/trailing spaces stripped",
            result.as_str(),
            "spaces",
        );
    }

    #[test]
    fn should_preserve_existing_hyphens() {
        let mut t = TestCase::new("should_preserve_existing_hyphens");
        t.phase("Act");
        let result = generate_branch_slug("abc-def");
        t.phase("Assert");
        t.eq("hyphen preserved", result.as_str(), "abc-def");
    }

    #[test]
    fn should_return_empty_string_for_empty_input() {
        let mut t = TestCase::new("should_return_empty_string_for_empty_input");
        t.phase("Act");
        let result = generate_branch_slug("");
        t.phase("Assert");
        t.eq("empty input gives empty slug", result.as_str(), "");
    }

    #[test]
    fn should_handle_unicode_by_replacing_non_alphanumeric() {
        let mut t = TestCase::new("should_handle_unicode_by_replacing_non_alphanumeric");
        t.phase("Act");
        let result = generate_branch_slug("cafe resume");
        t.phase("Assert");
        t.ok("result is non-empty", !result.is_empty());
        t.ok("result has no spaces", !result.contains(' '));
    }

    #[test]
    fn should_truncate_or_handle_very_long_names_without_panic() {
        let mut t = TestCase::new("should_truncate_or_handle_very_long_names_without_panic");
        t.phase("Act");
        let long = "a".repeat(300);
        let result = generate_branch_slug(&long);
        t.phase("Assert");
        t.ok("no panic and non-empty", !result.is_empty());
    }

    #[test]
    fn should_create_real_worktree_in_temp_git_repo() {
        let mut t = TestCase::new("should_create_real_worktree_in_temp_git_repo");
        t.phase("Seed");
        let dir = tempfile::TempDir::new().expect("tempdir");
        let repo = dir.path();

        for args in [
            vec!["init"],
            vec!["config", "user.email", "test@test.com"],
            vec!["config", "user.name", "Test"],
            vec!["commit", "--allow-empty", "-m", "init"],
        ] {
            let out = std::process::Command::new("git")
                .args(&args)
                .current_dir(repo)
                .output()
                .expect("git command failed to run");
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
        }

        t.phase("Act");
        let result = create_worktree(repo, "test-branch");

        t.phase("Assert");
        t.is_ok("worktree created successfully", &result);
        let wt_path = result.unwrap();
        t.ok("worktree directory exists", wt_path.exists());
        t.ok("worktree has .git file", wt_path.join(".git").exists());
    }
}

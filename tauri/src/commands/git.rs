use serde::Serialize;
use std::path::Path;
use std::process::Command;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitBranchInfo {
    name: String,
    full_name: String,
    kind: String,
    current: bool,
    upstream: Option<String>,
    ahead: u32,
    behind: u32,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitFileChange {
    id: String,
    path: String,
    file_name: String,
    group: String,
    status: String,
    staged: bool,
    untracked: bool,
    old_path: Option<String>,
    additions: Option<u32>,
    deletions: Option<u32>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitOverview {
    cwd: String,
    branch: Option<String>,
    upstream: Option<String>,
    ahead: u32,
    behind: u32,
    files: Vec<GitFileChange>,
    branches: Vec<GitBranchInfo>,
    status_output: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GitDiffFile {
    id: String,
    path: String,
    group: String,
    language: String,
    binary: bool,
    original: String,
    modified: String,
}

fn run_git(cwd: &str, args: &[&str]) -> Result<String, String> {
    if !Path::new(cwd).exists() {
        return Err(format!("Directory does not exist: {cwd}"));
    }

    let mut cmd = Command::new("git");
    cmd.args(args).current_dir(cwd);
    crate::services::process_util::apply_silent(&mut cmd);

    let output = cmd
        .output()
        .map_err(|e| format!("failed to run git: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            "git command failed".to_string()
        } else {
            stderr
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn parse_ahead_behind(line: &str) -> (u32, u32) {
    let start = line.find('[');
    let Some(start) = start else {
        return (0, 0);
    };

    let remaining = &line[start..];
    let end = remaining.find(']');
    let Some(end) = end else {
        return (0, 0);
    };

    let mut ahead = 0;
    let mut behind = 0;
    for part in remaining[1..end].split(',').map(str::trim) {
        if let Some(value) = part.strip_prefix("ahead ") {
            ahead = value.parse().unwrap_or(0);
        } else if let Some(value) = part.strip_prefix("behind ") {
            behind = value.parse().unwrap_or(0);
        }
    }

    (ahead, behind)
}

fn current_branch(cwd: &str) -> Result<(Option<String>, Option<String>, u32, u32), String> {
    let branch = run_git(cwd, &["branch", "--show-current"])?;
    let branch = branch.trim();
    let branch = if branch.is_empty() {
        None
    } else {
        Some(branch.to_string())
    };

    let upstream = run_git(
        cwd,
        &["rev-parse", "--abbrev-ref", "--symbolic-full-name", "@{u}"],
    )
    .ok()
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty());

    let status = run_git(cwd, &["status", "--short", "--branch"])?;
    let first_line = status.lines().next().unwrap_or_default();
    let (ahead, behind) = parse_ahead_behind(first_line);

    Ok((branch, upstream, ahead, behind))
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(path)
        .to_string()
}

fn status_name(code: char, untracked: bool) -> String {
    if untracked {
        return "untracked".to_string();
    }
    match code {
        'A' => "added",
        'D' => "deleted",
        'R' => "renamed",
        'C' => "copied",
        _ => "modified",
    }
    .to_string()
}

fn parse_status_line(line: &str) -> Vec<GitFileChange> {
    if line.len() < 4 {
        return Vec::new();
    }

    let staged_code = line.chars().next().unwrap_or(' ');
    let worktree_code = line.chars().nth(1).unwrap_or(' ');
    let raw_path = normalize_path(&line[3..]);
    let untracked = staged_code == '?' && worktree_code == '?';
    let mut changes = Vec::new();

    if untracked {
        changes.push(make_change("untracked", '?', &raw_path, true, None));
        return changes;
    }

    if staged_code != ' ' && staged_code != '?' {
        changes.push(make_change("staged", staged_code, &raw_path, false, None));
    }
    if worktree_code != ' ' && worktree_code != '?' {
        changes.push(make_change(
            "unstaged",
            worktree_code,
            &raw_path,
            false,
            None,
        ));
    }

    changes
}

fn make_change(
    group: &str,
    code: char,
    path: &str,
    untracked: bool,
    old_path: Option<String>,
) -> GitFileChange {
    GitFileChange {
        id: format!("{group}:{path}"),
        path: path.to_string(),
        file_name: file_name(path),
        group: group.to_string(),
        status: status_name(code, untracked),
        staged: group == "staged",
        untracked,
        old_path,
        additions: None,
        deletions: None,
    }
}

fn numstat_map(cwd: &str) -> Result<std::collections::HashMap<String, (u32, u32)>, String> {
    let mut map = std::collections::HashMap::new();
    for line in run_git(cwd, &["diff", "--numstat", "HEAD"])?.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let adds = parts[0].parse().unwrap_or(0);
            let dels = parts[1].parse().unwrap_or(0);
            map.insert(normalize_path(parts[2]), (adds, dels));
        }
    }
    for line in run_git(cwd, &["diff", "--numstat"])?.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 3 {
            let path = normalize_path(parts[2]);
            map.entry(path)
                .or_insert((parts[0].parse().unwrap_or(0), parts[1].parse().unwrap_or(0)));
        }
    }
    Ok(map)
}

fn apply_numstat(
    files: &mut [GitFileChange],
    stats: &std::collections::HashMap<String, (u32, u32)>,
) {
    for file in files.iter_mut() {
        if let Some((adds, dels)) = stats.get(&file.path) {
            file.additions = Some(*adds);
            file.deletions = Some(*dels);
        }
    }
}

fn changed_files(cwd: &str) -> Result<(Vec<GitFileChange>, String), String> {
    let output = run_git(cwd, &["status", "--porcelain=v1"])?;
    let mut files: Vec<GitFileChange> = output.lines().flat_map(parse_status_line).collect();
    if let Ok(stats) = numstat_map(cwd) {
        apply_numstat(&mut files, &stats);
    }
    Ok((files, output))
}

fn read_git_object(cwd: &str, spec: &str) -> String {
    run_git(cwd, &["show", spec]).unwrap_or_default()
}

fn read_worktree_file(cwd: &str, path: &str) -> String {
    std::fs::read_to_string(Path::new(cwd).join(path)).unwrap_or_default()
}

fn language_for(path: &str) -> String {
    match Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
    {
        "svelte" => "svelte",
        "ts" => "typescript",
        "js" => "javascript",
        "rs" => "rust",
        "json" => "json",
        "md" => "markdown",
        "css" => "css",
        _ => "plaintext",
    }
    .to_string()
}

/// Fast git branch detection — reads .git/HEAD without spawning git.
/// Returns `null` if not a git repo.
#[tauri::command]
pub fn git_branch(cwd: String) -> Option<String> {
    let head = std::fs::read_to_string(std::path::Path::new(&cwd).join(".git/HEAD")).ok()?;
    head.trim()
        .strip_prefix("ref: refs/heads/")
        .map(|b| b.to_string())
}

#[tauri::command]
pub fn git_overview(cwd: String) -> Result<GitOverview, String> {
    run_git(&cwd, &["rev-parse", "--is-inside-work-tree"])?;

    let cwd_clone = cwd.clone();
    let branch_handle = std::thread::spawn(move || current_branch(&cwd_clone));

    let cwd_clone2 = cwd.clone();
    let files_handle = std::thread::spawn(move || changed_files(&cwd_clone2));

    let (branch, upstream, ahead, behind) = branch_handle
        .join()
        .map_err(|_| "branch thread panicked".to_string())??;
    let (files, status_output) = files_handle
        .join()
        .map_err(|_| "files thread panicked".to_string())??;

    Ok(GitOverview {
        cwd,
        branch,
        upstream,
        ahead,
        behind,
        files,
        branches: Vec::new(),
        status_output,
    })
}

#[tauri::command]
pub fn git_diff_file(
    cwd: String,
    path: String,
    group: String,
    status_output: Option<String>,
) -> Result<GitDiffFile, String> {
    run_git(&cwd, &["rev-parse", "--is-inside-work-tree"])?;
    let deleted = if let Some(ref raw) = status_output {
        raw.lines().any(|line| {
            let trimmed = line.trim_start();
            if trimmed.len() < 4 {
                return false;
            }
            let file_part = trimmed[3..].trim();
            (trimmed.starts_with('D') || trimmed.chars().nth(1) == Some('D'))
                && file_part == path.as_str()
        })
    } else {
        let status = run_git(&cwd, &["status", "--porcelain=v1", "--", &path])?;
        status.starts_with('D') || status.chars().nth(1) == Some('D')
    };
    let untracked = group == "untracked";

    let (original, modified) = if untracked {
        (String::new(), read_worktree_file(&cwd, &path))
    } else if deleted {
        (
            read_git_object(&cwd, &format!("HEAD:{path}")),
            String::new(),
        )
    } else if group == "staged" {
        (
            read_git_object(&cwd, &format!("HEAD:{path}")),
            read_git_object(&cwd, &format!(":{path}")),
        )
    } else {
        (
            read_git_object(&cwd, &format!(":{path}")),
            read_worktree_file(&cwd, &path),
        )
    };

    let mut binary = false;
    if let Ok(numstat) = run_git(&cwd, &["diff", "--numstat", "--", &path]) {
        binary = numstat.lines().any(|line| {
            let parts: Vec<&str> = line.split('\t').collect();
            parts.len() >= 2 && parts[0] == "-" && parts[1] == "-"
        });
    }

    Ok(GitDiffFile {
        id: format!("{group}:{path}"),
        language: language_for(&path),
        binary,
        path,
        group,
        original,
        modified,
    })
}

/// Grab a quick git snapshot without spawning background watchers.
/// Useful for the frontend to refresh git state on demand.
#[tauri::command]
pub fn git_snapshot(cwd: String) -> Result<crate::services::git_service::GitSnapshot, String> {
    let p = std::path::PathBuf::from(&cwd);
    let snap = crate::services::git_service::GitWatcher::poll_snapshot(&p);
    Ok(snap)
}

/// Stage all unstaged changes to git index
#[tauri::command]
pub fn git_stage_all(cwd: String) -> Result<(), String> {
    run_git(&cwd, &["add", "."])
        .map(|_| ())
        .map_err(|e| format!("Failed to stage all files: {}", e))
}

/// Reset all staged changes from git index
#[tauri::command]
pub fn git_reset_staged(cwd: String) -> Result<(), String> {
    run_git(&cwd, &["reset", "HEAD"])
        .map(|_| ())
        .map_err(|e| format!("Failed to reset staged files: {}", e))
}

/// Commit staged changes with optional message
#[tauri::command]
pub fn git_commit(cwd: String, message: Option<String>) -> Result<(), String> {
    match message.as_ref().map(|m| m.trim()).filter(|m| !m.is_empty()) {
        Some(msg) => run_git(&cwd, &["commit", "-m", msg]),
        None => run_git(&cwd, &["commit"]),
    }
    .map(|_| ())
    .map_err(|e| format!("Failed to commit: {e}"))
}

#[cfg(unix)]
const NULL_DEVICE: &str = "/dev/null";
#[cfg(windows)]
const NULL_DEVICE: &str = "NUL";

/// Get formatted diff output for a file with syntax highlighting
#[tauri::command]
pub fn git_diff_formatted(cwd: String, file_path: String) -> Result<String, String> {
    let path = normalize_path(&file_path);
    let staged =
        run_git(&cwd, &["diff", "--cached", "--unified=3", "--", &path]).unwrap_or_default();
    let unstaged = run_git(&cwd, &["diff", "--unified=3", "--", &path]).unwrap_or_default();
    let output = if !staged.is_empty() {
        staged
    } else if !unstaged.is_empty() {
        unstaged
    } else {
        run_git(&cwd, &["diff", "--no-index", NULL_DEVICE, &path]).unwrap_or_default()
    };

    if output.trim().is_empty() {
        return Err(format!("No diff available for {path}"));
    }

    let lang = get_language_for(&path);
    Ok(format!("```diff\n{output}```\n\nLanguage hint: {lang}"))
}

/// Quick commit with auto-generated message from file changes
#[tauri::command]
pub fn git_quick_commit(cwd: String) -> Result<(), String> {
    let status =
        run_git(&cwd, &["status", "--short"]).map_err(|e| format!("Failed to get status: {e}"))?;

    let files: Vec<String> = status
        .lines()
        .filter(|line| line.len() >= 3)
        .map(|line| normalize_path(line[3..].trim()))
        .collect();

    if files.is_empty() {
        return Ok(());
    }

    git_stage_all(cwd.clone())?;

    let file_list = files
        .iter()
        .take(5)
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join(", ");
    let suffix = if files.len() > 5 {
        format!(" (+{} more)", files.len() - 5)
    } else {
        String::new()
    };
    let message = format!("Update {file_list}{suffix}");

    git_commit(cwd, Some(message))
}

/// Reset all working tree changes
#[tauri::command]
pub fn git_reset_working_tree(cwd: String) -> Result<(), String> {
    run_git(&cwd, &["checkout", "--", "."])
        .map(|_| ())
        .map_err(|e| format!("Failed to reset working tree: {}", e))
}

/// Validate git configuration before operations
#[tauri::command]
pub fn git_validate_config(cwd: String) -> Result<bool, String> {
    run_git(&cwd, &["rev-parse", "--is-inside-work-tree"])?;
    let name = run_git(&cwd, &["config", "user.name"]).unwrap_or_default();
    let email = run_git(&cwd, &["config", "user.email"]).unwrap_or_default();
    Ok(!name.trim().is_empty() && !email.trim().is_empty())
}

/// Stage a single file
#[tauri::command]
pub fn git_stage_file(cwd: String, file_path: String) -> Result<(), String> {
    run_git(&cwd, &["add", "--", &file_path]).map(|_| ())
}

/// Unstage a single file
#[tauri::command]
pub fn git_unstage_file(cwd: String, file_path: String) -> Result<(), String> {
    run_git(&cwd, &["reset", "HEAD", "--", &file_path]).map(|_| ())
}

/// Chain git commands to handle PowerShell 5.1 limitations
/// Converts && to ; if ($?) { } pattern
pub fn chain_git_commands(commands: Vec<String>) -> Vec<String> {
    commands
        .into_iter()
        .map(|cmd| cmd.replace("&&", "; if ($?) { }"))
        .collect()
}

/// Get language for syntax highlighting based on file extension
fn get_language_for(path: &str) -> String {
    match path.rsplit('.').next() {
        Some("svelte") => "svelte",
        Some("ts") | Some("tsx") => "typescript",
        Some("js") | Some("jsx") => "javascript",
        Some("rs") => "rust",
        Some("json") => "json",
        Some("md") => "markdown",
        Some("css") => "css",
        Some("html") => "html",
        Some("sh") => "shell",
        Some("py") => "python",
        Some("go") => "go",
        Some("java") => "java",
        Some("cs") => "csharp",
        Some("php") => "php",
        Some("rb") => "ruby",
        Some("sql") => "sql",
        Some("toml") => "toml",
        Some("yaml") | Some("yml") => "yaml",
        _ => "text",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chain_git_commands_replaces_and_with_powershell_pipe() {
        let out = chain_git_commands(vec!["git add . && git commit".to_string()]);
        assert_eq!(out[0], "git add . ; if ($?) { } git commit");
    }
}

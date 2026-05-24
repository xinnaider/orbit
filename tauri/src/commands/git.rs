use serde::Serialize;
use std::path::Path;
use std::process::Command;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x08000000;

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

    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW);

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

fn changed_files(cwd: &str) -> Result<(Vec<GitFileChange>, String), String> {
    let output = run_git(cwd, &["status", "--porcelain=v1"])?;
    let files: Vec<GitFileChange> = output.lines().flat_map(parse_status_line).collect();
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

    Ok(GitDiffFile {
        id: format!("{group}:{path}"),
        language: language_for(&path),
        binary: false,
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
    let commit_args: Vec<&str> = message
        .as_ref()
        .map(|m| vec!["-m", m.trim()])
        .unwrap_or_default();
    
    run_git(&cwd, &["commit"]
        .iter()
        .chain(commit_args.iter())
        .copied()
        .collect::<Vec<_>>()
        .as_slice()
    )
    .map(|_| ())
    .map_err(|e| format!("Failed to commit: {}", e))
}

/// Get formatted diff output for a file with syntax highlighting
#[tauri::command]
pub fn git_diff_formatted(cwd: String, file_path: String) -> Result<String, String> {
    run_git(&cwd, &["diff", "--unified=3", "--no-index", "-u", "HEAD", &file_path])
        .map(|output| {
            // Add markdown formatting
            format!("\`\`\`{}\ndiff --git a/{} b/{}\\n{}\`\`\`\n", 
                    get_language_for(path: &file_path),
                    normalize_path(&file_path),
                    normalize_path(&file_path),
                    output
            )
        })
        .map_err(|e| format!("Failed to get diff: {}", e))
}

/// Quick commit with auto-generated message from file changes
#[tauri::command]
pub fn git_quick_commit(cwd: String) -> Result<(), String> {
    // Get list of changed files
    let status = run_git(&cwd, &["status", "--short"])
        .map_err(|e| format!("Failed to get status: {}", e))?;
    
    let files: Vec<&str> = status
        .lines()
        .filter(|line| line.len() >= 3)
        .map(|line| &line[3..])
        .collect();
    
    if files.is_empty() {
        return Ok(());
    }
    
    // Generate message from file changes
    let mut message = "Update".to_string();
    for file in &files {
        message = format!("{} {}", message, file);
    }
    
    git_commit(cwd, Some(message))
}

/// Reset all working tree changes
#[tauri::command]
pub fn git_reset_working_tree(cwd: String) -> Result<(), String> {
    run_git(&cwd, &["checkout", "--", "."])
    .map(|_| ())
    .map_err(|e| format!("Failed to reset working tree: {}", e))
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
        Some("ts") => "typescript",
        Some("js") => "javascript",
        Some("rs") => "rust",
        Some("json") => "json",
        Some("md") => "markdown",
        Some("css") => "css",
        Some("html") => "html",
        Some("sh") => "shell",
        Some("py") => "python",
        _ => "text",
    }
}

/// Normalize path separators for cross-platform compatibility
fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}


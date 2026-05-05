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

    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
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

fn branches(cwd: &str) -> Result<Vec<GitBranchInfo>, String> {
    let output = run_git(
        cwd,
        &[
            "for-each-ref",
            "--format=%(refname)%00%(refname:short)%00%(HEAD)%00%(upstream:short)%00%(ahead-behind:HEAD)",
            "refs/heads",
            "refs/remotes",
        ],
    )?;

    Ok(output
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.split('\0').collect();
            if parts.len() < 5 {
                return None;
            }

            let full_name = parts[0].to_string();
            let kind = if full_name.starts_with("refs/remotes/") {
                "remote"
            } else {
                "local"
            };
            let upstream = if parts[3].is_empty() {
                None
            } else {
                Some(parts[3].to_string())
            };
            let mut counts = parts[4].split_whitespace();
            let ahead = counts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
            let behind = counts.next().and_then(|v| v.parse().ok()).unwrap_or(0);

            Some(GitBranchInfo {
                name: parts[1].to_string(),
                full_name,
                kind: kind.to_string(),
                current: parts[2] == "*",
                upstream,
                ahead,
                behind,
            })
        })
        .collect())
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

fn changed_files(cwd: &str) -> Result<Vec<GitFileChange>, String> {
    let output = run_git(cwd, &["status", "--porcelain=v1"])?;
    Ok(output.lines().flat_map(parse_status_line).collect())
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

#[tauri::command]
pub fn git_overview(cwd: String) -> Result<GitOverview, String> {
    run_git(&cwd, &["rev-parse", "--is-inside-work-tree"])?;
    let (branch, upstream, ahead, behind) = current_branch(&cwd)?;

    Ok(GitOverview {
        cwd: cwd.clone(),
        branch,
        upstream,
        ahead,
        behind,
        files: changed_files(&cwd)?,
        branches: branches(&cwd)?,
    })
}

#[tauri::command]
pub fn git_diff_file(cwd: String, path: String, group: String) -> Result<GitDiffFile, String> {
    run_git(&cwd, &["rev-parse", "--is-inside-work-tree"])?;
    let status = run_git(&cwd, &["status", "--porcelain=v1", "--", &path])?;
    let deleted = status.starts_with('D')
        || status.chars().nth(1) == Some('D');
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

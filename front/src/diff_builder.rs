use std::fs;
use std::path::PathBuf;

use serde::Serialize;

use crate::models::{DiffHunk, DiffLine, DiffLineKind, DiffResult};

/// Get all file versions from Claude's file-history for a session.
pub fn get_file_versions(session_id: &str) -> Vec<FileVersionInfo> {
    let history_dir = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("file-history").join(session_id),
        None => return vec![],
    };

    if !history_dir.is_dir() {
        return vec![];
    }

    let entries = match fs::read_dir(&history_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    let mut files: std::collections::HashMap<String, Vec<(u32, PathBuf)>> =
        std::collections::HashMap::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();

        // Format: {fileHashId}@v{N}
        if let Some((hash, version_str)) = name.split_once("@v") {
            if let Ok(version) = version_str.parse::<u32>() {
                files
                    .entry(hash.to_string())
                    .or_default()
                    .push((version, path));
            }
        }
    }

    let mut result = Vec::new();
    for (hash_id, mut versions) in files {
        versions.sort_by_key(|(v, _)| *v);
        let max_version = versions.last().map(|(v, _)| *v).unwrap_or(0);
        result.push(FileVersionInfo {
            hash_id,
            versions,
            max_version,
        });
    }

    result
}

#[derive(Debug, Serialize)]
pub struct FileVersionInfo {
    pub hash_id: String,
    pub versions: Vec<(u32, PathBuf)>,
    pub max_version: u32,
}

/// Generate a diff between two versions of a file.
pub fn build_diff(
    session_id: &str,
    file_hash: &str,
    from_version: u32,
    to_version: u32,
) -> Option<DiffResult> {
    let history_dir = dirs::home_dir()?
        .join(".claude")
        .join("file-history")
        .join(session_id);

    let old_path = history_dir.join(format!("{}@v{}", file_hash, from_version));
    let new_path = history_dir.join(format!("{}@v{}", file_hash, to_version));

    let old_content = fs::read_to_string(&old_path).unwrap_or_default();
    let new_content = fs::read_to_string(&new_path).ok()?;

    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    let hunks = compute_diff_hunks(&old_lines, &new_lines);
    let added: u32 = hunks
        .iter()
        .flat_map(|h| &h.lines)
        .filter(|l| l.kind == DiffLineKind::Added)
        .count() as u32;
    let removed: u32 = hunks
        .iter()
        .flat_map(|h| &h.lines)
        .filter(|l| l.kind == DiffLineKind::Removed)
        .count() as u32;

    Some(DiffResult {
        file_path: file_hash.to_string(), // caller should map to real path
        from_version,
        to_version,
        hunks,
        added,
        removed,
    })
}

/// Simple line-by-line diff using longest common subsequence.
fn compute_diff_hunks(old: &[&str], new: &[&str]) -> Vec<DiffHunk> {
    // Build edit script using simple O(nm) LCS
    let m = old.len();
    let n = new.len();

    // Use Myers-like approach: walk through both, collect changes
    let mut dp = vec![vec![0u32; n + 1]; m + 1];
    for i in (0..m).rev() {
        for j in (0..n).rev() {
            if old[i] == new[j] {
                dp[i][j] = dp[i + 1][j + 1] + 1;
            } else {
                dp[i][j] = dp[i + 1][j].max(dp[i][j + 1]);
            }
        }
    }

    // Walk the DP table to produce diff lines
    let mut lines = Vec::new();
    let mut i = 0;
    let mut j = 0;
    while i < m || j < n {
        if i < m && j < n && old[i] == new[j] {
            lines.push(DiffLine {
                kind: DiffLineKind::Context,
                content: old[i].to_string(),
            });
            i += 1;
            j += 1;
        } else if j < n && (i >= m || dp[i][j + 1] >= dp[i + 1][j]) {
            lines.push(DiffLine {
                kind: DiffLineKind::Added,
                content: new[j].to_string(),
            });
            j += 1;
        } else if i < m {
            lines.push(DiffLine {
                kind: DiffLineKind::Removed,
                content: old[i].to_string(),
            });
            i += 1;
        }
    }

    // Group into hunks (split on runs of 3+ context lines)
    if lines.is_empty() {
        return vec![];
    }

    let mut hunks = Vec::new();
    let mut current_lines = Vec::new();
    let mut context_run = 0;
    let mut hunk_old_start = 1u32;
    let mut hunk_new_start = 1u32;
    let mut old_line = 1u32;
    let mut new_line = 1u32;

    for line in &lines {
        match line.kind {
            DiffLineKind::Context => {
                context_run += 1;
                if context_run > 3 && !current_lines.is_empty() {
                    // End current hunk
                    hunks.push(DiffHunk {
                        old_start: hunk_old_start,
                        new_start: hunk_new_start,
                        lines: current_lines.clone(),
                    });
                    current_lines.clear();
                }
                if context_run <= 3 {
                    current_lines.push(line.clone());
                }
                old_line += 1;
                new_line += 1;
            }
            DiffLineKind::Added => {
                if current_lines.is_empty() {
                    hunk_old_start = old_line;
                    hunk_new_start = new_line;
                }
                context_run = 0;
                current_lines.push(line.clone());
                new_line += 1;
            }
            DiffLineKind::Removed => {
                if current_lines.is_empty() {
                    hunk_old_start = old_line;
                    hunk_new_start = new_line;
                }
                context_run = 0;
                current_lines.push(line.clone());
                old_line += 1;
            }
        }
    }

    if !current_lines.is_empty() {
        hunks.push(DiffHunk {
            old_start: hunk_old_start,
            new_start: hunk_new_start,
            lines: current_lines,
        });
    }

    hunks
}

use std::path::Path;

use crate::diff_builder;
use crate::journal_reader;
use crate::models::*;

#[tauri::command]
pub fn get_diff(
    session_id: String,
    file_hash: String,
    from_version: u32,
    to_version: u32,
) -> Result<DiffResult, String> {
    diff_builder::build_diff(&session_id, &file_hash, from_version, to_version)
        .ok_or_else(|| "Could not build diff".to_string())
}

#[tauri::command]
pub fn get_file_versions(session_id: String) -> Vec<diff_builder::FileVersionInfo> {
    diff_builder::get_file_versions(&session_id)
}

#[tauri::command]
pub fn get_subagent_journal(session_id: String, subagent_id: String) -> Vec<JournalEntry> {
    let projects_dir = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("projects"),
        None => return vec![],
    };

    let entries = match std::fs::read_dir(&projects_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    for project_entry in entries.flatten() {
        let jsonl_path = project_entry
            .path()
            .join(&session_id)
            .join("subagents")
            .join(format!("{}.jsonl", &subagent_id));

        if jsonl_path.exists() {
            let state = journal_reader::parse_journal(&jsonl_path, 0, None);
            let mut result = state.entries;
            for entry in &mut result {
                entry.session_id = subagent_id.clone();
            }
            return result;
        }
    }

    vec![]
}

/// Extract the `name` or `description` value from YAML frontmatter.
fn frontmatter_field(content: &str, field: &str) -> Option<String> {
    let body = content.strip_prefix("---")?;
    let end = body.find("---")?;
    let fm = &body[..end];
    for line in fm.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix(field) {
            let rest = rest.trim_start();
            if let Some(val) = rest.strip_prefix(':') {
                let val = val.trim().trim_matches('"');
                if !val.is_empty() {
                    return Some(val.to_string());
                }
            }
        }
    }
    None
}

/// Scan a plugin directory for skills, commands and agents.
fn scan_plugin(install_path: &Path, plugin_name: &str, out: &mut Vec<SlashCommand>) {
    let skills_dir = install_path.join("skills");
    if let Ok(entries) = std::fs::read_dir(&skills_dir) {
        for entry in entries.flatten() {
            let skill_file = entry.path().join("SKILL.md");
            if skill_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&skill_file) {
                    let name = frontmatter_field(&content, "name")
                        .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());
                    let desc = frontmatter_field(&content, "description").unwrap_or_default();
                    let desc_short = if desc.len() > 80 {
                        format!("{}...", &desc[..77])
                    } else {
                        desc
                    };
                    out.push(SlashCommand {
                        cmd: format!("/{}:{}", plugin_name, name),
                        desc: desc_short,
                        category: "skill".to_string(),
                    });
                }
            }
        }
    }

    let cmds_dir = install_path.join("commands");
    if let Ok(entries) = std::fs::read_dir(&cmds_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let desc = frontmatter_field(&content, "description").unwrap_or_default();
                    let desc_lower = desc.to_lowercase();
                    if desc_lower.contains("deprecated") {
                        continue;
                    }
                    let stem = path
                        .file_stem()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    let desc_short = if desc.len() > 80 {
                        format!("{}...", &desc[..77])
                    } else {
                        desc
                    };
                    out.push(SlashCommand {
                        cmd: format!("/{}", stem),
                        desc: desc_short,
                        category: "command".to_string(),
                    });
                }
            }
        }
    }

    let agents_dir = install_path.join("agents");
    if let Ok(entries) = std::fs::read_dir(&agents_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|e| e == "md") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let name = frontmatter_field(&content, "name").unwrap_or_else(|| {
                        path.file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .to_string()
                    });
                    let desc = frontmatter_field(&content, "description").unwrap_or_default();
                    let desc_short = if desc.len() > 80 {
                        format!("{}...", &desc[..77])
                    } else {
                        desc
                    };
                    out.push(SlashCommand {
                        cmd: format!("/{}:{}", plugin_name, name),
                        desc: desc_short,
                        category: "agent".to_string(),
                    });
                }
            }
        }
    }
}

#[tauri::command]
pub fn get_slash_commands() -> Vec<SlashCommand> {
    let mut result: Vec<SlashCommand> = Vec::new();

    let builtins = [
        ("/help", "Show help"),
        ("/compact", "Compact conversation context"),
        ("/clear", "Clear conversation"),
        ("/cost", "Show token usage and cost"),
        ("/model", "Switch model"),
        ("/fast", "Toggle fast output mode"),
        ("/permissions", "Manage tool permissions"),
        ("/status", "Show session status"),
        ("/memory", "Edit CLAUDE.md memory"),
        ("/review", "Review conversation"),
        ("/doctor", "Check installation health"),
        ("/init", "Initialize CLAUDE.md for project"),
        ("/login", "Log in to Anthropic"),
        ("/logout", "Log out"),
        ("/terminal-setup", "Setup terminal integration"),
        ("/vim", "Toggle vim mode"),
    ];
    for (cmd, desc) in builtins {
        result.push(SlashCommand {
            cmd: cmd.to_string(),
            desc: desc.to_string(),
            category: "built-in".to_string(),
        });
    }

    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return result,
    };

    let plugins_file = home
        .join(".claude")
        .join("plugins")
        .join("installed_plugins.json");
    let content = match std::fs::read_to_string(&plugins_file) {
        Ok(c) => c,
        Err(_) => return result,
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return result,
    };

    if let Some(plugins) = json.get("plugins").and_then(|p| p.as_object()) {
        for (key, entries) in plugins {
            // key format: "superpowers@claude-plugins-official"
            let plugin_name = key.split('@').next().unwrap_or(key);
            if let Some(arr) = entries.as_array() {
                for entry in arr {
                    if let Some(install_path) = entry.get("installPath").and_then(|p| p.as_str()) {
                        scan_plugin(Path::new(install_path), plugin_name, &mut result);
                    }
                }
            }
        }
    }

    // Deduplicate by cmd (keep first occurrence — built-ins win)
    let mut seen = std::collections::HashSet::new();
    result.retain(|c| seen.insert(c.cmd.clone()));

    result
}

#[tauri::command]
pub fn list_project_files(cwd: String) -> Vec<String> {
    use ignore::WalkBuilder;

    let mut files = Vec::new();
    let walker = WalkBuilder::new(&cwd)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .max_depth(Some(12))
        .build();

    for entry in walker.flatten() {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        if let Ok(rel) = entry.path().strip_prefix(&cwd) {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if !rel_str.is_empty() {
                files.push(rel_str.to_string());
                if files.len() >= 5000 {
                    break;
                }
            }
        }
    }

    files.sort();
    files
}

/// Extract tasks by scanning session_outputs for the last `TodoWrite` tool call.
/// Claude Code emits tool_use blocks with name="TodoWrite" and input.todos=[...].
#[tauri::command]
pub fn get_tasks(
    session_id: String,
    state: tauri::State<crate::ipc::session::SessionState>,
) -> Vec<TaskItem> {
    let id: i64 = match session_id.parse() {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let outputs = {
        let m = state.0.lock().unwrap();
        match m.db.get_outputs(id) {
            Ok(o) => o,
            Err(_) => return vec![],
        }
    };

    // Find the last TodoWrite call — its todos list represents the current state.
    let mut last_todos: Option<Vec<TaskItem>> = None;

    for raw in &outputs {
        let val: serde_json::Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if val.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }

        let content = match val
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_array())
        {
            Some(c) => c,
            None => continue,
        };

        for block in content {
            if block.get("type").and_then(|t| t.as_str()) != Some("tool_use") {
                continue;
            }
            if block.get("name").and_then(|n| n.as_str()) != Some("TodoWrite") {
                continue;
            }

            let todos_val = match block.get("input").and_then(|i| i.get("todos")) {
                Some(t) => t,
                None => continue,
            };

            let todos: Vec<TaskItem> = todos_val
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .enumerate()
                .filter_map(|(idx, t)| {
                    let id_str = t
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| (idx + 1).to_string());
                    let subject = t
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let status = t
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("pending")
                        .to_string();
                    if status == "deleted" || subject.is_empty() {
                        return None;
                    }
                    let active_form = t
                        .get("activeForm")
                        .or_else(|| t.get("active_form"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    Some(TaskItem {
                        id: id_str,
                        subject,
                        description: String::new(),
                        active_form,
                        status,
                        blocks: vec![],
                        blocked_by: vec![],
                    })
                })
                .collect();

            last_todos = Some(todos);
        }
    }

    last_todos.unwrap_or_default()
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClaudeUsageStats {
    pub weekly_tokens: u64,
    pub today_tokens: u64,
    pub weekly_messages: u64,
    pub today_messages: u64,
}

#[tauri::command]
pub fn get_claude_usage_stats() -> ClaudeUsageStats {
    let empty = ClaudeUsageStats {
        weekly_tokens: 0,
        today_tokens: 0,
        weekly_messages: 0,
        today_messages: 0,
    };

    let stats_path = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("stats-cache.json"),
        None => return empty,
    };

    let content = match std::fs::read_to_string(&stats_path) {
        Ok(c) => c,
        Err(_) => return empty,
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return empty,
    };

    // Compute today and 7-days-ago in YYYY-MM-DD format using std only
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let secs_per_day: u64 = 86400;
    let today_days = now / secs_per_day;
    let today = days_to_date(today_days);
    let week_start = days_to_date(today_days.saturating_sub(6));

    let mut weekly_tokens: u64 = 0;
    let mut today_tokens: u64 = 0;
    let mut weekly_messages: u64 = 0;
    let mut today_messages: u64 = 0;

    if let Some(arr) = json.get("dailyModelTokens").and_then(|v| v.as_array()) {
        for entry in arr {
            let date = entry.get("date").and_then(|d| d.as_str()).unwrap_or("");
            if date >= week_start.as_str() && date <= today.as_str() {
                if let Some(by_model) = entry.get("tokensByModel").and_then(|v| v.as_object()) {
                    let total: u64 = by_model.values().filter_map(|v| v.as_u64()).sum();
                    weekly_tokens += total;
                    if date == today.as_str() {
                        today_tokens = total;
                    }
                }
            }
        }
    }

    if let Some(arr) = json.get("dailyActivity").and_then(|v| v.as_array()) {
        for entry in arr {
            let date = entry.get("date").and_then(|d| d.as_str()).unwrap_or("");
            if date >= week_start.as_str() && date <= today.as_str() {
                let msgs = entry
                    .get("messageCount")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                weekly_messages += msgs;
                if date == today.as_str() {
                    today_messages = msgs;
                }
            }
        }
    }

    ClaudeUsageStats {
        weekly_tokens,
        today_tokens,
        weekly_messages,
        today_messages,
    }
}

fn days_to_date(days: u64) -> String {
    // Compute YYYY-MM-DD from days since Unix epoch (1970-01-01)
    let mut remaining = days;
    let mut year = 1970u64;
    loop {
        let days_in_year = if is_leap(year) { 366 } else { 365 };
        if remaining < days_in_year {
            break;
        }
        remaining -= days_in_year;
        year += 1;
    }
    let leap = is_leap(year);
    let month_days: [u64; 12] = [
        31,
        if leap { 29 } else { 28 },
        31,
        30,
        31,
        30,
        31,
        31,
        30,
        31,
        30,
        31,
    ];
    let mut month = 1u64;
    for &md in &month_days {
        if remaining < md {
            break;
        }
        remaining -= md;
        month += 1;
    }
    let day = remaining + 1;
    format!("{:04}-{:02}-{:02}", year, month, day)
}

fn is_leap(year: u64) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}

#[tauri::command]
pub fn get_changelog() -> String {
    include_str!("../../CHANGELOG.md").to_string()
}

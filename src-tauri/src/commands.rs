use std::path::Path;
use tauri::State;

use crate::journal_reader;
use crate::keystroke_sender;
use crate::models::*;
use crate::polling::AppState;
use crate::session_watcher;
use crate::diff_builder;

#[tauri::command]
pub fn send_keystroke(session_id: String, key: String, state: State<AppState>) -> Result<(), String> {
    let _journal_states = state.journal_states.lock().map_err(|e| e.to_string())?;

    // Find the PID for this session
    let live_sessions = session_watcher::discover_live_sessions();
    let session = live_sessions.iter()
        .find(|s| s.session_id == session_id)
        .ok_or("Session not found")?;

    keystroke_sender::send_keys(session.pid, &key)?;
    Ok(())
}

#[tauri::command]
pub fn send_message(session_id: String, message: String, state: State<AppState>) -> Result<(), String> {
    // send_keys already appends Enter
    send_keystroke(session_id, message, state)
}

#[tauri::command]
pub fn get_journal(session_id: String, state: State<AppState>) -> Result<Vec<JournalEntry>, String> {
    let journal_states = state.journal_states.lock().map_err(|e| e.to_string())?;

    match journal_states.get(&session_id) {
        Some(js) => {
            let mut entries = js.entries.clone();
            for entry in &mut entries {
                entry.session_id = session_id.clone();
            }
            Ok(entries)
        }
        None => Ok(vec![]),
    }
}

#[tauri::command]
pub fn get_diff(session_id: String, file_hash: String, from_version: u32, to_version: u32) -> Result<DiffResult, String> {
    diff_builder::build_diff(&session_id, &file_hash, from_version, to_version)
        .ok_or_else(|| "Could not build diff".to_string())
}

#[tauri::command]
pub fn get_file_versions(session_id: String) -> Vec<diff_builder::FileVersionInfo> {
    diff_builder::get_file_versions(&session_id)
}

#[tauri::command]
pub fn get_subagent_journal(session_id: String, subagent_id: String) -> Vec<JournalEntry> {
    // Find the subagent JSONL path
    let projects_dir = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("projects"),
        None => return vec![],
    };

    let entries = match std::fs::read_dir(&projects_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    for project_entry in entries.flatten() {
        let jsonl_path = project_entry.path()
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
    // Skills: skills/<name>/SKILL.md
    let skills_dir = install_path.join("skills");
    if let Ok(entries) = std::fs::read_dir(&skills_dir) {
        for entry in entries.flatten() {
            let skill_file = entry.path().join("SKILL.md");
            if skill_file.exists() {
                if let Ok(content) = std::fs::read_to_string(&skill_file) {
                    let name = frontmatter_field(&content, "name")
                        .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());
                    let desc = frontmatter_field(&content, "description")
                        .unwrap_or_default();
                    // Truncate long descriptions
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

    // Commands: commands/<name>.md (skip deprecated)
    let cmds_dir = install_path.join("commands");
    if let Ok(entries) = std::fs::read_dir(&cmds_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "md") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let desc = frontmatter_field(&content, "description")
                        .unwrap_or_default();
                    let desc_lower = desc.to_lowercase();
                    if desc_lower.contains("deprecated") {
                        continue;
                    }
                    let stem = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
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

    // Agents: agents/<name>.md
    let agents_dir = install_path.join("agents");
    if let Ok(entries) = std::fs::read_dir(&agents_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "md") {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    let name = frontmatter_field(&content, "name")
                        .unwrap_or_else(|| path.file_stem().unwrap_or_default().to_string_lossy().to_string());
                    let desc = frontmatter_field(&content, "description")
                        .unwrap_or_default();
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

    // Built-in Claude Code commands (always available)
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

    // Discover installed plugins
    let home = match dirs::home_dir() {
        Some(h) => h,
        None => return result,
    };

    let plugins_file = home.join(".claude").join("plugins").join("installed_plugins.json");
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
        .hidden(true)        // skip hidden files
        .git_ignore(true)    // respect .gitignore
        .git_global(true)
        .git_exclude(true)
        .max_depth(Some(12))
        .build();

    for entry in walker.flatten() {
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
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

#[tauri::command]
pub fn get_tasks(session_id: String) -> Vec<TaskItem> {
    let tasks_dir = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("tasks").join(&session_id),
        None => return vec![],
    };

    let entries = match std::fs::read_dir(&tasks_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    let mut tasks: Vec<TaskItem> = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.extension().is_some_and(|e| e == "json") {
            continue;
        }
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };
        if let Ok(task) = serde_json::from_str::<TaskItem>(&content) {
            if task.status != "deleted" {
                tasks.push(task);
            }
        }
    }

    // Sort by ID numerically
    tasks.sort_by(|a, b| {
        let a_num: u32 = a.id.parse().unwrap_or(u32::MAX);
        let b_num: u32 = b.id.parse().unwrap_or(u32::MAX);
        a_num.cmp(&b_num)
    });

    tasks
}

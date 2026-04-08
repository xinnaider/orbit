use std::path::Path;

use crate::models::*;

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

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

/// Truncate `s` to at most `max_bytes` bytes at a UTF-8 character boundary.
/// Appends "..." if truncated.
fn truncate_desc(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        return s.to_string();
    }
    let boundary = (0..=max_bytes)
        .rev()
        .find(|&i| s.is_char_boundary(i))
        .unwrap_or(0);
    format!("{}...", &s[..boundary])
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
                    let desc_short = truncate_desc(&desc, 77);
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
                    let desc_short = truncate_desc(&desc, 77);
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
                    let desc_short = truncate_desc(&desc, 77);
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
pub fn get_slash_commands(provider: Option<String>) -> Vec<SlashCommand> {
    let backend = provider.as_deref().unwrap_or("claude-code");
    match backend {
        "codex" => get_codex_commands(),
        "claude-code" => get_claude_commands(),
        _ => get_opencode_commands(),
    }
}

fn get_claude_commands() -> Vec<SlashCommand> {
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

    // Claude plugins from installed_plugins.json
    let plugins_file = home
        .join(".claude")
        .join("plugins")
        .join("installed_plugins.json");
    if let Ok(content) = std::fs::read_to_string(&plugins_file) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(plugins) = json.get("plugins").and_then(|p| p.as_object()) {
                for (key, entries) in plugins {
                    let plugin_name = key.split('@').next().unwrap_or(key);
                    if let Some(arr) = entries.as_array() {
                        for entry in arr {
                            if let Some(path) = entry.get("installPath").and_then(|p| p.as_str()) {
                                scan_plugin(Path::new(path), plugin_name, &mut result);
                            }
                        }
                    }
                }
            }
        }
    }

    dedup_commands(&mut result);
    result
}

fn get_codex_commands() -> Vec<SlashCommand> {
    let mut result: Vec<SlashCommand> = Vec::new();

    // Codex built-in commands
    let builtins = [("/model", "Switch model"), ("/help", "Show help")];
    for (cmd, desc) in builtins {
        result.push(SlashCommand {
            cmd: cmd.to_string(),
            desc: desc.to_string(),
            category: "built-in".to_string(),
        });
    }

    // Codex skills from ~/.agents/skills/
    if let Some(home) = dirs::home_dir() {
        let skills_dir = home.join(".agents").join("skills");
        if let Ok(entries) = std::fs::read_dir(&skills_dir) {
            for entry in entries.flatten() {
                let skill_file = entry.path().join("SKILL.md");
                if skill_file.exists() {
                    if let Ok(content) = std::fs::read_to_string(&skill_file) {
                        let name = frontmatter_field(&content, "name")
                            .unwrap_or_else(|| entry.file_name().to_string_lossy().to_string());
                        let desc = frontmatter_field(&content, "description").unwrap_or_default();
                        let desc_short = truncate_desc(&desc, 77);
                        result.push(SlashCommand {
                            cmd: format!("/{name}"),
                            desc: desc_short,
                            category: "skill".to_string(),
                        });
                    }
                }
            }
        }
    }

    dedup_commands(&mut result);
    result
}

fn get_opencode_commands() -> Vec<SlashCommand> {
    let mut result: Vec<SlashCommand> = Vec::new();

    // OpenCode built-in commands
    let builtins = [
        ("/model", "Switch model"),
        ("/help", "Show help"),
        ("/compact", "Compact conversation context"),
        ("/clear", "Clear conversation"),
        ("/cost", "Show token usage and cost"),
    ];
    for (cmd, desc) in builtins {
        result.push(SlashCommand {
            cmd: cmd.to_string(),
            desc: desc.to_string(),
            category: "built-in".to_string(),
        });
    }

    // OpenCode plugins from ~/.config/opencode/node_modules/
    // Plugins follow the same SKILL.md frontmatter format
    if let Some(home) = dirs::home_dir() {
        let plugins_dir = home.join(".config").join("opencode").join("node_modules");
        if let Ok(entries) = std::fs::read_dir(&plugins_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                // Scan scoped packages (@opencode-ai/plugin)
                if path
                    .file_name()
                    .is_some_and(|n| n.to_string_lossy().starts_with('@'))
                {
                    if let Ok(sub) = std::fs::read_dir(&path) {
                        for pkg in sub.flatten() {
                            let pkg_name = pkg.file_name().to_string_lossy().to_string();
                            scan_plugin(&pkg.path(), &pkg_name, &mut result);
                        }
                    }
                } else {
                    let pkg_name = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    scan_plugin(&path, &pkg_name, &mut result);
                }
            }
        }
    }

    dedup_commands(&mut result);
    result
}

fn dedup_commands(result: &mut Vec<SlashCommand>) {
    let mut seen = std::collections::HashSet::new();
    result.retain(|c| seen.insert(c.cmd.clone()));
}

#[cfg(test)]
mod tests {
    use super::truncate_desc;
    use crate::test_utils::TestCase;

    #[test]
    fn should_not_panic_on_multibyte_truncation() {
        let mut t = TestCase::new("should_not_panic_on_multibyte_truncation");
        // The string is: 76 'x' bytes + "é" (2 bytes) + 10 'y' bytes
        // truncate_desc at 77 bytes should: find boundary at 76 (start of é), truncate there,
        // append "..."
        let long_desc = format!("{}é{}", "x".repeat(76), "y".repeat(10));
        t.phase("Act");
        let result = truncate_desc(&long_desc, 77);
        t.phase("Assert");
        t.ok(
            "result is valid UTF-8",
            std::str::from_utf8(result.as_bytes()).is_ok(),
        );
        t.eq(
            "truncated at char boundary",
            result.as_str(),
            &format!("{}...", "x".repeat(76)),
        );
        t.ok(
            "result fits in max_bytes + ellipsis",
            result.len() <= 77 + 3,
        );
    }
}

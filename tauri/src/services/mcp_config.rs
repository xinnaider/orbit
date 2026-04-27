use std::path::{Path, PathBuf};

use toml_edit::{value, DocumentMut, Item, Table};

/// Find the orbit-mcp binary: next to current exe (with or without target triple), or in PATH.
pub fn find_orbit_mcp() -> Option<String> {
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            let ext = if cfg!(windows) { ".exe" } else { "" };

            let triple = env!("TARGET_TRIPLE");
            let sidecar = dir.join(format!("orbit-mcp-{triple}{ext}"));
            if sidecar.is_file() {
                return Some(sidecar.to_string_lossy().into_owned());
            }

            let plain = dir.join(format!("orbit-mcp{ext}"));
            if plain.is_file() {
                return Some(plain.to_string_lossy().into_owned());
            }
        }
    }

    crate::services::spawn_manager::find_cli_in_path("orbit-mcp")
}

/// Write provider-specific MCP configs in a project directory.
///
/// Claude Code reads `.mcp.json`, Codex reads `.codex/config.toml`, and
/// OpenCode reads `opencode.json`/`opencode.jsonc`.
pub fn write_orbit_mcp_configs(project_path: &Path, mcp_bin: &str) -> Result<(), String> {
    write_claude_mcp_config(project_path, mcp_bin)?;
    write_codex_mcp_config(project_path, mcp_bin)?;
    write_opencode_mcp_config(project_path, mcp_bin)?;

    Ok(())
}

fn write_claude_mcp_config(project_path: &Path, mcp_bin: &str) -> Result<(), String> {
    let config_path = project_path.join(".mcp.json");
    let mut config = read_json_config(&config_path, serde_json::json!({ "mcpServers": {} }))?;

    if !config
        .get("mcpServers")
        .is_some_and(serde_json::Value::is_object)
    {
        config["mcpServers"] = serde_json::json!({});
    }

    config["mcpServers"]["orbit"] = serde_json::json!({ "command": mcp_bin });
    write_json_config(&config_path, &config)
}

fn write_codex_mcp_config(project_path: &Path, mcp_bin: &str) -> Result<(), String> {
    let codex_dir = project_path.join(".codex");
    std::fs::create_dir_all(&codex_dir)
        .map_err(|e| format!("failed to create .codex directory: {e}"))?;

    let config_path = codex_dir.join("config.toml");
    let mut doc = if config_path.exists() {
        let raw = std::fs::read_to_string(&config_path)
            .map_err(|e| format!("failed to read .codex/config.toml: {e}"))?;
        raw.parse::<DocumentMut>()
            .map_err(|e| format!("failed to parse .codex/config.toml: {e}"))?
    } else {
        DocumentMut::new()
    };

    let root = doc.as_table_mut();
    if !root.get("mcp_servers").is_some_and(Item::is_table) {
        root.insert("mcp_servers", Item::Table(Table::new()));
    }

    let mcp_servers = root
        .get_mut("mcp_servers")
        .and_then(Item::as_table_mut)
        .ok_or_else(|| "failed to prepare Codex mcp_servers table".to_string())?;

    if !mcp_servers.get("orbit").is_some_and(Item::is_table) {
        mcp_servers.insert("orbit", Item::Table(Table::new()));
    }

    let orbit = mcp_servers
        .get_mut("orbit")
        .and_then(Item::as_table_mut)
        .ok_or_else(|| "failed to prepare Codex orbit MCP table".to_string())?;
    orbit["command"] = value(mcp_bin);
    orbit["enabled"] = value(true);

    std::fs::write(&config_path, doc.to_string())
        .map_err(|e| format!("failed to write .codex/config.toml: {e}"))
}

fn write_opencode_mcp_config(project_path: &Path, mcp_bin: &str) -> Result<(), String> {
    let config_path = opencode_config_path(project_path);
    let mut config = read_json_config(
        &config_path,
        serde_json::json!({ "$schema": "https://opencode.ai/config.json" }),
    )?;

    if !config.is_object() {
        config = serde_json::json!({});
    }

    if config.get("$schema").is_none() {
        config["$schema"] = serde_json::json!("https://opencode.ai/config.json");
    }

    if !config.get("mcp").is_some_and(serde_json::Value::is_object) {
        config["mcp"] = serde_json::json!({});
    }

    config["mcp"]["orbit"] = serde_json::json!({
        "type": "local",
        "command": [mcp_bin],
        "enabled": true
    });

    write_json_config(&config_path, &config)
}

fn opencode_config_path(project_path: &Path) -> PathBuf {
    let json = project_path.join("opencode.json");
    let jsonc = project_path.join("opencode.jsonc");

    if json.exists() || !jsonc.exists() {
        json
    } else {
        jsonc
    }
}

fn read_json_config(path: &Path, fallback: serde_json::Value) -> Result<serde_json::Value, String> {
    if !path.exists() {
        return Ok(fallback);
    }

    let raw = std::fs::read_to_string(path)
        .map_err(|e| format!("failed to read {}: {e}", path.display()))?;
    let stripped = normalize_jsonc(&raw);

    serde_json::from_str(&stripped).map_err(|e| format!("failed to parse {}: {e}", path.display()))
}

fn write_json_config(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    let content = serde_json::to_string_pretty(value)
        .map_err(|e| format!("failed to serialize {}: {e}", path.display()))?;

    std::fs::write(path, content).map_err(|e| format!("failed to write {}: {e}", path.display()))
}

fn normalize_jsonc(s: &str) -> String {
    strip_jsonc_trailing_commas(&strip_jsonc_comments(s))
}

fn strip_jsonc_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while let Some(ch) = chars.next() {
        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
                out.push(ch);
            }
            continue;
        }

        if in_block_comment {
            if ch == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block_comment = false;
            }
            continue;
        }

        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'/') {
            chars.next();
            in_line_comment = true;
            continue;
        }

        if ch == '/' && chars.peek() == Some(&'*') {
            chars.next();
            in_block_comment = true;
            continue;
        }

        out.push(ch);
    }

    out
}

fn strip_jsonc_trailing_commas(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if in_string {
            out.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            out.push(ch);
            continue;
        }

        if ch == ',' {
            let mut lookahead = chars.clone();
            while lookahead.peek().is_some_and(|c| c.is_whitespace()) {
                lookahead.next();
            }

            if matches!(lookahead.peek(), Some('}' | ']')) {
                continue;
            }
        }

        out.push(ch);
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestCase;

    #[test]
    fn should_write_orbit_mcp_config_for_all_supported_providers() {
        let mut t = TestCase::new("should_write_orbit_mcp_config_for_all_supported_providers");
        let dir = tempfile::TempDir::new().expect("temp dir");
        let mcp_bin = "/opt/orbit/orbit-mcp";

        t.phase("Act");
        write_orbit_mcp_configs(dir.path(), mcp_bin).expect("write configs");

        t.phase("Assert - Claude");
        let claude_config: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join(".mcp.json")).expect(".mcp.json"),
        )
        .expect("claude json");
        t.eq(
            "claude command",
            claude_config["mcpServers"]["orbit"]["command"].as_str(),
            Some(mcp_bin),
        );

        t.phase("Assert - Codex");
        let codex_config =
            std::fs::read_to_string(dir.path().join(".codex/config.toml")).expect("codex config");
        let codex_doc = codex_config
            .parse::<DocumentMut>()
            .expect("codex config toml");
        t.eq(
            "codex command",
            codex_doc["mcp_servers"]["orbit"]["command"].as_str(),
            Some(mcp_bin),
        );
        t.eq(
            "codex enabled",
            codex_doc["mcp_servers"]["orbit"]["enabled"].as_bool(),
            Some(true),
        );

        t.phase("Assert - OpenCode");
        let opencode_config: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("opencode.json")).expect("opencode.json"),
        )
        .expect("opencode json");
        t.eq(
            "opencode type",
            opencode_config["mcp"]["orbit"]["type"].as_str(),
            Some("local"),
        );
        t.eq(
            "opencode command",
            opencode_config["mcp"]["orbit"]["command"][0].as_str(),
            Some(mcp_bin),
        );
    }

    #[test]
    fn should_preserve_existing_provider_configs_when_registering_orbit() {
        let mut t =
            TestCase::new("should_preserve_existing_provider_configs_when_registering_orbit");
        let dir = tempfile::TempDir::new().expect("temp dir");
        let codex_dir = dir.path().join(".codex");
        std::fs::create_dir_all(&codex_dir).expect("codex dir");

        std::fs::write(
            dir.path().join(".mcp.json"),
            r#"{"mcpServers":{"docs":{"command":"docs-server"}}}"#,
        )
        .expect("seed claude");
        std::fs::write(
            codex_dir.join("config.toml"),
            r#"model = "gpt-5.5"

[mcp_servers.docs]
command = "docs-server"
"#,
        )
        .expect("seed codex");
        std::fs::write(
            dir.path().join("opencode.json"),
            r#"{"plugin":["superpowers"],"mcp":{"docs":{"type":"local","command":["docs-server"]}}}"#,
        )
        .expect("seed opencode");

        t.phase("Act");
        write_orbit_mcp_configs(dir.path(), "/opt/orbit/orbit-mcp").expect("write configs");

        t.phase("Assert");
        let claude_config: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join(".mcp.json")).expect(".mcp.json"),
        )
        .expect("claude json");
        t.eq(
            "claude preserves docs server",
            claude_config["mcpServers"]["docs"]["command"].as_str(),
            Some("docs-server"),
        );

        let codex_config =
            std::fs::read_to_string(dir.path().join(".codex/config.toml")).expect("codex config");
        let codex_doc = codex_config
            .parse::<DocumentMut>()
            .expect("codex config toml");
        t.eq(
            "codex preserves model",
            codex_doc["model"].as_str(),
            Some("gpt-5.5"),
        );
        t.eq(
            "codex preserves docs server",
            codex_doc["mcp_servers"]["docs"]["command"].as_str(),
            Some("docs-server"),
        );

        let opencode_config: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("opencode.json")).expect("opencode.json"),
        )
        .expect("opencode json");
        t.eq(
            "opencode preserves plugin",
            opencode_config["plugin"][0].as_str(),
            Some("superpowers"),
        );
        t.eq(
            "opencode preserves docs server",
            opencode_config["mcp"]["docs"]["command"][0].as_str(),
            Some("docs-server"),
        );
    }

    #[test]
    fn should_merge_existing_opencode_jsonc_with_comments_and_trailing_commas() {
        let mut t =
            TestCase::new("should_merge_existing_opencode_jsonc_with_comments_and_trailing_commas");
        let dir = tempfile::TempDir::new().expect("temp dir");
        std::fs::write(
            dir.path().join("opencode.jsonc"),
            r#"
            {
              "$schema": "https://opencode.ai/config.json",
              // Keep the user's plugin list intact.
              "plugin": [
                "superpowers",
              ],
            }
            "#,
        )
        .expect("seed opencode jsonc");

        t.phase("Act");
        write_orbit_mcp_configs(dir.path(), "/opt/orbit/orbit-mcp").expect("write configs");

        t.phase("Assert");
        let opencode_config: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("opencode.jsonc")).expect("opencode.jsonc"),
        )
        .expect("opencode json");
        t.eq(
            "opencode preserves plugin",
            opencode_config["plugin"][0].as_str(),
            Some("superpowers"),
        );
        t.eq(
            "opencode registers orbit",
            opencode_config["mcp"]["orbit"]["command"][0].as_str(),
            Some("/opt/orbit/orbit-mcp"),
        );
    }
}

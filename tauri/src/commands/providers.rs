use crate::services::spawn_manager::{find_claude, find_codex, find_opencode};

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    pub context: Option<u64>,
    pub output: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderInfo {
    pub id: String,
    pub name: String,
    pub env: Vec<String>,
    pub models: Vec<ModelInfo>,
    pub configured: bool,
    pub cli_available: bool,
    /// "claude" | "opencode" | "codex"
    pub cli_backend: String,
}

/// Read providers from ~/.cache/opencode/models.json + detect installed CLIs.
#[tauri::command]
pub fn get_providers() -> Vec<ProviderInfo> {
    let claude_available = find_claude().is_some();
    let opencode_available = find_opencode().is_some();
    let codex_available = find_codex().is_some();

    let mut providers = vec![];

    // 1. Claude Code — always first, hardcoded models
    providers.push(ProviderInfo {
        id: "claude-code".to_string(),
        name: "Claude Code".to_string(),
        env: vec![],
        models: vec![
            ModelInfo {
                id: "auto".to_string(),
                name: "auto".to_string(),
                context: None,
                output: None,
            },
            ModelInfo {
                id: "claude-sonnet-4-6".to_string(),
                name: "sonnet-4.6".to_string(),
                context: Some(1_000_000),
                output: Some(64_000),
            },
            ModelInfo {
                id: "claude-opus-4-6".to_string(),
                name: "opus-4.6".to_string(),
                context: Some(1_000_000),
                output: Some(128_000),
            },
            ModelInfo {
                id: "claude-haiku-4-5-20251001".to_string(),
                name: "haiku-4.5".to_string(),
                context: Some(200_000),
                output: Some(64_000),
            },
        ],
        configured: true,
        cli_available: claude_available,
        cli_backend: "claude".to_string(),
    });

    // 2. Codex — hardcoded models
    providers.push(ProviderInfo {
        id: "codex".to_string(),
        name: "Codex".to_string(),
        env: vec!["OPENAI_API_KEY".to_string()],
        models: vec![
            ModelInfo {
                id: "o3".to_string(),
                name: "o3".to_string(),
                context: Some(200_000),
                output: Some(100_000),
            },
            ModelInfo {
                id: "o4-mini".to_string(),
                name: "o4-mini".to_string(),
                context: Some(200_000),
                output: Some(100_000),
            },
            ModelInfo {
                id: "gpt-4.1".to_string(),
                name: "gpt-4.1".to_string(),
                context: Some(1_000_000),
                output: Some(32_768),
            },
            ModelInfo {
                id: "codex-mini-latest".to_string(),
                name: "codex-mini".to_string(),
                context: Some(200_000),
                output: Some(100_000),
            },
        ],
        configured: std::env::var("OPENAI_API_KEY").is_ok(),
        cli_available: codex_available,
        cli_backend: "codex".to_string(),
    });

    // 3. Read opencode models.json for all other providers
    if let Some(opencode_providers) = read_opencode_models() {
        for (id, provider) in opencode_providers {
            let env_vars: Vec<String> = provider
                .get("env")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();

            let configured = env_vars.iter().all(|var| std::env::var(var).is_ok());

            let name = provider
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(&id)
                .to_string();

            let models: Vec<ModelInfo> = provider
                .get("models")
                .and_then(|v| v.as_object())
                .map(|m| {
                    m.iter()
                        .map(|(mid, mval)| ModelInfo {
                            id: mid.clone(),
                            name: mval
                                .get("name")
                                .and_then(|v| v.as_str())
                                .unwrap_or(mid)
                                .to_string(),
                            context: mval.pointer("/limit/context").and_then(|v| v.as_u64()),
                            output: mval.pointer("/limit/output").and_then(|v| v.as_u64()),
                        })
                        .collect()
                })
                .unwrap_or_default();

            providers.push(ProviderInfo {
                id,
                name,
                env: env_vars,
                models,
                configured,
                cli_available: opencode_available,
                cli_backend: "opencode".to_string(),
            });
        }
    }

    providers
}

/// Check if an environment variable exists (without exposing the value).
#[tauri::command]
pub fn check_env_var(name: String) -> bool {
    std::env::var(&name).is_ok()
}

fn read_opencode_models() -> Option<Vec<(String, serde_json::Value)>> {
    let home = dirs::home_dir()?;
    let path = home.join(".cache").join("opencode").join("models.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let data: serde_json::Value = serde_json::from_str(&content).ok()?;
    let obj = data.as_object()?;
    let mut result: Vec<(String, serde_json::Value)> =
        obj.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    result.sort_by(|a, b| a.0.cmp(&b.0));
    Some(result)
}

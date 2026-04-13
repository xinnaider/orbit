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
pub struct SubProvider {
    pub id: String,
    pub name: String,
    pub env: Vec<String>,
    pub configured: bool,
    pub models: Vec<ModelInfo>,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CliBackend {
    pub id: String,
    pub name: String,
    pub cli_available: bool,
    /// Direct models (for claude-code and codex)
    pub models: Vec<ModelInfo>,
    /// Sub-providers (for opencode only)
    pub sub_providers: Vec<SubProvider>,
}

/// Return the 3 CLI backends with their models/sub-providers.
#[tauri::command]
pub fn get_providers() -> Vec<CliBackend> {
    let claude_available = find_claude().is_some();
    let opencode_available = find_opencode().is_some();
    let codex_available = find_codex().is_some();

    let mut backends = vec![];

    // 1. Claude Code
    backends.push(CliBackend {
        id: "claude-code".to_string(),
        name: "Claude Code".to_string(),
        cli_available: claude_available,
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
        sub_providers: vec![],
    });

    // 2. Codex — auth via CLI (codex login), no API key needed
    backends.push(CliBackend {
        id: "codex".to_string(),
        name: "Codex".to_string(),
        cli_available: codex_available,
        models: vec![
            ModelInfo {
                id: "gpt-5.4".to_string(),
                name: "gpt-5.4".to_string(),
                context: None,
                output: None,
            },
            ModelInfo {
                id: "gpt-5.4-mini".to_string(),
                name: "gpt-5.4-mini".to_string(),
                context: None,
                output: None,
            },
            ModelInfo {
                id: "gpt-5.3-codex".to_string(),
                name: "gpt-5.3-codex".to_string(),
                context: None,
                output: None,
            },
            ModelInfo {
                id: "gpt-5.2".to_string(),
                name: "gpt-5.2".to_string(),
                context: None,
                output: None,
            },
        ],
        sub_providers: vec![],
    });

    // 3. OpenCode — sub-providers from ~/.cache/opencode/models.json
    let sub_providers = read_opencode_providers().unwrap_or_default();
    backends.push(CliBackend {
        id: "opencode".to_string(),
        name: "OpenCode".to_string(),
        cli_available: opencode_available,
        models: vec![],
        sub_providers,
    });

    backends
}

/// Check if an environment variable exists (without exposing the value).
#[tauri::command]
pub fn check_env_var(name: String) -> bool {
    std::env::var(&name).is_ok()
}

fn read_opencode_providers() -> Option<Vec<SubProvider>> {
    let home = dirs::home_dir()?;
    let path = home.join(".cache").join("opencode").join("models.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let data: serde_json::Value = serde_json::from_str(&content).ok()?;
    let obj = data.as_object()?;

    let mut result: Vec<SubProvider> = obj
        .iter()
        .map(|(id, provider)| {
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
                .unwrap_or(id)
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

            SubProvider {
                id: id.clone(),
                name,
                env: env_vars,
                configured,
                models,
            }
        })
        .collect();

    result.sort_by(|a, b| a.name.cmp(&b.name));
    Some(result)
}

/// Look up the context window for a model from models.json.
/// `provider` is the opencode sub-provider (e.g. "openrouter").
/// `model` is the model ID within that provider (e.g. "minimax/minimax-m2.5:free").
pub fn lookup_context_window(provider: &str, model: &str) -> Option<u64> {
    let home = dirs::home_dir()?;
    let path = home.join(".cache").join("opencode").join("models.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let data: serde_json::Value = serde_json::from_str(&content).ok()?;
    data.pointer(&format!("/{provider}/models/{model}/limit/context"))
        .and_then(|v| v.as_u64())
}

/// Context window for Codex models (hardcoded — not in models.json).
pub fn codex_context_window(model: &str) -> u64 {
    match model {
        "gpt-5.4" | "gpt-5.4-mini" => 200_000,
        "gpt-5.3-codex" => 200_000,
        "gpt-5.2" => 200_000,
        _ => 200_000,
    }
}

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
    pub cli_name: String,
    pub cli_available: bool,
    pub install_hint: String,
    pub supports_effort: bool,
    pub supports_ssh: bool,
    pub supports_subagents: bool,
    pub has_sub_providers: bool,
    /// Direct models (for claude-code and codex)
    pub models: Vec<ModelInfo>,
    /// Sub-providers (for opencode only)
    pub sub_providers: Vec<SubProvider>,
    /// Effort levels keyed by model glob; empty when effort not supported.
    pub effort_levels: std::collections::HashMap<String, Vec<String>>,
}

/// Return all CLI backends with their capabilities and models.
/// Built dynamically from the provider registry — no hardcoded list.
#[tauri::command]
pub fn get_providers(
    registry: tauri::State<crate::ipc::session::ProviderRegistryState>,
) -> Vec<CliBackend> {
    // Stable ordering: claude-code first, then codex, then opencode
    let order = ["claude-code", "codex", "opencode"];
    let mut providers = registry.0.all();
    providers.sort_by_key(|p| {
        order
            .iter()
            .position(|&id| id == p.id())
            .unwrap_or(usize::MAX)
    });

    let opencode_sub_providers = read_opencode_providers().unwrap_or_default();

    providers
        .iter()
        .map(|p| {
            let has_subs = p.id() == "opencode" && !opencode_sub_providers.is_empty();
            let models = get_provider_models(p.id());
            let mut effort_levels = std::collections::HashMap::new();
            for model in &models {
                let levels = p.effort_levels(&model.id);
                if !levels.is_empty() {
                    effort_levels.insert(
                        model.id.clone(),
                        levels.iter().map(|s| s.to_string()).collect(),
                    );
                }
            }
            CliBackend {
                id: p.id().to_string(),
                name: p.display_name().to_string(),
                cli_name: p.cli_name().to_string(),
                cli_available: p.find_cli().is_some(),
                install_hint: p.install_hint().to_string(),
                supports_effort: p.supports_effort(),
                supports_ssh: p.supports_ssh(),
                supports_subagents: p.supports_subagents(),
                has_sub_providers: has_subs,
                effort_levels,
                models,
                sub_providers: if has_subs {
                    opencode_sub_providers.clone()
                } else {
                    vec![]
                },
            }
        })
        .collect()
}

/// Return the hardcoded model list for a provider.
/// Models are intrinsic to each CLI — not worth abstracting into the trait.
fn get_provider_models(provider_id: &str) -> Vec<ModelInfo> {
    match provider_id {
        "claude-code" => vec![
            ModelInfo {
                id: "auto".into(),
                name: "auto".into(),
                context: None,
                output: None,
            },
            ModelInfo {
                id: "claude-opus-4-7".into(),
                name: "opus-4.7".into(),
                context: Some(1_000_000),
                output: Some(128_000),
            },
            ModelInfo {
                id: "claude-sonnet-4-6".into(),
                name: "sonnet-4.6".into(),
                context: Some(1_000_000),
                output: Some(64_000),
            },
            ModelInfo {
                id: "claude-opus-4-6".into(),
                name: "opus-4.6".into(),
                context: Some(1_000_000),
                output: Some(128_000),
            },
            ModelInfo {
                id: "claude-haiku-4-5-20251001".into(),
                name: "haiku-4.5".into(),
                context: Some(200_000),
                output: Some(64_000),
            },
        ],
        "codex" => vec![
            ModelInfo {
                id: "gpt-5.4".into(),
                name: "gpt-5.4".into(),
                context: None,
                output: None,
            },
            ModelInfo {
                id: "gpt-5.4-mini".into(),
                name: "gpt-5.4-mini".into(),
                context: None,
                output: None,
            },
            ModelInfo {
                id: "gpt-5.3-codex".into(),
                name: "gpt-5.3-codex".into(),
                context: None,
                output: None,
            },
            ModelInfo {
                id: "gpt-5.2".into(),
                name: "gpt-5.2".into(),
                context: None,
                output: None,
            },
        ],
        _ => vec![], // OpenCode models come from sub-providers
    }
}

/// Check if an environment variable exists (without exposing the value).
#[tauri::command]
pub fn check_env_var(name: String) -> bool {
    std::env::var(&name).is_ok()
}

/// Diagnose a provider: check if CLI is found, get version, report path,
/// and verify the project directory exists.
/// When SSH params are provided, first tests the SSH connection, then checks
/// for the CLI and directory on the remote machine.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn diagnose_provider(
    backend: String,
    project_path: Option<String>,
    ssh_host: Option<String>,
    ssh_user: Option<String>,
    ssh_key_path: Option<String>,
    registry: tauri::State<crate::ipc::session::ProviderRegistryState>,
) -> serde_json::Value {
    let provider = match registry.0.resolve(&backend) {
        Some(p) => p,
        None => {
            return serde_json::json!({
                "backend": backend,
                "cliName": backend,
                "found": false,
                "path": null,
                "version": null,
                "installHint": "unknown provider",
                "ssh": null,
                "projectDirOk": null,
            });
        }
    };

    let cli_name = provider.cli_name().to_string();
    let install_hint = provider.install_hint().to_string();

    // SSH mode: test connection → check CLI → check dir on remote
    if let (Some(ref host), Some(ref user)) = (&ssh_host, &ssh_user) {
        let ssh_result =
            crate::services::ssh::test_ssh_connection(host, user, ssh_key_path.as_deref());

        if !ssh_result.ok {
            return serde_json::json!({
                "backend": backend,
                "cliName": cli_name,
                "found": false,
                "path": null,
                "version": null,
                "installHint": install_hint,
                "ssh": {
                    "ok": false,
                    "latencyMs": ssh_result.latency_ms,
                    "error": ssh_result.error,
                },
                "projectDirOk": null,
            });
        }

        // SSH connected — check CLI + dir in one remote call
        let dir_check = if let Some(ref pp) = project_path {
            format!(" && test -d {pp} && echo __dir_ok__")
        } else {
            String::new()
        };
        let remote_script = format!("which {cli_name} && {cli_name} --version{dir_check}");

        let (path, version, dir_ok) = match crate::services::ssh::spawn_via_ssh(
            host,
            user,
            ssh_key_path.as_deref(),
            &remote_script,
        ) {
            Ok((child, _guard)) => {
                let output = child.wait_with_output().ok();
                match output {
                    Some(o) => {
                        let stdout = String::from_utf8_lossy(&o.stdout);
                        let mut lines = stdout.lines();
                        let p = lines.next().map(|l| l.trim().to_string());
                        let v = lines
                            .next()
                            .map(|l| l.trim().to_string())
                            .filter(|s| !s.is_empty() && !s.contains("__dir_ok__"));
                        let has_dir = stdout.contains("__dir_ok__");
                        if o.status.success() {
                            (p, v, Some(has_dir))
                        } else {
                            // `which` succeeded but `test -d` failed → CLI found, dir missing
                            let cli_found =
                                p.is_some() && p.as_ref().is_some_and(|s| !s.is_empty());
                            if cli_found {
                                (p, v, Some(false))
                            } else {
                                (None, None, None)
                            }
                        }
                    }
                    _ => (None, None, None),
                }
            }
            Err(_) => (None, None, None),
        };

        let found = path.is_some();
        return serde_json::json!({
            "backend": backend,
            "cliName": cli_name,
            "found": found,
            "path": path,
            "version": version,
            "installHint": install_hint,
            "ssh": {
                "ok": true,
                "latencyMs": ssh_result.latency_ms,
                "error": "",
            },
            "projectDirOk": if project_path.is_some() { serde_json::json!(dir_ok) } else { serde_json::json!(null) },
        });
    }

    // Local mode: check CLI + directory
    let path = provider.find_cli();
    let found = path.is_some();

    let version = if let Some(ref p) = path {
        run_cli_version(p)
    } else {
        None
    };

    let project_dir_ok = project_path.as_deref().filter(|p| !p.is_empty()).map(|p| {
        let result = std::path::Path::new(p).is_dir();
        if cfg!(debug_assertions) {
            eprintln!(
                "[orbit:debug] diagnose project_path={p:?} is_dir={result} exists={}",
                std::path::Path::new(p).exists()
            );
        }
        result
    });

    serde_json::json!({
        "backend": backend,
        "cliName": cli_name,
        "found": found,
        "path": path,
        "version": version,
        "installHint": install_hint,
        "ssh": null,
        "projectDirOk": project_dir_ok,
    })
}

fn run_cli_version(path: &str) -> Option<String> {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let out = std::process::Command::new(path)
            .arg("--version")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        Some(if stdout.is_empty() { stderr } else { stdout })
    }
    #[cfg(not(windows))]
    {
        let out = std::process::Command::new(path)
            .arg("--version")
            .output()
            .ok()?;
        Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
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

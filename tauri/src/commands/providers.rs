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
    pub supports_tasks: bool,
    pub has_sub_providers: bool,
    /// Direct models (for claude-code and codex)
    pub models: Vec<ModelInfo>,
    /// Sub-providers (for opencode only)
    pub sub_providers: Vec<SubProvider>,
    /// Effort levels keyed by model glob; empty when effort not supported.
    pub effort_levels: std::collections::HashMap<String, Vec<String>>,
    /// Tool names that trigger task detection for this provider (e.g. ["TodoWrite"]).
    pub task_tool_names: Vec<String>,
    /// Format this provider uses to emit tasks.
    pub task_format: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedOpenCodeRequest {
    pub provider_id: String,
    pub model: Option<String>,
}

/// Build the full provider list — used by both the Tauri command and the MCP handler.
pub fn build_cli_backends(registry: &crate::providers::ProviderRegistry) -> Vec<CliBackend> {
    // Stable ordering: claude-code first, then codex, then opencode
    let order = ["claude-code", "codex", "opencode"];
    let mut providers = registry.all();
    providers.sort_by_key(|p| {
        order
            .iter()
            .position(|&id| id == p.id())
            .unwrap_or(usize::MAX)
    });

    let opencode_sub_providers = opencode_subproviders();

    providers
        .iter()
        .map(|p| {
            let has_subs = p.id() == "opencode" && !opencode_sub_providers.is_empty();
            let models = get_provider_models(p.id());
            let mut effort_levels = std::collections::HashMap::new();
            let default_levels = p.effort_levels("auto");
            if !default_levels.is_empty() {
                effort_levels.insert(
                    "auto".to_string(),
                    default_levels.iter().map(|s| s.to_string()).collect(),
                );
            }
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
                supports_tasks: p.supports_tasks(),
                has_sub_providers: has_subs,
                effort_levels,
                models,
                sub_providers: if has_subs {
                    opencode_sub_providers.clone()
                } else {
                    vec![]
                },
                task_tool_names: p.task_tool_names().iter().map(|s| s.to_string()).collect(),
                task_format: match p.task_format() {
                    crate::models::TaskFormat::ClaudeToolUse => "claude_tool_use".to_string(),
                    crate::models::TaskFormat::OpenCodeToolUse => "opencode_tool_use".to_string(),
                    crate::models::TaskFormat::CodexItemList => "codex_item_list".to_string(),
                },
            }
        })
        .collect()
}

pub fn opencode_subproviders() -> Vec<SubProvider> {
    let mut opencode_sub_providers = read_opencode_providers().unwrap_or_default();
    let jsonc_providers = read_opencode_jsonc_providers().unwrap_or_default();
    for custom in jsonc_providers {
        if !opencode_sub_providers.iter().any(|sp| sp.id == custom.id) {
            opencode_sub_providers.push(custom);
        }
    }
    opencode_sub_providers.sort_by(|a, b| a.name.cmp(&b.name));
    opencode_sub_providers
}

pub fn normalize_session_provider_model(
    registry: &crate::providers::ProviderRegistry,
    provider_id: Option<&str>,
    model: Option<&str>,
) -> Result<(Option<String>, Option<String>), String> {
    let subproviders = opencode_subproviders();
    let mut normalized_provider = nonempty(provider_id).map(ToString::to_string);
    let mut normalized_model = nonempty(model).map(ToString::to_string);

    if should_try_opencode_resolution(
        registry,
        normalized_provider.as_deref(),
        normalized_model.as_deref(),
        &subproviders,
    ) {
        if let Some(resolved) = resolve_opencode_request_from_subproviders(
            &subproviders,
            normalized_provider.as_deref(),
            normalized_model.as_deref(),
        ) {
            normalized_provider = Some(resolved.provider_id);
            normalized_model = resolved.model;
        }
    }

    if let Some(pid) = normalized_provider.as_deref() {
        let is_registered = registry.get(pid).is_some();
        let is_opencode_subprovider = subproviders.iter().any(|sub| sub.id == pid);
        if !is_registered && !is_opencode_subprovider {
            return Err(format_unknown_provider(pid, registry, &subproviders));
        }
    }

    Ok((normalized_provider, normalized_model))
}

pub fn resolve_opencode_request(
    provider_id: Option<&str>,
    model: Option<&str>,
) -> Option<ResolvedOpenCodeRequest> {
    let subproviders = opencode_subproviders();
    resolve_opencode_request_from_subproviders(&subproviders, provider_id, model)
}

fn nonempty(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

fn should_try_opencode_resolution(
    registry: &crate::providers::ProviderRegistry,
    provider_id: Option<&str>,
    model: Option<&str>,
    subproviders: &[SubProvider],
) -> bool {
    if let Some(pid) = provider_id {
        return pid.eq_ignore_ascii_case("opencode") || registry.get(pid).is_none();
    }

    model
        .and_then(|raw| raw.split_once('/').map(|(provider, _)| provider.trim()))
        .is_some_and(|provider| {
            provider.eq_ignore_ascii_case("opencode")
                || subproviders.iter().any(|sub| sub.id == provider)
        })
}

fn resolve_opencode_request_from_subproviders(
    subproviders: &[SubProvider],
    provider_id: Option<&str>,
    model: Option<&str>,
) -> Option<ResolvedOpenCodeRequest> {
    if subproviders.is_empty() {
        return None;
    }

    let mut provider_hint = nonempty(provider_id)
        .filter(|provider| !provider.eq_ignore_ascii_case("opencode"))
        .map(ToString::to_string);
    let mut model_query = nonempty(model).map(ToString::to_string);

    if let Some(raw_model) = nonempty(model) {
        if let Some((provider, model_id)) = raw_model.split_once('/') {
            let provider = provider.trim();
            let model_id = model_id.trim();
            if !provider.is_empty() && !provider.eq_ignore_ascii_case("opencode") {
                provider_hint = Some(provider.to_string());
            }
            if !model_id.is_empty() {
                model_query = Some(model_id.to_string());
            }
        }
    }

    let model_query = match model_query {
        Some(query) => query,
        None => {
            let provider = best_opencode_provider(subproviders, provider_hint.as_deref()?)?;
            return Some(ResolvedOpenCodeRequest {
                provider_id: provider.id.clone(),
                model: None,
            });
        }
    };

    let mut best: Option<(i64, &SubProvider, &ModelInfo)> = None;
    for subprovider in subproviders {
        let provider_score = match provider_hint.as_deref() {
            Some(hint) => provider_match_score(subprovider, hint),
            None => 0,
        };
        if provider_hint.is_some() && provider_score <= 0 {
            continue;
        }

        for model in &subprovider.models {
            let model_score = model_match_score(subprovider, model, &model_query);
            if model_score <= 0 {
                continue;
            }

            let score = provider_score + model_score;
            let should_replace = match best {
                Some((best_score, best_provider, best_model)) => {
                    score > best_score
                        || (score == best_score
                            && candidate_tie_breaker(subprovider, model)
                                < candidate_tie_breaker(best_provider, best_model))
                }
                None => true,
            };

            if should_replace {
                best = Some((score, subprovider, model));
            }
        }
    }

    let (_, subprovider, model) = best?;
    Some(ResolvedOpenCodeRequest {
        provider_id: subprovider.id.clone(),
        model: Some(format!("{}/{}", subprovider.id, model.id)),
    })
}

fn best_opencode_provider<'a>(
    subproviders: &'a [SubProvider],
    provider_hint: &str,
) -> Option<&'a SubProvider> {
    let mut best: Option<(i64, &SubProvider)> = None;
    for subprovider in subproviders {
        let score = provider_match_score(subprovider, provider_hint);
        if score <= 0 {
            continue;
        }
        let should_replace = match best {
            Some((best_score, best_provider)) => {
                score > best_score
                    || (score == best_score
                        && provider_tie_breaker(subprovider) < provider_tie_breaker(best_provider))
            }
            None => true,
        };
        if should_replace {
            best = Some((score, subprovider));
        }
    }
    best.map(|(_, provider)| provider)
}

fn provider_match_score(provider: &SubProvider, query: &str) -> i64 {
    let query = query.trim();
    if query.is_empty() {
        return 0;
    }

    if provider.id.eq_ignore_ascii_case(query) || provider.name.eq_ignore_ascii_case(query) {
        return 100_000;
    }

    let normalized_query = normalize_search_key(query);
    let normalized_id = normalize_search_key(&provider.id);
    let normalized_name = normalize_search_key(&provider.name);
    if normalized_id == normalized_query || normalized_name == normalized_query {
        return 90_000;
    }
    if normalized_id.contains(&normalized_query)
        || normalized_query.contains(&normalized_id)
        || normalized_name.contains(&normalized_query)
        || normalized_query.contains(&normalized_name)
    {
        return 70_000;
    }

    if tokens_match(
        &search_tokens(query),
        &format!("{} {}", provider.id, provider.name),
    ) {
        return 60_000;
    }

    0
}

fn model_match_score(provider: &SubProvider, model: &ModelInfo, query: &str) -> i64 {
    let query = query.trim();
    if query.is_empty() {
        return 0;
    }

    if model.id.eq_ignore_ascii_case(query) || model.name.eq_ignore_ascii_case(query) {
        return 1_000_000;
    }

    let normalized_query = normalize_search_key(query);
    let normalized_id = normalize_search_key(&model.id);
    let normalized_name = normalize_search_key(&model.name);
    if normalized_id == normalized_query || normalized_name == normalized_query {
        return 900_000;
    }

    let candidate_text = format!(
        "{} {} {} {}",
        provider.id, provider.name, model.id, model.name
    );
    if candidate_text
        .to_ascii_lowercase()
        .contains(&query.to_ascii_lowercase())
    {
        return 700_000;
    }

    let tokens = search_tokens(query);
    if tokens_match(&tokens, &candidate_text) {
        let specificity = tokens.iter().map(String::len).sum::<usize>() as i64;
        return 500_000 + specificity;
    }

    let normalized_candidate = normalize_search_key(&candidate_text);
    let partial = tokens
        .iter()
        .filter(|token| normalized_candidate.contains(token.as_str()))
        .count();
    if partial > 0 {
        return partial as i64 * 1_000;
    }

    0
}

fn tokens_match(tokens: &[String], candidate: &str) -> bool {
    if tokens.is_empty() {
        return false;
    }
    let normalized_candidate = normalize_search_key(candidate);
    tokens
        .iter()
        .all(|token| normalized_candidate.contains(token.as_str()))
}

fn search_tokens(query: &str) -> Vec<String> {
    query
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter_map(|token| {
            let token = token.trim();
            if token.is_empty() {
                None
            } else {
                Some(normalize_search_key(token))
            }
        })
        .collect()
}

fn normalize_search_key(value: &str) -> String {
    value
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect()
}

fn provider_tie_breaker(provider: &SubProvider) -> (bool, usize, &str) {
    (
        !provider.configured,
        provider.id.len(),
        provider.id.as_str(),
    )
}

fn candidate_tie_breaker<'a>(
    provider: &'a SubProvider,
    model: &'a ModelInfo,
) -> (bool, usize, &'a str, &'a str) {
    (
        !provider.configured,
        model.id.len(),
        provider.id.as_str(),
        model.id.as_str(),
    )
}

fn format_unknown_provider(
    provider_id: &str,
    registry: &crate::providers::ProviderRegistry,
    subproviders: &[SubProvider],
) -> String {
    let mut available: Vec<String> = registry
        .all()
        .into_iter()
        .map(|p| p.id().to_string())
        .collect();
    available.extend(subproviders.iter().map(|sub| sub.id.clone()));
    available.sort();
    available.dedup();
    format!(
        "Unknown provider \"{provider_id}\". Available: {}",
        available.join(", ")
    )
}

/// Return all CLI backends with their capabilities and models.
/// Built dynamically from the provider registry — no hardcoded list.
#[tauri::command]
pub fn get_providers(
    registry: tauri::State<crate::ipc::session::ProviderRegistryState>,
) -> Vec<CliBackend> {
    build_cli_backends(&registry.0)
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
                id: "gpt-5.5".into(),
                name: "gpt-5.5".into(),
                context: Some(1_000_000),
                output: Some(128_000),
            },
            ModelInfo {
                id: "gpt-5.4".into(),
                name: "gpt-5.4".into(),
                context: Some(1_050_000),
                output: Some(128_000),
            },
            ModelInfo {
                id: "gpt-5.4-mini".into(),
                name: "gpt-5.4-mini".into(),
                context: Some(400_000),
                output: Some(128_000),
            },
            ModelInfo {
                id: "gpt-5.3-codex".into(),
                name: "gpt-5.3-codex".into(),
                context: Some(400_000),
                output: Some(128_000),
            },
            ModelInfo {
                id: "gpt-5.2".into(),
                name: "gpt-5.2".into(),
                context: Some(400_000),
                output: Some(128_000),
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
    parse_cache_subproviders(&data)
}

fn strip_jsonc_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();
    let mut i = 0;
    let mut in_string = false;
    let mut in_line_comment = false;
    let mut in_block_comment = false;

    while i < len {
        let c = chars[i];
        let next = if i + 1 < len {
            Some(chars[i + 1])
        } else {
            None
        };

        if in_line_comment {
            if c == '\n' {
                in_line_comment = false;
                out.push(c);
            }
            i += 1;
            continue;
        }

        if in_block_comment {
            if c == '*' && next == Some('/') {
                in_block_comment = false;
                i += 2;
            } else {
                if c == '\n' {
                    out.push(c);
                }
                i += 1;
            }
            continue;
        }

        if in_string {
            out.push(c);
            if c == '\\' && next.is_some() {
                i += 1;
                out.push(chars[i]);
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if c == '/' && next == Some('/') {
            in_line_comment = true;
            i += 2;
            continue;
        }
        if c == '/' && next == Some('*') {
            in_block_comment = true;
            i += 2;
            continue;
        }
        if c == '"' {
            in_string = true;
        }
        out.push(c);
        i += 1;
    }

    // Second pass: remove trailing commas before `}` or `]`
    let chars: Vec<char> = out.chars().collect();
    let len = chars.len();
    let mut cleaned = String::with_capacity(len);
    let mut i = 0;
    let mut in_string = false;

    while i < len {
        let c = chars[i];

        if in_string {
            cleaned.push(c);
            if c == '\\' && i + 1 < len {
                i += 1;
                cleaned.push(chars[i]);
            } else if c == '"' {
                in_string = false;
            }
            i += 1;
            continue;
        }

        if c == '"' {
            in_string = true;
            cleaned.push(c);
            i += 1;
            continue;
        }

        if c == ',' {
            // Look ahead past whitespace — if the next non-whitespace char is `}` or `]`,
            // this is a trailing comma and we drop it
            let mut j = i + 1;
            while j < len && chars[j].is_whitespace() {
                j += 1;
            }
            if j < len && (chars[j] == '}' || chars[j] == ']') {
                i += 1;
                continue;
            }
        }

        cleaned.push(c);
        i += 1;
    }

    cleaned
}

fn read_opencode_jsonc_providers() -> Option<Vec<SubProvider>> {
    let home = dirs::home_dir()?;
    let dir = home.join(".config").join("opencode");
    let mut merged = Vec::new();

    for path in [dir.join("opencode.jsonc"), dir.join("opencode.json")] {
        let raw = match std::fs::read_to_string(&path) {
            Ok(raw) => raw,
            Err(_) => continue,
        };
        let stripped = strip_jsonc_comments(&raw);
        let data: serde_json::Value = match serde_json::from_str(&stripped) {
            Ok(data) => data,
            Err(_) => continue,
        };
        let providers = parse_config_subproviders(&data).unwrap_or_default();
        merge_subproviders(&mut merged, providers);
    }

    if merged.is_empty() {
        None
    } else {
        Some(merged)
    }
}

/// Look up the context window for a model from models.json.
/// `provider` is the opencode sub-provider (e.g. "openrouter").
/// `model` is the model ID within that provider (e.g. "minimax/minimax-m2.5:free").
pub fn lookup_context_window(provider: &str, model: &str) -> Option<u64> {
    let home = dirs::home_dir()?;
    let path = home.join(".cache").join("opencode").join("models.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let data: serde_json::Value = serde_json::from_str(&content).ok()?;
    lookup_context_window_in_value(&data, provider, model).or_else(|| {
        read_opencode_jsonc_providers().and_then(|providers| {
            providers
                .iter()
                .find(|sub| sub.id == provider)
                .and_then(|sub| sub.models.iter().find(|m| m.id == model))
                .and_then(|model| model.context)
        })
    })
}

pub fn resolve_context_window(
    provider_id: &str,
    model: Option<&str>,
    explicit: Option<u64>,
) -> Option<u64> {
    if explicit.is_some() {
        return explicit;
    }

    let model = model?;
    match provider_id {
        "claude-code" => Some(crate::models::context_window(model)),
        "codex" => None,
        "opencode" => resolve_opencode_model_context(model),
        custom => {
            lookup_context_window(custom, model).or_else(|| resolve_opencode_model_context(model))
        }
    }
}

/// Context window for Codex models (hardcoded — not in models.json).
pub fn codex_context_window(model: &str) -> u64 {
    match model {
        "gpt-5.5" => 1_000_000,
        "gpt-5.4" => 1_050_000,
        "gpt-5.4-mini" => 400_000,
        "gpt-5.3-codex" => 400_000,
        "gpt-5.2" => 400_000,
        _ => 400_000,
    }
}

fn parse_cache_subproviders(data: &serde_json::Value) -> Option<Vec<SubProvider>> {
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

            SubProvider {
                id: id.clone(),
                name: provider
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(id)
                    .to_string(),
                env: env_vars,
                configured,
                models: parse_models(provider),
            }
        })
        .collect();

    result.sort_by(|a, b| a.name.cmp(&b.name));
    Some(result)
}

fn parse_config_subproviders(data: &serde_json::Value) -> Option<Vec<SubProvider>> {
    let providers_obj = data.get("provider").and_then(|v| v.as_object())?;
    let mut result: Vec<SubProvider> = providers_obj
        .iter()
        .map(|(id, provider)| SubProvider {
            id: id.clone(),
            name: provider
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or(id)
                .to_string(),
            env: vec![],
            configured: provider
                .pointer("/options/apiKey")
                .and_then(|v| v.as_str())
                .is_some_and(|key| !key.is_empty()),
            models: parse_models(provider),
        })
        .collect();

    result.sort_by(|a, b| a.name.cmp(&b.name));
    Some(result)
}

fn parse_models(provider: &serde_json::Value) -> Vec<ModelInfo> {
    provider
        .get("models")
        .and_then(|v| v.as_object())
        .map(|models| {
            models
                .iter()
                .map(|(id, value)| ModelInfo {
                    id: id.clone(),
                    name: value
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or(id)
                        .to_string(),
                    context: value.pointer("/limit/context").and_then(|v| v.as_u64()),
                    output: value.pointer("/limit/output").and_then(|v| v.as_u64()),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn merge_subproviders(target: &mut Vec<SubProvider>, incoming: Vec<SubProvider>) {
    for provider in incoming {
        if let Some(existing) = target.iter_mut().find(|sub| sub.id == provider.id) {
            *existing = provider;
        } else {
            target.push(provider);
        }
    }
    target.sort_by(|a, b| a.name.cmp(&b.name));
}

fn lookup_context_window_in_value(
    data: &serde_json::Value,
    provider: &str,
    model: &str,
) -> Option<u64> {
    data.pointer(&format!("/{provider}/models/{model}/limit/context"))
        .and_then(|v| v.as_u64())
}

fn resolve_opencode_model_context(model: &str) -> Option<u64> {
    let (provider, model_id) = model.split_once('/')?;
    lookup_context_window(provider, model_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestCase;

    fn sample_opencode_subproviders() -> Vec<SubProvider> {
        vec![
            SubProvider {
                id: "ollama-cloud".to_string(),
                name: "Ollama Cloud".to_string(),
                env: vec![],
                configured: true,
                models: vec![
                    ModelInfo {
                        id: "kimi-k2.5".to_string(),
                        name: "Kimi K2.5".to_string(),
                        context: Some(200_000),
                        output: Some(32_000),
                    },
                    ModelInfo {
                        id: "kimi-k2.6:cloud".to_string(),
                        name: "Kimi K2.6 Cloud".to_string(),
                        context: Some(256_000),
                        output: Some(32_000),
                    },
                ],
            },
            SubProvider {
                id: "openrouter".to_string(),
                name: "OpenRouter".to_string(),
                env: vec![],
                configured: false,
                models: vec![ModelInfo {
                    id: "anthropic/claude-sonnet-4.5".to_string(),
                    name: "Claude Sonnet 4.5".to_string(),
                    context: Some(200_000),
                    output: Some(64_000),
                }],
            },
        ]
    }

    #[test]
    fn should_parse_custom_provider_from_jsonc_config() {
        let mut t = TestCase::new("should_parse_custom_provider_from_jsonc_config");
        let raw = r#"
        {
          // custom route
          "provider": {
            "omniroute": {
              "name": "Omniroute",
              "options": { "apiKey": "secret" },
              "models": {
                "gpt-4.1": {
                  "name": "GPT 4.1",
                  "limit": { "context": 123456, "output": 4096 }
                }
              }
            }
          }
        }
        "#;

        t.phase("Act");
        let stripped = strip_jsonc_comments(raw);
        let parsed: serde_json::Value = serde_json::from_str(&stripped).expect("valid json");
        let providers = parse_config_subproviders(&parsed).expect("providers");

        t.phase("Assert");
        t.len("one provider", &providers, 1);
        t.eq("provider id", providers[0].id.as_str(), "omniroute");
        t.eq("provider configured", providers[0].configured, true);
        t.len("one model", &providers[0].models, 1);
        t.eq(
            "context limit",
            providers[0].models[0].context,
            Some(123456),
        );
    }

    #[test]
    fn should_resolve_context_window_for_custom_provider_models() {
        let mut t = TestCase::new("should_resolve_context_window_for_custom_provider_models");
        let data = serde_json::json!({
            "omniroute": {
                "models": {
                    "gpt-4.1": {
                        "limit": { "context": 654321 }
                    }
                }
            }
        });

        t.phase("Assert");
        t.eq(
            "context limit from provider/model map",
            lookup_context_window_in_value(&data, "omniroute", "gpt-4.1"),
            Some(654321),
        );
    }

    #[test]
    fn should_resolve_context_window_from_prefixed_opencode_model() {
        let mut t = TestCase::new("should_resolve_context_window_from_prefixed_opencode_model");

        t.phase("Assert");
        t.eq(
            "claude fallback stays hardcoded",
            resolve_context_window("claude-code", Some("claude-opus-4-7[1m]"), None),
            Some(1_000_000),
        );
        t.eq(
            "codex fallback stays hardcoded",
            resolve_context_window("codex", Some("gpt-5.4"), None),
            None,
        );
        t.none(
            "unknown provider without metadata should not fabricate 200k",
            &resolve_context_window("gemini-cli", Some("gemini-2.5-pro"), None),
        );
    }

    #[test]
    fn should_resolve_opencode_prefix_to_real_subprovider_model() {
        let mut t = TestCase::new("should_resolve_opencode_prefix_to_real_subprovider_model");
        let providers = sample_opencode_subproviders();

        t.phase("Act");
        let resolved = resolve_opencode_request_from_subproviders(
            &providers,
            Some("opencode"),
            Some("opencode/kimi-k2.6:cloud"),
        )
        .expect("resolved opencode model");

        t.phase("Assert");
        t.eq("provider", resolved.provider_id.as_str(), "ollama-cloud");
        t.eq(
            "model",
            resolved.model.as_deref(),
            Some("ollama-cloud/kimi-k2.6:cloud"),
        );
    }

    #[test]
    fn should_resolve_fuzzy_ollama_kimi_request() {
        let mut t = TestCase::new("should_resolve_fuzzy_ollama_kimi_request");
        let providers = sample_opencode_subproviders();

        t.phase("Act");
        let resolved = resolve_opencode_request_from_subproviders(
            &providers,
            Some("ollama"),
            Some("kimi 2.6"),
        )
        .expect("resolved fuzzy opencode model");

        t.phase("Assert");
        t.eq("provider", resolved.provider_id.as_str(), "ollama-cloud");
        t.eq(
            "model",
            resolved.model.as_deref(),
            Some("ollama-cloud/kimi-k2.6:cloud"),
        );
    }

    #[test]
    fn should_keep_exact_subprovider_and_prefix_model_for_opencode() {
        let mut t = TestCase::new("should_keep_exact_subprovider_and_prefix_model_for_opencode");
        let providers = sample_opencode_subproviders();

        t.phase("Act");
        let resolved = resolve_opencode_request_from_subproviders(
            &providers,
            Some("ollama-cloud"),
            Some("kimi-k2.6:cloud"),
        )
        .expect("resolved exact subprovider model");

        t.phase("Assert");
        t.eq("provider", resolved.provider_id.as_str(), "ollama-cloud");
        t.eq(
            "model",
            resolved.model.as_deref(),
            Some("ollama-cloud/kimi-k2.6:cloud"),
        );
    }

    #[test]
    fn should_resolve_provider_from_prefixed_model_without_provider() {
        let mut t = TestCase::new("should_resolve_provider_from_prefixed_model_without_provider");
        let providers = sample_opencode_subproviders();

        t.phase("Act");
        let resolved = resolve_opencode_request_from_subproviders(
            &providers,
            None,
            Some("ollama-cloud/kimi-k2.6:cloud"),
        )
        .expect("resolved prefixed model");

        t.phase("Assert");
        t.eq("provider", resolved.provider_id.as_str(), "ollama-cloud");
        t.eq(
            "model",
            resolved.model.as_deref(),
            Some("ollama-cloud/kimi-k2.6:cloud"),
        );
    }
}

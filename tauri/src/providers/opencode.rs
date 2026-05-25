use super::{Provider, ProviderSpawnConfig};
use crate::journal::JournalState;
use crate::models::SlashCommand;
use crate::services::spawn_manager::{spawn_opencode, OpenCodeConfig, SpawnHandle};
use crate::services::ssh::{self, SpawnMode};

pub struct OpenCodeProvider;

impl Provider for OpenCodeProvider {
    fn id(&self) -> &str {
        "opencode"
    }

    fn display_name(&self) -> &str {
        "opencode"
    }

    fn spawn(&self, config: ProviderSpawnConfig) -> Result<SpawnHandle, String> {
        match config.spawn_mode {
            SpawnMode::Local => spawn_opencode(OpenCodeConfig {
                session_id: config.session_id,
                cwd: config.cwd,
                provider_id: config.provider_id.clone(),
                model: config.model,
                prompt: config.prompt,
                opencode_session_id: config.resume_id,
                extra_env: config.extra_env,
            }),
            SpawnMode::Ssh { ref host, ref user } => {
                let cwd_str = config.cwd.to_string_lossy();
                let cli_model = crate::services::spawn_manager::opencode_cli_model_arg(
                    &config.provider_id,
                    &config.model,
                );
                let mut parts: Vec<String> =
                    crate::services::spawn_manager::opencode_ssh_command_tokens(
                        &cwd_str,
                        &cli_model,
                        config.resume_id.as_deref(),
                    )
                    .into_iter()
                    .map(|token| match token.as_str() {
                        "opencode" | "run" | "--format" | "json" | "--dir" | "-m"
                        | "--continue" | "-s" => token,
                        _ => ssh::posix_escape(&token),
                    })
                    .collect();
                parts.push(ssh::posix_escape(&config.prompt));

                // Inline env vars: KEY=val KEY2=val2 cmd args
                let mut env_prefix = String::new();
                for (k, v) in &config.extra_env {
                    env_prefix.push_str(&format!("{k}={v} "));
                }

                let remote_script = format!("cd {cwd_str} && {env_prefix}{}", parts.join(" "));

                let (mut child, askpass) =
                    ssh::spawn_via_ssh(host, user, config.ssh_key_path.as_deref(), &remote_script)
                        .map_err(|e| format!("ssh spawn failed: {e}"))?;

                let pid = child.id();
                let stdout = child.stdout.take().ok_or("no stdout")?;
                let stderr = child.stderr.take().ok_or("no stderr")?;

                Ok(SpawnHandle {
                    pid,
                    reader: Box::new(stdout),
                    stderr: Box::new(stderr),
                    child,
                    stdin: None,
                    _askpass: askpass,
                })
            }
        }
    }

    fn process_line(&self, state: &mut JournalState, line: &str) {
        crate::journal::process_line_opencode(state, line);
    }

    fn context_window(&self, model: &str) -> Option<u64> {
        // model is "provider/model_id" — split to get provider and model parts
        // e.g. "openrouter/minimax/minimax-m2.5:free" → provider="openrouter",
        //       model="minimax/minimax-m2.5:free"
        let (provider, model_id) = model.split_once('/').unwrap_or(("", model));
        crate::commands::providers::lookup_context_window(provider, model_id)
    }

    fn slash_commands(&self) -> Vec<SlashCommand> {
        crate::commands::plugins::get_opencode_commands()
    }

    fn supports_effort(&self) -> bool {
        false
    }
    fn effort_levels(&self, _model: &str) -> &[&str] {
        &[]
    }
    fn supports_ssh(&self) -> bool {
        false // SSH not yet supported for OpenCode
    }
    fn supports_subagents(&self) -> bool {
        true
    }
    fn subagent_tool_names(&self) -> &[&str] {
        &["Task"]
    }
    fn supports_tasks(&self) -> bool {
        true
    }
    fn task_tool_names(&self) -> &[&str] {
        &["todowrite"]
    }
    fn task_format(&self) -> crate::models::TaskFormat {
        crate::models::TaskFormat::OpenCodeToolUse
    }
    fn line_processor(&self) -> fn(&mut JournalState, &str) {
        crate::journal::process_line_opencode
    }
    fn format_model(&self, raw_model: &str, provider_id: &str) -> String {
        let raw = crate::services::spawn_manager::normalize_opencode_model_ref(raw_model);
        if raw.is_empty() {
            return raw;
        }

        if provider_id == "opencode" {
            if raw
                .split_once('/')
                .is_some_and(|(provider, _)| provider != "opencode")
            {
                return raw;
            }
            if raw.starts_with("opencode/") {
                return raw;
            }

            if let Some(model) =
                crate::commands::providers::resolve_opencode_request(Some(provider_id), Some(&raw))
                    .and_then(|resolved| resolved.model)
            {
                return crate::services::spawn_manager::normalize_opencode_model_ref(&model);
            }
        } else if crate::commands::providers::opencode_subproviders()
            .iter()
            .any(|sp| sp.id.eq_ignore_ascii_case(provider_id))
        {
            return crate::commands::providers::build_opencode_cli_model_arg(provider_id, &raw);
        } else if raw.starts_with(&format!("{provider_id}/")) {
            return raw;
        } else if !raw.contains('/') {
            if let Some(model) =
                crate::commands::providers::resolve_opencode_request(Some(provider_id), Some(&raw))
                    .and_then(|resolved| resolved.model)
            {
                return crate::services::spawn_manager::normalize_opencode_model_ref(&model);
            }
        }

        if provider_id == "opencode"
            && raw.split_once('/').is_some_and(|(provider, _)| {
                crate::commands::providers::opencode_subproviders()
                    .iter()
                    .any(|sub| sub.id == provider)
            })
        {
            raw
        } else {
            format!("{provider_id}/{raw}")
        }
    }
    fn cli_name(&self) -> &str {
        "opencode"
    }
    fn find_cli(&self) -> Option<String> {
        crate::services::spawn_manager::find_opencode()
    }
    fn install_hint(&self) -> &str {
        "npm install -g opencode"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestCase;

    #[test]
    fn should_return_opencode_as_id() {
        let mut t = TestCase::new("should_return_opencode_as_id");
        let provider = OpenCodeProvider;

        t.phase("Assert");
        t.eq("id", provider.id(), "opencode");
    }

    #[test]
    fn should_not_support_effort() {
        let mut t = TestCase::new("should_not_support_effort");
        let provider = OpenCodeProvider;

        t.phase("Assert");
        t.ok("supports_effort is false", !provider.supports_effort());
    }

    #[test]
    fn should_return_opencode_as_display_name() {
        let mut t = TestCase::new("should_return_opencode_as_display_name");
        let provider = OpenCodeProvider;

        t.phase("Assert");
        t.eq("display_name", provider.display_name(), "opencode");
    }

    #[test]
    fn should_parse_opencode_text_line() {
        let mut t = TestCase::new("should_parse_opencode_text_line");

        t.phase("Seed");
        let mut state = JournalState::default();
        let line = r#"{"type":"text","part":{"type":"text","text":"hello"}}"#;

        t.phase("Act");
        let provider = OpenCodeProvider;
        provider.process_line(&mut state, line);

        t.phase("Assert");
        t.len("1 entry produced", &state.entries, 1);
        t.eq(
            "entry type is Assistant",
            state.entries[0].entry_type,
            crate::models::JournalEntryType::Assistant,
        );
    }

    #[test]
    fn should_normalize_trailing_slash_in_format_model() {
        let mut t = TestCase::new("should_normalize_trailing_slash_in_format_model");
        let provider = OpenCodeProvider;
        t.phase("Assert");
        t.eq(
            "no trailing slash",
            provider
                .format_model("kimi-k2.6-precision/", "crof")
                .as_str(),
            "crof/kimi-k2.6-precision",
        );
    }

    #[test]
    fn should_prefix_custom_provider_even_when_model_id_contains_slashes() {
        let mut t =
            TestCase::new("should_prefix_custom_provider_even_when_model_id_contains_slashes");
        let provider = OpenCodeProvider;

        t.phase("Act");
        let formatted = provider.format_model("crof/glm-5.1", "omniroute");

        t.phase("Assert");
        t.eq(
            "custom provider is prefixed to raw model id",
            formatted.as_str(),
            "omniroute/crof/glm-5.1",
        );
    }

    #[test]
    fn should_keep_prefixed_model_when_provider_is_top_level_opencode() {
        let mut t = TestCase::new("should_keep_prefixed_model_when_provider_is_top_level_opencode");
        let provider = OpenCodeProvider;

        t.phase("Act");
        let formatted = provider.format_model("ollama-cloud/kimi-k2.6:cloud", "opencode");

        t.phase("Assert");
        t.eq(
            "full OpenCode provider/model is kept",
            formatted.as_str(),
            "ollama-cloud/kimi-k2.6:cloud",
        );
    }
}

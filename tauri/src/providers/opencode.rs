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
                model: config.model,
                prompt: config.prompt,
                opencode_session_id: config.resume_id,
                extra_env: config.extra_env,
            }),
            SpawnMode::Ssh { ref host, ref user } => {
                let mut parts = vec![
                    "opencode".to_string(),
                    "run".to_string(),
                    "--format".to_string(),
                    "json".to_string(),
                ];

                let cwd_str = config.cwd.to_string_lossy();
                parts.extend([
                    "--dir".to_string(),
                    ssh::posix_escape(&cwd_str),
                    "-m".to_string(),
                    ssh::posix_escape(&config.model),
                ]);

                if let Some(ref sid) = config.resume_id {
                    parts.extend([
                        "--continue".to_string(),
                        "-s".to_string(),
                        ssh::posix_escape(sid),
                    ]);
                }

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
    fn line_processor(&self) -> fn(&mut JournalState, &str) {
        crate::journal::process_line_opencode
    }
    fn format_model(&self, raw_model: &str, provider_id: &str) -> String {
        if raw_model.starts_with(&format!("{provider_id}/")) {
            raw_model.to_string()
        } else {
            format!("{provider_id}/{raw_model}")
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
}

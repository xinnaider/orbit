use super::{Provider, ProviderSpawnConfig};
use crate::journal::JournalState;
use crate::models::SlashCommand;
use crate::services::spawn_manager::{spawn_claude, SpawnConfig, SpawnHandle};
use crate::services::ssh::{self, SpawnMode};

pub struct ClaudeProvider;

impl Provider for ClaudeProvider {
    fn id(&self) -> &str {
        "claude-code"
    }

    fn display_name(&self) -> &str {
        "claude"
    }

    fn spawn(&self, config: ProviderSpawnConfig) -> Result<SpawnHandle, String> {
        match config.spawn_mode {
            SpawnMode::Local => spawn_claude(SpawnConfig {
                session_id: config.session_id,
                cwd: config.cwd,
                permission_mode: if config.skip_permissions {
                    "ignore".to_string()
                } else {
                    "approve".to_string()
                },
                model: if config.model == "auto" {
                    None
                } else {
                    Some(config.model)
                },
                effort: config.effort,
                prompt: config.prompt,
                claude_session_id: config.resume_id,
            }),
            SpawnMode::Ssh { ref host, ref user } => {
                let mut parts = vec![
                    "claude".to_string(),
                    "--output-format".to_string(),
                    "stream-json".to_string(),
                    "--verbose".to_string(),
                ];
                if config.skip_permissions {
                    parts.push("--dangerously-skip-permissions".to_string());
                }
                if config.model != "auto" && !config.model.is_empty() {
                    parts.push("--model".to_string());
                    parts.push(ssh::posix_escape(&config.model));
                }
                if let Some(ref effort) = config.effort {
                    parts.push("--effort".to_string());
                    parts.push(ssh::posix_escape(effort));
                }
                if let Some(ref resume_id) = config.resume_id {
                    parts.push("--resume".to_string());
                    parts.push(ssh::posix_escape(resume_id));
                }
                parts.push("-p".to_string());
                parts.push(ssh::posix_escape(&config.prompt));

                let cwd_str = config.cwd.to_string_lossy();
                let remote_script = format!("cd {} && {}", cwd_str, parts.join(" "));

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
        crate::journal::process_line(state, line);
    }

    fn context_window(&self, model: &str) -> Option<u64> {
        let w = crate::models::context_window(model);
        if w > 0 {
            Some(w)
        } else {
            None
        }
    }

    fn slash_commands(&self) -> Vec<SlashCommand> {
        crate::commands::plugins::get_claude_commands()
    }

    fn supports_effort(&self) -> bool {
        true
    }
    fn supports_ssh(&self) -> bool {
        true
    }
    fn supports_subagents(&self) -> bool {
        true
    }
    fn subagent_tool_names(&self) -> &[&str] {
        &["Agent", "Task"]
    }
    fn line_processor(&self) -> fn(&mut JournalState, &str) {
        crate::journal::process_line
    }
    fn format_model(&self, raw_model: &str, _provider_id: &str) -> String {
        if raw_model.is_empty() || raw_model == "auto" {
            "auto".to_string()
        } else {
            raw_model.to_string()
        }
    }
    fn cli_name(&self) -> &str {
        "claude"
    }
    fn find_cli(&self) -> Option<String> {
        crate::services::spawn_manager::find_claude()
    }
    fn install_hint(&self) -> &str {
        "npm install -g @anthropic-ai/claude-code"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestCase;

    #[test]
    fn should_return_claude_code_as_id() {
        let mut t = TestCase::new("should_return_claude_code_as_id");
        let provider = ClaudeProvider;

        t.phase("Assert");
        t.eq("id", provider.id(), "claude-code");
    }

    #[test]
    fn should_support_effort() {
        let mut t = TestCase::new("should_support_effort");
        let provider = ClaudeProvider;

        t.phase("Assert");
        t.ok("supports_effort is true", provider.supports_effort());
    }

    #[test]
    fn should_return_known_context_window() {
        let mut t = TestCase::new("should_return_known_context_window");
        let provider = ClaudeProvider;

        t.phase("Assert");
        let window = provider.context_window("claude-sonnet-4-6");
        t.some("context_window for sonnet-4-6 is Some", &window);
        t.eq("context_window value", window.unwrap(), 200_000);
    }

    #[test]
    fn should_parse_claude_assistant_line() {
        let mut t = TestCase::new("should_parse_claude_assistant_line");

        t.phase("Seed");
        let mut state = JournalState::default();
        let line = crate::test_utils::assistant_text("hello");

        t.phase("Act");
        provider_process_line(&mut state, &line);

        t.phase("Assert");
        t.len("1 entry produced", &state.entries, 1);
        t.eq(
            "entry type is Assistant",
            state.entries[0].entry_type,
            crate::models::JournalEntryType::Assistant,
        );
    }

    /// Helper to call process_line via the provider trait.
    fn provider_process_line(state: &mut JournalState, line: &str) {
        let provider = ClaudeProvider;
        provider.process_line(state, line);
    }
}

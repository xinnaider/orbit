use super::{Provider, ProviderSpawnConfig};
use crate::journal::JournalState;
use crate::models::SlashCommand;
use crate::services::spawn_manager::{spawn_codex, CodexConfig, SpawnHandle};
use crate::services::ssh::{self, SpawnMode};

pub struct CodexProvider;

const CODEX_EFFORT_LEVELS: &[&str] = &["low", "medium", "high", "xhigh"];

/// argv tokens for `codex` over SSH (before shell escaping).
pub(crate) fn codex_ssh_command_tokens(
    resume_id: Option<&str>,
    skip_permissions: bool,
    model: &str,
    effort: Option<&str>,
) -> Vec<String> {
    let mut parts = vec!["codex".to_string()];
    if let Some(sid) = resume_id {
        parts.extend([
            "exec".to_string(),
            "resume".to_string(),
            "--json".to_string(),
        ]);
        if skip_permissions {
            parts.push("--dangerously-bypass-approvals-and-sandbox".to_string());
        }
        if let Some(effort) = effort {
            parts.push("--config".to_string());
            parts.push(format!("model_reasoning_effort=\"{effort}\""));
        }
        if model != "auto" && !model.is_empty() {
            parts.extend(["-m".to_string(), model.to_string()]);
        }
        parts.push(sid.to_string());
    } else {
        parts.extend(["exec".to_string(), "--json".to_string()]);
        if skip_permissions {
            parts.push("--dangerously-bypass-approvals-and-sandbox".to_string());
        }
        if let Some(effort) = effort {
            parts.push("--config".to_string());
            parts.push(format!("model_reasoning_effort=\"{effort}\""));
        }
        if model != "auto" && !model.is_empty() {
            parts.extend(["-m".to_string(), model.to_string()]);
        }
    }
    parts
}

impl Provider for CodexProvider {
    fn id(&self) -> &str {
        "codex"
    }

    fn display_name(&self) -> &str {
        "codex"
    }

    fn spawn(&self, config: ProviderSpawnConfig) -> Result<SpawnHandle, String> {
        match config.spawn_mode {
            SpawnMode::Local => spawn_codex(CodexConfig {
                session_id: config.session_id,
                cwd: config.cwd,
                model: config.model,
                effort: config.effort,
                prompt: config.prompt,
                codex_session_id: config.resume_id,
                skip_permissions: config.skip_permissions,
            }),
            SpawnMode::Ssh { ref host, ref user } => {
                let mut parts: Vec<String> = codex_ssh_command_tokens(
                    config.resume_id.as_deref(),
                    config.skip_permissions,
                    &config.model,
                    config.effort.as_deref(),
                )
                .into_iter()
                .map(|token| match token.as_str() {
                    "codex"
                    | "exec"
                    | "resume"
                    | "--json"
                    | "--dangerously-bypass-approvals-and-sandbox"
                    | "--config"
                    | "-m" => token,
                    _ => ssh::posix_escape(&token),
                })
                .collect();
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
        crate::journal::process_line_codex(state, line);
    }

    fn context_window(&self, model: &str) -> Option<u64> {
        let _ = model;
        None
    }

    fn slash_commands(&self) -> Vec<SlashCommand> {
        crate::commands::plugins::get_codex_commands()
    }

    fn supports_effort(&self) -> bool {
        true
    }
    fn effort_levels(&self, _model: &str) -> &[&str] {
        CODEX_EFFORT_LEVELS
    }
    fn supports_ssh(&self) -> bool {
        true
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
        &["todo_list"]
    }
    fn task_format(&self) -> crate::models::TaskFormat {
        crate::models::TaskFormat::CodexItemList
    }
    fn line_processor(&self) -> fn(&mut JournalState, &str) {
        crate::journal::process_line_codex
    }
    fn format_model(&self, raw_model: &str, _provider_id: &str) -> String {
        if raw_model.is_empty() || raw_model == "auto" {
            "auto".to_string()
        } else {
            raw_model.to_string()
        }
    }
    fn cli_name(&self) -> &str {
        "codex"
    }
    fn find_cli(&self) -> Option<String> {
        crate::services::spawn_manager::find_codex()
    }
    fn install_hint(&self) -> &str {
        "npm install -g @openai/codex"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::TestCase;

    #[test]
    fn should_return_codex_as_id() {
        let mut t = TestCase::new("should_return_codex_as_id");
        let provider = CodexProvider;

        t.phase("Assert");
        t.eq("id", provider.id(), "codex");
    }

    #[test]
    fn should_support_effort() {
        let mut t = TestCase::new("should_support_effort");
        let provider = CodexProvider;

        t.phase("Assert");
        t.ok("supports_effort is true", provider.supports_effort());
        t.eq(
            "xhigh is exposed for codex",
            provider.effort_levels("gpt-5.5")[3],
            "xhigh",
        );
    }

    #[test]
    fn should_normalize_empty_model_to_auto() {
        let mut t = TestCase::new("should_normalize_empty_model_to_auto");
        let provider = CodexProvider;

        t.phase("Act");
        let formatted = provider.format_model("", "codex");

        t.phase("Assert");
        t.eq("empty model becomes auto", formatted.as_str(), "auto");
    }

    #[test]
    fn should_not_report_runtime_context_window_for_gpt54() {
        let mut t = TestCase::new("should_not_report_runtime_context_window_for_gpt54");
        let provider = CodexProvider;

        t.phase("Assert");
        let window = provider.context_window("gpt-5.4");
        t.none("runtime context window is unknown for codex", &window);
    }

    #[test]
    fn codex_ssh_command_tokens_should_resume_with_model_and_effort() {
        let mut t = TestCase::new("codex_ssh_command_tokens_should_resume_with_model_and_effort");
        t.phase("Act");
        let parts = codex_ssh_command_tokens(Some("thread-9"), true, "gpt-5.4", Some("xhigh"));
        t.phase("Assert");
        t.ok(
            "resume branch",
            parts.windows(3).any(|w| w == ["exec", "resume", "--json"]),
        );
        t.ok(
            "model flag",
            parts.windows(2).any(|w| w == ["-m", "gpt-5.4"]),
        );
        t.ok(
            "effort config",
            parts
                .iter()
                .any(|p| p.contains("model_reasoning_effort=\"xhigh\"")),
        );
        t.ok("thread id", parts.contains(&"thread-9".to_string()));
    }

    #[test]
    fn should_parse_codex_agent_message() {
        let mut t = TestCase::new("should_parse_codex_agent_message");

        t.phase("Seed");
        let mut state = JournalState::default();
        let line = r#"{"type":"item.completed","item":{"type":"agent_message","text":"hello"}}"#;

        t.phase("Act");
        let provider = CodexProvider;
        provider.process_line(&mut state, line);

        t.phase("Assert");
        t.len("1 entry produced", &state.entries, 1);
        t.eq(
            "entry type is Assistant",
            state.entries[0].entry_type,
            crate::models::JournalEntryType::Assistant,
        );
        t.eq(
            "entry text is hello",
            state.entries[0].text.as_deref().unwrap_or(""),
            "hello",
        );
    }
}

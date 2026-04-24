use super::{Provider, ProviderSpawnConfig};
use crate::journal::JournalState;
use crate::models::SlashCommand;
use crate::services::spawn_manager::{spawn_codex, CodexConfig, SpawnHandle};
use crate::services::ssh::{self, SpawnMode};

pub struct CodexProvider;

const CODEX_EFFORT_LEVELS: &[&str] = &["none", "minimal", "low", "medium", "high", "xhigh"];

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
                let mut parts = vec!["codex".to_string()];
                if let Some(ref sid) = config.resume_id {
                    parts.extend([
                        "exec".to_string(),
                        "resume".to_string(),
                        "--json".to_string(),
                    ]);
                    if config.skip_permissions {
                        parts.push("--dangerously-bypass-approvals-and-sandbox".to_string());
                    }
                    if let Some(ref effort) = config.effort {
                        parts.push("--config".to_string());
                        parts.push(ssh::posix_escape(&format!(
                            "model_reasoning_effort=\"{effort}\""
                        )));
                    }
                    parts.extend([
                        "-m".to_string(),
                        ssh::posix_escape(&config.model),
                        ssh::posix_escape(sid),
                        ssh::posix_escape(&config.prompt),
                    ]);
                } else {
                    parts.extend(["exec".to_string(), "--json".to_string()]);
                    if config.skip_permissions {
                        parts.push("--dangerously-bypass-approvals-and-sandbox".to_string());
                    }
                    if let Some(ref effort) = config.effort {
                        parts.push("--config".to_string());
                        parts.push(ssh::posix_escape(&format!(
                            "model_reasoning_effort=\"{effort}\""
                        )));
                    }
                    parts.extend([
                        "-m".to_string(),
                        ssh::posix_escape(&config.model),
                        ssh::posix_escape(&config.prompt),
                    ]);
                }

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
        raw_model.to_string()
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
            provider.effort_levels("gpt-5.5")[5],
            "xhigh",
        );
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

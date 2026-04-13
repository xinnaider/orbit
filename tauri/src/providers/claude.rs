use super::{Provider, ProviderSpawnConfig};
use crate::journal::JournalState;
use crate::models::SlashCommand;
use crate::services::spawn_manager::{spawn_claude, SpawnConfig, SpawnHandle};

pub struct ClaudeProvider;

impl Provider for ClaudeProvider {
    fn id(&self) -> &str {
        "claude-code"
    }

    fn display_name(&self) -> &str {
        "claude"
    }

    fn spawn(&self, config: ProviderSpawnConfig) -> Result<SpawnHandle, String> {
        spawn_claude(SpawnConfig {
            session_id: config.session_id,
            cwd: config.cwd,
            permission_mode: "ignore".to_string(),
            model: if config.model == "auto" {
                None
            } else {
                Some(config.model)
            },
            effort: None,
            prompt: config.prompt,
            claude_session_id: config.resume_id,
        })
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

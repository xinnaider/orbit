pub mod acp;
pub mod claude;
pub mod codex;
pub mod opencode;

use std::collections::HashMap;

/// Configuration passed to a provider when spawning a CLI process.
pub struct ProviderSpawnConfig {
    pub session_id: crate::models::SessionId,
    pub cwd: std::path::PathBuf,
    pub model: String,
    pub prompt: String,
    pub resume_id: Option<String>,
    pub extra_env: Vec<(String, String)>,
    /// Effort level for thinking (low, medium, high, max). Only used by Claude Code.
    pub effort: Option<String>,
    /// How to spawn: locally or via SSH tunnel.
    pub spawn_mode: crate::services::ssh::SpawnMode,
    /// SSH private key file path (memory only, never persisted directly).
    pub ssh_key_path: Option<String>,
    /// Whether to skip permission prompts (auto-approve all tool calls).
    pub skip_permissions: bool,
}

/// Trait that every CLI backend (Claude Code, Codex, OpenCode, etc.) must implement.
///
/// Each provider knows how to spawn its CLI, parse its output, and report its
/// capabilities (context window, slash commands, effort support).
pub trait Provider: Send + Sync {
    /// Unique identifier for this provider (e.g. "claude", "codex", "opencode").
    fn id(&self) -> &str;

    /// Human-readable name shown in the UI.
    fn display_name(&self) -> &str;

    /// Spawn the CLI process and return a handle to its I/O streams.
    fn spawn(
        &self,
        config: ProviderSpawnConfig,
    ) -> Result<crate::services::spawn_manager::SpawnHandle, String>;

    /// Parse a single JSONL line from the CLI output and update journal state.
    fn process_line(&self, state: &mut crate::journal::JournalState, line: &str);

    /// Return the context window size for a given model ID, or `None` if unknown.
    fn context_window(&self, model: &str) -> Option<u64>;

    /// Return the list of slash commands this provider supports.
    fn slash_commands(&self) -> Vec<crate::models::SlashCommand>;

    /// Whether this provider supports the `effort` parameter.
    fn supports_effort(&self) -> bool;

    /// Effort levels supported for a specific model, or empty slice if effort is not supported.
    /// For Claude Code, Opus 4.7 supports an extended list (low, medium, high, xhigh, max, auto).
    fn effort_levels(&self, model: &str) -> &[&str];

    /// Whether this provider supports SSH remote sessions.
    fn supports_ssh(&self) -> bool;

    /// Whether this CLI can spawn sub-agents (shown in the agents tab).
    fn supports_subagents(&self) -> bool;

    /// Tool names that represent a sub-agent spawn.
    /// When the journal sees a `ToolCall` with one of these names,
    /// a `session:subagent-created` event is emitted.
    fn subagent_tool_names(&self) -> &[&str];

    /// Return a fn pointer for parsing JSONL lines in a Send thread.
    /// Needed because `&dyn Provider` is not Send across thread boundaries.
    fn line_processor(&self) -> fn(&mut crate::journal::JournalState, &str);

    /// Format a raw model string for this provider's CLI.
    /// E.g. Claude returns "auto" as-is, OpenCode prefixes with "provider/model".
    fn format_model(&self, raw_model: &str, provider_id: &str) -> String;

    /// CLI binary name for diagnostics (e.g. "claude", "codex", "opencode").
    fn cli_name(&self) -> &str;

    /// Find the CLI binary path, or None if not installed.
    fn find_cli(&self) -> Option<String>;

    /// Install hint shown when CLI is not found.
    fn install_hint(&self) -> &str;
}

/// Registry that maps provider IDs to their implementations.
///
/// When resolving an unknown provider ID, falls back to the "opencode" provider
/// (since opencode sub-providers like "openrouter" are handled by the same CLI).
pub struct ProviderRegistry {
    providers: HashMap<String, Box<dyn Provider>>,
}

impl Default for ProviderRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ProviderRegistry {
    pub fn new() -> Self {
        Self {
            providers: HashMap::new(),
        }
    }

    /// Register a provider. Replaces any existing provider with the same ID.
    pub fn register(&mut self, provider: Box<dyn Provider>) {
        let id = provider.id().to_string();
        self.providers.insert(id, provider);
    }

    /// Look up a provider by exact ID. Returns `None` if not found.
    pub fn get(&self, id: &str) -> Option<&dyn Provider> {
        self.providers.get(id).map(|p| p.as_ref())
    }

    /// Return all registered providers (unordered).
    pub fn all(&self) -> Vec<&dyn Provider> {
        self.providers.values().map(|p| p.as_ref()).collect()
    }

    /// Resolve a provider by ID, falling back to "opencode" for unknown IDs.
    ///
    /// This handles the case where opencode sub-providers (e.g. "openrouter")
    /// don't have their own registry entries — they all route through opencode.
    pub fn resolve(&self, id: &str) -> Option<&dyn Provider> {
        self.get(id).or_else(|| self.get("opencode"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Mock Provider ───────────────────────────────────────────────────────

    struct MockProvider {
        mock_id: String,
        mock_name: String,
        mock_effort: bool,
    }

    impl MockProvider {
        fn new(id: &str, name: &str, effort: bool) -> Self {
            Self {
                mock_id: id.to_string(),
                mock_name: name.to_string(),
                mock_effort: effort,
            }
        }
    }

    impl Provider for MockProvider {
        fn id(&self) -> &str {
            &self.mock_id
        }

        fn display_name(&self) -> &str {
            &self.mock_name
        }

        fn spawn(
            &self,
            _config: ProviderSpawnConfig,
        ) -> Result<crate::services::spawn_manager::SpawnHandle, String> {
            Err("MockProvider does not spawn".to_string())
        }

        fn process_line(&self, _state: &mut crate::journal::JournalState, _line: &str) {
            // no-op
        }

        fn context_window(&self, _model: &str) -> Option<u64> {
            Some(100_000)
        }

        fn slash_commands(&self) -> Vec<crate::models::SlashCommand> {
            vec![]
        }

        fn supports_effort(&self) -> bool {
            self.mock_effort
        }
        fn effort_levels(&self, _model: &str) -> &[&str] {
            if self.mock_effort {
                &["low", "medium", "high", "max"]
            } else {
                &[]
            }
        }
        fn supports_ssh(&self) -> bool {
            false
        }
        fn supports_subagents(&self) -> bool {
            false
        }
        fn subagent_tool_names(&self) -> &[&str] {
            &[]
        }
        fn line_processor(&self) -> fn(&mut crate::journal::JournalState, &str) {
            |_, _| {}
        }
        fn format_model(&self, raw: &str, _pid: &str) -> String {
            raw.to_string()
        }
        fn cli_name(&self) -> &str {
            "mock"
        }
        fn find_cli(&self) -> Option<String> {
            None
        }
        fn install_hint(&self) -> &str {
            "install mock"
        }
    }

    // ─── Tests ───────────────────────────────────────────────────────────────

    #[test]
    fn should_register_and_retrieve_provider() {
        let mut t = crate::test_utils::TestCase::new("should_register_and_retrieve_provider");

        t.phase("Seed");
        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(MockProvider::new("claude", "Claude Code", false)));

        t.phase("Assert — found by exact ID");
        let found = registry.get("claude");
        t.some("get('claude') returns Some", &found);
        t.eq("provider id", found.unwrap().id(), "claude");
        t.eq(
            "provider display_name",
            found.unwrap().display_name(),
            "Claude Code",
        );

        t.phase("Assert — unknown ID returns None");
        t.ok(
            "get('nonexistent') returns None",
            registry.get("nonexistent").is_none(),
        );
    }

    #[test]
    fn should_resolve_unknown_id_to_opencode_fallback() {
        let mut t =
            crate::test_utils::TestCase::new("should_resolve_unknown_id_to_opencode_fallback");

        t.phase("Seed");
        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(MockProvider::new("opencode", "OpenCode", false)));

        t.phase("Assert — resolve known ID");
        let resolved = registry.resolve("opencode");
        t.some("resolve('opencode') returns Some", &resolved);
        t.eq("resolved id", resolved.unwrap().id(), "opencode");

        t.phase("Assert — resolve unknown ID falls back to opencode");
        let fallback = registry.resolve("openrouter");
        t.some("resolve('openrouter') returns Some", &fallback);
        t.eq(
            "fallback id is opencode",
            fallback.unwrap().id(),
            "opencode",
        );
    }

    #[test]
    fn should_report_effort_support_correctly() {
        let mut t = crate::test_utils::TestCase::new("should_report_effort_support_correctly");

        t.phase("Seed");
        let mut registry = ProviderRegistry::new();
        registry.register(Box::new(MockProvider::new("claude", "Claude Code", true)));
        registry.register(Box::new(MockProvider::new("codex", "Codex CLI", false)));

        t.phase("Assert");
        let claude = registry.get("claude").unwrap();
        t.ok("claude supports effort", claude.supports_effort());

        let codex = registry.get("codex").unwrap();
        t.ok("codex does not support effort", !codex.supports_effort());
    }
}

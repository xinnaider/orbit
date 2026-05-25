use crate::agent_tree;
use crate::ipc::session::SessionState;
use crate::models::{SessionId, SubagentInfo};
use tauri::State;

#[tauri::command]
pub fn get_subagents(session_id: SessionId, state: State<SessionState>) -> Vec<SubagentInfo> {
    let m = state.read();
    let mut subagents = m
        .db
        .get_claude_session_id(session_id)
        .ok()
        .flatten()
        .map(|id| agent_tree::read_subagents(&id))
        .unwrap_or_default();
    for mcp in m.get_mcp_subagents(session_id) {
        if !subagents.iter().any(|s| s.id == mcp.id) {
            subagents.push(mcp);
        }
    }
    subagents
}

#[cfg(test)]
mod tests {
    use crate::test_utils::TestCase;

    #[test]
    fn should_return_empty_vec_for_unknown_claude_session_id() {
        let mut t = TestCase::new("should_return_empty_vec_for_unknown_claude_session_id");
        t.phase("Act");
        let result = crate::agent_tree::read_subagents("nonexistent-session-00000000-test");
        t.phase("Assert");
        t.empty("no subagents returned", &result);
    }

    #[test]
    fn should_register_and_list_mcp_subagents_in_memory() {
        use std::sync::Arc;

        use crate::services::database::DatabaseService;
        use crate::services::session_manager::SessionManager;
        use crate::test_utils::seed_session;

        let mut t = TestCase::new("should_register_and_list_mcp_subagents_in_memory");
        let db = Arc::new(DatabaseService::open_in_memory().expect("in-memory db"));
        let mut manager = SessionManager::new(db);
        let parent_id = seed_session(&manager.db);

        t.phase("Act");
        manager.register_mcp_subagent(parent_id, 42, "review auth", "claude-code");
        let listed = manager.get_mcp_subagents(parent_id);

        t.phase("Assert");
        t.eq("one mcp subagent", &listed.len(), &1);
        t.eq("mcp child id", &listed[0].id, &"42".to_string());
        t.eq(
            "mcp agent type",
            &listed[0].agent_type,
            &"mcp:claude-code".to_string(),
        );
        t.eq(
            "mcp description",
            &listed[0].description,
            &"review auth".to_string(),
        );
    }
}

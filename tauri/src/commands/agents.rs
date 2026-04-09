use crate::agent_tree;
use crate::ipc::session::SessionState;
use crate::models::{SessionId, SubagentInfo};
use tauri::State;

#[tauri::command]
pub fn get_subagents(session_id: SessionId, state: State<SessionState>) -> Vec<SubagentInfo> {
    let claude_id = {
        let m = state.read();
        m.db.get_claude_session_id(session_id).ok().flatten()
    };
    match claude_id {
        Some(id) => agent_tree::read_subagents(&id),
        None => vec![],
    }
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
}

use crate::models::*;

/// Extract tasks by scanning session_outputs for the last `TodoWrite` tool call.
/// Claude Code emits tool_use blocks with name="TodoWrite" and input.todos=[...].
#[tauri::command]
pub fn get_tasks(
    session_id: String,
    state: tauri::State<crate::ipc::session::SessionState>,
) -> Vec<TaskItem> {
    let id: i64 = match session_id.parse() {
        Ok(v) => v,
        Err(_) => return vec![],
    };

    let outputs = {
        let m = state.0.lock().unwrap();
        match m.db.get_outputs(id) {
            Ok(o) => o,
            Err(_) => return vec![],
        }
    };

    // Find the last TodoWrite call — its todos list represents the current state.
    let mut last_todos: Option<Vec<TaskItem>> = None;

    for raw in &outputs {
        let val: serde_json::Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        if val.get("type").and_then(|t| t.as_str()) != Some("assistant") {
            continue;
        }

        let content = match val
            .get("message")
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_array())
        {
            Some(c) => c,
            None => continue,
        };

        for block in content {
            if block.get("type").and_then(|t| t.as_str()) != Some("tool_use") {
                continue;
            }
            if block.get("name").and_then(|n| n.as_str()) != Some("TodoWrite") {
                continue;
            }

            let todos_val = match block.get("input").and_then(|i| i.get("todos")) {
                Some(t) => t,
                None => continue,
            };

            let todos: Vec<TaskItem> = todos_val
                .as_array()
                .unwrap_or(&vec![])
                .iter()
                .enumerate()
                .filter_map(|(idx, t)| {
                    let id_str = t
                        .get("id")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| (idx + 1).to_string());
                    let subject = t
                        .get("content")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let status = t
                        .get("status")
                        .and_then(|v| v.as_str())
                        .unwrap_or("pending")
                        .to_string();
                    if status == "deleted" || subject.is_empty() {
                        return None;
                    }
                    let active_form = t
                        .get("activeForm")
                        .or_else(|| t.get("active_form"))
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string());
                    Some(TaskItem {
                        id: id_str,
                        subject,
                        description: String::new(),
                        active_form,
                        status,
                        blocks: vec![],
                        blocked_by: vec![],
                    })
                })
                .collect();

            last_todos = Some(todos);
        }
    }

    last_todos.unwrap_or_default()
}

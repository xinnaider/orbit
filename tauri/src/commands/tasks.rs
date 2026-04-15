use crate::models::*;

/// Extract tasks by scanning session_outputs for todo/task events.
/// Supports 3 formats:
///   Claude:  type=assistant → content[].type=tool_use, name=TodoWrite
///   OpenCode: type=tool_use → part.tool=todowrite → state.input.todos[]
///   Codex:   type=item.completed → item.type=todo_list → item.items[]
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
        let m = state.read();
        match m.db.get_outputs(id) {
            Ok(o) => o,
            Err(_) => return vec![],
        }
    };

    let mut last_todos: Option<Vec<TaskItem>> = None;

    for raw in &outputs {
        let val: serde_json::Value = match serde_json::from_str(raw) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let event_type = val.get("type").and_then(|t| t.as_str()).unwrap_or("");

        match event_type {
            // Claude: assistant message with TodoWrite tool_use
            "assistant" => {
                if let Some(todos) = parse_claude_todos(&val) {
                    last_todos = Some(todos);
                }
            }
            // OpenCode: tool_use with tool=todowrite
            "tool_use" => {
                if let Some(todos) = parse_opencode_todos(&val) {
                    last_todos = Some(todos);
                }
            }
            // Codex: item.completed with item.type=todo_list
            "item.completed" | "item.started" => {
                if let Some(todos) = parse_codex_todos(&val) {
                    last_todos = Some(todos);
                }
            }
            _ => {}
        }
    }

    last_todos.unwrap_or_default()
}

/// Parse todos from Claude's TodoWrite tool_use format.
fn parse_claude_todos(val: &serde_json::Value) -> Option<Vec<TaskItem>> {
    let content = val.get("message")?.get("content")?.as_array()?;

    for block in content {
        if block.get("type").and_then(|t| t.as_str()) != Some("tool_use") {
            continue;
        }
        if block.get("name").and_then(|n| n.as_str()) != Some("TodoWrite") {
            continue;
        }
        let todos_val = block.get("input")?.get("todos")?;
        return Some(parse_todo_array(todos_val));
    }
    None
}

/// Parse todos from OpenCode's tool_use todowrite format.
fn parse_opencode_todos(val: &serde_json::Value) -> Option<Vec<TaskItem>> {
    let tool = val.pointer("/part/tool").and_then(|v| v.as_str())?;
    if tool != "todowrite" {
        return None;
    }
    // Try state.input.todos first, then state.metadata.todos
    let todos_val = val
        .pointer("/part/state/input/todos")
        .or_else(|| val.pointer("/part/state/metadata/todos"))?;
    Some(parse_todo_array(todos_val))
}

/// Parse todos from Codex's item.type=todo_list format.
fn parse_codex_todos(val: &serde_json::Value) -> Option<Vec<TaskItem>> {
    let item_type = val.pointer("/item/type").and_then(|v| v.as_str())?;
    if item_type != "todo_list" {
        return None;
    }
    let items = val.pointer("/item/items")?.as_array()?;
    let todos: Vec<TaskItem> = items
        .iter()
        .enumerate()
        .filter_map(|(idx, t)| {
            let subject = t
                .get("text")
                .or_else(|| t.get("content"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            if subject.is_empty() {
                return None;
            }
            let completed = t
                .get("completed")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let status = if completed {
                "completed".to_string()
            } else {
                t.get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("pending")
                    .to_string()
            };
            Some(TaskItem {
                id: (idx + 1).to_string(),
                subject,
                description: String::new(),
                active_form: None,
                status,
                blocks: vec![],
                blocked_by: vec![],
            })
        })
        .collect();
    if todos.is_empty() {
        None
    } else {
        Some(todos)
    }
}

/// Shared parser for todo arrays (Claude and OpenCode use the same structure).
fn parse_todo_array(todos_val: &serde_json::Value) -> Vec<TaskItem> {
    todos_val
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
        .collect()
}

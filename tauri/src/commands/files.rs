use crate::ipc::session::SessionState;
use crate::journal;
use crate::models::*;
use tauri::State;

#[tauri::command]
pub fn get_subagent_journal(
    session_id: SessionId,
    subagent_id: String,
    state: State<SessionState>,
) -> Vec<JournalEntry> {
    // Resolve the numeric session ID to the Claude session UUID,
    // which is used as the directory name under ~/.claude/projects/<project>/
    let claude_id = {
        let m = state.read();
        m.db.get_claude_session_id(session_id).ok().flatten()
    };
    let claude_session_id = match claude_id {
        Some(id) => id,
        None => return vec![],
    };

    let projects_dir = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("projects"),
        None => return vec![],
    };

    let entries = match std::fs::read_dir(&projects_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    for project_entry in entries.flatten() {
        let jsonl_path = project_entry
            .path()
            .join(&claude_session_id)
            .join("subagents")
            .join(format!("{}.jsonl", &subagent_id));

        if jsonl_path.exists() {
            let journal_state = journal::parse_journal(&jsonl_path, 0, None);
            let mut result = journal_state.entries;
            for entry in &mut result {
                entry.session_id = subagent_id.clone();
            }
            return result;
        }
    }

    vec![]
}

#[tauri::command]
pub fn list_project_files(cwd: String) -> Vec<String> {
    use ignore::WalkBuilder;

    let mut files = Vec::new();
    let walker = WalkBuilder::new(&cwd)
        .hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .max_depth(Some(12))
        .build();

    for entry in walker.flatten() {
        if !entry.file_type().is_some_and(|ft| ft.is_file()) {
            continue;
        }
        if let Ok(rel) = entry.path().strip_prefix(&cwd) {
            let rel_str = rel.to_string_lossy().replace('\\', "/");
            if !rel_str.is_empty() {
                files.push(rel_str.to_string());
                if files.len() >= 5000 {
                    break;
                }
            }
        }
    }

    files.sort();
    files
}

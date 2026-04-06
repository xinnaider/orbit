use std::fs;
use std::path::Path;

use crate::models::SubagentInfo;
use serde::Deserialize;

#[derive(Deserialize)]
struct AgentMeta {
    #[serde(rename = "agentType", default)]
    agent_type: String,
    #[serde(default)]
    description: String,
}

/// Read all .meta.json files from a session's subagents directory.
pub fn read_subagents(session_id: &str) -> Vec<SubagentInfo> {
    let projects_dir = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("projects"),
        None => return vec![],
    };

    let entries = match fs::read_dir(&projects_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    for project_entry in entries.flatten() {
        let subagents_dir = project_entry.path().join(session_id).join("subagents");
        if !subagents_dir.is_dir() {
            continue;
        }

        return read_meta_files(&subagents_dir);
    }

    vec![]
}

/// Read the JSONL log for a specific subagent, returning raw lines as strings.
pub fn read_subagent_log(session_id: &str, subagent_id: &str) -> Vec<String> {
    let projects_dir = match dirs::home_dir() {
        Some(h) => h.join(".claude").join("projects"),
        None => return vec![],
    };

    let entries = match fs::read_dir(&projects_dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    for project_entry in entries.flatten() {
        let jsonl_path = project_entry
            .path()
            .join(session_id)
            .join("subagents")
            .join(format!("{}.jsonl", subagent_id));

        if jsonl_path.exists() {
            if let Ok(content) = fs::read_to_string(&jsonl_path) {
                return content
                    .lines()
                    .filter(|l| !l.trim().is_empty())
                    .map(|l| l.to_string())
                    .collect();
            }
        }
    }

    vec![]
}

fn read_meta_files(dir: &Path) -> Vec<SubagentInfo> {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    let mut agents = Vec::new();

    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        if !name.ends_with(".meta.json") {
            continue;
        }

        let content = match fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if let Ok(meta) = serde_json::from_str::<AgentMeta>(&content) {
            let id = name.replace(".meta.json", "");
            // Check if corresponding JSONL exists and has data (indicates completion)
            let jsonl_name = format!("{}.jsonl", &id);
            let jsonl_path = dir.join(&jsonl_name);
            let status = if jsonl_path.exists() {
                let size = jsonl_path.metadata().map(|m| m.len()).unwrap_or(0);
                if size > 0 {
                    "done".to_string()
                } else {
                    "running".to_string()
                }
            } else {
                "running".to_string()
            };

            agents.push(SubagentInfo {
                id,
                agent_type: meta.agent_type,
                description: meta.description,
                status,
            });
        }
    }

    agents
}

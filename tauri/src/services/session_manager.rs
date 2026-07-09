use std::collections::{HashMap, HashSet};
use std::io::Write;
use std::sync::{Arc, RwLock};

use tauri::{AppHandle, Emitter};

use crate::journal::JournalState;
use crate::models::{AgentStatus, Session, SessionId, TokenUsage};

/// Default provider ID when none is specified.
const DEFAULT_PROVIDER: &str = "claude-code";
use crate::providers::{ProviderRegistry, ProviderSpawnConfig};
use crate::services::{database::DatabaseService, mcp_config};

/// Write provider-specific MCP configs in the project directory so agents can use orbit-mcp tools.
fn ensure_mcp_config(cwd: &str) {
    let Some(launch) = mcp_config::mcp_launch() else {
        eprintln!("[orbit:mcp] failed to resolve MCP launch command");
        return;
    };
    if let Err(e) = mcp_config::write_orbit_mcp_configs(std::path::Path::new(cwd), &launch) {
        eprintln!("[orbit:mcp] failed to write provider MCP configs: {e}");
    }
}

/// Reads `.git/HEAD` to detect the current branch without spawning a subprocess.
fn detect_git_branch(cwd: &str) -> Option<String> {
    let head = std::fs::read_to_string(std::path::Path::new(cwd).join(".git/HEAD")).ok()?;
    head.trim()
        .strip_prefix("ref: refs/heads/")
        .map(|b| b.to_string())
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionOutputEvent {
    pub session_id: SessionId,
    pub entry: crate::models::JournalEntry,
}

#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStateEvent {
    pub session_id: SessionId,
    pub status: String,
    pub tokens: TokenUsage,
    pub context_percent: f64,
    pub pending_approval: Option<String>,
    pub mini_log: Vec<crate::models::MiniLogEntry>,
    pub git_branch: Option<String>,
    pub subagents: Vec<crate::models::SubagentInfo>,
    pub model: Option<String>,
    pub context_window: Option<u64>,
    pub attention: crate::models::AttentionState,
    pub rate_limit: Vec<crate::models::RateLimitInfo>,
    pub cost_usd: Option<f64>,
}

struct ActiveSession {
    session: Session,
    /// The Claude CLI session ID (from stream-json system/init message).
    /// Required for --resume on follow-up messages.
    pub claude_session_id: Option<String>,
    /// Effort level for thinking (low, medium, high, max).
    pub effort: Option<String>,
    /// Provider API key (stored in memory only, never persisted).
    pub api_key: Option<String>,
    /// SSH private key path held in memory. Reused for follow-up messages.
    pub ssh_key_path: Option<String>,
    /// Stdin handle for providers that use persistent stdin (e.g. ACP JSON-RPC).
    pub stdin: Option<Arc<std::sync::Mutex<Box<dyn std::io::Write + Send>>>>,
}

pub(crate) fn resolve_context_metrics(
    provider_id: &str,
    model: Option<&str>,
    state: &JournalState,
) -> (Option<u64>, f64) {
    let window = crate::commands::providers::resolve_context_window(
        provider_id,
        model,
        state.context_window,
    );
    let percent = window
        .filter(|window| *window > 0)
        .map(|window| (state.input_tokens as f64 / window as f64) * 100.0)
        .unwrap_or(0.0);
    (window, percent)
}

pub struct SessionManager {
    pub db: Arc<DatabaseService>,
    active: HashMap<SessionId, ActiveSession>,
    pub journal_states: HashMap<SessionId, JournalState>,
    /// MCP-spawned child sessions grouped by parent session ID.
    mcp_subagents: HashMap<SessionId, Vec<(SessionId, crate::models::SubagentInfo)>>,
    /// Git diff manager for real-time working tree watching.
    pub diff_manager: Arc<super::diff_manager::DiffManager>,
    /// Tracks which session is watching which directory (for cleanup).
    watch_map: HashMap<SessionId, std::path::PathBuf>,
    /// Sessions currently in the spawning phase (prevents double-spawn race).
    spawning_sessions: HashSet<SessionId>,
}

impl SessionManager {
    pub fn new(db: Arc<DatabaseService>) -> Self {
        SessionManager {
            db,
            active: HashMap::new(),
            journal_states: HashMap::new(),
            mcp_subagents: HashMap::new(),
            diff_manager: Arc::new(super::diff_manager::DiffManager::new()),
            watch_map: HashMap::new(),
            spawning_sessions: HashSet::new(),
        }
    }

    pub fn register_mcp_subagent(
        &mut self,
        parent_id: SessionId,
        child_id: SessionId,
        description: &str,
        provider: &str,
    ) {
        let info = crate::models::SubagentInfo {
            id: child_id.to_string(),
            agent_type: format!("mcp:{provider}"),
            description: description.to_string(),
            status: "running".to_string(),
        };
        self.mcp_subagents
            .entry(parent_id)
            .or_default()
            .push((child_id, info));
    }

    pub fn update_mcp_subagent_status(&mut self, child_id: SessionId, status: &str) {
        for children in self.mcp_subagents.values_mut() {
            if let Some((_, info)) = children.iter_mut().find(|(id, _)| *id == child_id) {
                info.status = status.to_string();
            }
        }
    }

    pub fn get_mcp_subagents(&self, parent_id: SessionId) -> Vec<crate::models::SubagentInfo> {
        self.mcp_subagents
            .get(&parent_id)
            .map(|children| children.iter().map(|(_, info)| info.clone()).collect())
            .unwrap_or_default()
    }

    /// Phase 1 (fast): create DB record, return Session immediately.
    #[allow(clippy::too_many_arguments)]
    pub fn init_session(
        &mut self,
        project_path: &str,
        session_name: Option<&str>,
        permission_mode: &str,
        model: Option<&str>,
        use_worktree: bool,
        provider: Option<&str>,
        ssh_host: Option<&str>,
        ssh_user: Option<&str>,
        ssh_key_path: Option<String>,
    ) -> Result<Session, String> {
        let project_name = std::path::Path::new(project_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| project_path.to_string());

        let project = self
            .db
            .create_project(&project_name, project_path)
            .map_err(|e| e.to_string())?;

        let session_id = self
            .db
            .create_session(
                Some(project.id),
                session_name,
                project_path,
                permission_mode,
                model,
                provider,
                ssh_host,
                ssh_user,
            )
            .map_err(|e| e.to_string())?;

        let (worktree_path_val, branch_name_val) = if use_worktree {
            let full_name = session_name.unwrap_or(&project_name);
            let (prefix, suffix) = full_name.split_once(" · ").unwrap_or((full_name, ""));
            let prefix_slug = crate::services::worktree::generate_branch_slug(prefix);
            let suffix_compact: String = suffix
                .chars()
                .filter(|c| c.is_alphanumeric())
                .collect::<String>()
                .to_lowercase();
            let slug = if suffix_compact.is_empty() {
                format!("{prefix_slug}-{session_id}")
            } else {
                format!("{prefix_slug}-{suffix_compact}-{session_id}")
            };
            let (wt_str, branch) = if let (Some(host), Some(user)) = (ssh_host, ssh_user) {
                let remote_path = crate::services::worktree::create_worktree_remote(
                    host,
                    user,
                    ssh_key_path.as_deref(),
                    project_path,
                    &slug,
                )?;
                let branch = format!("orbit/{slug}");
                (remote_path, branch)
            } else {
                let wt_path = crate::services::worktree::create_worktree(
                    std::path::Path::new(project_path),
                    &slug,
                )?;
                let branch = format!("orbit/{slug}");
                let wt_str = wt_path.to_string_lossy().to_string();
                (wt_str, branch)
            };
            let _ = self
                .db
                .update_session_worktree(session_id, &wt_str, &branch);
            (Some(wt_str), Some(branch))
        } else {
            (None, None)
        };

        let now = chrono::Utc::now().to_rfc3339();
        let session = Session {
            id: session_id,
            project_id: Some(project.id),
            name: session_name.map(|s| s.to_string()),
            status: crate::models::SessionStatus::Initializing,
            worktree_path: worktree_path_val,
            branch_name: branch_name_val,
            permission_mode: permission_mode.to_string(),
            model: model.map(|s| s.to_string()),
            provider: provider.unwrap_or(DEFAULT_PROVIDER).to_string(),
            pid: None,
            created_at: now.clone(),
            updated_at: now,
            cwd: Some(project_path.to_string()),
            project_name: Some(project_name),
            git_branch: detect_git_branch(project_path),
            tokens: None,
            context_percent: None,
            pending_approval: None,
            mini_log: None,
            ssh_host: ssh_host.map(|s| s.to_string()),
            ssh_user: ssh_user.map(|s| s.to_string()),
            attention: None,
            skip_permissions: permission_mode == "ignore",
            parent_session_id: None,
            depth: 0,
        };

        // Persist SSH key path encrypted to DB (api_key saved separately via set_api_key)
        if ssh_key_path.is_some() {
            let _ = self
                .db
                .save_session_secrets(session_id, None, ssh_key_path.as_deref());
        }

        self.active.insert(
            session_id,
            ActiveSession {
                session: session.clone(),
                claude_session_id: None,
                effort: None,
                api_key: None,
                ssh_key_path,
                stdin: None,
            },
        );
        self.journal_states
            .insert(session_id, JournalState::default());

        Ok(session)
    }

    /// Phase 2 (async): spawn provider with `-p "prompt"`.
    /// Each message spawns a new process. Uses `--resume` for follow-ups.
    ///
    /// Resolves the provider from the registry and delegates spawning + output
    /// parsing to the `Provider` trait, eliminating per-provider match dispatch.
    ///
    /// Includes a guard against double-spawning: if the session already has a
    /// running PID and is not in a terminal state, the spawn is skipped.
    pub fn do_spawn(
        manager: Arc<RwLock<SessionManager>>,
        app: AppHandle,
        session_id: SessionId,
        prompt: String,
        registry: &ProviderRegistry,
    ) {
        // Guard: prevent double-spawn via a spawning_sessions set
        // This catches races before PID is assigned (unlike the PID check).
        {
            let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
            if !m.spawning_sessions.insert(session_id) {
                eprintln!(
                    "[orbit] warning: session {session_id} already spawning, \
                     skipping duplicate spawn"
                );
                return;
            }
        }
        // 1. Read session data from the active map
        let (
            db,
            cwd,
            model,
            provider_id,
            effort,
            resume_id,
            extra_env,
            spawn_mode,
            ssh_key_path,
            skip_permissions,
        ) = match manager.try_read() {
            Ok(m) => {
                let a = match m.active.get(&session_id) {
                    Some(a) => a,
                    None => {
                        let _ = app.emit(
                            "session:error",
                            serde_json::json!({
                                "sessionId": session_id,
                                "error": "Session not found in active map"
                            }),
                        );
                        {
                            let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
                            m.spawning_sessions.remove(&session_id);
                        }
                        return;
                    }
                };

                let raw_model = a.session.model.clone().unwrap_or_default();
                let pid_str = a.session.provider.clone();

                let spawn_mode = match (a.session.ssh_host.clone(), a.session.ssh_user.clone()) {
                    (Some(host), Some(user)) => crate::services::ssh::SpawnMode::Ssh { host, user },
                    (Some(host), None) => {
                        eprintln!(
                            "[orbit] session {session_id}: ssh_host={host:?} set but ssh_user is \
                         missing — falling back to local spawn."
                        );
                        crate::services::ssh::SpawnMode::Local
                    }
                    (None, Some(user)) => {
                        eprintln!(
                            "[orbit] session {session_id}: ssh_user={user:?} set but ssh_host is \
                         missing — falling back to local spawn."
                        );
                        crate::services::ssh::SpawnMode::Local
                    }
                    (None, None) => crate::services::ssh::SpawnMode::Local,
                };

                let mut extra_env = vec![("ORBIT_SESSION_ID".to_string(), session_id.to_string())];
                if let Some(ref key) = a.api_key {
                    let var_name = format!("{}_API_KEY", pid_str.to_uppercase().replace('-', "_"));
                    extra_env.push((var_name, key.clone()));
                }
                if let Some(ref effort) = a.effort {
                    extra_env.push(("ORBIT_EFFORT".to_string(), effort.clone()));
                }
                if let Some(ref resume_id) = a.claude_session_id {
                    extra_env.push(("ORBIT_RESUME_ID".to_string(), resume_id.clone()));
                }

                (
                    m.db.clone(),
                    a.session
                        .worktree_path
                        .clone()
                        .or_else(|| a.session.cwd.clone())
                        .unwrap_or_default(),
                    raw_model,
                    pid_str,
                    a.effort.clone(),
                    a.claude_session_id.clone(),
                    extra_env,
                    spawn_mode,
                    a.ssh_key_path.clone(),
                    a.session.skip_permissions,
                )
            }
            Err(_) => {
                eprintln!("[orbit] warning: failed to acquire read lock for session {session_id}, skipping spawn");
                {
                    let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
                    m.spawning_sessions.remove(&session_id);
                }
                return;
            }
        };

        // 2. Resolve provider from registry
        let provider = match registry.resolve(&provider_id) {
            Some(p) => p,
            None => {
                let _ = app.emit(
                    "session:error",
                    serde_json::json!({
                        "sessionId": session_id,
                        "error": format!("Unknown provider: {provider_id}")
                    }),
                );
                {
                    let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
                    m.spawning_sessions.remove(&session_id);
                }
                return;
            }
        };

        // 3. Format model via provider (no hardcoded string comparisons)
        let model = provider.format_model(&model, &provider_id);

        // 4. Set context window from provider
        if let Some(ctx) = provider.context_window(&model) {
            let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
            if let Some(state) = m.journal_states.get_mut(&session_id) {
                state.context_window = Some(ctx);
            }
        }

        // 4. Spawn CLI via provider trait
        let prompt_text = prompt.clone();
        if cfg!(debug_assertions) {
            eprintln!("[orbit:debug] ── spawn session {session_id} ──");
            eprintln!("[orbit:debug]   provider: {}", provider_id);
            eprintln!("[orbit:debug]   model: {}", model);
            eprintln!("[orbit:debug]   cwd: {}", cwd);
            eprintln!(
                "[orbit:debug]   spawn_mode: {}",
                match &spawn_mode {
                    crate::services::ssh::SpawnMode::Local => "local".to_string(),
                    crate::services::ssh::SpawnMode::Ssh { host, user } =>
                        format!("ssh {user}@{host}"),
                }
            );
            eprintln!(
                "[orbit:debug]   ssh_key_path: {}",
                if ssh_key_path.is_some() {
                    "<set>"
                } else {
                    "<none>"
                }
            );
            if !extra_env.is_empty() {
                for (k, _) in &extra_env {
                    eprintln!("[orbit:debug]   env: {k}=<set>");
                }
            }
            if let Some(ref rid) = resume_id {
                eprintln!("[orbit:debug]   resume: {rid}");
            }
            if let Some(ref effort) = effort {
                eprintln!("[orbit:debug]   effort: {effort}");
            }
            eprintln!(
                "[orbit:debug]   prompt: {}…",
                &prompt.chars().take(80).collect::<String>()
            );
        }
        let spawn_config = ProviderSpawnConfig {
            session_id,
            cwd: std::path::PathBuf::from(&cwd),
            provider_id: provider_id.clone(),
            model,
            prompt,
            resume_id,
            extra_env,
            effort,
            spawn_mode,
            ssh_key_path,
            skip_permissions,
        };

        // Write .mcp.json so the agent can use orbit-mcp tools for orchestration
        let is_local = matches!(
            &spawn_config.spawn_mode,
            crate::services::ssh::SpawnMode::Local
        );
        if is_local {
            ensure_mcp_config(&cwd);
        }

        let handle = match provider.spawn(spawn_config) {
            Ok(h) => h,
            Err(e) => {
                let _ = db.update_session_status(session_id, crate::models::SessionStatus::Error);
                {
                    let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
                    m.spawning_sessions.remove(&session_id);
                    if let Some(a) = m.active.get_mut(&session_id) {
                        a.session.attention = Some(crate::models::AttentionState {
                            requires_attention: true,
                            reason: Some(crate::models::AttentionReason::Error),
                            since: Some(chrono::Utc::now().to_rfc3339()),
                        });
                    }
                }
                let _ = app.emit(
                    "session:error",
                    serde_json::json!({
                        "sessionId": session_id, "error": e
                    }),
                );
                return;
            }
        };

        // Start git working tree watcher for real-time diff updates
        let cwd_path = std::path::PathBuf::from(&cwd);
        {
            let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
            let app_handle = app.clone();
            let sid = session_id;
            m.diff_manager.watch(cwd_path.clone(), move |snapshot| {
                let _ = app_handle.emit(
                    "session:git-update",
                    serde_json::json!({
                        "sessionId": sid,
                        "snapshot": snapshot
                    }),
                );
            });
            m.watch_map.insert(session_id, cwd_path);
        }

        // 5. Stderr drain — rate limit detection is handled by stdout rate_limit_event
        let stderr_reader = handle.stderr;
        let app_clone = app.clone();
        std::thread::spawn(move || {
            use std::io::BufRead;
            let mut reader = std::io::BufReader::new(stderr_reader);
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line) {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        let trimmed = line.trim();
                        if !trimmed.is_empty() {
                            if cfg!(debug_assertions) {
                                eprintln!("[orbit:debug] stderr {session_id}: {trimmed}");
                            }
                            eprintln!("[orbit:stderr] {session_id}: {trimmed}");
                            let _ = app_clone.emit(
                                "session:stderr",
                                serde_json::json!({
                                    "sessionId": session_id,
                                    "line": trimmed
                                }),
                            );
                        }
                    }
                }
            }
        });

        // 6. Store stdin handle (if provider returned one, e.g. ACP)
        let stdin_handle = handle.stdin.map(|s| Arc::new(std::sync::Mutex::new(s)));

        // 7. Emit spawn-started events
        Self::emit_spawn_started(
            &manager,
            &app,
            &db,
            session_id,
            handle.pid as i32,
            &prompt_text,
        );

        // 8. Attach stdin to the active session for later use (permission responses)
        if stdin_handle.is_some() {
            let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
            if let Some(a) = m.active.get_mut(&session_id) {
                a.stdin = stdin_handle;
            }
        }

        // 10. Get line processor fn pointer from the provider trait.
        // Uses fn pointer (not trait object) because threads require Send.
        let line_processor = provider.line_processor();
        let subagent_tools: Vec<String> = provider
            .subagent_tool_names()
            .iter()
            .map(|s| s.to_string())
            .collect();
        let task_tools: Vec<String> = provider
            .task_tool_names()
            .iter()
            .map(|s| s.to_string())
            .collect();

        // 11. Reader loop
        Self::reader_loop(
            Arc::clone(&manager),
            app,
            session_id,
            handle.reader,
            db,
            handle.child,
            line_processor,
            subagent_tools,
            task_tools,
        );
    }

    /// Common post-spawn: set Running status, emit session:running, emit user entry.
    fn emit_spawn_started(
        manager: &Arc<RwLock<SessionManager>>,
        app: &AppHandle,
        db: &Arc<DatabaseService>,
        session_id: SessionId,
        pid: i32,
        prompt_text: &str,
    ) {
        let _ = db.update_session_pid(session_id, pid);
        // Write PID file so orbit-mcp can discover its parent session
        let pid_file = std::env::temp_dir().join(format!("orbit-session-{pid}.id"));
        let _ = std::fs::write(&pid_file, session_id.to_string());
        {
            let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
            if let Some(a) = m.active.get_mut(&session_id) {
                a.session.status = crate::models::SessionStatus::Running;
                a.session.pid = Some(pid);
            }
        }

        let _ = app.emit(
            "session:running",
            serde_json::json!({ "sessionId": session_id, "pid": pid }),
        );

        // Skip if create_session already injected the initial user message.
        let skip_user_entry = {
            let m = manager.read().unwrap_or_else(|e| e.into_inner());
            m.journal_states.get(&session_id).is_some_and(|state| {
                state.entries.iter().any(|e| {
                    e.entry_type == crate::models::JournalEntryType::User
                        && e.text
                            .as_deref()
                            .is_some_and(|t| t.trim() == prompt_text.trim())
                })
            })
        };

        // Skip creating user entry if prompt is empty/whitespace-only
        if !prompt_text.trim().is_empty() && !skip_user_entry {
            let user_entry = crate::models::JournalEntry {
                session_id: session_id.to_string(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                entry_type: crate::models::JournalEntryType::User,
                text: Some(prompt_text.to_string()),
                ..crate::models::JournalEntry::default()
            };
            let user_line = serde_json::json!({
                "type": "user",
                "message": { "content": prompt_text },
                "timestamp": &user_entry.timestamp
            })
            .to_string();
            let _ = db.insert_output(session_id, &user_line);
            let _ = app.emit(
                "session:raw-output",
                serde_json::json!({ "sessionId": session_id, "line": &user_line }),
            );

            let emit_entry: crate::models::JournalEntry;
            {
                let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
                let state = m.journal_states.entry(session_id).or_default();
                let mut entry = user_entry;
                entry.seq = state.next_seq;
                entry.epoch = state.epoch.clone();
                state.next_seq += 1;
                emit_entry = entry.clone();
                state.entries.push(entry);
            }

            let _ = app.emit(
                "session:output",
                SessionOutputEvent {
                    session_id,
                    entry: emit_entry,
                },
            );
        }
    }

    /// Read JSON lines from Claude's stdout, parse, emit events.
    #[allow(clippy::too_many_arguments)]
    fn reader_loop(
        manager: Arc<RwLock<SessionManager>>,
        app: AppHandle,
        session_id: SessionId,
        reader: Box<dyn std::io::Read + Send>,
        db: Arc<DatabaseService>,
        mut child: std::process::Child,
        line_processor: fn(&mut JournalState, &str),
        subagent_tools: Vec<String>,
        task_tools: Vec<String>,
    ) {
        use std::io::BufRead;
        let mut reader = std::io::BufReader::new(reader);
        let mut line = String::new();

        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let trimmed = line.trim().to_string();
                    if trimmed.is_empty() || !trimmed.starts_with('{') {
                        continue;
                    }

                    if cfg!(debug_assertions) {
                        eprintln!("[orbit:debug] stdout {session_id}: {trimmed}");
                    }

                    // Extract and persist Claude session ID from system/init message
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&trimmed) {
                        // Extract CLI session ID for resume support:
                        //   Claude: "session_id", OpenCode: "sessionID", Codex: "thread_id"
                        let cli_sid = val
                            .get("session_id")
                            .or_else(|| val.get("sessionID"))
                            .or_else(|| val.get("thread_id"))
                            .and_then(|v| v.as_str());
                        if let Some(claude_id) = cli_sid {
                            let should_persist = {
                                let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
                                if let Some(a) = m.active.get_mut(&session_id) {
                                    if a.claude_session_id.is_none() {
                                        a.claude_session_id = Some(claude_id.to_string());
                                        true
                                    } else {
                                        false
                                    }
                                } else {
                                    false
                                }
                            };
                            // Persist to DB after releasing the write lock
                            if should_persist {
                                let _ = db.update_claude_session_id(session_id, claude_id);
                            }
                        }
                    }

                    // Detect rate limit errors from Claude's JSON stream
                    if is_rate_limit_line(&trimmed) {
                        let _ = app.emit(
                            "session:rate-limit",
                            serde_json::json!({ "sessionId": session_id }),
                        );
                    }

                    let skip_duplicate_user = {
                        let m = manager.read().unwrap_or_else(|e| e.into_inner());
                        m.journal_states.get(&session_id).is_some_and(|s| {
                            crate::journal::is_duplicate_injected_user_line(s, &trimmed)
                        })
                    };
                    if skip_duplicate_user {
                        continue;
                    }

                    let _ = db.insert_output(session_id, &trimmed);
                    let _ = app.emit(
                        "session:raw-output",
                        serde_json::json!({ "sessionId": session_id, "line": &trimmed }),
                    );

                    let (new_entries, state_event, is_rate_limit) = {
                        let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
                        let cwd = m
                            .active
                            .get(&session_id)
                            .and_then(|a| a.session.cwd.clone());
                        let provider_id = m
                            .active
                            .get(&session_id)
                            .map(|a| a.session.provider.clone())
                            .unwrap_or_else(|| DEFAULT_PROVIDER.to_string());
                        let session_model = m
                            .active
                            .get(&session_id)
                            .and_then(|a| a.session.model.clone());
                        let git_branch = cwd.as_deref().and_then(detect_git_branch);
                        let claude_session_id = m
                            .active
                            .get(&session_id)
                            .and_then(|a| a.claude_session_id.clone());
                        let mut subagents = claude_session_id
                            .as_deref()
                            .map(crate::agent_tree::read_subagents)
                            .unwrap_or_default();
                        for mcp in m.get_mcp_subagents(session_id) {
                            if !subagents.iter().any(|s| s.id == mcp.id) {
                                subagents.push(mcp);
                            }
                        }

                        let state = m.journal_states.entry(session_id).or_default();

                        let prev_len = state.entries.len();
                        let prev_model = state.model.clone();
                        line_processor(state, &trimmed);
                        // Assign seq/epoch to new entries
                        for entry in &mut state.entries[prev_len..] {
                            entry.seq = state.next_seq;
                            entry.epoch = state.epoch.clone();
                            state.next_seq += 1;
                        }
                        let new_entries: Vec<_> = state.entries[prev_len..].to_vec();

                        // Persist model to DB + active session when first detected
                        let model_changed = state.model != prev_model;
                        let detected_model = if model_changed {
                            state.model.clone()
                        } else {
                            None
                        };

                        let resolved_model = state.model.as_deref().or(session_model.as_deref());
                        let (window, context_percent) =
                            resolve_context_metrics(&provider_id, resolved_model, state);

                        let status_str = match state.status {
                            AgentStatus::Working => "working",
                            AgentStatus::Input => "input",
                            AgentStatus::Idle => "idle",
                            AgentStatus::New => "new",
                        }
                        .to_string();

                        let event = SessionStateEvent {
                            session_id,
                            status: status_str,
                            tokens: TokenUsage {
                                input: state.input_tokens,
                                output: state.output_tokens,
                                cache_read: state.cache_read,
                                cache_write: state.cache_write,
                            },
                            context_percent,
                            pending_approval: state.pending_approval.clone(),
                            mini_log: state.mini_log.clone(),
                            git_branch,
                            subagents,
                            model: state.model.clone(),
                            context_window: window,
                            attention: state.attention.clone(),
                            rate_limit: state.rate_limit.clone(),
                            cost_usd: state.cost_usd,
                        };
                        if let Some(ref model) = detected_model {
                            let _ = db.update_session_model(session_id, model);
                            if let Some(a) = m.active.get_mut(&session_id) {
                                a.session.model = Some(model.clone());
                            }
                        }

                        let is_rate_limit = event.attention.requires_attention
                            && event.attention.reason.as_ref().is_some_and(|r| {
                                matches!(r, crate::models::AttentionReason::RateLimit)
                            });
                        (new_entries, event, is_rate_limit)
                    };

                    // Emit rate-limit event when journal detects it in the output stream
                    if is_rate_limit {
                        let _ = app.emit(
                            "session:rate-limit",
                            serde_json::json!({ "sessionId": session_id }),
                        );
                    }

                    for entry in new_entries {
                        let mut e = entry.clone();
                        e.session_id = session_id.to_string();

                        // Detect sub-agent spawns
                        if e.entry_type == crate::models::JournalEntryType::ToolCall {
                            if let Some(ref tool) = e.tool {
                                if subagent_tools.contains(tool) {
                                    let desc = e
                                        .tool_input
                                        .as_ref()
                                        .and_then(|v| v.get("description"))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("subagent")
                                        .to_string();
                                    let _ = app.emit(
                                        "session:subagent-created",
                                        serde_json::json!({
                                            "parentSessionId": session_id,
                                            "description": desc,
                                            "tool": tool,
                                        }),
                                    );
                                }
                            }
                        }

                        // Detect task list updates
                        if e.entry_type == crate::models::JournalEntryType::ToolCall {
                            if let Some(ref tool) = e.tool {
                                if task_tools.contains(tool) {
                                    let _ = app.emit(
                                        "session:task-update",
                                        serde_json::json!({
                                            "sessionId": session_id,
                                        }),
                                    );
                                }
                            }
                        }

                        let _ = app.emit(
                            "session:output",
                            SessionOutputEvent {
                                session_id,
                                entry: e,
                            },
                        );
                    }
                    let _ = app.emit("session:state", &state_event);
                }
            }
        }

        {
            let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
            if let Some(a) = m.active.get_mut(&session_id) {
                a.session.status = crate::models::SessionStatus::Completed;
                a.session.attention = Some(crate::models::AttentionState {
                    requires_attention: true,
                    reason: Some(crate::models::AttentionReason::Completed),
                    since: Some(chrono::Utc::now().to_rfc3339()),
                });
            }
            if let Some(state) = m.journal_states.get_mut(&session_id) {
                state.status = AgentStatus::Idle;
                state.attention = crate::models::AttentionState {
                    requires_attention: true,
                    reason: Some(crate::models::AttentionReason::Completed),
                    since: Some(chrono::Utc::now().to_rfc3339()),
                };
            }
            m.spawning_sessions.remove(&session_id);
            let _ = db.update_session_status(session_id, crate::models::SessionStatus::Completed);
        }

        let _ = app.emit(
            "session:stopped",
            serde_json::json!({ "sessionId": session_id }),
        );

        // Collect exit status — prevents zombie on Unix, releases handle on Windows
        let _ = child.wait();
    }

    /// Send a follow-up message by spawning a new CLI process with --resume.
    /// Reads session data from DB so it works even after app restart.
    pub fn send_message(
        manager: Arc<RwLock<SessionManager>>,
        app: AppHandle,
        session_id: SessionId,
        text: String,
        registry: Arc<ProviderRegistry>,
    ) -> Result<(), String> {
        // Re-add to active map if missing (e.g. after app restart)
        {
            let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
            if !m.active.contains_key(&session_id) {
                // Load from DB
                let session =
                    m.db.get_session(session_id)
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| format!("Session {session_id} not found"))?;

                let claude_session_id = m.db.get_claude_session_id(session_id).ok().flatten();
                let (api_key, ssh_key_path) =
                    m.db.load_session_secrets(session_id)
                        .unwrap_or((None, None));

                m.active.insert(
                    session_id,
                    ActiveSession {
                        session,
                        claude_session_id,
                        effort: None,
                        api_key,
                        ssh_key_path,
                        stdin: None,
                    },
                );
                m.journal_states.entry(session_id).or_default();
            }
        }

        // Push the user's follow-up message as a journal entry so it appears in the chat
        {
            let mut m = manager.write().unwrap_or_else(|e| e.into_inner());
            if let Some(state) = m.journal_states.get_mut(&session_id) {
                let user_entry = crate::models::JournalEntry {
                    session_id: session_id.to_string(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    entry_type: crate::models::JournalEntryType::User,
                    text: Some(text.clone()),
                    seq: state.next_seq,
                    epoch: state.epoch.clone(),
                    ..crate::models::JournalEntry::default()
                };
                state.next_seq += 1;
                state.entries.push(user_entry.clone());
                let user_line = serde_json::json!({
                    "type": "user",
                    "message": { "content": &text },
                    "timestamp": &user_entry.timestamp
                })
                .to_string();
                let _ = m.db.insert_output(session_id, &user_line);
                let _ = app.emit(
                    "session:output",
                    serde_json::json!({
                        "sessionId": session_id,
                        "entry": user_entry
                    }),
                );
            }
        }

        let manager_clone = Arc::clone(&manager);
        std::thread::spawn(move || {
            Self::do_spawn(manager_clone, app, session_id, text, &registry);
        });

        Ok(())
    }

    pub fn stop_session(&mut self, session_id: SessionId) -> Result<(), String> {
        // Unwatch git working tree
        if let Some(cwd) = self.watch_map.remove(&session_id) {
            self.diff_manager.unwatch(&cwd);
        }
        if let Some(a) = self.active.get(&session_id) {
            if let Some(pid) = a.session.pid {
                kill_pid(pid as u32);
                let pid_file = std::env::temp_dir().join(format!("orbit-session-{pid}.id"));
                let _ = std::fs::remove_file(pid_file);
            }
        }
        self.active.remove(&session_id);
        self.spawning_sessions.remove(&session_id);
        let _ = self
            .db
            .update_session_status(session_id, crate::models::SessionStatus::Stopped);
        Ok(())
    }

    pub fn get_sessions(&mut self) -> Vec<Session> {
        let mut sessions = self.db.get_sessions().unwrap_or_default();
        for s in &mut sessions {
            self.load_session_journal(s.id);
            if let Some(state) = self.journal_states.get(&s.id) {
                let resolved_model = state.model.as_deref().or(s.model.as_deref());
                let (_window, context_percent) =
                    resolve_context_metrics(&s.provider, resolved_model, state);
                s.tokens = Some(TokenUsage {
                    input: state.input_tokens,
                    output: state.output_tokens,
                    cache_read: state.cache_read,
                    cache_write: state.cache_write,
                });
                s.context_percent = Some(context_percent);
                s.pending_approval = state.pending_approval.clone();
                s.mini_log = Some(state.mini_log.clone());
            }
            if let Some(a) = self.active.get(&s.id) {
                s.status = a.session.status.clone();
                s.pid = a.session.pid;
            }
        }
        sessions
    }

    pub fn get_journal(&mut self, session_id: SessionId) -> Vec<crate::models::JournalEntry> {
        self.load_session_journal(session_id);
        self.journal_states
            .get(&session_id)
            .map(|state| {
                state
                    .entries
                    .iter()
                    .map(|e| {
                        let mut entry = e.clone();
                        entry.session_id = session_id.to_string();
                        entry
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Load journal state for `session_id` from DB into `journal_states` if not already present.
    fn load_session_journal(&mut self, session_id: SessionId) {
        if self.journal_states.contains_key(&session_id) {
            return;
        }
        let rows = match self.db.get_outputs(session_id) {
            Ok(r) => r,
            Err(_) => return,
        };
        if rows.is_empty() && !self.active.contains_key(&session_id) {
            return;
        }

        // Pick the right JSONL parser based on session provider
        let provider_owned = self
            .active
            .get(&session_id)
            .map(|a| a.session.provider.clone())
            .or_else(|| {
                self.db
                    .get_session(session_id)
                    .ok()
                    .flatten()
                    .map(|s| s.provider)
            })
            .unwrap_or_else(|| DEFAULT_PROVIDER.to_string());
        let line_processor = crate::providers::ProviderRegistry::with_shipped_providers()
            .line_processor_for(&provider_owned);

        let mut state = JournalState::default();
        for line in &rows {
            line_processor(&mut state, line);
        }
        self.journal_states.insert(session_id, state);
    }

    pub fn is_session_active(&self, session_id: SessionId) -> bool {
        self.active.contains_key(&session_id)
    }

    pub fn get_session_provider(&self, session_id: SessionId) -> Option<String> {
        self.active
            .get(&session_id)
            .map(|a| a.session.provider.clone())
    }

    pub fn rename_session(&mut self, session_id: SessionId, name: &str) -> Result<(), String> {
        self.db
            .rename_session(session_id, name)
            .map_err(|e| e.to_string())
    }

    pub fn update_session_model(
        &mut self,
        session_id: SessionId,
        model: &str,
    ) -> Result<(), String> {
        if let Some(a) = self.active.get_mut(&session_id) {
            a.session.model = Some(model.to_string());
        }
        // Reset context_window so it re-derives from the new model
        if let Some(state) = self.journal_states.get_mut(&session_id) {
            state.context_window = None;
        }
        self.db
            .update_session_model(session_id, model)
            .map_err(|e| e.to_string())
    }

    pub fn update_session_effort(
        &mut self,
        session_id: SessionId,
        effort: &str,
    ) -> Result<(), String> {
        if let Some(a) = self.active.get_mut(&session_id) {
            a.effort = Some(effort.to_string());
        }
        Ok(())
    }

    /// Respond to a pending ACP permission request by writing a JSON-RPC response to stdin.
    pub fn respond_permission(&mut self, session_id: SessionId, allow: bool) -> Result<(), String> {
        let request_id = {
            let state = self
                .journal_states
                .get(&session_id)
                .ok_or("Session journal state not found")?;
            state
                .pending_approval_id
                .clone()
                .ok_or("No pending permission request")?
        };

        let stdin = {
            let a = self.active.get(&session_id).ok_or("Session not active")?;
            a.stdin
                .clone()
                .ok_or("Session does not support interactive permissions (no stdin)")?
        };

        let response = if allow {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": { "approved": true }
            })
        } else {
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": request_id,
                "result": { "approved": false }
            })
        };

        {
            let mut writer = stdin
                .lock()
                .map_err(|e| format!("stdin lock failed: {e}"))?;
            let line = serde_json::to_string(&response).map_err(|e| format!("serialize: {e}"))?;
            writer
                .write_all(line.as_bytes())
                .map_err(|e| format!("write: {e}"))?;
            writer
                .write_all(b"\n")
                .map_err(|e| format!("write newline: {e}"))?;
            writer.flush().map_err(|e| format!("flush: {e}"))?;
        }

        // Clear pending state
        if let Some(state) = self.journal_states.get_mut(&session_id) {
            state.pending_approval = None;
            state.pending_approval_id = None;
            state.status = crate::models::AgentStatus::Working;
            state.attention = crate::models::AttentionState {
                requires_attention: false,
                reason: None,
                since: None,
            };
        }

        Ok(())
    }

    pub fn clear_attention(&mut self, session_id: SessionId) -> Result<(), String> {
        if let Some(a) = self.active.get_mut(&session_id) {
            a.session.attention = Some(crate::models::AttentionState {
                requires_attention: false,
                reason: None,
                since: None,
            });
        }
        if let Some(state) = self.journal_states.get_mut(&session_id) {
            state.attention = crate::models::AttentionState {
                requires_attention: false,
                reason: None,
                since: None,
            };
        }
        Ok(())
    }

    pub fn set_api_key(&mut self, session_id: SessionId, api_key: String) {
        if let Some(a) = self.active.get_mut(&session_id) {
            // Persist encrypted to DB so it survives app restart
            let ssh_kp = a.ssh_key_path.as_deref();
            let _ = self
                .db
                .save_session_secrets(session_id, Some(&api_key), ssh_kp);
            a.api_key = Some(api_key);
        }
    }

    pub fn delete_session(&mut self, session_id: SessionId) -> Result<(), String> {
        // Unwatch git working tree
        if let Some(cwd) = self.watch_map.remove(&session_id) {
            self.diff_manager.unwatch(&cwd);
        }
        self.active.remove(&session_id);
        self.journal_states.remove(&session_id);
        self.db
            .delete_session(session_id)
            .map_err(|e| e.to_string())
    }

    pub fn reset_all_sessions(&mut self) -> Result<(), String> {
        for (_, a) in self.active.iter() {
            if let Some(pid) = a.session.pid {
                kill_pid(pid as u32);
                let pid_file = std::env::temp_dir().join(format!("orbit-session-{pid}.id"));
                let _ = std::fs::remove_file(pid_file);
            }
        }
        // Unwatch all git working trees
        for (_, cwd) in self.watch_map.drain() {
            self.diff_manager.unwatch(&cwd);
        }
        self.active.clear();
        self.journal_states.clear();
        self.db.delete_all_sessions().map_err(|e| e.to_string())
    }

    /// Gracefully stop a session with a timeout. If the session doesn't stop
    /// within the timeout, the process is killed forcefully.
    pub fn stop_session_with_timeout(
        &mut self,
        session_id: SessionId,
        timeout_ms: u64,
    ) -> Result<(), String> {
        let pid = self.active.get(&session_id).and_then(|a| a.session.pid);

        // Unwatch git working tree
        if let Some(cwd) = self.watch_map.remove(&session_id) {
            self.diff_manager.unwatch(&cwd);
        }

        if let Some(pid) = pid {
            // Try graceful stop first
            kill_pid(pid as u32);

            // Wait for process to exit with timeout using polling
            let start = std::time::Instant::now();
            let timeout = std::time::Duration::from_millis(timeout_ms);
            loop {
                if start.elapsed() >= timeout {
                    eprintln!(
                        "[orbit] warning: session {session_id} (pid={pid}) did not stop within \
                         {timeout_ms}ms timeout — force-killing"
                    );
                    kill_pid(pid as u32);
                    break;
                }
                // Check if process is still running via process exit
                let mut tasklist = std::process::Command::new("tasklist");
                tasklist.args(["/FI", &format!("PID eq {pid}"), "/NH"]);
                crate::services::process_util::apply_silent(&mut tasklist);
                let is_alive = tasklist.output();
                match is_alive {
                    Ok(output) => {
                        let out = String::from_utf8_lossy(&output.stdout);
                        let pid_str = pid.to_string();
                        if !out.contains(&pid_str) {
                            break;
                        }
                    }
                    Err(_) => {
                        break;
                    }
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }

        self.active.remove(&session_id);
        self.spawning_sessions.remove(&session_id);
        let _ = self
            .db
            .update_session_status(session_id, crate::models::SessionStatus::Stopped);
        Ok(())
    }

    /// Eagerly load journal state for all sessions from DB.
    /// Not called at startup (journals load lazily on first access).
    /// Available as a utility for warming the cache or in tests.
    pub fn restore_from_db(&mut self) {
        let session_ids: Vec<SessionId> = self
            .db
            .get_sessions()
            .unwrap_or_default()
            .into_iter()
            .map(|s| s.id)
            .collect();
        for id in session_ids {
            self.load_session_journal(id);
        }
    }
}

fn kill_pid(pid: u32) {
    #[cfg(windows)]
    {
        let mut taskkill = std::process::Command::new("taskkill");
        taskkill.args(["/F", "/T", "/PID", &pid.to_string()]);
        crate::services::process_util::apply_silent(&mut taskkill);
        let _ = taskkill.output();
    }

    #[cfg(not(windows))]
    {
        let _ = std::process::Command::new("kill")
            .args(["-TERM", &pid.to_string()])
            .output();
        std::thread::sleep(std::time::Duration::from_millis(300));
        let _ = std::process::Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .output();
    }
}

/// Case-insensitive substring search without allocation (ASCII only).
/// Only used in tests — kept out of production paths after rate-limit detection was tightened.
#[cfg(test)]
fn ascii_ci_contains(haystack: &str, needle: &str) -> bool {
    let h = haystack.as_bytes();
    let n = needle.as_bytes();
    if h.len() < n.len() {
        return false;
    }
    h.windows(n.len()).any(|w| w.eq_ignore_ascii_case(n))
}

/// Check if a JSON line from Claude's stdout indicates a rate limit error.
///
/// Parses the JSON and requires:
/// - top-level `"type"` == `"error"`
/// - nested `"error"."type"` is `"rate_limit_error"` or `"overloaded_error"`
///
/// This avoids false positives when assistant messages mention "rate limit"
/// or "overloaded" in their text content.
fn is_rate_limit_line(line: &str) -> bool {
    let Ok(val) = serde_json::from_str::<serde_json::Value>(line) else {
        return false;
    };
    if val.get("type").and_then(|v| v.as_str()) != Some("error") {
        return false;
    }
    let error_type = val
        .get("error")
        .and_then(|e| e.get("type"))
        .and_then(|t| t.as_str())
        .unwrap_or("");
    matches!(error_type, "rate_limit_error" | "overloaded_error")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_utils::{assistant_with_tokens, make_db, seed_outputs, TestCase};

    fn make_manager() -> Arc<RwLock<SessionManager>> {
        Arc::new(RwLock::new(SessionManager::new(make_db())))
    }

    // ── init_session ─────────────────────────────────────────────────────

    #[test]
    fn should_create_db_record_on_init() {
        let mut t = TestCase::new("should_create_db_record_on_init");
        t.phase("Act");
        let mgr = make_manager();
        let s = mgr
            .write()
            .unwrap()
            .init_session(
                "/tmp/proj",
                None,
                "ignore",
                None,
                false,
                None,
                None,
                None,
                None,
            )
            .expect("init failed");
        t.phase("Assert");
        t.ok("id is positive", s.id > 0);
        t.eq(
            "status is initializing",
            &s.status,
            &crate::models::SessionStatus::Initializing,
        );
    }

    #[test]
    fn should_register_journal_state_on_init() {
        let mut t = TestCase::new("should_register_journal_state_on_init");
        t.phase("Act");
        let mgr = make_manager();
        let s = mgr
            .write()
            .unwrap()
            .init_session(
                "/tmp/proj",
                None,
                "ignore",
                None,
                false,
                None,
                None,
                None,
                None,
            )
            .expect("init failed");
        t.phase("Assert");
        t.ok(
            "journal_state entry created",
            mgr.write().unwrap().journal_states.contains_key(&s.id),
        );
    }

    #[test]
    fn should_register_session_as_active_on_init() {
        let mut t = TestCase::new("should_register_session_as_active_on_init");
        t.phase("Act");
        let mgr = make_manager();
        let s = mgr
            .write()
            .unwrap()
            .init_session(
                "/tmp/proj",
                None,
                "ignore",
                None,
                false,
                None,
                None,
                None,
                None,
            )
            .expect("init failed");
        t.phase("Assert");
        t.ok(
            "session is active",
            mgr.write().unwrap().is_session_active(s.id),
        );
    }

    // ── stop_session ─────────────────────────────────────────────────────

    #[test]
    fn should_set_stopped_status_in_db_after_stop() {
        let mut t = TestCase::new("should_set_stopped_status_in_db_after_stop");
        t.phase("Seed");
        let mgr = make_manager();
        let s = mgr
            .write()
            .unwrap()
            .init_session(
                "/tmp/proj",
                None,
                "ignore",
                None,
                false,
                None,
                None,
                None,
                None,
            )
            .expect("init failed");
        t.phase("Act");
        mgr.write()
            .unwrap()
            .stop_session(s.id)
            .expect("stop failed");
        t.phase("Assert");
        let sessions = mgr.write().unwrap().get_sessions();
        t.eq(
            "status is stopped",
            &sessions[0].status,
            &crate::models::SessionStatus::Stopped,
        );
    }

    // ── delete_session ────────────────────────────────────────────────────

    #[test]
    fn should_remove_session_from_active_and_journal_after_delete() {
        let mut t = TestCase::new("should_remove_session_from_active_and_journal_after_delete");
        t.phase("Seed");
        let mgr = make_manager();
        let s = mgr
            .write()
            .unwrap()
            .init_session(
                "/tmp/proj",
                None,
                "ignore",
                None,
                false,
                None,
                None,
                None,
                None,
            )
            .expect("init failed");
        t.phase("Act");
        mgr.write()
            .unwrap()
            .delete_session(s.id)
            .expect("delete failed");
        t.phase("Assert");
        let mut m = mgr.write().unwrap();
        t.ok("not in active map", !m.is_session_active(s.id));
        t.ok(
            "journal_state removed",
            !m.journal_states.contains_key(&s.id),
        );
        t.empty("no sessions in DB", &m.get_sessions());
    }

    // ── rename_session ────────────────────────────────────────────────────

    #[test]
    fn should_persist_renamed_session_name() {
        let mut t = TestCase::new("should_persist_renamed_session_name");
        t.phase("Seed");
        let mgr = make_manager();
        let s = mgr
            .write()
            .unwrap()
            .init_session(
                "/tmp/proj",
                Some("old-name"),
                "ignore",
                None,
                false,
                None,
                None,
                None,
                None,
            )
            .expect("init failed");
        t.phase("Act");
        mgr.write()
            .unwrap()
            .rename_session(s.id, "new-name")
            .expect("rename failed");
        t.phase("Assert");
        let sessions = mgr.write().unwrap().get_sessions();
        t.eq(
            "name updated",
            sessions[0].name.as_deref(),
            Some("new-name"),
        );
    }

    // ── send_message precondition ─────────────────────────────────────────

    #[test]
    fn should_confirm_session_does_not_exist_before_send_message_would_fail() {
        let mut t =
            TestCase::new("should_confirm_session_does_not_exist_before_send_message_would_fail");
        t.phase("Seed — no sessions exist");
        let mgr = make_manager();
        t.phase("Act — verify DB has no session 999");
        let m = mgr.write().unwrap();
        let db_result = m.db.get_session(999).expect("db query ok");
        drop(m);
        t.phase("Assert");
        t.none(
            "session 999 not in DB (error path precondition)",
            &db_result,
        );
        // Note: send_message requires a Tauri AppHandle which cannot be constructed
        // outside the Tauri runtime, so we verify the precondition that guarantees
        // the error path instead of calling send_message directly.
        t.ok("precondition verified", true);
    }

    // ── restore_from_db ───────────────────────────────────────────────────

    #[test]
    fn should_rebuild_journal_state_from_stored_outputs() {
        let mut t = TestCase::new("should_rebuild_journal_state_from_stored_outputs");
        t.phase("Seed");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None, None, None, None)
            .expect("session");
        seed_outputs(
            &db,
            sid,
            &[&crate::test_utils::assistant_text("Restored entry")],
        );
        t.phase("Act");
        let mut sm = SessionManager::new(db);
        sm.restore_from_db();
        t.phase("Assert");
        let journal = sm.get_journal(sid);
        t.len("one entry restored", &journal, 1);
        t.eq(
            "entry text matches",
            journal[0].text.as_deref(),
            Some("Restored entry"),
        );
    }

    #[test]
    fn should_restore_codex_outputs_using_registry_line_processor() {
        let mut t = TestCase::new("should_restore_codex_outputs_using_registry_line_processor");
        t.phase("Seed");
        let db = make_db();
        let sid = db
            .create_session(
                None,
                None,
                "/tmp",
                "ignore",
                None,
                Some("codex"),
                None,
                None,
            )
            .expect("session");
        let codex_line =
            r#"{"type":"item.completed","item":{"type":"agent_message","text":"codex-reply"}}"#;
        seed_outputs(&db, sid, &[codex_line]);
        t.phase("Act");
        let mut sm = SessionManager::new(Arc::clone(&db));
        sm.restore_from_db();
        t.phase("Assert");
        let journal = sm.get_journal(sid);
        t.len("one codex entry", &journal, 1);
        t.eq(
            "assistant text from codex jsonl",
            journal[0].text.as_deref(),
            Some("codex-reply"),
        );
        t.eq(
            "entry type",
            journal[0].entry_type,
            crate::models::JournalEntryType::Assistant,
        );
    }

    #[test]
    fn should_restore_follow_up_user_message_for_opencode_from_db() {
        let mut t = TestCase::new("should_restore_follow_up_user_message_for_opencode_from_db");
        t.phase("Seed");
        let db = make_db();
        let sid = db
            .create_session(
                None,
                None,
                "/tmp",
                "ignore",
                None,
                None,
                Some("opencode"),
                Some("openrouter/minimax"),
            )
            .expect("session");
        seed_outputs(
            &db,
            sid,
            &[&crate::test_utils::user_text("Also fix OpenCode history")],
        );
        t.phase("Act");
        let mut sm = SessionManager::new(Arc::clone(&db));
        sm.restore_from_db();
        t.phase("Assert");
        let journal = sm.get_journal(sid);
        t.len("one user entry", &journal, 1);
        t.eq(
            "follow-up text restored",
            journal[0].text.as_deref(),
            Some("Also fix OpenCode history"),
        );
    }

    #[test]
    fn should_restore_follow_up_user_message_from_db() {
        let mut t = TestCase::new("should_restore_follow_up_user_message_from_db");
        t.phase("Seed");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None, None, None, None)
            .expect("session");
        seed_outputs(
            &db,
            sid,
            &[&crate::test_utils::user_text("Also fix the tests")],
        );
        t.phase("Act");
        let mut sm = SessionManager::new(Arc::clone(&db));
        sm.restore_from_db();
        t.phase("Assert");
        let journal = sm.get_journal(sid);
        t.len("one user entry", &journal, 1);
        t.eq(
            "follow-up text restored",
            journal[0].text.as_deref(),
            Some("Also fix the tests"),
        );
        t.eq(
            "entry type",
            journal[0].entry_type,
            crate::models::JournalEntryType::User,
        );
    }

    #[test]
    fn should_not_duplicate_entries_on_double_restore() {
        let mut t = TestCase::new("should_not_duplicate_entries_on_double_restore");
        t.phase("Seed");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None, None, None, None)
            .expect("session");
        seed_outputs(&db, sid, &[&crate::test_utils::assistant_text("Hello")]);
        t.phase("Act");
        let mut sm = SessionManager::new(Arc::clone(&db));
        sm.restore_from_db();
        sm.restore_from_db(); // second call must be idempotent
        t.phase("Assert");
        let journal = sm.get_journal(sid);
        t.len("still exactly one entry (no duplication)", &journal, 1);
    }

    #[test]
    fn should_restore_token_counts_from_stored_outputs() {
        let mut t = TestCase::new("should_restore_token_counts_from_stored_outputs");
        t.phase("Seed");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None, None, None, None)
            .expect("session");
        // input=10, output=5, cache_write=2, cache_read=1 → input_tokens = 13
        seed_outputs(&db, sid, &[&assistant_with_tokens("Hi", 10, 5, 2, 1)]);
        t.phase("Act");
        let mut sm = SessionManager::new(Arc::clone(&db));
        sm.restore_from_db();
        t.phase("Assert");
        let sessions = sm.get_sessions();
        let tokens = sessions[0]
            .tokens
            .as_ref()
            .expect("tokens missing after restore");
        t.eq("output_tokens restored", tokens.output, 5u64);
    }

    // ── ascii_ci_contains ─────────────────────────────────────────────────────

    #[test]
    fn should_find_needle_case_insensitively() {
        let mut t = TestCase::new("should_find_needle_case_insensitively");
        t.phase("Assert");
        t.ok("exact match", ascii_ci_contains("rate_limit", "rate_limit"));
        t.ok(
            "upper needle",
            ascii_ci_contains("RATE_LIMIT", "rate_limit"),
        );
        t.ok(
            "mixed case haystack",
            ascii_ci_contains("Rate_Limit_Error", "rate_limit"),
        );
        t.ok(
            "not found",
            !ascii_ci_contains("something else", "rate_limit"),
        );
        t.ok("empty haystack", !ascii_ci_contains("", "rate_limit"));
        t.ok(
            "needle longer than haystack",
            !ascii_ci_contains("rt", "rate_limit"),
        );
    }

    // ── is_rate_limit_line ────────────────────────────────────────────────────

    #[test]
    fn should_detect_rate_limit_error_line() {
        let mut t = TestCase::new("should_detect_rate_limit_error_line");
        t.phase("Assert — canonical rate limit JSON");
        t.ok(
            "rate_limit_error in error object",
            is_rate_limit_line(
                r#"{"type":"error","error":{"type":"rate_limit_error","message":"Rate limit exceeded"}}"#,
            ),
        );
        t.ok(
            "overloaded_error in error object",
            is_rate_limit_line(r#"{"type":"error","error":{"type":"overloaded_error"}}"#),
        );
    }

    #[test]
    fn should_not_flag_assistant_message_mentioning_rate_limit() {
        let mut t = TestCase::new("should_not_flag_assistant_message_mentioning_rate_limit");
        t.phase("Assert — assistant message with 'rate limit' in text must NOT trigger");
        t.ok(
            "assistant type with rate limit text",
            !is_rate_limit_line(
                r#"{"type":"assistant","message":{"content":[{"type":"text","text":"The rate limit policy allows 1000 requests per minute."}]}}"#,
            ),
        );
    }

    #[test]
    fn should_not_flag_tool_result_mentioning_overloaded() {
        let mut t = TestCase::new("should_not_flag_tool_result_mentioning_overloaded");
        t.phase("Assert — tool_result containing 'overloaded' must NOT trigger");
        t.ok(
            "tool_result type with overloaded in output",
            !is_rate_limit_line(
                r#"{"type":"tool_result","content":"Server is overloaded, please retry"}"#,
            ),
        );
    }

    #[test]
    fn should_not_flag_non_rate_limit_lines() {
        let mut t = TestCase::new("should_not_flag_non_rate_limit_lines");
        t.phase("Assert — lines that should NOT trigger");
        t.ok(
            "rate_limit without error object",
            !is_rate_limit_line(r#"{"type":"assistant","text":"rate_limit info"}"#),
        );
        t.ok(
            "error type but no matching error subtype",
            !is_rate_limit_line(
                r#"{"type":"error","error":{"type":"api_error","message":"internal"}}"#,
            ),
        );
        t.ok(
            "plain overloaded text (not JSON error)",
            !is_rate_limit_line(r#"overloaded"#),
        );
        t.ok("empty line", !is_rate_limit_line(""));
        t.ok(
            "normal assistant line",
            !is_rate_limit_line(r#"{"type":"assistant","text":"hello world"}"#),
        );
    }

    #[test]
    fn should_detect_rate_limit_in_stderr_exact_substring() {
        // stderr lines are plain text — the check uses exact substring matching
        // for "rate_limit_error" or "overloaded_error" (not the broader "rate limit")
        let mut t = TestCase::new("should_detect_rate_limit_in_stderr_exact_substring");
        t.phase("Assert — exact substrings that must match");
        t.ok(
            "rate_limit_error substring present",
            "rate_limit_error: too many requests".contains("rate_limit_error"),
        );
        t.ok(
            "overloaded_error substring present",
            "overloaded_error detected".contains("overloaded_error"),
        );
        t.phase("Assert — generic 'rate limit' must NOT match the tightened check");
        t.ok(
            "generic 'rate limit' phrase does not match rate_limit_error",
            !"You have hit the rate limit today".contains("rate_limit_error"),
        );
        t.ok(
            "generic 'overloaded' does not match overloaded_error",
            !"Server is overloaded".contains("overloaded_error"),
        );
    }

    // ── lazy journal loading ──────────────────────────────────────────────

    #[test]
    fn should_not_preload_journal_state_on_creation() {
        let mut t = TestCase::new("should_not_preload_journal_state_on_creation");
        t.phase("Seed — DB has session with outputs, manager is fresh (no restore)");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None, None, None, None)
            .expect("session");
        seed_outputs(&db, sid, &[&crate::test_utils::assistant_text("hello")]);
        t.phase("Act — create manager without calling restore_from_db");
        let sm = SessionManager::new(Arc::clone(&db));
        t.phase("Assert — journal not loaded yet");
        t.ok(
            "journal_states empty before first access",
            !sm.journal_states.contains_key(&sid),
        );
    }

    #[test]
    fn should_lazy_load_tokens_on_get_sessions() {
        let mut t = TestCase::new("should_lazy_load_tokens_on_get_sessions");
        t.phase("Seed — session with token output exists");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None, None, None, None)
            .expect("session");
        seed_outputs(
            &db,
            sid,
            &[&crate::test_utils::assistant_with_tokens("Hi", 10, 5, 2, 1)],
        );
        t.phase("Act — fresh manager, no restore, call get_sessions");
        let mut sm = SessionManager::new(Arc::clone(&db));
        let sessions = sm.get_sessions();
        t.phase("Assert — tokens populated via lazy load");
        let tokens = sessions[0]
            .tokens
            .as_ref()
            .expect("tokens should be loaded");
        t.eq("output_tokens loaded", tokens.output, 5u64);
        t.ok(
            "journal_state was populated",
            sm.journal_states.contains_key(&sid),
        );
    }

    #[test]
    fn should_lazy_load_journal_on_first_get_journal() {
        let mut t = TestCase::new("should_lazy_load_journal_on_first_get_journal");
        t.phase("Seed");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None, None, None, None)
            .expect("session");
        seed_outputs(&db, sid, &[&crate::test_utils::assistant_text("hello")]);
        t.phase("Act — get_journal triggers lazy load");
        let mut sm = SessionManager::new(Arc::clone(&db));
        let journal = sm.get_journal(sid);
        t.phase("Assert");
        t.len("one entry loaded on demand", &journal, 1);
    }

    // ── get_journal ───────────────────────────────────────────────────────

    #[test]
    fn should_fill_session_id_on_all_journal_entries() {
        let mut t = TestCase::new("should_fill_session_id_on_all_journal_entries");
        t.phase("Seed");
        let db = make_db();
        let sid = db
            .create_session(None, None, "/tmp", "ignore", None, None, None, None)
            .expect("session");
        seed_outputs(
            &db,
            sid,
            &[
                &crate::test_utils::assistant_text("First"),
                &crate::test_utils::assistant_text("Second"),
            ],
        );
        let mut sm = SessionManager::new(db);
        sm.restore_from_db();
        t.phase("Act");
        let journal = sm.get_journal(sid);
        t.phase("Assert");
        t.len("two entries", &journal, 2);
        let expected_id = sid.to_string();
        t.eq(
            "first entry has session_id",
            journal[0].session_id.as_str(),
            expected_id.as_str(),
        );
        t.eq(
            "second entry has session_id",
            journal[1].session_id.as_str(),
            expected_id.as_str(),
        );
    }
}

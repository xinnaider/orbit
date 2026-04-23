use std::io::{BufRead, Write};

use interprocess::local_socket::prelude::*;
use interprocess::local_socket::{GenericFilePath, GenericNamespaced, Stream};
use interprocess::TryClone;
use serde_json::Value;

const SOCKET_NAME: &str = "orbit-mcp";
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);

fn parent_session_id() -> Option<i64> {
    // 1. Explicit env var (set by Orbit if propagated)
    if let Some(id) = std::env::var("ORBIT_SESSION_ID")
        .ok()
        .and_then(|s| s.parse().ok())
    {
        return Some(id);
    }
    // 2. PID file: Orbit writes {tmp}/orbit-session-{cli_pid}.id when spawning.
    //    Walk up the process tree to find a matching PID file.
    let ppid = get_parent_pid()?;
    try_pid_file(ppid).or_else(|| {
        // grandparent (Claude Code may spawn via wrapper)
        get_parent_pid_of(ppid).and_then(try_pid_file)
    })
}

fn try_pid_file(pid: u32) -> Option<i64> {
    let path = std::env::temp_dir().join(format!("orbit-session-{pid}.id"));
    std::fs::read_to_string(path)
        .ok()
        .and_then(|s| s.trim().parse().ok())
}

#[cfg(windows)]
fn get_parent_pid() -> Option<u32> {
    get_parent_pid_of(std::process::id())
}

#[cfg(windows)]
fn get_parent_pid_of(pid: u32) -> Option<u32> {
    use std::mem;
    use windows_sys::Win32::Foundation::CloseHandle;
    use windows_sys::Win32::System::Diagnostics::ToolHelp::*;
    unsafe {
        let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snap.is_null() || snap == usize::MAX as _ {
            return None;
        }
        let mut entry: PROCESSENTRY32W = mem::zeroed();
        entry.dwSize = mem::size_of::<PROCESSENTRY32W>() as u32;
        if Process32FirstW(snap, &mut entry) != 0 {
            loop {
                if entry.th32ProcessID == pid {
                    CloseHandle(snap);
                    return Some(entry.th32ParentProcessID);
                }
                if Process32NextW(snap, &mut entry) == 0 {
                    break;
                }
            }
        }
        CloseHandle(snap);
    }
    None
}

#[cfg(not(windows))]
fn get_parent_pid() -> Option<u32> {
    Some(unsafe { libc::getppid() } as u32)
}

#[cfg(not(windows))]
fn get_parent_pid_of(pid: u32) -> Option<u32> {
    // Read /proc/{pid}/status for PPid
    let status = std::fs::read_to_string(format!("/proc/{pid}/status")).ok()?;
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("PPid:") {
            return rest.trim().parse().ok();
        }
    }
    None
}

fn inject_orbit_context(line: &str, orbit_sid: Option<i64>) -> String {
    let parent = match orbit_sid {
        Some(id) => id,
        None => return line.to_string(),
    };
    let mut msg: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => return line.to_string(),
    };
    let method = msg
        .pointer("/method")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    match method {
        "initialize" => {
            if let Some(params) = msg.get_mut("params") {
                if let Some(obj) = params.as_object_mut() {
                    obj.insert(
                        "_orbitSessionId".into(),
                        Value::Number(parent.into()),
                    );
                }
            } else {
                msg.as_object_mut().map(|o| {
                    o.insert(
                        "params".into(),
                        serde_json::json!({ "_orbitSessionId": parent }),
                    )
                });
            }
        }
        "tools/call" => {
            let is_create = msg.pointer("/params/name")
                == Some(&Value::String("orbit_create_agent".into()));
            if is_create {
                if let Some(args) = msg.pointer_mut("/params/arguments") {
                    if args.get("parentSessionId").is_none() {
                        args.as_object_mut().map(|o| {
                            o.insert(
                                "parentSessionId".into(),
                                Value::Number(parent.into()),
                            )
                        });
                    }
                }
            }
        }
        _ => {}
    }
    serde_json::to_string(&msg).unwrap_or_else(|_| line.to_string())
}

fn socket_name() -> interprocess::local_socket::Name<'static> {
    if GenericNamespaced::is_supported() {
        SOCKET_NAME.to_ns_name::<GenericNamespaced>().unwrap()
    } else {
        let path = format!("/tmp/{SOCKET_NAME}.sock");
        let leaked: &'static str = Box::leak(path.into_boxed_str());
        leaked.to_fs_name::<GenericFilePath>().unwrap()
    }
}

pub fn run() {
    match try_connect() {
        Some(stream) => connected_mode(stream),
        None => {
            eprintln!("[orbit-mcp] Orbit app not running — MCP operating in standalone mode");
            standalone::run();
        }
    }
}

fn try_connect() -> Option<Stream> {
    let name = socket_name();
    let start = std::time::Instant::now();
    while start.elapsed() < CONNECT_TIMEOUT {
        match Stream::connect(name.clone()) {
            Ok(s) => return Some(s),
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(100)),
        }
    }
    None
}

fn connected_mode(stream: Stream) {
    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let orbit_sid = parent_session_id();

    let stream_for_read = stream.try_clone().unwrap_or_else(|e| {
        eprintln!("[orbit-mcp] failed to clone stream: {e}");
        std::process::exit(1);
    });

    // pipe → stdout
    let stdout_handle = std::thread::spawn(move || {
        let mut reader = std::io::BufReader::new(&stream_for_read);
        let mut stdout = stdout.lock();
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let _ = stdout.write_all(line.as_bytes());
                    let _ = stdout.flush();
                }
            }
        }
    });

    // stdin → pipe (inject orbit context: parentSessionId + _orbitSessionId)
    {
        let mut writer = &stream;
        let stdin = stdin.lock();
        let mut reader = std::io::BufReader::new(stdin);
        let mut line = String::new();
        loop {
            line.clear();
            match reader.read_line(&mut line) {
                Ok(0) | Err(_) => break,
                Ok(_) => {
                    let patched = inject_orbit_context(line.trim(), orbit_sid);
                    if writer.write_all(patched.as_bytes()).is_err() {
                        break;
                    }
                    if writer.write_all(b"\n").is_err() {
                        break;
                    }
                    if writer.flush().is_err() {
                        break;
                    }
                }
            }
        }
    }

    let _ = stdout_handle.join();
}

/// Standalone mode — preserves the original orbit-mcp behavior when Tauri is not running.
pub mod standalone {
    use std::collections::HashMap;
    use std::io::{BufRead, Write};
    use std::process::{Child, Command, Stdio};
    use std::sync::{Arc, Mutex};

    use serde_json::{json, Value};

    struct AgentProcess {
        child: Child,
        provider: String,
        model: String,
        cwd: String,
        output: Arc<Mutex<Vec<String>>>,
        finished: Arc<Mutex<bool>>,
    }

    struct McpState {
        agents: HashMap<u64, AgentProcess>,
        next_id: u64,
    }

    impl McpState {
        fn new() -> Self {
            Self {
                agents: HashMap::new(),
                next_id: 1,
            }
        }
    }

    fn extended_path() -> String {
        let current = std::env::var("PATH").unwrap_or_default();

        #[cfg(windows)]
        {
            let extra: Vec<String> = dirs::home_dir()
                .map(|h| {
                    vec![
                        h.join(".local").join("bin").to_string_lossy().into_owned(),
                        h.join("AppData")
                            .join("Roaming")
                            .join("npm")
                            .to_string_lossy()
                            .into_owned(),
                        h.join("AppData")
                            .join("Local")
                            .join("pnpm")
                            .to_string_lossy()
                            .into_owned(),
                    ]
                })
                .unwrap_or_default();
            format!("{};{}", extra.join(";"), current)
        }

        #[cfg(not(windows))]
        {
            let extra: Vec<String> = dirs::home_dir()
                .map(|h| {
                    vec![
                        h.join(".local").join("bin").to_string_lossy().into_owned(),
                        h.join(".nvm")
                            .join("current")
                            .join("bin")
                            .to_string_lossy()
                            .into_owned(),
                        "/usr/local/bin".to_string(),
                    ]
                })
                .unwrap_or_default();
            format!("{}:{}", extra.join(":"), current)
        }
    }

    fn find_cli(name: &str) -> Option<String> {
        let aug = extended_path();

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            let out = Command::new("where")
                .arg(name)
                .env("PATH", &aug)
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .ok()?;
            if !out.status.success() {
                return None;
            }
            let stdout = String::from_utf8_lossy(&out.stdout);
            let lines: Vec<&str> = stdout
                .lines()
                .map(|l| l.trim())
                .filter(|l| !l.is_empty())
                .collect();
            if let Some(win) = lines.iter().find(|l| {
                let lower = l.to_lowercase();
                lower.ends_with(".cmd") || lower.ends_with(".exe")
            }) {
                return Some(win.to_string());
            }
            lines.first().map(|l| l.to_string())
        }

        #[cfg(not(windows))]
        {
            let out = Command::new("which")
                .arg(name)
                .env("PATH", &aug)
                .output()
                .ok()?;
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        }
    }

    fn provider_to_cli(provider: &str) -> &str {
        match provider {
            "claude-code" | "claude" => "claude",
            "codex" => "codex",
            "opencode" => "opencode",
            "gemini-cli" | "gemini" => "gemini",
            "copilot-cli" | "copilot" => "copilot",
            other => other,
        }
    }

    fn build_spawn_args(provider: &str, model: &str, prompt: &str) -> Vec<String> {
        match provider {
            "claude-code" | "claude" => {
                let mut args = vec![
                    "--output-format".to_string(),
                    "stream-json".to_string(),
                    "--verbose".to_string(),
                    "--dangerously-skip-permissions".to_string(),
                ];
                if !model.is_empty() && model != "auto" {
                    args.extend(["--model".to_string(), model.to_string()]);
                }
                args.extend(["-p".to_string(), prompt.to_string()]);
                args
            }
            "codex" => {
                let mut args = vec![
                    "exec".to_string(),
                    "--json".to_string(),
                    "--dangerously-bypass-approvals-and-sandbox".to_string(),
                ];
                if !model.is_empty() {
                    args.extend(["-m".to_string(), model.to_string()]);
                }
                args.push(prompt.to_string());
                args
            }
            "opencode" => {
                let mut args = vec![
                    "run".to_string(),
                    "--format".to_string(),
                    "json".to_string(),
                ];
                if !model.is_empty() {
                    args.extend(["--model".to_string(), model.to_string()]);
                }
                args.push(prompt.to_string());
                args
            }
            _ => vec!["--acp".to_string()],
        }
    }

    fn tool_create_agent(state: &mut McpState, params: &Value) -> Result<Value, String> {
        let provider = params
            .get("provider")
            .and_then(|v| v.as_str())
            .unwrap_or("claude-code");
        let model = params.get("model").and_then(|v| v.as_str()).unwrap_or("");
        let cwd = params
            .get("cwd")
            .and_then(|v| v.as_str())
            .ok_or("missing 'cwd' parameter")?;
        let prompt = params
            .get("prompt")
            .and_then(|v| v.as_str())
            .ok_or("missing 'prompt' parameter")?;
        let wait = params.get("wait").and_then(|v| v.as_bool()).unwrap_or(true);
        let timeout_secs = params
            .get("timeoutSecs")
            .and_then(|v| v.as_u64())
            .unwrap_or(300);

        let cli_name = provider_to_cli(provider);
        let cli_path = find_cli(cli_name)
            .ok_or_else(|| format!("{cli_name} not found in PATH. Install it first."))?;

        let args = build_spawn_args(provider, model, prompt);

        let use_stdin = provider == "opencode" && (cfg!(windows) || prompt.contains('\n'));

        let mut cmd = Command::new(&cli_path);
        if use_stdin {
            let mut stdin_args = args.clone();
            stdin_args.pop();
            stdin_args.push("-".to_string());
            cmd.args(&stdin_args);
            cmd.stdin(Stdio::piped());
        } else {
            cmd.args(&args);
        }
        cmd.current_dir(cwd);
        cmd.env("PATH", extended_path());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut child = cmd.spawn().map_err(|e| format!("spawn failed: {e}"))?;

        if use_stdin {
            let mut stdin_pipe = child.stdin.take().ok_or("no stdin")?;
            stdin_pipe
                .write_all(prompt.as_bytes())
                .map_err(|e| format!("write prompt to stdin: {e}"))?;
            drop(stdin_pipe);
        }

        let pid = child.id();
        let session_id = state.next_id;
        state.next_id += 1;

        let output = Arc::new(Mutex::new(Vec::new()));
        let finished = Arc::new(Mutex::new(false));

        let stdout = child.stdout.take().ok_or("no stdout")?;
        let output_clone = Arc::clone(&output);
        let finished_clone = Arc::clone(&finished);
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        let trimmed = l.trim().to_string();
                        if !trimmed.is_empty() {
                            output_clone.lock().unwrap().push(trimmed);
                        }
                    }
                    Err(_) => break,
                }
            }
            *finished_clone.lock().unwrap() = true;
        });

        state.agents.insert(
            session_id,
            AgentProcess {
                child,
                provider: provider.to_string(),
                model: model.to_string(),
                cwd: cwd.to_string(),
                output: Arc::clone(&output),
                finished: Arc::clone(&finished),
            },
        );

        if wait {
            let deadline =
                std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
            loop {
                if *finished.lock().unwrap() {
                    break;
                }
                if std::time::Instant::now() > deadline {
                    return Ok(json!({
                        "sessionId": session_id,
                        "pid": pid,
                        "status": "timeout",
                        "output": extract_assistant_text(&output.lock().unwrap()),
                    }));
                }
                std::thread::sleep(std::time::Duration::from_millis(200));
            }

            let text = extract_assistant_text(&output.lock().unwrap());
            if let Some(mut agent) = state.agents.remove(&session_id) {
                let _ = agent.child.wait();
            }
            Ok(json!({
                "sessionId": session_id,
                "pid": pid,
                "status": "completed",
                "output": text,
            }))
        } else {
            Ok(json!({
                "sessionId": session_id,
                "pid": pid,
                "status": "running",
            }))
        }
    }

    fn tool_get_status(state: &McpState, params: &Value) -> Result<Value, String> {
        let session_id = params
            .get("sessionId")
            .and_then(|v| v.as_u64())
            .ok_or("missing 'sessionId' parameter")?;

        match state.agents.get(&session_id) {
            Some(agent) => {
                let done = *agent.finished.lock().unwrap();
                let lines = agent.output.lock().unwrap();
                let text = extract_assistant_text(&lines);
                Ok(json!({
                    "sessionId": session_id,
                    "provider": agent.provider,
                    "model": agent.model,
                    "status": if done { "completed" } else { "running" },
                    "output": text,
                    "lineCount": lines.len(),
                }))
            }
            None => Ok(json!({
                "sessionId": session_id,
                "status": "not_found",
            })),
        }
    }

    fn tool_send_message(state: &mut McpState, params: &Value) -> Result<Value, String> {
        let session_id = params
            .get("sessionId")
            .and_then(|v| v.as_u64())
            .ok_or("missing 'sessionId' parameter")?;
        let message = params
            .get("message")
            .and_then(|v| v.as_str())
            .ok_or("missing 'message' parameter")?;

        let old_agent = state.agents.remove(&session_id).ok_or("agent not found")?;

        let lines = old_agent.output.lock().unwrap();
        let resume_id = extract_session_id(&lines);
        drop(lines);

        let mut child = old_agent.child;
        let _ = child.kill();
        let _ = child.wait();

        let cli_name = provider_to_cli(&old_agent.provider);
        let cli_path =
            find_cli(cli_name).ok_or_else(|| format!("{cli_name} not found in PATH"))?;

        let mut args = vec![];
        match old_agent.provider.as_str() {
            "claude-code" | "claude" => {
                args.extend([
                    "--output-format".to_string(),
                    "stream-json".to_string(),
                    "--verbose".to_string(),
                    "--dangerously-skip-permissions".to_string(),
                ]);
                if let Some(ref rid) = resume_id {
                    args.extend(["--resume".to_string(), rid.clone()]);
                }
                args.extend(["-p".to_string(), message.to_string()]);
            }
            _ => {
                return Err(format!(
                    "send_message not yet supported for provider '{}'",
                    old_agent.provider
                ));
            }
        }

        let mut cmd = Command::new(&cli_path);
        cmd.args(&args);
        cmd.current_dir(&old_agent.cwd);
        cmd.env("PATH", extended_path());
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let mut new_child = cmd.spawn().map_err(|e| format!("respawn failed: {e}"))?;
        let pid = new_child.id();

        let output = Arc::new(Mutex::new(Vec::new()));
        let finished = Arc::new(Mutex::new(false));

        let stdout = new_child.stdout.take().ok_or("no stdout")?;
        let output_clone = Arc::clone(&output);
        let finished_clone = Arc::clone(&finished);
        std::thread::spawn(move || {
            let reader = std::io::BufReader::new(stdout);
            for line in reader.lines() {
                match line {
                    Ok(l) => {
                        let trimmed = l.trim().to_string();
                        if !trimmed.is_empty() {
                            output_clone.lock().unwrap().push(trimmed);
                        }
                    }
                    Err(_) => break,
                }
            }
            *finished_clone.lock().unwrap() = true;
        });

        state.agents.insert(
            session_id,
            AgentProcess {
                child: new_child,
                provider: old_agent.provider,
                model: old_agent.model,
                cwd: old_agent.cwd,
                output,
                finished,
            },
        );

        Ok(json!({
            "sessionId": session_id,
            "pid": pid,
            "status": "running",
        }))
    }

    fn tool_cancel_agent(state: &mut McpState, params: &Value) -> Result<Value, String> {
        let session_id = params
            .get("sessionId")
            .and_then(|v| v.as_u64())
            .ok_or("missing 'sessionId' parameter")?;

        if let Some(mut agent) = state.agents.remove(&session_id) {
            let _ = agent.child.kill();
            let _ = agent.child.wait();
            Ok(json!({ "sessionId": session_id, "status": "cancelled" }))
        } else {
            Ok(json!({ "sessionId": session_id, "status": "not_found" }))
        }
    }

    fn tool_list_providers() -> Result<Value, String> {
        Ok(json!([
            { "id": "claude-code", "name": "Claude Code", "cliName": "claude" },
            { "id": "codex", "name": "Codex CLI", "cliName": "codex" },
            { "id": "opencode", "name": "OpenCode", "cliName": "opencode" },
            { "id": "gemini-cli", "name": "Gemini CLI", "cliName": "gemini" },
            { "id": "copilot-cli", "name": "Copilot CLI", "cliName": "copilot" },
        ]))
    }

    fn tool_list_sessions(state: &McpState) -> Result<Value, String> {
        let sessions: Vec<Value> = state
            .agents
            .iter()
            .map(|(id, agent)| {
                let done = *agent.finished.lock().unwrap();
                json!({
                    "sessionId": id,
                    "provider": agent.provider,
                    "model": agent.model,
                    "cwd": agent.cwd,
                    "status": if done { "completed" } else { "running" },
                })
            })
            .collect();
        Ok(json!(sessions))
    }

    fn tool_get_subagents() -> Result<Value, String> {
        Ok(json!([]))
    }

    fn extract_assistant_text(lines: &[String]) -> String {
        let mut texts = Vec::new();
        for line in lines {
            if let Ok(val) = serde_json::from_str::<Value>(line) {
                if val.get("type").and_then(|t| t.as_str()) == Some("assistant") {
                    if let Some(content) = val.pointer("/message/content") {
                        if let Some(arr) = content.as_array() {
                            for block in arr {
                                if let Some(text) = block.get("text").and_then(|t| t.as_str()) {
                                    texts.push(text.to_string());
                                }
                            }
                        }
                    }
                }
                if val.pointer("/item/type").and_then(|t| t.as_str()) == Some("agent_message") {
                    if let Some(text) = val.pointer("/item/text").and_then(|t| t.as_str()) {
                        texts.push(text.to_string());
                    }
                }
                if val.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(text) = val.pointer("/part/text").and_then(|t| t.as_str()) {
                        texts.push(text.to_string());
                    }
                }
                if val.get("type").and_then(|t| t.as_str()) == Some("error") {
                    let msg = val
                        .pointer("/error/data/message")
                        .or_else(|| val.pointer("/error/message"))
                        .or_else(|| val.pointer("/error/name"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("Unknown error");
                    texts.push(format!("Error: {msg}"));
                }
            }
        }
        texts.join("\n")
    }

    fn extract_session_id(lines: &[String]) -> Option<String> {
        for line in lines {
            if let Ok(val) = serde_json::from_str::<Value>(line) {
                if val.get("type").and_then(|t| t.as_str()) == Some("system") {
                    if let Some(sid) = val.get("session_id").and_then(|s| s.as_str()) {
                        return Some(sid.to_string());
                    }
                }
            }
        }
        None
    }

    fn tools_schema() -> Value {
        json!([
            {
                "name": "orbit_create_agent",
                "description": "Create and spawn a new AI coding agent. By default waits for completion and returns the output.",
                "inputSchema": {
                    "type": "object",
                    "properties": {
                        "provider": { "type": "string", "description": "Agent provider: claude-code, codex, opencode, gemini-cli, copilot-cli", "default": "claude-code" },
                        "model": { "type": "string", "description": "Model ID (e.g. claude-sonnet-4-6). Empty for default." },
                        "cwd": { "type": "string", "description": "Working directory for the agent" },
                        "prompt": { "type": "string", "description": "Initial prompt/task for the agent" },
                        "wait": { "type": "boolean", "description": "Wait for agent to complete. Default: true", "default": true },
                        "timeoutSecs": { "type": "integer", "description": "Max seconds to wait. Default: 300", "default": 300 }
                    },
                    "required": ["cwd", "prompt"]
                }
            },
            {
                "name": "orbit_get_status",
                "description": "Check the status of a running agent.",
                "inputSchema": { "type": "object", "properties": { "sessionId": { "type": "integer", "description": "Session ID" } }, "required": ["sessionId"] }
            },
            {
                "name": "orbit_send_message",
                "description": "Send a follow-up message to an existing agent session.",
                "inputSchema": { "type": "object", "properties": { "sessionId": { "type": "integer" }, "message": { "type": "string" } }, "required": ["sessionId", "message"] }
            },
            {
                "name": "orbit_cancel_agent",
                "description": "Cancel/kill a running agent.",
                "inputSchema": { "type": "object", "properties": { "sessionId": { "type": "integer" } }, "required": ["sessionId"] }
            },
            {
                "name": "orbit_list_providers",
                "description": "List available CLI providers (standalone mode — limited info).",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "orbit_list_sessions",
                "description": "List active sessions (standalone mode — in-memory only).",
                "inputSchema": { "type": "object", "properties": {} }
            },
            {
                "name": "orbit_get_subagents",
                "description": "Get subagents for a session (not available in standalone mode).",
                "inputSchema": { "type": "object", "properties": { "sessionId": { "type": "integer" } }, "required": ["sessionId"] }
            }
        ])
    }

    pub fn run() {
        let stdin = std::io::stdin();
        let stdout = std::io::stdout();
        let mut state = McpState::new();

        for line in stdin.lock().lines() {
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };

            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }

            let patched = super::inject_orbit_context(trimmed, super::parent_session_id());
            let msg: Value = match serde_json::from_str(&patched) {
                Ok(v) => v,
                Err(e) => {
                    eprintln!("[orbit-mcp] invalid JSON: {e}");
                    continue;
                }
            };

            let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
            let id = msg.get("id").cloned();

            if id.is_none() {
                continue;
            }

            let id = id.unwrap();
            let params = msg.get("params").cloned().unwrap_or(json!({}));

            let response = match method {
                "initialize" => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "protocolVersion": "2024-11-05",
                        "capabilities": { "tools": {} },
                        "serverInfo": {
                            "name": "orbit-mcp",
                            "version": env!("CARGO_PKG_VERSION")
                        }
                    }
                }),
                "tools/list" => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": { "tools": tools_schema() }
                }),
                "tools/call" => {
                    let tool_name = params.get("name").and_then(|n| n.as_str()).unwrap_or("");
                    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

                    let result = match tool_name {
                        "orbit_create_agent" => tool_create_agent(&mut state, &arguments),
                        "orbit_get_status" => tool_get_status(&state, &arguments),
                        "orbit_send_message" => tool_send_message(&mut state, &arguments),
                        "orbit_cancel_agent" => tool_cancel_agent(&mut state, &arguments),
                        "orbit_list_providers" => tool_list_providers(),
                        "orbit_list_sessions" => tool_list_sessions(&state),
                        "orbit_get_subagents" => tool_get_subagents(),
                        _ => Err(format!("unknown tool: {tool_name}")),
                    };

                    match result {
                        Ok(content) => json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [{
                                    "type": "text",
                                    "text": serde_json::to_string_pretty(&content).unwrap_or_default()
                                }]
                            }
                        }),
                        Err(e) => json!({
                            "jsonrpc": "2.0",
                            "id": id,
                            "result": {
                                "content": [{ "type": "text", "text": format!("Error: {e}") }],
                                "isError": true
                            }
                        }),
                    }
                }
                "ping" => json!({ "jsonrpc": "2.0", "id": id, "result": {} }),
                _ => json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "error": {
                        "code": -32601,
                        "message": format!("method not found: {method}")
                    }
                }),
            };

            let out = serde_json::to_string(&response).unwrap_or_default();
            let mut stdout = stdout.lock();
            let _ = writeln!(stdout, "{out}");
            let _ = stdout.flush();
        }

        for (_, mut agent) in state.agents.drain() {
            let _ = agent.child.kill();
            let _ = agent.child.wait();
        }
    }
}

# MCP Integration — Embedded in Tauri with Dashboard Visibility

## Objective

Agents spawned via MCP (from Claude Code, OpenCode, etc.) must appear in the Orbit dashboard in real time, with full subagent tracking, token counting, and model/provider visibility. The MCP server also exposes available providers and models so callers can make informed choices.

## Current State

`orbit-mcp` is a standalone Rust binary (`tauri/src/bin/orbit_mcp.rs`) that communicates via stdio JSON-RPC 2.0. It has:

- 4 tools: `orbit_create_agent`, `orbit_get_status`, `orbit_send_message`, `orbit_cancel_agent`
- In-memory state only — no DB, no shared state with the Tauri app
- Duplicated logic: CLI resolution (`find_cli`, `extended_path`), provider-to-CLI mapping (`provider_to_cli`), spawn args (`build_spawn_args`), output parsing (`extract_assistant_text`)
- Sessions created via MCP are invisible to the dashboard
- No visibility into subagents, providers, or models

## Architecture

### Embedded MCP Handler + Thin Proxy

The MCP protocol handler runs as a **thread inside the Tauri process**, sharing `Arc<Mutex<SessionManager>>`, `DatabaseService`, and `ProviderRegistry` directly. The `orbit-mcp` binary becomes a **thin stdio proxy** that bridges to the Tauri app via IPC (named pipe on Windows, Unix socket on Linux/macOS).

```
Claude/Opencode/any MCP client
         │
    stdio (JSON-RPC 2.0)
         │
    orbit-mcp (thin proxy)
         │
    named pipe (Win) / Unix socket (Linux/Mac)
         │
    Tauri app (MCP handler thread)
         │
    Arc<Mutex<SessionManager>>, DatabaseService, ProviderRegistry
```

**Key invariant**: If the Tauri app is running, the proxy connects to it and all tools operate on shared state. If the Tauri app is not running, the proxy falls back to the current standalone behavior (no dashboard integration).

### Transport Layer

| Platform | Method | Address |
|----------|--------|---------|
| Windows | Named pipe | `\\.\pipe\orbit-mcp` |
| Linux | Unix socket | `/tmp/orbit-mcp.sock` |
| macOS | Unix socket | `/tmp/orbit-mcp.sock` |

The pipe/socket is created by the Tauri app on startup. The proxy attempts to connect with a timeout (2s). On failure, it logs a warning and runs standalone.

### Protocol

The proxy forwards raw JSON-RPC 2.0 messages over the pipe. The Tauri-side handler parses and dispatches to the appropriate tool implementation. Responses are sent back over the same pipe.

Message framing: each JSON-RPC message is one line (newline-delimited), matching the stdio transport MCP already uses. This keeps the proxy trivially simple — it's just a byte pipe between stdio and the socket.

### Data Flow: Creating an Agent via MCP (Connected Mode)

```
MCP client → orbit_create_agent({ provider, model, cwd, prompt })
  ↓
Proxy forwards to Tauri via pipe
  ↓
MCP handler thread receives, calls session_manager.init_session()
  → Creates session record in DB
  → Emits session:created event → appears in dashboard sidebar
  → Background thread calls do_spawn() via Provider trait
  → Emits session:running with PID
  → Reader loop: journal entries, tokens, status, subagent events
  → Emits session:output, session:state, session:subagent-created
  ↓
MCP handler returns { sessionId, status: "running" }
  (or blocks until completed if wait=true)
```

### Data Flow: Fallback (Standalone Mode)

When the Tauri app is not running, the proxy detects connection failure and falls back to the existing standalone logic. No DB, no dashboard integration, in-memory state only. This path is unchanged from the current implementation.

## Tools (7 total)

### Existing (4) — Updated

All four existing tools are reimplemented to use shared state when connected.

#### `orbit_create_agent`

Creates a session in the database and spawns a CLI agent. In connected mode, uses `SessionManager::init_session()` + `Provider::spawn()`. In standalone mode, uses current logic.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `provider` | string | no | `"claude-code"` | Provider ID (from `orbit_list_providers`) |
| `model` | string | no | provider default | Model ID (e.g. `"claude-sonnet-4-20250514"`) |
| `cwd` | string | **yes** | — | Working directory |
| `prompt` | string | **yes** | — | Initial prompt |
| `wait` | boolean | no | `true` | Block until agent completes |
| `timeoutSecs` | number | no | `300` | Max wait time (seconds) |

**Connected mode response:**
```json
{
  "sessionId": 42,
  "status": "running",
  "pid": 12345
}
```

If `wait=true`, blocks and returns when session reaches `Completed`/`Stopped`/`Error`:
```json
{
  "sessionId": 42,
  "status": "completed",
  "tokens": { "input": 15000, "output": 3200 },
  "contextPercent": 45.2,
  "subagents": [
    { "id": "sub_abc", "agentType": "Task", "description": "Fix lint errors", "status": "done" }
  ]
}
```

#### `orbit_get_status`

Returns session state. In connected mode, reads from `SessionManager` + DB. Includes subagents.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `sessionId` | number | **yes** | Session ID returned by `orbit_create_agent` |

**Response:**
```json
{
  "sessionId": 42,
  "status": "running",
  "model": "claude-sonnet-4-20250514",
  "pid": 12345,
  "tokens": { "input": 12000, "output": 2800, "cacheRead": 5000, "cacheWrite": 2000 },
  "contextPercent": 38.5,
  "pendingApproval": false,
  "miniLog": [
    { "tool": "Edit", "target": "src/main.rs", "result": "success", "success": true }
  ],
  "subagents": [
    { "id": "sub_abc", "agentType": "Task", "description": "Refactor parser", "status": "running" }
  ]
}
```

#### `orbit_send_message`

Sends a follow-up message. In connected mode, uses `SessionManager` resume flow (provider-aware, dispatches to `Provider::spawn()` with resume config). In standalone mode, only works for Claude Code (hardcoded `--resume` flag, same as current behavior).

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `sessionId` | number | **yes** | Session ID |
| `message` | string | **yes** | Follow-up prompt |

**Response:**
```json
{
  "sessionId": 42,
  "status": "running"
}
```

#### `orbit_cancel_agent`

Kills the session process. In connected mode, uses `SessionManager` stop flow.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `sessionId` | number | **yes** | Session ID |

**Response:**
```json
{
  "sessionId": 42,
  "status": "stopped"
}
```

### New (3)

#### `orbit_list_providers`

Returns all providers with their models, capabilities, and sub-providers. Reads from `ProviderRegistry`.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| _(none)_ | | | |

**Response:**
```json
[
  {
    "id": "claude-code",
    "name": "Claude Code",
    "cliName": "claude",
    "cliAvailable": true,
    "installHint": "npm install -g @anthropic-ai/claude-code",
    "capabilities": {
      "effort": true,
      "ssh": true,
      "subagents": true,
      "tasks": true
    },
    "effortLevels": {
      "default": ["low", "medium", "high", "max"],
      "claude-opus-4-7-20250415": ["low", "medium", "high", "max", "xhigh", "auto"]
    },
    "models": [
      { "id": "auto", "name": "Auto", "contextWindow": null },
      { "id": "claude-opus-4-7-20250415", "name": "Opus 4.7", "contextWindow": 1000000 },
      { "id": "claude-sonnet-4-20250514", "name": "Sonnet 4.6", "contextWindow": 200000 }
    ],
    "subProviders": []
  },
  {
    "id": "opencode",
    "name": "OpenCode",
    "cliName": "opencode",
    "cliAvailable": true,
    "installHint": "go install github.com/opencode-ai/opencode@latest",
    "capabilities": { "effort": false, "ssh": false, "subagents": true, "tasks": false },
    "effortLevels": {},
    "models": [],
    "subProviders": [
      {
        "id": "opencode/anthropic",
        "name": "Anthropic",
        "configured": true,
        "models": [
          { "id": "claude-sonnet-4-20250514", "name": "Sonnet 4.6", "contextWindow": 200000 }
        ]
      }
    ]
  }
]
```

#### `orbit_list_sessions`

Returns sessions visible in the dashboard. Reads from DB.

| Parameter | Type | Required | Default | Description |
|-----------|------|----------|---------|-------------|
| `status` | string | no | _(all)_ | Filter by status: `running`, `completed`, `error`, etc. |

**Response:**
```json
[
  {
    "sessionId": 42,
    "name": "Fix parser bug",
    "provider": "claude-code",
    "model": "claude-sonnet-4-20250514",
    "status": "running",
    "cwd": "/home/user/project",
    "tokens": { "input": 12000, "output": 2800 },
    "contextPercent": 38.5,
    "createdAt": "2026-04-22T14:30:00Z",
    "parentId": null
  }
]
```

#### `orbit_get_subagents`

Returns subagents of a given session. Uses `agent_tree::read_subagents()`.

| Parameter | Type | Required | Description |
|-----------|------|----------|-------------|
| `sessionId` | number | **yes** | Parent session ID |

**Response:**
```json
[
  {
    "id": "sub_abc123",
    "agentType": "Task",
    "description": "Refactor the parser module",
    "status": "running"
  },
  {
    "id": "sub_def456",
    "agentType": "Agent",
    "description": "Write unit tests",
    "status": "done"
  }
]
```

## Dashboard Integration Details

### Session Visibility

Sessions created via MCP are inserted into the same `sessions` table with all the same fields. The `session:created` Tauri event fires, so the sidebar updates in real time. The journal feed, token counter, and context bar all work identically.

### Subagent Tracking

When a session spawned via MCP invokes a subagent tool (e.g. `Task` for Claude), the `session:subagent-created` event fires. The `orbit_get_status` and `orbit_get_subagents` tools expose this data to the MCP caller.

In the dashboard, subagents appear in the agents tab (right panel) as they do today for dashboard-created sessions.

### Token and Context Tracking

Handled by the existing reader loop in `SessionManager`. No changes needed — MCP-created sessions participate in the same `session:state` event flow.

## Fallback: Standalone Mode

When the proxy cannot connect to the Tauri app (pipe/socket doesn't exist or connection times out after 2 seconds):

1. Log a warning: `"Orbit app not running — MCP operating in standalone mode"`
2. Use the **existing** logic in `orbit_mcp.rs` (spawn, in-memory state, output parsing)
3. Sessions created in standalone mode do not appear in the dashboard (even retroactively)
4. All 7 tools are still available, but `orbit_list_providers` returns only hardcoded data for known providers, and `orbit_list_sessions` returns only in-memory sessions from the current proxy process

## Implementation Modules

### New Rust modules

| File | Responsibility |
|------|---------------|
| `tauri/src/ipc/mcp.rs` | MCP JSON-RPC handler thread inside Tauri. Dispatches tool calls to SessionManager/DB/Registry. Manages pipe/socket lifecycle. |
| `tauri/src/mcp_transport.rs` | Named pipe (Windows) / Unix socket (Linux/macOS) server. Accepts connections, reads lines, forwards to handler. |
| `tauri/src/mcp_proxy.rs` | Thin stdio proxy. Connects to pipe/socket or falls back to standalone. Replaces `bin/orbit_mcp.rs` entry point. |

### Modified Rust modules

| File | Changes |
|------|---------|
| `tauri/src/lib.rs` | Start MCP transport server on app init. Register `ipc/mcp` module. |
| `tauri/src/commands/providers.rs` | Add `list_providers_mcp()` returning the format for `orbit_list_providers` (or reuse `get_providers`). |
| `tauri/src/commands/agents.rs` | Ensure `get_subagents` works with session IDs from MCP-created sessions. |
| `tauri/src/bin/orbit_mcp.rs` | Replace current logic with proxy that connects to Tauri pipe or falls back to standalone. |

### Frontend changes

None. The dashboard already handles `session:created`, `session:output`, `session:state`, and `session:subagent-created` events. MCP-created sessions are indistinguishable from dashboard-created ones.

## Edge Cases

| Case | Behavior |
|------|----------|
| Orbit app closes while MCP session is running | Pipe breaks. Proxy detects, prints warning. Session continues running in the OS — Orbit picks it up on restart via DB record. |
| Multiple MCP clients connect simultaneously | Pipe accepts multiple connections. Each client gets independent request/response. Thread safety via `Arc<Mutex<...>>` on shared state. |
| `orbit_list_sessions` called in standalone mode | Returns only sessions created in the current proxy process (in-memory). |
| `orbit_get_subagents` on a session with no subagents | Returns empty array `[]`. |
| `orbit_create_agent` with invalid provider ID | Returns error: `"Provider 'xyz' not found. Available: claude-code, codex, opencode, ..."` |
| `orbit_get_status` on non-existent session ID | Returns error: `"Session 999 not found"` |
| SSH session via MCP | Supported in connected mode (SessionManager handles SSH). Not supported in standalone mode. |

## Acceptance Criteria

1. Agent created via `orbit_create_agent` appears in the Orbit sidebar immediately
2. Journal entries stream to the dashboard Feed in real time
3. Tokens, context%, and status update live in the dashboard
4. Subagents spawned by the MCP agent are listed in `orbit_get_subagents` and appear in the dashboard agents tab
5. `orbit_list_providers` returns all providers with models, capabilities, and effort levels
6. `orbit_list_sessions` returns all sessions from the DB
7. When Orbit is not running, `orbit-mcp` falls back to standalone mode without crashing
8. All 7 tools work in both connected and standalone modes
9. Pre-commit checks pass (clippy, eslint, svelte-check, prettier, rustfmt)
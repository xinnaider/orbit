# Plano de Implementação — MCP Integration (Embedded in Tauri)

## Contexto

O `orbit-mcp` atual (760 linhas) é um binário standalone que spawna CLIs na mão, mantém estado em memória, e não tem integração nenhuma com o dashboard. Sessões criadas por MCP são invisíveis.

O objetivo é mover o handler MCP para dentro do processo Tauri, compartilhando `SessionManager`, `DatabaseService` e `ProviderRegistry` — sessões criadas via MCP aparecem no dashboard em tempo real, com journal, tokens, subagents, diffs. O binário `orbit-mcp` vira um proxy fino que conecta ao Tauri via IPC.

**Todos os providers** (claude-code, codex, opencode, gemini-cli, copilot-cli) funcionam via `Provider` trait — o handler MCP não faz nenhum `match` em provider ID.

**Todos os OS** (Windows, Linux, macOS) — transport usa named pipe (Windows) ou Unix socket (Linux/Mac).

**Zero mudanças no frontend** — sessões MCP emitem os mesmos eventos Tauri que sessões criadas pelo dashboard.

---

## Fase 1 — Dependência cross-platform para IPC (`Cargo.toml`)

**Arquivo:** `tauri/Cargo.toml`

Adicionar a crate `interprocess`:

```toml
interprocess = { version = "2", features = ["tokio"] }
```

**Por quê `interprocess`:** a stdlib do Rust não tem API de named pipe **server** no Windows (`std::os::windows::io` só tem handles, não `CreateNamedPipe`). `interprocess` fornece `LocalSocketListener` unificado — named pipe no Windows, Unix socket no Linux/Mac — com a mesma API. Zero `cfg(target_os)` no código do transport.

**Alternativas descartadas:**
- `tokio::net::windows::named_pipe` — exigiria `#[cfg(windows)]` em todo lugar + `UnixListener` separado para Linux/Mac
- `windows-sys` direto — muito baixo nível, reimplementaria o que `interprocess` já faz

---

## Fase 2 — Transport Layer (`mcp_transport.rs`)

**Arquivo:** `tauri/src/mcp_transport.rs` (novo)

**Responsabilidade:** Servidor IPC que aceita conexões e roteia mensagens JSON-RPC para o handler.

### API pública

```rust
pub struct McpTransport {
    stop: Arc<AtomicBool>,
}

impl McpTransport {
    /// Inicia o servidor em background thread. Retorna handle para shutdown.
    pub fn start(handler: Arc<dyn Fn(&str) -> String + Send + Sync>) -> Self;
    
    /// Sinaliza shutdown graceful.
    pub fn stop(&self);
}
```

### Detalhes

1. **Endereço:** constante cross-platform:
   - Windows: `\\.\pipe\orbit-mcp`
   - Unix: `/tmp/orbit-mcp.sock`
   - Usar `interprocess::local_socket::ListenerOptions` — resolve o endereço correto por OS

2. **Cleanup no startup (Unix only):** remover stale `/tmp/orbit-mcp.sock` se existir — evita `EADDRINUSE` se o Tauri crashou antes

3. **Multi-client:** cada conexão aceita em sua própria `std::thread::spawn` (consistente com o resto do Tauri que usa threads, não async)

4. **Framing:** newline-delimited JSON — `BufReader::read_line()` para ler, `write_all(response + "\n")` para responder. Igual ao stdio MCP.

5. **Shutdown:** `Arc<AtomicBool>` checked no accept loop + no read loop de cada conexão. Quando o Tauri fecha, sinaliza stop.

6. **Erros de conexão:** logar no stderr e continuar (uma conexão ruim não derruba o servidor)

### Testes

- Teste que cria listener, conecta, envia JSON line, recebe response
- Teste de múltiplas conexões simultâneas

---

## Fase 3 — MCP Handler (`ipc/mcp.rs`)

**Arquivo:** `tauri/src/ipc/mcp.rs` (novo)

**Responsabilidade:** Parseia JSON-RPC 2.0, despacha para as 7 tools, retorna responses.

### Struct

```rust
pub struct McpHandler {
    session_manager: Arc<RwLock<SessionManager>>,
    registry: Arc<ProviderRegistry>,
    app: AppHandle,
}
```

### Protocolo MCP

O handler implementa o subconjunto do MCP que um server precisa:

| Method | Ação |
|--------|------|
| `initialize` | Retorna `serverInfo` + `capabilities: { tools: {} }` |
| `notifications/initialized` | No-op (ack do client) |
| `tools/list` | Retorna schema das 7 tools |
| `tools/call` | Despacha por `name` para o handler da tool |

### 7 Tools — Todas provider-agnostic

**Princípio:** nenhum handler faz `if provider == "claude-code"`. Tudo passa pelo `Provider` trait e `ProviderRegistry::resolve()`.

#### `orbit_create_agent`

```
Params: { provider?, model?, cwd, prompt, wait?, timeoutSecs? }
```

1. `registry.resolve(provider)` → se `None`, retorna erro com lista de providers disponíveis via `registry.all()`
2. `session_manager.write().init_session(cwd, ...)` → cria no DB
3. `app.emit("session:created", &session)` — dashboard atualiza
4. `std::thread::spawn` → `SessionManager::do_spawn(manager, app, session_id, prompt, registry)`
5. Se `wait=true` (default): loop poll com `thread::sleep(500ms)`:
   - Lê status do `session_manager.read().active[session_id]`
   - Terminal states: `Completed | Stopped | Error` → retorna com tokens, contextPercent, subagents
   - Timeout (default 300s) → retorna status atual com `"timedOut": true`
6. Se `wait=false`: retorna `{ sessionId, status: "running" }` imediatamente

**Multi-provider:** funciona com qualquer provider no registry. O `do_spawn` já resolve via `registry.resolve()` e chama `provider.spawn()`.

#### `orbit_get_status`

```
Params: { sessionId }
```

1. Busca no `session_manager.active` (in-memory) para dados live
2. Fallback: `db.get_session(id)` para sessões que não estão no active map
3. `journal_states[id]` para tokens, context%, miniLog
4. `agent_tree::read_subagents()` para subagents

#### `orbit_send_message`

```
Params: { sessionId, message }
```

1. `SessionManager::send_message(manager, app, session_id, text, registry)`
2. Internamente faz `--resume` via provider (cada provider implementa resume no `spawn()` com `resume_id`)
3. Funciona com qualquer provider que suporte resume (Claude Code, ACP providers com stdin)

#### `orbit_cancel_agent`

```
Params: { sessionId }
```

1. `session_manager.write().stop_session(session_id)`
2. Mata PID, remove do active map, atualiza DB
3. `app.emit("session:stopped", ...)`

#### `orbit_list_providers`

```
Params: (nenhum)
```

1. Reutiliza a lógica de `commands/providers.rs` — extrair `build_cli_backends(registry)` como função pública
2. Retorna todos os providers com capabilities, models, effort levels, sub-providers
3. `find_cli()` reporta se o CLI está instalado

#### `orbit_list_sessions`

```
Params: { status? }
```

1. `db.get_sessions()` com filtro opcional por status
2. Enriquece com dados live do active map (tokens, context%, status atualizado)

#### `orbit_get_subagents`

```
Params: { sessionId }
```

1. Busca `claude_session_id` do active map ou DB
2. `agent_tree::read_subagents(&claude_session_id)` — lê `.meta.json`

### Testes

- Testes unitários para cada tool handler com DB in-memory e MockProvider
- Teste de JSON-RPC framing (request → response)

---

## Fase 4 — Refactor `commands/providers.rs`

**Arquivo:** `tauri/src/commands/providers.rs` (modificar)

**Mudança:** extrair a lógica de `get_providers` para uma função pública reutilizável:

```rust
/// Build the full provider list — used by both the Tauri command and the MCP handler.
pub fn build_cli_backends(registry: &ProviderRegistry) -> Vec<CliBackend> {
    // ... lógica atual de get_providers, sem o wrapper #[tauri::command]
}

#[tauri::command]
pub fn get_providers(
    registry: tauri::State<ProviderRegistryState>,
) -> Vec<CliBackend> {
    build_cli_backends(&registry.0)
}
```

Assim `McpHandler::orbit_list_providers` chama `build_cli_backends(registry)` sem duplicação.

---

## Fase 5 — Startup no Tauri (`lib.rs` + `ipc/mod.rs`)

**Arquivo:** `tauri/src/lib.rs` (modificar)

### Mudanças

1. Adicionar `pub mod mcp_transport;` nos módulos do topo
2. No `.setup()`, após criar SessionManager e ProviderRegistry:

```rust
// Refactor: create Arc<ProviderRegistry> first, share with both Tauri state and MCP
let registry = Arc::new(registry);  // ← já é Arc hoje via ProviderRegistryState
let registry_for_mcp = Arc::clone(&registry);
app.manage(ProviderRegistryState(Arc::clone(&registry)));

let session_mgr_for_mcp = Arc::clone(&session_manager); // já é Arc<RwLock<SM>>
let app_handle = app.handle().clone();

// Start embedded MCP server
let mcp_handler = Arc::new(ipc::mcp::McpHandler::new(
    session_mgr_for_mcp,
    registry_for_mcp,
    app_handle,
));
let handler_fn = {
    let h = Arc::clone(&mcp_handler);
    move |request: &str| -> String { h.handle(request) }
};
let _mcp_transport = mcp_transport::McpTransport::start(Arc::new(handler_fn));
```

3. O `McpTransport` é guardado em variável para que o stop flag fique vivo enquanto o app rodar

**Arquivo:** `tauri/src/ipc/mod.rs` (modificar)

Adicionar: `pub mod mcp;`

---

## Fase 6 — Proxy (`mcp_proxy.rs` + `bin/orbit_mcp.rs`)

**Arquivo:** `tauri/src/mcp_proxy.rs` (novo, módulo na lib)
**Arquivo:** `tauri/src/bin/orbit_mcp.rs` (reescrever)

### Estrutura

```
bin/orbit_mcp.rs          (entry point, ~15 linhas)
  └── orbit_lib::mcp_proxy
        ├── run()                    → tenta conectar, despacha modo
        ├── connected_mode(stream)   → byte pipe stdin↔socket
        └── standalone_mode()        → lógica atual do orbit_mcp.rs
```

### `connected_mode`

1. Conectar ao pipe/socket com timeout de 2s:
   - `interprocess::local_socket::Stream::connect()` com endereço adequado por OS
2. Duas threads:
   - **stdin → pipe:** `BufReader<Stdin>::read_line()` → `stream.write_all(line + "\n")`
   - **pipe → stdout:** `BufReader<Stream>::read_line()` → `stdout.write_all(line + "\n")`
3. Se o pipe quebrar (Tauri fechou): `eprintln!("warning: Orbit app disconnected")` e encerrar

### `standalone_mode`

Mover a lógica dos 760 linhas atuais do `orbit_mcp.rs` para um submódulo `mcp_proxy::standalone`. Mudanças mínimas:
- `fn main()` → `pub fn run()`
- Adaptar os novos tools (`orbit_list_providers`, `orbit_list_sessions`, `orbit_get_subagents`) com respostas hardcoded/in-memory
- `orbit_list_providers` retorna lista estática dos providers conhecidos
- `orbit_list_sessions` retorna só sessões in-memory do proxy
- `orbit_get_subagents` retorna `[]` (sem acesso ao filesystem dos sessions)

### `bin/orbit_mcp.rs` (novo conteúdo)

```rust
fn main() {
    orbit_lib::mcp_proxy::run();
}
```

---

## Fase 7 — Verificação e Testes

### Testes automatizados

| Tipo | O que testa | Como |
|------|------------|------|
| Unit: transport | Conexão, framing, multi-client | `LocalSocketStream::connect()` em teste |
| Unit: handler | Cada tool com MockProvider + DB in-memory | `McpHandler::handle(json_request)` |
| Unit: proxy | Connected vs standalone detection | Mock socket listener |
| Integration: `cargo test` | Tudo acima junto | Rodar na CI |

### Teste manual E2E

1. `npm run tauri:dev`
2. Em outro terminal: `echo '{"jsonrpc":"2.0","id":1,"method":"tools/list"}' | orbit-mcp`
3. Verificar que retorna as 7 tools
4. `echo '{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"orbit_list_providers","arguments":{}}}' | orbit-mcp`
5. Verificar que retorna todos os 5 providers com capabilities corretas
6. Criar sessão via MCP e verificar que aparece no dashboard

### Checks obrigatórios (antes de commit)

```bash
npx prettier --check "ui/**/*.{ts,svelte,css}"
npx eslint ui --max-warnings 0
npx svelte-check --fail-on-warnings
cargo clippy --manifest-path tauri/Cargo.toml -- -D warnings
cargo test --manifest-path tauri/Cargo.toml
```

---

## Ordem de Execução

| Passo | O quê | Arquivos | Depende de |
|-------|-------|----------|------------|
| 1 | Dependência IPC | `Cargo.toml` | — |
| 2 | Transport server | `mcp_transport.rs` (novo) | 1 |
| 3 | Refactor providers | `commands/providers.rs` | — |
| 4 | MCP handler | `ipc/mcp.rs` (novo) | 2, 3 |
| 5 | Wiring no Tauri | `lib.rs`, `ipc/mod.rs` | 4 |
| 6 | Proxy + standalone | `mcp_proxy.rs` (novo), `bin/orbit_mcp.rs` (reescrito) | 2 |
| 7 | Testes + lint | todos | 5, 6 |

**Passos 2 e 3 são independentes** — podem ser feitos em paralelo.
**Passos 5 e 6 são independentes** — podem ser feitos em paralelo (um é o server side, outro é o client side).

---

## Resumo de Arquivos

| Arquivo | Ação | Linhas estimadas |
|---------|------|-----------------|
| `tauri/Cargo.toml` | Adicionar `interprocess` | +1 |
| `tauri/src/mcp_transport.rs` | **Novo** — servidor IPC cross-platform | ~150 |
| `tauri/src/ipc/mcp.rs` | **Novo** — handler JSON-RPC + 7 tools | ~500 |
| `tauri/src/ipc/mod.rs` | Adicionar `pub mod mcp;` | +1 |
| `tauri/src/mcp_proxy.rs` | **Novo** — proxy connected + standalone fallback | ~100 (proxy) + ~760 (standalone movido) |
| `tauri/src/lib.rs` | Start MCP transport no setup | +15 |
| `tauri/src/commands/providers.rs` | Extrair `build_cli_backends()` | +5, refactor |
| `tauri/src/bin/orbit_mcp.rs` | **Reescrito** — entry point de 3 linhas | 3 (vs 760 antes) |

**Total novo:** ~770 linhas de código novo (transport + handler + proxy wiring)
**Movido:** ~760 linhas (standalone preservado intacto como fallback)

---

## Invariantes

1. **Zero hardcoded provider strings no MCP handler** — tudo via `Provider` trait / `ProviderRegistry`
2. **Cross-platform transparente** — `interprocess` resolve named pipe vs Unix socket, zero `#[cfg]` no handler
3. **Frontend inalterado** — sessões MCP emitem `session:created`, `session:output`, `session:state` como qualquer outra
4. **Fallback standalone funcional** — se o Tauri não estiver rodando, o `orbit-mcp` continua funcionando como hoje
5. **Thread-safe** — `Arc<RwLock<SessionManager>>` + `Arc<ProviderRegistry>` compartilhados entre Tauri commands e MCP handler

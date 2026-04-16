# Orbit vs Paseo — Análise Comparativa e Melhorias Propostas

Baseado na análise do repositório [getpaseo/paseo](https://github.com/getpaseo/paseo) (3.6k stars, 2.3k commits, AGPL-3.0).

---

## Sumário Executivo

Paseo é um dashboard para orquestrar múltiplos AI coding agents (Claude Code, Codex, OpenCode, Copilot, Gemini CLI, etc.) a partir de qualquer dispositivo (desktop, mobile, web, CLI). Orbit e Paseo resolvem o mesmo problema fundamental, mas com arquiteturas radicalmente diferentes. Esta análise identifica features, patterns e melhorias que Orbit pode adotar.

| Dimensão | Orbit | Paseo |
|-----------|-------|-------|
| Runtime | Tauri 2 (Rust + Svelte) — processo único nativo | Node.js daemon + múltiplos clientes (Electron, Expo, web, CLI) |
| Transporte | Tauri events (IPC in-process) | WebSocket multi-cliente, E2E encrypted relay |
| Persistência | SQLite (rusqlite) | File-based JSON (atomic write + rename, Zod schemas) |
| Providers | 3 (Claude, Codex, OpenCode) com parsing customizado | 5+ (Claude SDK, Codex AppServer, ACP para Copilot/Gemini/OpenCode/Pi) |
| Permissões | `--dangerously-skip-permissions` (bypass total) | Fluxo bidirecional agent → server → client → user |
| Terminal | Não | node-pty + xterm.js (WebGL) |
| Multi-agente | Não (subagentes são read-only) | MCP server + chat rooms + Ralph loops + handoff |
| Mobile/Web | Desktop only | iOS, Android, Web, Desktop, CLI |
| Voz | Não | STT dictation + realtime voice agent (OpenAI Realtime) |
| ACP | Não | Sim — qualquer agente ACP via config, sem código |
| Notificações | Não | Push (Expo), attention system, desktop badges |

---

## 1. PTY Interativo + xterm.js

### O que Paseo faz

Paseo spawna cada agente dentro de um PTY (pseudo-terminal) via `node-pty`. O output bruto (com ANSI codes, cores, progress bars) é transmitido ao frontend via WebSocket e renderizado por `@xterm/xterm` com renderer WebGL. O usuário pode digitar diretamente no terminal, interagir com programas TUI, e responder a pedidos de permissão inline.

### Como Orbit faz hoje

Orbit spawna o CLI como processo filho (`std::process::Command`), captura `stdout`/`stderr` como pipes brutos (não PTY), e parseia cada linha como JSONL. Não há interação stdin — follow-up messages são enviadas via `--resume` como novo processo. O `--dangerously-skip-permissions` bypassa todo o permission flow.

### Stack recomendada para Orbit

| Camada | Tecnologia | Justificativa |
|--------|-----------|---------------|
| **PTY** | `portable-pty` 0.9 (WezTerm) | Único crate Rust multi-plataforma maduro. ConPTY no Windows, Unix PTY no macOS/Linux. Powers WezTerm (25.6k stars) |
| **Terminal emulator** | `@xterm/xterm` 5.5+ com `@xterm/addon-webgl` | Renderer mais rápido com fallback canvas. VS Code usa o mesmo |
| **Data pipeline** | Tauri events (`app.emit`) | Provado por AgentsCommander (Tauri 2 + xterm.js + portable-pty). Latência <1ms |
| **Input** | `invoke("pty_write")` do Svelte | Input humano é low-throughput, overhead insignificativo |

### Fluxo de dados proposto

```
User digita no xterm.js
  → invoke("pty_write", { sessionId, data })
    → Rust escreve no PTY stdin

PTY produz output
  → tokio::spawn_blocking lê do MasterPty
    → app.emit("pty_output", { sessionId, data })
      → xterm.js terminal.write(data)

Resize do terminal
  → invoke("pty_resize", { sessionId, rows, cols })
    → master.resize(PtySize{ rows, cols, pixel_width: 0, pixel_height: 0 })
```

### Por que `portable-pty`?

| Crate | Status | Windows | Veredito |
|-------|--------|---------|----------|
| `portable-pty` 0.9 | Production-grade (WezTerm) | ConPTY nativo via `winapi` | **Usar este** |
| `alacritty_terminal` | Acoplado ao renderer GPU do Alacritty | Sim | Não — traz Grid/Term/Selection que xterm.js já resolve |
| `vte` (sozinho) | Parser VT100/ANSI puro | Sim | Útil se precisar parsear ANSI server-side, mas não é PTY |
| `tokio-pty-process` | Abandonado (Tokio 0.1.x) | Não | Descartar |
| `async-pty` | Inexistente | — | Descartar |

### Coexistência com o Feed atual

O terminal seria uma **aba/mode opcional** por sessão, não um substituto:

- **Feed (journal parseado)** — view principal para output estruturado (tool calls, diffs, tokens, thinking)
- **Terminal** — view para output cru (cores, progress bars, TUI), interação stdin, pedidos de permissão inline

### O que muda

| Aspecto | Antes | Depois |
|---------|-------|-------|
| Permissões | `--dangerously-skip-permissions` (bypass) | Usuário responde inline no terminal |
| Stdin | Não — follow-up via `--resume` como novo processo | Interação direta com o processo em tempo real |
| Programas TUI | Não funciona (requer PTY) | `vim`, `htop`, etc. funcionam |
| Idle detection | Baseado em parsing de journal (best-effort) | Monitorar inatividade no PTY reader (timeout) |
| Output raw | Perdido (só chega parseado) | Disponível no terminal com formatação original |

### Arquitetura Rust proposta

```rust
// Novos módulos
tauri/src/
├── services/
│   └── pty_manager.rs    // PTY session lifecycle, read loop, resize
├── ipc/
│   └── terminal.rs        // Tauri commands: pty_create, pty_write, pty_resize, pty_kill

// PtyManager
struct PtySession {
    master: Box<dyn portable_pty::MasterPty + Send>,
    writer: Box<dyn std::io::Write + Send>,
    child: Box<dyn portable_pty::Child + Send + Sync>,
    read_task: tokio::task::JoinHandle<()>,
}

// PtyManager holds Arc<Mutex<HashMap<SessionId, PtySession>>>

// Comandos Tauri
#[tauri::command]
fn pty_create(session_id: u32, command: String, cwd: String, cols: u16, rows: u16) -> Result<()>

#[tauri::command]
fn pty_write(session_id: u32, data: String) -> Result<()>

#[tauri::command]
fn pty_resize(session_id: u32, cols: u16, rows: u16) -> Result<()>

#[tauri::command]
fn pty_kill(session_id: u32) -> Result<()>
```

### Frontend proposto

```svelte
<!-- ui/components/TerminalPanel.svelte -->
<script>
  import { Terminal } from '@xterm/xterm'
  import { FitAddon } from '@xterm/addon-fit'
  import { WebglAddon } from '@xterm/addon-webgl'
  import { invoke } from '../lib/tauri'
  import { listen } from '@tauri-apps/api/event'
</script>
```

### Dependências npm

```json
{
  "@xterm/xterm": "^5.5",
  "@xterm/addon-webgl": "^0.18",
  "@xterm/addon-fit": "^0.10",
  "@xterm/addon-unicode11": "^0.8"
}
```

### Dependências Rust (Cargo.toml)

```toml
portable-pty = "0.9"
```

### Performance

| Cenário | Data rate | Tauri events aguenta? |
|---------|-----------|----------------------|
| Output normal de agente | ~1 KB/s | Facilmente |
| Build logs (cargo build) | ~10-100 KB/s | Facilmente |
| `cat` arquivo grande | ~1-5 MB/s | Com batching de 16ms, sim |
| VS Code usa o mesmo pattern (xterm.js + IPC JSON) | — | Comprovado em produção |

**Otimização de batching:** Buffer output por até 16ms antes de emitir, combinando múltiplos reads PTY em um evento. Isso reduz overhead de serialização e mantém latência dentro de um frame.

### Riscos

| Risco | Severidade | Mitigação |
|-------|-----------|-----------|
| ConPTY no Windows requer 1809+ | Baixo | Orbit já target 1903+ |
| Throughput de Tauri events | Baixo | Batching (16ms) + WebGL renderer. AgentsCommander prova que funciona |
| `portable-pty` no monorepo do WezTerm | Médio | Crate estável (0.9), fork viável se necessário |
| Race condition PTY vs journal parser | Médio | Operam em canais diferentes (raw bytes vs JSONL parseado), sync cuidadoso |
| WebGL2 indisponível em hardware antigo | Baixo | Fallback automático para canvas renderer |
| Bundle size xterm.js (~200KB) | Baixo | Lazy import — só carregar quando terminal panel abre |

### Projetos de referência

| Projeto | Stack | Relevância |
|---------|-------|------------|
| **AgentsCommander** | Tauri 2 + SolidJS + xterm.js WebGL + portable-pty | **Mais próximo do Orbit** — roda Claude/Codex/OpenCode via PTY |
| **JayarOnde/Tauri_TerminalEmulator** | Tauri + xterm.js + portable-pty | Demo mínimo funcional |
| **freethinkel/fluffy** | Tauri + Svelte + xterm.js | Svelte-based, mesma stack do Orbit |
| VS Code integrated terminal | xterm.js + node-pty | Prova que IPC + xterm.js é viável em produção |

### Esforço estimado

~7-10 dias para MVP funcional em Windows + macOS + Linux.

---

## 2. Agent Client Protocol (ACP)

### O que é

ACP é um protocolo aberto que define como **clientes** (editores, IDEs, dashboards) se comunicam com **AI coding agents** (Claude Code, Copilot, Gemini CLI, etc.). É o **"LSP dos coding agents"** — assim como LSP padronizou integração de linguagens entre editores, ACP padroniza comunicação com agentes.

**Site:** agentclientprotocol.com
**Governança:** Co-governado por Zed Industries + JetBrains (não é só da Anthropic)
**Licença:** Apache 2.0
**Estado:** Protocol version 1 estável, com processo formal de RFD para evolução

### Problema que resolve

Sem ACP, cada ferramenta precisa escrever parsing customizado para stdout de cada agente. É a matriz N×M. Com ACP, é N+M: um protocolo por lado.

| Sem ACP (Orbit hoje) | Com ACP |
|----------------------|---------|
| Parse stdout de cada CLI (3 parsers diferentes, um por provider) | Um protocolo JSON-RPC 2.0 para todos |
| `--dangerously-skip-permissions` | `session/request_permission` bidireacional |
| Scrape session ID do output para `--resume` | `session/load` e `session/list` como métodos first-class |
| Capabilities hardcoded no Provider trait | Negotiated via `initialize` handshake |
| Adicionar provider = novo `.rs` + parser customizado | Adicionar provider = uma linha de config |
| Tool calls parseados de campos proprietários | Structured `tool_call`/`tool_call_update` com kind, status, diffs |

### Formato do protocolo

JSON-RPC 2.0 sobre stdio (NDJSON). O cliente spawna o agente como subprocesso, mensagens vão via stdin/stdout.

```
→ Client:  {"jsonrpc":"2.0","id":0,"method":"initialize","params":{"protocolVersion":1,"clientCapabilities":{"fs":{"readTextFile":true,"writeTextFile":true},"terminal":true},"clientInfo":{"name":"orbit","version":"0.5.0"}}}
← Agent:   {"jsonrpc":"2.0","id":0,"result":{"protocolVersion":1,"agentCapabilities":{"loadSession":true,"promptCapabilities":{"image":true},"sessionCapabilities":{"list":true,"close":true}},"agentInfo":{"name":"gemini-cli","version":"1.0.0"}}}

→ Client:  {"jsonrpc":"2.0","id":1,"method":"session/new","params":{"cwd":"/home/user/project"}}
← Agent:   {"jsonrpc":"2.0","id":1,"result":{"sessionId":"sess_abc123"}}

→ Client:  {"jsonrpc":"2.0","id":2,"method":"session/prompt","params":{"sessionId":"sess_abc123","prompt":[{"type":"text","text":"Fix the bug in auth.rs"}]}}

← Agent:   {"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"sess_abc123","update":{"sessionUpdate":"agent_message_chunk","content":{"type":"text","text":"I'll fix the bug..."}}}}

← Agent:   {"jsonrpc":"2.0","method":"session/update","params":{"sessionId":"sess_abc123","update":{"sessionUpdate":"tool_call","toolCallId":"call_001","title":"Reading file","kind":"read","status":"pending"}}}

← Agent:   {"jsonrpc":"2.0","id":5,"method":"session/request_permission","params":{"sessionId":"sess_abc123","toolCall":{"toolCallId":"call_001"},"options":[{"optionId":"allow","name":"Allow","kind":"allow_once"}]}}

→ Client:  {"jsonrpc":"2.0","id":5,"result":{"outcome":{"outcome":"selected","optionId":"allow"}}}

← Agent:   {"jsonrpc":"2.0","id":2,"result":{"stopReason":"end_turn"}}
```

### Métodos do protocolo

**Cliente → Agente:**

| Método | Propósito | Obrigatório? |
|--------|-----------|-------------|
| `initialize` | Negociar versão, trocar capabilities | Sim |
| `session/new` | Criar nova sessão | Sim |
| `session/prompt` | Enviar prompt (inicia um turn) | Sim |
| `session/load` | Carregar/resumir sessão existente | Opcional |
| `session/set_mode` | Trocar modo do agente | Opcional |
| `session/set_config_option` | Mudar model, thinking level, etc. | Opcional |
| `session/list` | Listar sessões existentes | Opcional |
| `session/close` | Fechar sessão | Opcional |
| `session/cancel` | Cancelar turn em andamento | Notificação |

**Agente → Cliente:**

| Método | Propósito |
|--------|-----------|
| `session/request_permission` | Pedir permissão para executar tool |
| `fs/read_text_file` | Ler arquivo do filesystem do cliente |
| `fs/write_text_file` | Escrever arquivo no filesystem do cliente |
| `terminal/create` | Criar terminal |
| `terminal/output` | Obter output do terminal |
| `terminal/kill` | Matar terminal |

**Notificações (Agente → Cliente, sem resposta):**

| Notificação | O que carrega |
|-------------|--------------|
| `session/update` | **Workhorse** — carrega toda saída de streaming |

**Tipos de session/update:**

| Tipo | Conteúdo |
|------|----------|
| `user_message_chunk` | Mensagem do usuário (streaming) |
| `agent_message_chunk` | Resposta do agente (streaming) |
| `agent_thought_chunk` | Reasoning/thinking (streaming) |
| `tool_call` | Nova invocação de tool |
| `tool_call_update` | Atualização de status/conteúdo de tool |
| `plan` | Plano de execução |
| `current_mode_update` | Agente mudou de modo |
| `config_option_update` | Opções de config mudaram (modelos, thinking) |
| `session_info_update` | Título, timestamp |
| `usage_update` | Estatísticas de tokens |
| `available_commands_update` | Slash commands disponíveis |

### Quem suporta ACP hoje

**30+ agentes:**

| Agente | Suporte ACP | Notas |
|--------|------------|-------|
| Gemini CLI | Nativo | Google |
| GitHub Copilot | `copilot --acp` | Public preview Jan 2026 |
| Cursor | `cursor --acp` | Nativo |
| OpenCode | Nativo | |
| Cline | Nativo | VS Code extension |
| Augment Code | Nativo | |
| Junie (JetBrains) | Nativo | |
| Docker cagent | Nativo | |
| Goose | Nativo | Block |
| Kimi CLI | Nativo | Moonshot AI |
| Kiro CLI | Nativo | AWS |
| Qwen Code | Nativo | Alibaba |
| Mistral Vibe | Nativo | |
| Claude Code | Adapter Zed | Ainda não nativo |
| Codex CLI | Adapter Zed | Ainda não nativo |

**Clientes ACP:** Zed, JetBrains, VS Code (extensão), Neovim (plugins), Emacs, Paseo, Jockey.

**SDKs oficiais:** TypeScript (`@agentclientprotocol/sdk`), **Rust** (`agent-client-protocol` no crates.io), Python, Java, Kotlin.

### Como Paseo implementa

Paseo usa o SDK TypeScript (`@agentclientprotocol/sdk`):

1. **`ACPAgentClient`** — Classe base que faz todo o boilerplate: spawn do subprocesso com `--acp`, stream NDJSON sobre stdio, handshake `initialize`, lifecycle de sessões, streaming via subscriber pattern.

2. **Subclasses são finas** — Cada provider fornece apenas: `defaultCommand`, `defaultModes`, `capabilities`, `isAvailable()`.

3. **`ACPAgentSession`** implementa `AgentSession` E serve como `ACPClient` — lida com `session/request_permission`, todos os 12 tipos de `session/update`, `fs/read_text_file`, `fs/write_text_file`, `terminal/*`, model/mode switching.

4. **Provider customizado via config** — Um `GenericACPAgentClient` permite adicionar qualquer agente ACP especificando apenas o command no config:

```json
{
  "agents": {
    "providers": {
      "my-agent": {
        "extends": "acp",
        "label": "My Agent",
        "command": ["my-agent-cli", "--acp"],
        "models": [{ "id": "model-1", "label": "Model 1" }]
      }
    }
  }
}
```

### Proposta de adoção no Orbit

**Abordagem pragmática: ACP como transport first-class, Provider trait como fallback.**

#### Fase 1: `AcpProvider` que implementa o `Provider` trait existente

```rust
// Novo arquivo
tauri/src/providers/acp.rs

pub struct AcpProvider {
    command: Vec<String>,       // e.g. ["gemini", "--acp"]
    label: String,
    modes: Vec<SessionMode>,
}

impl Provider for AcpProvider {
    fn id(&self) -> &str { /* from config */ }
    fn display_name(&self) -> &str { &self.label }

    fn spawn(&self, config: ProviderSpawnConfig) -> Result<SpawnHandle, String> {
        // 1. Spawn subprocess with self.command
        // 2. Send JSON-RPC initialize over stdin
        // 3. Send session/new
        // 4. Send session/prompt (if initial prompt provided)
        // Return SpawnHandle with JSON-RPC reader/writer
    }

    fn process_line(&self, state: &mut JournalState, line: &str) {
        // Parse JSON-RPC notifications
        // Map session/update types to JournalEntry variants
        // Handle session/request_permission -> emit attention event
    }

    fn supports_ssh(&self) -> bool { true }  // ACP over SSH is pipeable
    // ... remaining trait methods
}
```

#### Fase 2: Client-side ACP (Orbit como ACP Client)

Implementar os métodos que o agente pode chamar no cliente:

- `session/request_permission` → Mostrar diálogo no Orbit, retornar decisão
- `fs/read_text_file` → Ler arquivo do filesystem local
- `fs/write_text_file` → Escrever arquivo (com confirmação)
- `terminal/create` → Criar PTY (via `portable-pty`, integra com seção 1)

Isso transforma Orbit de "leitor passivo de stdout" para **parceiro ativo do protocolo**.

#### Fase 3: Custom providers via config

```json
// config ou DB
{
  "providers": {
    "gemini": {
      "extends": "acp",
      "command": ["gemini", "--acp"],
      "label": "Gemini CLI",
      "models": []
    },
    "copilot": {
      "extends": "acp",
      "command": ["copilot", "--acp"],
      "label": "Copilot",
      "models": []
    },
    "kimi": {
      "extends": "acp",
      "command": ["kimi", "--acp"],
      "label": "Kimi",
      "models": []
    }
  }
}
```

Adicionar um provider ACP = uma entrada de config. Zero código novo.

#### Fase 4: Claude Code via ACP (quando disponível)

Claude Code ainda não fala ACP nativamente. Opções:

1. **Manter `stream-json`** até Anthropic lançar suporte nativo (questão de tempo)
2. **Adapter Zed** (`claude-agent-acp`) como intermediário — funciona mas adiciona latência
3. **Escrever bridge próprio** — wrapper que spawna `claude --output-format stream-json` e traduz para ACP JSON-RPC

### Vantagens da adoção

| Vantagem | Impacto |
|----------|---------|
| 30+ agentes disponíveis sem código novo | Alto |
| Permissões reais (`session/request_permission`) | Crítico para segurança |
| Session lifecycle padronizado | Elimina fragilidade do `--resume` scraping |
| Capability discovery dinâmico | Elimina hardcoded capabilities |
| Future-proof — ecossistema converge para ACP | Estratégico |

### Riscos

| Risco | Severidade | Mitigação |
|-------|-----------|-----------|
| Claude Code sem ACP nativo | Médio | Manter stream-json como fallback, adapter como bridge |
| Perda de campos granulares (miniLog, pendingApproval) | Baixo | Mapear ACP updates para campos existentes |
| SSH + ACP | Médio | Pipe JSON-RPC over SSH — funciona mas não é standardizado |
| Custo de migração | Médio | Incremental — ACP como opção, não substituição imediata |
| SDK Rust (`agent-client-protocol`) menos testado que TS | Baixo | Powers Zed em produção, é robusto |

---

## 3. Sistema de Atenção / Notificação

### O que Paseo faz

Cada agente tem `requiresAttention` + `attentionReason` + `attentionTimestamp`. Quando o agente precisa do usuário (permissão, completou, erro), o attention é setado.

`agent-attention-policy.ts` decide **quem notificar** baseado no estado dos clientes:
- Se algum cliente está focado no agente → não notifica (ele já vê)
- Desktop client sempre recebe notificação
- Cliente stalado → notifica só se não houver cliente ativo melhor
- Push notifications via Expo (iOS/Android) com batching e remoção de tokens inválidos

### Como Orbit faz hoje

Nenhum conceito de "atenção". O usuário precisa verificar cada sessão manualmente. O status `Waiting` existe mas não é propagado para fora da sidebar.

### Proposta para Orbit

1. **Adicionar `attentionState` ao `Session` e `JournalState`**:

```rust
struct AttentionState {
    requires_attention: bool,
    reason: AttentionReason,  // Permission | Completed | Error | RateLimit
    since: DateTime<Utc>,
}
```

2. **Setar attention quando:**
   - Status muda para `Waiting` (permissão pendente) → `Permission`
   - Status muda para `Completed` → `Completed`
   - Status muda para `Error` → `Error`
   - Rate limit detectado → `RateLimit`

3. **Limpar attention quando:**
   - Usuário foca na sessão
   - Usuário responde à permissão
   - Nova atividade no agente (continuou trabalhando)

4. **Frontend:**
   - Badge/ícone na sidebar (círculo colorido por reason)
   - Notificações desktop via `tauri-plugin-notification`
   - Opcional: som quando atenção é requerida

5. **Esforço:** ~2-3 dias

---

## 4. Permissões Reais

### O que Paseo faz

O Agent SDK do Claude tem callback `canUseTool` que intercepta pedidos de permissão e roteia pelo pipeline: agent → server → client → usuário → server → agent. O status do agente muda para `requiresAttention: "permission"`. ACP tem `session/request_permission` como método bidirecional.

### Como Orbit faz hoje

`--dangerously-skip-permissions` em **todos** os providers. Nenhum gate de permissão. `pending_approval` é detectado retroativamente andando o journal de trás pra frente.

### Proposta para Orbit

**Se PTY for implementado (seção 1):** Permissões aparecem inline no terminal, usuário responde diretamente. Simples e efetivo.

**Se ACP for implementado (seção 2):** `session/request_permission` dá o fluxo estruturado — agente pede, Orbit mostra diálogo, usuário decide, resposta volta.

**Sem PTY nem ACP (mínimo viável):** Detectar linhas de permissão no JSONL output e emitir um evento `session:permission-request` com opções. Mostrar diálogo no frontend. Enviar resposta via `--permission-mode-allowed-tools` ou flag similar.

---

## 5. Provider Config Flexível

### O que Paseo faz

**Three-tier command resolution:**
- `default` — Usa o binário padrão do provider
- `append` — Usa o binário padrão mas adiciona args customizados
- `replace` — Substitui o comando inteiro

**Provider profiles:** Múltiplas configs do mesmo provider (ex: Claude pessoal vs trabalho).

**Custom ACP providers:** Qualquer agente ACP adicionado via config JSON sem código.

**Model discovery:** `listModels()` consulta o provider em runtime.

### Como Orbit faz hoje

Cada provider hardcode o binário e os args. CLI discovery via PATH. Para customizar, não há opção — teria que modificar o código ou usar a env var `ORBIT_PROVIDER_COMMAND` (se existisse).

### Proposta para Orbit

1. **ProviderConfig com command_mode:**

```rust
enum CommandMode {
    Default,                              // usa find_cli() normal
    Append { extra_args: Vec<String> },   // adiciona args ao comando padrão
    Replace { command: Vec<String> },      // comando inteiro customizado
}

struct ProviderProfile {
    id: String,           // "claude-work"
    extends: String,      // "claude-code"
    label: String,        // "Claude (Work)"
    command: CommandMode,
    env: HashMap<String, String>,
}
```

2. **Persistir profiles no DB** (tabela `provider_profiles`).

3. **UI para criar/editar profiles** no Settings.

4. **Model discovery:** Para providers ACP, usar `initialize` → `listModels`. Para stream-json, manter lista hardcoded com opção de override manual.

---

## 6. Multi-Agente e Orquestração

### O que Paseo faz

- **MCP Server:** Expõe tools (`create_agent`, `wait_for_agent`, `send_agent_prompt`, `cancel_agent`, etc.) que permitem que qualquer agente crie e gerencie sub-agentes programaticamente.
- **Chat rooms:** Sistema de mensagens com `@mentions` para comunicação inter-agente.
- **Ralph loops:** Worker faz trabalho → Verifier chega → Itera até critério ou max iterations.
- **Handoff:** Transfere contexto de um agente para outro.
- **Orchestration skills:** Equipes com papéis (planner, implementer, reviewer).

### Como Orbit faz hoje

Subagentes são read-only (`.meta.json`). Sem MCP server, sem chat, sem loops.

### Proposta incremental para Orbit

**Fase 1 — Sub-sessões (parent_session_id):**

```sql
ALTER TABLE sessions ADD COLUMN parent_session_id INTEGER REFERENCES sessions(id);
ALTER TABLE sessions ADD COLUMN depth INTEGER DEFAULT 0;
```

- Detectar `ToolCall` com `tool: "Agent"` no reader_loop
- Emitir `session:subagent-created` com dados da sub-sessão
- Criar sessão filha automaticamente
- UI: sub-sessões collapsíveis sob a sessão pai no sidebar

**Fase 2 — MCP Server:**

Implementar MCP server no lado Rust que expõe tools:
- `orbit_create_agent` — Spawna novo agente
- `orbit_send_message` — Envia follow-up
- `orbit_wait_for_agent` — Espera completar
- `orbit_cancel_agent` — Cancela

Isso permite que Claude Code use Orbit como ferramenta via MCP.

**Fase 3 — Orchestration patterns:**

- Handoff: transferir contexto entre sessões
- Loops: worker + verifier com max iterations
- Chat: mensagens simples entre sessões do mesmo projeto

---

## 7. Timeline com Paginação

### O que Paseo faz

Timeline com `seq` + `epoch` — contador monotônico por agente, cursor-based pagination (`direction: tail/before/after`, `limit`). Limite de 200 items em memória. Gap detection quando agente é recarregado.

### Como Orbit faz hoje

Journal é `Vec<JournalEntry>` flat — cresce sem limite. `get_outputs` retorna tudo. Sem paginação. Pode causar problemas de memória em sessões longas.

### Proposta para Orbit

1. **Adicionar `seq` (u32) e `epoch` (String) ao `JournalEntry`** — epoch muda quando sessão é resumida.
2. **Cursor-based pagination no `get_outputs`:**

```rust
fn get_outputs(session_id: u32, cursor: Option<u32>, limit: u32, direction: Direction) -> Vec<JournalEntry>
```

3. **Limite em memória:** Manter últimos N entries (ex: 500). Restante sob demanda do DB.

4. **Esforço:** ~1-2 dias

---

## Priorização Sugerida

| # | Melhoria | Impacto | Esforço | Dependência |
|---|----------|---------|---------|-------------|
| 1 | Sistema de atenção | Alto | 2-3 dias | Nenhuma |
| 2 | PTY + xterm.js | Alto | 7-10 dias | Nenhuma |
| 3 | Permissões reais | Crítico | 3-5 dias | PTY ou ACP |
| 4 | ACP support | Alto | 5-7 dias | Nenhuma |
| 5 | Provider config flexível | Médio | 3-4 dias | Nenhuma |
| 6 | Timeline pagination | Médio | 1-2 dias | Nenhuma |
| 7 | Sub-sessões (multi-agente) | Alto | 5-7 dias | Nenhuma |
| 8 | MCP server | Alto | 5-7 dias | Sub-sessões |
| 9 | Orchestration patterns | Alto | 10-15 dias | MCP server |

**Ordem recomendada:** 1 → 6 → 2 → 4 → 5 → 3 → 7 → 8 → 9

Sistema de atenção e timeline pagination são quick wins. PTY e ACP são as mudanças arquiteturais de maior impacto. O resto constrói em cima dessas fundações.
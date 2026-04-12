# Multi-Provider — Spec

## Objetivo

Suportar 3 CLI backends no Orbit: Claude Code, OpenCode (qualquer provider do models.json), e Codex (modelos OpenAI). O NewSessionModal lê providers/modelos dinamicamente e despacha para o CLI correto.

## CLI Backends

| Provider | CLI | Output flag | Model flag | Permissions flag | Resume |
|---|---|---|---|---|---|
| `claude-code` | `claude` | `--output-format stream-json --verbose` | `--model <alias>` | `--dangerously-skip-permissions` | `--resume <session_id>` |
| opencode/* | `opencode` | `run --format json` | `-m provider/model` | `--dangerously-skip-permissions` | `--continue -s <session_id>` |
| `codex` | `codex` | `exec --json` | `-m <model>` | `--dangerously-bypass-approvals-and-sandbox` | `exec resume --last` |

## JSONL Formats

### Claude (existente)
```json
{"type":"assistant","message":{"content":[{"type":"text","text":"..."}],"usage":{"input_tokens":N,...}}}
{"type":"assistant","message":{"content":[{"type":"tool_use","name":"Bash","input":{"command":"..."}}]}}
{"type":"user","message":{"content":[{"type":"tool_result","content":"..."}]}}
{"type":"result","subtype":"success","stop_reason":"end_turn"}
```

### OpenCode
```json
{"type":"step_start","sessionID":"...","part":{"type":"step-start"}}
{"type":"text","part":{"type":"text","text":"hello"}}
{"type":"tool_use","part":{"type":"tool","tool":"bash","state":{"input":{"command":"..."},"output":"...","metadata":{"exit":0}}}}
{"type":"step_finish","part":{"type":"step-finish","reason":"stop","tokens":{"input":N,"output":N,"cache":{"write":N,"read":N}},"cost":N}}
{"type":"error","error":{"name":"APIError","data":{"message":"..."}}}
```

### Codex
```json
{"type":"thread.started","thread_id":"..."}
{"type":"turn.started"}
{"type":"item.completed","item":{"type":"agent_message","text":"..."}}
{"type":"item.started","item":{"type":"command_execution","command":"...","status":"in_progress"}}
{"type":"item.completed","item":{"type":"command_execution","command":"...","aggregated_output":"...","exit_code":0,"status":"completed"}}
{"type":"turn.completed","usage":{"input_tokens":N,"output_tokens":N,"cached_input_tokens":N}}
```

## Provider Selector (NewSessionModal)

### Favoritos (sempre no topo)
- Claude Code (se `claude` no PATH)
- Codex (se `codex` no PATH)
- OpenRouter
- Anthropic
- OpenAI
- Google
- DeepSeek

### Busca
Campo de texto filtra os ~100 providers do `~/.cache/opencode/models.json`.

### Indicadores visuais
- `✓` verde: CLI instalado + key configurada
- `!` amarelo: CLI instalado, key não configurada
- Cinza/desabilitado: CLI não instalado (para claude-code/codex)

### Modelos
Ao selecionar provider, model dropdown popula com os modelos do `models.json`. Para Claude Code, mantém a lista atual (auto, sonnet, opus, haiku). Para Codex, lista hardcoded (o3, o4-mini, gpt-4.1, codex-mini).

## API Key Flow

1. Lê campo `env` do provider no models.json (ex: `["OPENROUTER_API_KEY"]`)
2. Checa se env var existe no ambiente
3. Se sim: mostra "(configured via OPENROUTER_API_KEY)", campo para override
4. Se não: mostra campo vazio com placeholder
5. Override passado como env var no processo filho — nunca persiste no DB
6. Claude Code e Codex usam suas próprias autenticações — não pedem key

## Diagnose
Só aparece para `claude-code`. Removido para outros providers.

## Dispatch (do_spawn)

```rust
match provider {
    "claude-code" => spawn_claude(),
    "codex" => spawn_codex(),
    _ => spawn_opencode(),  // qualquer provider do opencode
}
```

## IPC Commands novos

### `get_providers`
- Lê `~/.cache/opencode/models.json`
- Detecta quais CLIs estão instalados (claude, opencode, codex)
- Retorna: `Vec<ProviderInfo>` com id, name, models[], env[], configured, cli_available

### `check_env_var`
- Recebe nome da env var, retorna se existe (sem expor o valor)

## O que sai
- `spawn_openrouter` e todo HTTP SSE direto
- Dependências `reqwest`, `futures-util`, `os_pipe` do Cargo.toml
- Lista hardcoded de modelos OpenRouter no NewSessionModal
- `OpenRouterConfig`, `OpenRouterHandle` de spawn_manager.rs
- `do_spawn_openrouter`, `build_openrouter_messages`, `reader_loop_stream` de session_manager.rs

## O que entra
- `spawn_opencode()` e `spawn_codex()` em spawn_manager.rs
- `find_opencode()` e `find_codex()` análogos a `find_claude()`
- `process_line_opencode()` e `process_line_codex()` — adapters JSONL → JournalEntry
- IPC `get_providers` e `check_env_var`
- NewSessionModal redesenhado com provider selector dinâmico

## Critérios de Aceitação
- [ ] Criar sessão com qualquer provider do opencode (ex: opencode/big-pickle)
- [ ] Criar sessão com Codex (ex: o3)
- [ ] Sessão Claude Code continua funcionando como antes
- [ ] Resposta streama no feed em tempo real para os 3 backends
- [ ] Tokens atualizam para os 3 backends
- [ ] Follow-up messages funcionam para os 3 backends
- [ ] Provider/model seletor popula dinamicamente do models.json
- [ ] API key lida de env var + permite override
- [ ] Provider exibido na sidebar/MetaPanel
